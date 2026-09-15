//! Terrarium: перенесення груп модів і конфігів між клієнтською та серверною
//! збірками (обидві — локальні примірники адміна). Нічого не видаляється в
//! цільовому примірнику, крім старої копії того самого jar-а в іншій групі.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::instances::commands;
use crate::util::io;

/// Папки з конфігами, які можна переносити. `config/` показуємо по записах
/// верхнього рівня (щоб обирати окремі моди), решту — цілком.
const CONFIG_ROOTS: &[&str] =
    &["config", "defaultconfigs", "kubejs", "scripts"];
/// Понад цей розмір конфіги не хешуємо — порівнюємо за розміром.
const HASH_LIMIT: u64 = 4 << 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncKind {
    /// Група модів (`key` — назва; порожній `key` — моди без групи).
    ModGroup,
    /// Конфіги (`key` — шлях відносно примірника, напр. `config/jei`).
    Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEntry {
    pub kind: SyncKind,
    pub key: String,
    pub name: String,
    pub files: usize,
    /// Файлів, яких у цілі немає.
    pub new_files: usize,
    /// Файлів, які в цілі відрізняються.
    pub changed_files: usize,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPreview {
    pub groups: Vec<SyncEntry>,
    pub configs: Vec<SyncEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub source_instance_id: String,
    pub target_instance_id: String,
    /// Ключі груп (`""` — без групи).
    #[serde(default)]
    pub groups: Vec<String>,
    /// Ключі конфігів (`config/jei`, `kubejs`).
    #[serde(default)]
    pub configs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub copied: usize,
    pub skipped_same: usize,
}

struct FileInfo {
    path: PathBuf,
    size: u64,
}

fn collect_files(
    dir: &Path,
    out: &mut BTreeMap<String, FileInfo>,
    prefix: &str,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let rel = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        if path.is_dir() {
            collect_files(&path, out, &rel);
        } else if let Ok(meta) = entry.metadata() {
            out.insert(
                rel,
                FileInfo {
                    path,
                    size: meta.len(),
                },
            );
        }
    }
}

/// Файли лише верхнього рівня папки (моди в групі — без підпапок).
fn collect_flat(dir: &Path, out: &mut BTreeMap<String, FileInfo>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if name.starts_with('.') || !path.is_file() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            out.insert(
                name,
                FileInfo {
                    path,
                    size: meta.len(),
                },
            );
        }
    }
}

/// Чи однакові файли: за розміром, а для невеликих — і за вмістом.
fn same_file(a: &FileInfo, b: &FileInfo) -> bool {
    if a.size != b.size {
        return false;
    }
    if a.size > HASH_LIMIT {
        return true;
    }
    match (
        super::terrarium::sha1_of(&a.path),
        super::terrarium::sha1_of(&b.path),
    ) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

fn diff_entry(
    kind: SyncKind,
    key: String,
    name: String,
    source: &BTreeMap<String, FileInfo>,
    target: &BTreeMap<String, FileInfo>,
) -> SyncEntry {
    let mut new_files = 0;
    let mut changed_files = 0;
    let mut size = 0;
    for (rel, info) in source {
        size += info.size;
        match target.get(rel) {
            None => new_files += 1,
            Some(other) if !same_file(info, other) => changed_files += 1,
            _ => {}
        }
    }
    SyncEntry {
        kind,
        key,
        name,
        files: source.len(),
        new_files,
        changed_files,
        size,
    }
}

/// Усі моди цілі за ім'ям файлу → де лежать (корінь або будь-яка група).
fn target_mods_by_name(mods_dir: &Path) -> BTreeMap<String, FileInfo> {
    let mut all = BTreeMap::new();
    collect_flat(mods_dir, &mut all);
    if let Ok(entries) = std::fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_string)
            else {
                continue;
            };
            if path.is_dir() && commands::is_group_dir_name(&name) {
                collect_flat(&path, &mut all);
            }
        }
    }
    all
}

fn mod_groups_of(mods_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut groups = Vec::new();
    if let Ok(entries) = std::fs::read_dir(mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_string)
            else {
                continue;
            };
            if path.is_dir() && commands::is_group_dir_name(&name) {
                groups.push((name, path));
            }
        }
    }
    groups.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    groups
}

/// Записи конфігів джерела: `config/<запис>` окремо, інші корені — цілком.
fn config_entries(instance_dir: &Path) -> Vec<(String, PathBuf)> {
    let mut entries = Vec::new();
    for root in CONFIG_ROOTS {
        let dir = instance_dir.join(root);
        if !dir.exists() {
            continue;
        }
        if *root == "config" {
            if let Ok(read) = std::fs::read_dir(&dir) {
                let mut names: Vec<(String, PathBuf)> = read
                    .flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_str()?.to_string();
                        (!name.starts_with('.'))
                            .then(|| (format!("config/{name}"), e.path()))
                    })
                    .collect();
                names.sort_by(|a, b| {
                    a.0.to_lowercase().cmp(&b.0.to_lowercase())
                });
                entries.extend(names);
            }
        } else {
            entries.push((root.to_string(), dir));
        }
    }
    entries
}

fn files_of(path: &Path) -> BTreeMap<String, FileInfo> {
    let mut out = BTreeMap::new();
    if path.is_dir() {
        collect_files(path, &mut out, "");
    } else if let Ok(meta) = std::fs::metadata(path) {
        out.insert(
            String::new(),
            FileInfo {
                path: path.to_path_buf(),
                size: meta.len(),
            },
        );
    }
    out
}

async fn instance_dirs(
    source: &str,
    target: &str,
) -> crate::Result<(PathBuf, PathBuf)> {
    if source == target {
        return Err(crate::ErrorKind::InputError(
            "Джерело і ціль — той самий примірник".to_string(),
        )
        .into());
    }
    let source_dir = crate::api::instance::get_full_path(source).await?;
    let target_dir = crate::api::instance::get_full_path(target).await?;
    for dir in [&source_dir, &target_dir] {
        if commands::flatten_reason(dir).is_some() {
            return Err(crate::ErrorKind::OtherError(
                "Закрий гру (або дочекайся оновлення збірки) перед перенесенням"
                    .to_string(),
            )
            .into());
        }
    }
    Ok((source_dir, target_dir))
}

/// Що можна перенести з джерела в ціль і скільки з того нове/змінене.
#[tracing::instrument]
pub async fn sync_preview(
    source_instance_id: String,
    target_instance_id: String,
) -> crate::Result<SyncPreview> {
    let (source_dir, target_dir) =
        instance_dirs(&source_instance_id, &target_instance_id).await?;
    tokio::task::spawn_blocking(move || {
        let source_mods = source_dir.join(commands::MODS_FOLDER);
        let target_mods = target_dir.join(commands::MODS_FOLDER);
        let target_all = target_mods_by_name(&target_mods);

        let mut groups = Vec::new();
        for (name, path) in mod_groups_of(&source_mods) {
            let mut files = BTreeMap::new();
            collect_flat(&path, &mut files);
            if files.is_empty() {
                continue;
            }
            groups.push(diff_entry(
                SyncKind::ModGroup,
                name.clone(),
                name,
                &files,
                &target_all,
            ));
        }
        let mut root = BTreeMap::new();
        collect_flat(&source_mods, &mut root);
        root.retain(|name, _| name != commands::README_FILE);
        if !root.is_empty() {
            groups.push(diff_entry(
                SyncKind::ModGroup,
                String::new(),
                String::new(),
                &root,
                &target_all,
            ));
        }

        let mut configs = Vec::new();
        for (key, path) in config_entries(&source_dir) {
            let files = files_of(&path);
            if files.is_empty() {
                continue;
            }
            let target_files = files_of(&target_dir.join(&key));
            configs.push(diff_entry(
                SyncKind::Config,
                key.clone(),
                key,
                &files,
                &target_files,
            ));
        }
        Ok(SyncPreview { groups, configs })
    })
    .await?
}

fn copy_file(from: &Path, to: &Path) -> crate::Result<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| io::IOError::with_path(e, parent))?;
    }
    std::fs::copy(from, to).map_err(|e| io::IOError::with_path(e, to))?;
    Ok(())
}

/// Переносить обрані групи й конфіги. Однакові файли пропускає; jar із тим
/// самим ім'ям в іншій групі цілі прибирає, щоб гра не вантажила два.
#[tracing::instrument]
pub async fn sync_apply(request: SyncRequest) -> crate::Result<SyncResult> {
    let state = State::get().await?;
    let (source_dir, target_dir) =
        instance_dirs(&request.source_instance_id, &request.target_instance_id)
            .await?;
    let target_id = request.target_instance_id.clone();
    let result =
        tokio::task::spawn_blocking(move || -> crate::Result<SyncResult> {
            let source_mods = source_dir.join(commands::MODS_FOLDER);
            let target_mods = target_dir.join(commands::MODS_FOLDER);
            let mut copied = 0;
            let mut skipped_same = 0;

            for group in &request.groups {
                let (from, to) = if group.is_empty() {
                    (source_mods.clone(), target_mods.clone())
                } else {
                    (source_mods.join(group), target_mods.join(group))
                };
                let mut files = BTreeMap::new();
                collect_flat(&from, &mut files);
                files.retain(|name, _| name != commands::README_FILE);
                let target_all = target_mods_by_name(&target_mods);
                for (name, info) in &files {
                    let dest = to.join(name);
                    if let Some(existing) = target_all.get(name) {
                        if existing.path == dest && same_file(info, existing) {
                            skipped_same += 1;
                            continue;
                        }
                        if existing.path != dest {
                            let _ = std::fs::remove_file(&existing.path);
                        }
                    }
                    copy_file(&info.path, &dest)?;
                    copied += 1;
                }
            }

            for key in &request.configs {
                let from = source_dir.join(key);
                let to = target_dir.join(key);
                let files = files_of(&from);
                let target_files = files_of(&to);
                for (rel, info) in &files {
                    if let Some(existing) = target_files.get(rel)
                        && same_file(info, existing)
                    {
                        skipped_same += 1;
                        continue;
                    }
                    let dest = if rel.is_empty() {
                        to.clone()
                    } else {
                        to.join(rel)
                    };
                    copy_file(&info.path, &dest)?;
                    copied += 1;
                }
            }
            Ok(SyncResult {
                copied,
                skipped_same,
            })
        })
        .await??;

    // Ціль отримала нові файли — оновлюємо її список умісту в БД
    crate::state::sync_content_files(&target_id, &state).await?;
    Ok(result)
}
