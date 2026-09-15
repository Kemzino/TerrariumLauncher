//! Terrarium: групи модів як справжні підпапки `mods/<Група>/`.
//!
//! Структура лежить на диску (щоб її бачив і той, хто просто скопіює папку в
//! інший лаунчер — там же лежить README.txt із поясненням), але завантажувачі
//! Minecraft підпапок не читають. Тому на час гри jar-и переносяться в корінь
//! `mods/`, карта «файл → група» пишеться в `mods/.terrarium-flatten.json`, а
//! після виходу з гри все повертається на місця. Поки карта існує, сканер
//! показує файли з кореня під їхніми груповими шляхами — БД не смикається.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::instances::commands::{
    instance_full_path, resolve_content_scope,
};
use crate::state::process::instance_has_running_process;
use crate::util::io::{self, IOError};

pub const MODS_FOLDER: &str = "mods";
pub const FLATTEN_FILE: &str = ".terrarium-flatten.json";
pub const README_FILE: &str = "README.txt";
/// Групи з пакета збірки (`overrides/.terrarium/groups.json`): файл → група.
pub const PACK_GROUPS_FILE: &str = ".terrarium/groups.json";

const MAX_GROUP_NAME: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlattenReason {
    Launch,
    PackUpdate,
}

/// Карта розкладеного стану: ім'я файлу в корені → група, звідки він узятий.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlattenMap {
    pub reason: Option<FlattenReason>,
    #[serde(default)]
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGroup {
    pub name: String,
    pub files: usize,
}

/// `mods/Група/x.jar` → `Some("Група")`; `mods/x.jar` → `None`.
pub fn group_of(relative_path: &str) -> Option<&str> {
    let mut parts = relative_path.split('/');
    if parts.next()? != MODS_FOLDER {
        return None;
    }
    let group = parts.next()?;
    parts.next()?; // є третій сегмент — отже, другий був папкою
    Some(group)
}

/// `mods/Група/x.jar` → `mods/x.jar` (шлях, яким файл бачить гра та .mrpack).
pub fn flat_path(relative_path: &str) -> String {
    match group_of(relative_path) {
        Some(group) => relative_path.replacen(&format!("/{group}/"), "/", 1),
        None => relative_path.to_string(),
    }
}

pub fn grouped_path(group: Option<&str>, file_name: &str) -> String {
    match group {
        Some(group) => format!("{MODS_FOLDER}/{group}/{file_name}"),
        None => format!("{MODS_FOLDER}/{file_name}"),
    }
}

/// Папки, які сканер вважає групою: не приховані й не службові.
pub fn is_group_dir_name(name: &str) -> bool {
    !name.starts_with('.') && name != "__MACOSX"
}

fn validate_group_name(name: &str) -> crate::Result<String> {
    let name = name.trim();
    let bad = name.is_empty()
        || name.len() > MAX_GROUP_NAME
        || name.starts_with('.')
        || name.ends_with('.')
        || name.chars().any(|c| {
            matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
                || c.is_control()
        })
        || crate::state::ProjectType::iterator()
            .any(|t| t.get_folder().eq_ignore_ascii_case(name));
    if bad {
        return Err(crate::ErrorKind::InputError(format!(
            "Неприпустима назва групи: «{name}»"
        ))
        .into());
    }
    Ok(name.to_string())
}

fn flatten_file_path(mods_dir: &Path) -> PathBuf {
    mods_dir.join(FLATTEN_FILE)
}

pub(crate) fn read_flatten_map_sync(mods_dir: &Path) -> Option<FlattenMap> {
    let raw = std::fs::read(flatten_file_path(mods_dir)).ok()?;
    serde_json::from_slice(&raw).ok()
}

async fn write_flatten_map(
    mods_dir: &Path,
    map: &FlattenMap,
) -> crate::Result<()> {
    if map.files.is_empty() {
        match io::remove_file(flatten_file_path(mods_dir)).await {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        return Ok(());
    }
    io::write(flatten_file_path(mods_dir), serde_json::to_vec_pretty(map)?)
        .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// README

const README_TEXT: &str = "\
Terrarium Launcher — групи модів / mod groups
=============================================

Підпапки в цій теці (наприклад mods/Оптимізація/) — це групи модів, створені в
Terrarium Launcher для зручності. Minecraft (NeoForge/Forge/Fabric) НЕ читає моди
з підпапок: на час гри лаунчер сам переносить jar-файли в корінь mods/ і повертає
їх назад після виходу з гри.

Якщо копіюєш цю теку в інший лаунчер — перенеси всі .jar з підпапок прямо в mods/:
  Windows (PowerShell, у теці mods):
    Get-ChildItem -Directory | Get-ChildItem -Filter *.jar | Move-Item -Destination .
  Linux/macOS (у теці mods):
    find . -mindepth 2 -name '*.jar' -exec mv -t . {} +

Файли .disabled — вимкнені моди, їх копіювати не треба.
Службовий файл .terrarium-flatten.json існує лише поки гра запущена — не чіпай.

---

Subfolders here are mod groups made by Terrarium Launcher. Minecraft mod loaders
do not load mods from subfolders, so the launcher moves the jars into mods/ while
the game runs and back afterwards. Copying this folder to another launcher?
Move every .jar from the subfolders straight into mods/ (commands above).
";

async fn ensure_readme(mods_dir: &Path) -> crate::Result<()> {
    let path = mods_dir.join(README_FILE);
    if !path.exists() {
        io::write(path, README_TEXT.as_bytes()).await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Читання структури

/// Групи примірника: підпапки `mods/` (і групи з карти розкладеного стану).
pub(crate) async fn list_mod_groups(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<ContentGroup>> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    let mut groups: BTreeMap<String, usize> = BTreeMap::new();
    if mods_dir.is_dir() {
        let mut dir = io::read_dir(&mods_dir).await?;
        while let Some(entry) = dir.next_entry().await.map_err(IOError::from)? {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            if !is_group_dir_name(&name)
                || !entry.path().is_dir()
            {
                continue;
            }
            let count = std::fs::read_dir(entry.path())
                .map(|d| d.flatten().filter(|e| e.path().is_file()).count())
                .unwrap_or(0);
            groups.insert(name, count);
        }
        if let Some(map) = read_flatten_map_sync(&mods_dir) {
            for group in map.files.values() {
                *groups.entry(group.clone()).or_default() += 1;
            }
        }
    }
    Ok(groups
        .into_iter()
        .map(|(name, files)| ContentGroup { name, files })
        .collect())
}

// ---------------------------------------------------------------------------
// Керування групами

pub(crate) async fn create_mod_group(
    instance_id: &str,
    name: &str,
    state: &State,
) -> crate::Result<String> {
    let name = validate_group_name(name)?;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    io::create_dir_all(mods_dir.join(&name)).await?;
    ensure_readme(&mods_dir).await?;
    Ok(name)
}

/// Перемістити файл у групу (`None` — у корінь). Повертає новий відносний шлях.
pub(crate) async fn set_mod_group(
    instance_id: &str,
    project_path: &str,
    group: Option<&str>,
    state: &State,
) -> crate::Result<String> {
    let group = group.map(validate_group_name).transpose()?;
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    if !project_path.starts_with(&format!("{MODS_FOLDER}/")) {
        return Err(crate::ErrorKind::InputError(
            "Групи є лише для модів".to_string(),
        )
        .into());
    }
    if instance_has_running_process(instance_id, state).await? {
        return Err(crate::ErrorKind::OtherError(
            "Закрий гру, щоб переміщати моди між групами".to_string(),
        )
        .into());
    }
    let file_name = Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!("Bad path {project_path}"))
        })?
        .to_string();
    let new_path = grouped_path(group.as_deref(), &file_name);
    if new_path == project_path {
        return Ok(new_path);
    }
    let mods_dir = base.join(MODS_FOLDER);
    if !base.join(project_path).exists() {
        return Err(crate::ErrorKind::FSError(format!(
            "Файл {project_path} не знайдено"
        ))
        .into());
    }
    if base.join(&new_path).exists() {
        return Err(crate::ErrorKind::FSError(format!(
            "У цільовій групі вже є {file_name}"
        ))
        .into());
    }
    if let Some(group) = &group {
        io::create_dir_all(mods_dir.join(group)).await?;
    }
    io::rename_or_move(base.join(project_path), base.join(&new_path)).await?;
    ensure_readme(&mods_dir).await?;

    let enabled = !new_path.ends_with(".disabled");
    let mut tx = state.pool.begin().await?;
    content_rows::rename_instance_file(
        &scope.instance.id,
        project_path,
        &new_path,
        &file_name,
        enabled,
        &mut tx,
    )
    .await?;
    tx.commit().await?;
    Ok(new_path)
}

pub(crate) async fn rename_mod_group(
    instance_id: &str,
    old_name: &str,
    new_name: &str,
    state: &State,
) -> crate::Result<String> {
    let new_name = validate_group_name(new_name)?;
    if new_name == old_name {
        return Ok(new_name);
    }
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    if instance_has_running_process(instance_id, state).await? {
        return Err(crate::ErrorKind::OtherError(
            "Закрий гру, щоб перейменувати групу".to_string(),
        )
        .into());
    }
    if mods_dir.join(&new_name).exists() {
        return Err(crate::ErrorKind::FSError(format!(
            "Група «{new_name}» уже існує"
        ))
        .into());
    }
    io::rename_or_move(mods_dir.join(old_name), mods_dir.join(&new_name))
        .await?;

    let old_prefix = format!("{MODS_FOLDER}/{old_name}/");
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?;
    let mut tx = state.pool.begin().await?;
    for file in files.iter().filter(|f| f.relative_path.starts_with(&old_prefix))
    {
        let new_path = grouped_path(Some(&new_name), &file.file_name);
        content_rows::rename_instance_file(
            &scope.instance.id,
            &file.relative_path,
            &new_path,
            &file.file_name,
            file.enabled,
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(new_name)
}

/// Розформувати групу: файли — в корінь `mods/`, папку прибрати.
pub(crate) async fn delete_mod_group(
    instance_id: &str,
    name: &str,
    state: &State,
) -> crate::Result<()> {
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    let mods_dir = base.join(MODS_FOLDER);
    let group_dir = mods_dir.join(name);
    if instance_has_running_process(instance_id, state).await? {
        return Err(crate::ErrorKind::OtherError(
            "Закрий гру, щоб видалити групу".to_string(),
        )
        .into());
    }
    if !group_dir.is_dir() {
        return Ok(());
    }
    let prefix = format!("{MODS_FOLDER}/{name}/");
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?;
    let mut tx = state.pool.begin().await?;
    let mut dir = io::read_dir(&group_dir).await?;
    while let Some(entry) = dir.next_entry().await.map_err(IOError::from)? {
        let Ok(file_name) = entry.file_name().into_string() else {
            continue;
        };
        let target = mods_dir.join(&file_name);
        if target.exists() {
            return Err(crate::ErrorKind::FSError(format!(
                "У корені mods/ уже є {file_name} — спершу прибери дублікат"
            ))
            .into());
        }
        io::rename_or_move(entry.path(), &target).await?;
        let old_path = format!("{prefix}{file_name}");
        if let Some(file) = files.iter().find(|f| f.relative_path == old_path) {
            content_rows::rename_instance_file(
                &scope.instance.id,
                &old_path,
                &grouped_path(None, &file_name),
                &file_name,
                file.enabled,
                &mut tx,
            )
            .await?;
        }
    }
    tx.commit().await?;
    let _ = io::remove_dir(&group_dir).await;
    Ok(())
}

// ---------------------------------------------------------------------------
// Розкладання на час гри / оновлення збірки

/// На час гри: перенести всі jar-и з груп у корінь `mods/` і запам'ятати,
/// звідки вони. БД не чіпаємо — сканер показує файли з кореня під груповими
/// шляхами, поки існує карта. Ідемпотентно: якщо карта вже є — доповнює її.
pub(crate) async fn flatten_mods(
    instance_dir: &Path,
    reason: FlattenReason,
) -> crate::Result<()> {
    let mods_dir = instance_dir.join(MODS_FOLDER);
    if !mods_dir.is_dir() {
        return Ok(());
    }
    let mut map = read_flatten_map_sync(&mods_dir).unwrap_or_default();
    map.reason = Some(reason);
    let mut dir = io::read_dir(&mods_dir).await?;
    while let Some(entry) = dir.next_entry().await.map_err(IOError::from)? {
        let Ok(group) = entry.file_name().into_string() else {
            continue;
        };
        if !is_group_dir_name(&group) || !entry.path().is_dir() {
            continue;
        }
        let mut files = io::read_dir(entry.path()).await?;
        while let Some(file) =
            files.next_entry().await.map_err(IOError::from)?
        {
            let Ok(file_name) = file.file_name().into_string() else {
                continue;
            };
            if !file.path().is_file() {
                continue;
            }
            let target = mods_dir.join(&file_name);
            if target.exists() {
                tracing::warn!(
                    "Terrarium: {file_name} є і в корені mods/, і в групі «{group}» — лишаю в групі"
                );
                continue;
            }
            io::rename_or_move(file.path(), &target).await?;
            map.files.insert(file_name, group.clone());
        }
    }
    ensure_readme(&mods_dir).await?;
    write_flatten_map(&mods_dir, &map).await
}

/// Повернути файли з кореня в їхні групи за картою. Те, чого вже нема
/// (мод видалили/оновили під час гри), просто забувається.
pub(crate) async fn restore_mods(instance_dir: &Path) -> crate::Result<()> {
    let mods_dir = instance_dir.join(MODS_FOLDER);
    let Some(mut map) = read_flatten_map_sync(&mods_dir) else {
        return Ok(());
    };
    let mut remaining = BTreeMap::new();
    for (file_name, group) in std::mem::take(&mut map.files) {
        let source = mods_dir.join(&file_name);
        if !source.is_file() {
            continue;
        }
        let group_dir = mods_dir.join(&group);
        if let Err(err) = io::create_dir_all(&group_dir).await {
            tracing::warn!("Terrarium: не створити групу «{group}»: {err}");
            remaining.insert(file_name, group);
            continue;
        }
        // Одразу після виходу гри JVM може ще тримати jar відкритим —
        // кілька спроб із паузою.
        let target = group_dir.join(&file_name);
        let mut moved = false;
        for attempt in 0..6 {
            match io::rename_or_move(&source, &target).await {
                Ok(()) => {
                    moved = true;
                    break;
                }
                Err(err) if attempt == 5 => tracing::warn!(
                    "Terrarium: не повернути {file_name} у «{group}»: {err}"
                ),
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(500))
                        .await;
                }
            }
        }
        if !moved {
            remaining.insert(file_name, group);
        }
    }
    map.files = remaining;
    write_flatten_map(&mods_dir, &map).await
}

/// Чи є примірник у розкладеному стані (і з якої причини).
pub(crate) fn flatten_reason(instance_dir: &Path) -> Option<FlattenReason> {
    read_flatten_map_sync(&instance_dir.join(MODS_FOLDER))
        .and_then(|m| m.reason)
}

/// Самовідновлення: карта від запуску гри лишилась (краш лаунчера), а гра
/// вже не працює — повернути моди по групах.
pub(crate) async fn restore_if_stale(
    instance_id: &str,
    instance_dir: &Path,
    state: &State,
) -> crate::Result<()> {
    if flatten_reason(instance_dir) != Some(FlattenReason::Launch) {
        return Ok(());
    }
    if instance_has_running_process(instance_id, state).await? {
        return Ok(());
    }
    restore_mods(instance_dir).await
}

/// Перед оновленням збірки: моди з груп — у корінь, але через БД (шлях у
/// рядку файлу теж змінюється, тож джерело «зі збірки» зберігається, а
/// встановлення бачить файли за тими самими шляхами, що в попередньому пакеті).
pub(crate) async fn flatten_for_pack_update(
    instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let instance_dir = instance_full_path(state, &scope.instance);
    let mods_dir = instance_dir.join(MODS_FOLDER);
    if flatten_reason(&instance_dir) == Some(FlattenReason::Launch) {
        return Err(crate::ErrorKind::OtherError(
            "Закрий гру перед оновленням збірки".to_string(),
        )
        .into());
    }
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?;
    let mut map = read_flatten_map_sync(&mods_dir).unwrap_or_default();
    map.reason = Some(FlattenReason::PackUpdate);
    for file in files.iter().filter(|f| !f.missing) {
        let Some(group) = group_of(&file.relative_path) else {
            continue;
        };
        let group = group.to_string();
        set_mod_group(instance_id, &file.relative_path, None, state).await?;
        map.files.insert(file.file_name.clone(), group);
    }
    if !mods_dir.is_dir() {
        return Ok(());
    }
    write_flatten_map(&mods_dir, &map).await
}

/// Після оновлення збірки: повернути особисті моди в їхні групи (через БД).
/// Нічого не робить, якщо карти від оновлення нема.
pub(crate) async fn restore_after_pack_update(
    instance_id: &str,
    state: &State,
) -> crate::Result<()> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    let Some(map) = read_flatten_map_sync(&mods_dir) else {
        return Ok(());
    };
    if map.reason != Some(FlattenReason::PackUpdate) {
        return Ok(());
    }
    crate::state::sync_content_files(instance_id, state).await?;
    for (file_name, group) in &map.files {
        let path = grouped_path(None, file_name);
        if !mods_dir.join(file_name).is_file() {
            continue;
        }
        if let Err(err) =
            set_mod_group(instance_id, &path, Some(group), state).await
        {
            tracing::warn!(
                "Terrarium: не повернути {file_name} у «{group}»: {err}"
            );
        }
    }
    write_flatten_map(&mods_dir, &FlattenMap::default()).await
}

/// Застосувати групи з пакета збірки (`.terrarium/groups.json`: файл → група)
/// до модів примірника, де б вони зараз не лежали. Повертає кількість переміщених.
pub(crate) async fn apply_pack_groups(
    instance_id: &str,
    groups: &BTreeMap<String, String>,
    state: &State,
) -> crate::Result<usize> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?;
    let mut moved = 0;
    for (file_name, group) in groups {
        // Файл міг бути й вимкненим (.disabled), і вже в якійсь групі
        let current = files.iter().find(|f| {
            f.relative_path.starts_with(&format!("{MODS_FOLDER}/"))
                && f.file_name.trim_end_matches(".disabled") == file_name
        });
        let from = match current {
            Some(file) => file.relative_path.clone(),
            None => grouped_path(None, file_name),
        };
        if group_of(&from) == Some(group.as_str()) {
            continue;
        }
        match set_mod_group(instance_id, &from, Some(group), state).await {
            Ok(_) => moved += 1,
            Err(err) => tracing::debug!(
                "Terrarium: групу «{group}» для {file_name} не застосовано: {err}"
            ),
        }
    }
    Ok(moved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        assert_eq!(group_of("mods/Опт/x.jar"), Some("Опт"));
        assert_eq!(group_of("mods/x.jar"), None);
        assert_eq!(group_of("config/mods/x.jar"), None);
        assert_eq!(flat_path("mods/Опт/x.jar"), "mods/x.jar");
        assert_eq!(flat_path("mods/x.jar.disabled"), "mods/x.jar.disabled");
        assert_eq!(grouped_path(Some("A"), "y.jar"), "mods/A/y.jar");
        assert!(validate_group_name("mods").is_err());
        assert!(validate_group_name(".hidden").is_err());
        assert!(validate_group_name("a/b").is_err());
        assert_eq!(validate_group_name("  Опт ").unwrap(), "Опт");
    }

    #[tokio::test]
    async fn flatten_and_restore_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mods = dir.path().join("mods");
        std::fs::create_dir_all(mods.join("A")).unwrap();
        std::fs::create_dir_all(mods.join("B")).unwrap();
        std::fs::create_dir_all(mods.join(".hidden")).unwrap();
        std::fs::write(mods.join("A/a.jar"), b"a").unwrap();
        std::fs::write(mods.join("B/b.jar"), b"b").unwrap();
        std::fs::write(mods.join("B/off.jar.disabled"), b"o").unwrap();
        std::fs::write(mods.join("root.jar"), b"r").unwrap();
        std::fs::write(mods.join(".hidden/h.jar"), b"h").unwrap();

        flatten_mods(dir.path(), FlattenReason::Launch).await.unwrap();
        assert!(mods.join("a.jar").is_file());
        assert!(mods.join("b.jar").is_file());
        assert!(mods.join("off.jar.disabled").is_file());
        assert!(mods.join(".hidden/h.jar").is_file());
        assert!(mods.join(README_FILE).is_file());
        let map = read_flatten_map_sync(&mods).unwrap();
        assert_eq!(map.reason, Some(FlattenReason::Launch));
        assert_eq!(map.files.get("a.jar").map(String::as_str), Some("A"));
        assert_eq!(flatten_reason(dir.path()), Some(FlattenReason::Launch));

        // мод видалили під час гри — просто зникає з карти
        std::fs::remove_file(mods.join("b.jar")).unwrap();
        restore_mods(dir.path()).await.unwrap();
        assert!(mods.join("A/a.jar").is_file());
        assert!(!mods.join("a.jar").exists());
        assert!(mods.join("B/off.jar.disabled").is_file());
        assert!(mods.join("root.jar").is_file());
        assert!(!mods.join(FLATTEN_FILE).exists());
        assert_eq!(flatten_reason(dir.path()), None);
    }
}
