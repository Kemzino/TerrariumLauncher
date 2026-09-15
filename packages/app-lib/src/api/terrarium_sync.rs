//! Terrarium: перенесення модів і конфігів між клієнтською та серверною
//! збірками (обидві — локальні примірники адміна). Перегляд — пофайлово, з
//! обох боків; нічого не видаляється в цілі, лише замінюються файли з тим
//! самим іменем.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::instances::commands;
use crate::util::io;

/// Папки з конфігами, які можна переносити. `config/` розбиваємо по записах
/// верхнього рівня (щоб секції відповідали модам), решту — цілком.
const CONFIG_ROOTS: &[&str] =
    &["config", "defaultconfigs", "kubejs", "scripts"];
/// Понад цей розмір файли не хешуємо — порівнюємо за розміром.
const HASH_LIMIT: u64 = 4 << 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncKind {
    /// Мод; секція — група (`key` секції порожній — без групи).
    Mod,
    /// Конфіг; секція — `config/<запис>` або корінь (`kubejs`).
    Config,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Same,
    Differs,
    ClientOnly,
    ServerOnly,
}

/// Один файл у порівнянні. `key` — чим ідентифікуємо файл: для модів ім'я
/// jar-а (група може відрізнятися з боків), для конфігів шлях від примірника.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFile {
    pub kind: SyncKind,
    pub key: String,
    pub name: String,
    pub client_size: Option<u64>,
    pub server_size: Option<u64>,
    /// Група мода з кожного боку (для конфігів — None).
    pub client_group: Option<String>,
    pub server_group: Option<String>,
    pub status: SyncStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSection {
    pub kind: SyncKind,
    pub key: String,
    pub name: String,
    pub files: Vec<SyncFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPreview {
    pub sections: Vec<SyncSection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRequest {
    pub client_instance_id: String,
    pub server_instance_id: String,
    /// Ключі файлів (`SyncFile.key`), які скопіювати з клієнта на сервер.
    #[serde(default)]
    pub to_server: Vec<String>,
    /// … і з сервера на клієнт.
    #[serde(default)]
    pub to_client: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub copied: usize,
}

#[derive(Clone)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    /// Група (лише для модів).
    group: Option<String>,
}

/// Знімок одного примірника: моди за ім'ям jar-а, конфіги за шляхом.
struct Snapshot {
    mods: BTreeMap<String, FileInfo>,
    configs: BTreeMap<String, FileInfo>,
}

fn walk(dir: &Path, prefix: &str, out: &mut BTreeMap<String, FileInfo>) {
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
        let rel = format!("{prefix}/{name}");
        if path.is_dir() {
            walk(&path, &rel, out);
        } else if let Ok(meta) = entry.metadata() {
            out.insert(
                rel,
                FileInfo {
                    path,
                    size: meta.len(),
                    group: None,
                },
            );
        }
    }
}

fn mods_in(
    dir: &Path,
    group: Option<&str>,
    out: &mut BTreeMap<String, FileInfo>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if name.starts_with('.')
            || name == commands::README_FILE
            || !path.is_file()
        {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            out.insert(
                name,
                FileInfo {
                    path,
                    size: meta.len(),
                    group: group.map(str::to_string),
                },
            );
        }
    }
}

fn snapshot(instance_dir: &Path) -> Snapshot {
    let mods_dir = instance_dir.join(commands::MODS_FOLDER);
    let mut mods = BTreeMap::new();
    mods_in(&mods_dir, None, &mut mods);
    if let Ok(entries) = std::fs::read_dir(&mods_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = entry.file_name().to_str().map(str::to_string)
            else {
                continue;
            };
            if path.is_dir() && commands::is_group_dir_name(&name) {
                mods_in(&path, Some(&name), &mut mods);
            }
        }
    }
    let mut configs = BTreeMap::new();
    for root in CONFIG_ROOTS {
        let dir = instance_dir.join(root);
        if dir.is_dir() {
            walk(&dir, root, &mut configs);
        }
    }
    Snapshot { mods, configs }
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

fn status_of(
    client: Option<&FileInfo>,
    server: Option<&FileInfo>,
) -> SyncStatus {
    match (client, server) {
        (Some(c), Some(s)) if same_file(c, s) => SyncStatus::Same,
        (Some(_), Some(_)) => SyncStatus::Differs,
        (Some(_), None) => SyncStatus::ClientOnly,
        _ => SyncStatus::ServerOnly,
    }
}

/// Секція конфігу: `config/jei/x.json` → `config/jei`; `kubejs/a/b.js` → `kubejs`.
fn config_section(rel: &str) -> String {
    let mut parts = rel.split('/');
    let root = parts.next().unwrap_or_default();
    if root == "config" {
        match parts.next() {
            Some(entry) => format!("config/{entry}"),
            None => root.to_string(),
        }
    } else {
        root.to_string()
    }
}

async fn instance_dirs(
    client: &str,
    server: &str,
) -> crate::Result<(PathBuf, PathBuf)> {
    if client == server {
        return Err(crate::ErrorKind::InputError(
            "Клієнтська й серверна збірка — той самий примірник".to_string(),
        )
        .into());
    }
    let client_dir = crate::api::instance::get_full_path(client).await?;
    let server_dir = crate::api::instance::get_full_path(server).await?;
    for dir in [&client_dir, &server_dir] {
        if commands::flatten_reason(dir).is_some() {
            return Err(crate::ErrorKind::OtherError(
                "Закрий гру (або дочекайся оновлення збірки) перед перенесенням"
                    .to_string(),
            )
            .into());
        }
    }
    Ok((client_dir, server_dir))
}

/// Пофайлове порівняння двох збірок, згруповане по групах модів і конфігах.
#[tracing::instrument]
pub async fn sync_preview(
    client_instance_id: String,
    server_instance_id: String,
) -> crate::Result<SyncPreview> {
    let (client_dir, server_dir) =
        instance_dirs(&client_instance_id, &server_instance_id).await?;
    tokio::task::spawn_blocking(move || {
        let client = snapshot(&client_dir);
        let server = snapshot(&server_dir);

        // Моди: секція — група з боку клієнта, інакше з боку сервера
        let mut mod_sections: BTreeMap<String, Vec<SyncFile>> = BTreeMap::new();
        let names: BTreeSet<&String> =
            client.mods.keys().chain(server.mods.keys()).collect();
        for name in names {
            let c = client.mods.get(name);
            let s = server.mods.get(name);
            let group = c
                .and_then(|f| f.group.clone())
                .or_else(|| s.and_then(|f| f.group.clone()))
                .unwrap_or_default();
            mod_sections.entry(group).or_default().push(SyncFile {
                kind: SyncKind::Mod,
                key: name.clone(),
                name: name.clone(),
                client_size: c.map(|f| f.size),
                server_size: s.map(|f| f.size),
                client_group: c.and_then(|f| f.group.clone()),
                server_group: s.and_then(|f| f.group.clone()),
                status: status_of(c, s),
            });
        }

        let mut config_sections: BTreeMap<String, Vec<SyncFile>> =
            BTreeMap::new();
        let paths: BTreeSet<&String> =
            client.configs.keys().chain(server.configs.keys()).collect();
        for rel in paths {
            let c = client.configs.get(rel);
            let s = server.configs.get(rel);
            let section = config_section(rel);
            let name = rel
                .strip_prefix(&format!("{section}/"))
                .unwrap_or(rel)
                .to_string();
            config_sections.entry(section).or_default().push(SyncFile {
                kind: SyncKind::Config,
                key: rel.clone(),
                name,
                client_size: c.map(|f| f.size),
                server_size: s.map(|f| f.size),
                client_group: None,
                server_group: None,
                status: status_of(c, s),
            });
        }

        let mut sections = Vec::new();
        // Без групи — першою, далі групи за абеткою без регістру
        let mut groups: Vec<(String, Vec<SyncFile>)> =
            mod_sections.into_iter().collect();
        groups.sort_by(|a, b| {
            a.0.is_empty()
                .cmp(&b.0.is_empty())
                .reverse()
                .then_with(|| a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
        for (key, files) in groups {
            sections.push(SyncSection {
                kind: SyncKind::Mod,
                name: key.clone(),
                key,
                files,
            });
        }
        let mut configs: Vec<(String, Vec<SyncFile>)> =
            config_sections.into_iter().collect();
        configs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        for (key, files) in configs {
            sections.push(SyncSection {
                kind: SyncKind::Config,
                name: key.clone(),
                key,
                files,
            });
        }
        Ok(SyncPreview { sections })
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

/// Копіює обрані файли з одного знімка в інший примірник. Мод, який у цілі
/// вже є (хай і в іншій групі), замінюється на місці; новий — кладеться в ту
/// ж групу, що й у джерелі.
fn copy_selected(
    keys: &[String],
    from: &Snapshot,
    to: &Snapshot,
    to_dir: &Path,
) -> crate::Result<usize> {
    let mut copied = 0;
    for key in keys {
        if let Some(info) = from.mods.get(key) {
            let dest = match to.mods.get(key) {
                Some(existing) => existing.path.clone(),
                None => to_dir
                    .join(commands::grouped_path(info.group.as_deref(), key)),
            };
            copy_file(&info.path, &dest)?;
            copied += 1;
        } else if let Some(info) = from.configs.get(key) {
            copy_file(&info.path, &to_dir.join(key))?;
            copied += 1;
        }
    }
    Ok(copied)
}

#[tracing::instrument]
pub async fn sync_apply(request: SyncRequest) -> crate::Result<SyncResult> {
    let state = State::get().await?;
    let (client_dir, server_dir) =
        instance_dirs(&request.client_instance_id, &request.server_instance_id)
            .await?;
    let client_id = request.client_instance_id.clone();
    let server_id = request.server_instance_id.clone();
    let touched_server = !request.to_server.is_empty();
    let touched_client = !request.to_client.is_empty();
    let result =
        tokio::task::spawn_blocking(move || -> crate::Result<SyncResult> {
            let client = snapshot(&client_dir);
            let server = snapshot(&server_dir);
            let mut copied = 0;
            copied += copy_selected(
                &request.to_server,
                &client,
                &server,
                &server_dir,
            )?;
            copied += copy_selected(
                &request.to_client,
                &server,
                &client,
                &client_dir,
            )?;
            Ok(SyncResult { copied })
        })
        .await??;

    // Ціль отримала нові файли — оновлюємо її список умісту в БД
    if touched_server {
        crate::state::sync_content_files(&server_id, &state).await?;
    }
    if touched_client {
        crate::state::sync_content_files(&client_id, &state).await?;
    }
    Ok(result)
}
