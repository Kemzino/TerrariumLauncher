//! Terrarium: групи модів як справжні підпапки `mods/`, з вкладенням
//! (`mods/Оптимізація/Шейдери/x.jar`). Група ідентифікується шляхом сегментів
//! `Оптимізація/Шейдери`.
//!
//! Структура лежить на диску (щоб її бачив і той, хто просто скопіює папку в
//! інший лаунчер — там же лежить README.txt із поясненням). Підпапки в грі
//! читає мод зі складу збірки, тож на час запуску нічого не переноситься.
//! Вимкнена група — папка з суфіксом `.disabled` (`mods/Шейдери.disabled/`):
//! мод (як і завантажувачі) її ігнорує; усе всередині, включно з підгрупами,
//! теж вимкнене. Шлях у БД відповідає диску (`mods/A/B.disabled/x.jar`), а
//! назва групи/шлях групи — без суфіксів (`A/B`).
//!
//! Карта розкладання `mods/.terrarium-flatten.json` лишилась лише для
//! оновлення збірки (моди тимчасово в корені, щоб інсталятор зіставив шляхи з
//! попереднім пакетом) і для самовідновлення після старих версій лаунчера.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::instances::commands::{
    instance_full_path, resolve_content_scope,
};
use crate::state::process::instance_has_running_process;
use crate::util::io;

pub const MODS_FOLDER: &str = "mods";
pub const FLATTEN_FILE: &str = ".terrarium-flatten.json";
pub const README_FILE: &str = "README.txt";
/// Групи з пакета збірки (`overrides/.terrarium/groups.json`): файл → група.
pub const PACK_GROUPS_FILE: &str = ".terrarium/groups.json";
/// Суфікс папки вимкненої групи (той самий, що й у файлів модів).
pub const DISABLED_SUFFIX: &str = ".disabled";

const MAX_GROUP_NAME: usize = 64;
/// Глибина вкладення — щоб хтось не зробив 50 рівнів і не зламав шляхи Windows.
const MAX_GROUP_DEPTH: usize = 8;

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

/// Вузол дерева груп. `path` — `A/B/C` (без `.disabled`), `name` — останній
/// сегмент, `parent` — шлях батька (`None` для верхнього рівня).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGroup {
    pub path: String,
    pub name: String,
    pub parent: Option<String>,
    /// Файлів безпосередньо в цій папці (без підгруп).
    pub files: usize,
    /// `false` — ця папка або хтось із предків має суфікс `.disabled`.
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Шляхи

/// Назва сегмента без суфікса: `Група.disabled` → `Група`.
pub fn group_name_of_dir(dir_name: &str) -> &str {
    dir_name.strip_suffix(DISABLED_SUFFIX).unwrap_or(dir_name)
}

/// Сегменти папок групи з відносного шляху файлу (як на диску, з можливими
/// `.disabled`): `mods/A/B.disabled/x.jar` → `["A", "B.disabled"]`.
fn group_dir_segments(relative_path: &str) -> Option<Vec<&str>> {
    let mut parts: Vec<&str> = relative_path.split('/').collect();
    if parts.len() < 3 || parts[0] != MODS_FOLDER {
        return None;
    }
    parts.pop(); // ім'я файлу
    parts.remove(0); // mods
    Some(parts)
}

/// Шлях групи без суфіксів: `mods/A/B.disabled/x.jar` → `Some("A/B")`;
/// `mods/x.jar` → `None`.
pub fn group_of(relative_path: &str) -> Option<String> {
    group_dir_segments(relative_path).map(|segments| {
        segments
            .into_iter()
            .map(group_name_of_dir)
            .collect::<Vec<_>>()
            .join("/")
    })
}

/// Чи файл у вимкненій групі (будь-який сегмент має `.disabled`).
pub fn group_disabled_in_path(relative_path: &str) -> bool {
    group_dir_segments(relative_path)
        .map(|segments| segments.iter().any(|s| s.ends_with(DISABLED_SUFFIX)))
        .unwrap_or(false)
}

/// `mods/A/B/x.jar` → `mods/x.jar` (шлях, яким файл бачить .mrpack).
pub fn flat_path(relative_path: &str) -> String {
    if !relative_path.starts_with(&format!("{MODS_FOLDER}/")) {
        return relative_path.to_string();
    }
    match relative_path.rsplit_once('/') {
        Some((_, file_name)) => format!("{MODS_FOLDER}/{file_name}"),
        None => relative_path.to_string(),
    }
}

/// Шлях файлу в групі (`group_dir` — сегменти як на диску, `None` — корінь).
pub fn grouped_path(group_dir: Option<&str>, file_name: &str) -> String {
    match group_dir {
        Some(group) if !group.is_empty() => {
            format!("{MODS_FOLDER}/{group}/{file_name}")
        }
        _ => format!("{MODS_FOLDER}/{file_name}"),
    }
}

/// Папки, які сканер вважає групою: не приховані й не службові.
pub fn is_group_dir_name(name: &str) -> bool {
    !name.starts_with('.') && name != "__MACOSX"
}

fn validate_segment(name: &str) -> crate::Result<String> {
    let name = name.trim();
    let bad = name.is_empty()
        || name.len() > MAX_GROUP_NAME
        || name.starts_with('.')
        || name.ends_with('.')
        || name.ends_with(DISABLED_SUFFIX)
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

/// Валідація шляху групи `A/B/C` → нормалізований шлях.
fn validate_group_path(path: &str) -> crate::Result<String> {
    let segments = path
        .split('/')
        .map(validate_segment)
        .collect::<crate::Result<Vec<_>>>()?;
    if segments.len() > MAX_GROUP_DEPTH {
        return Err(crate::ErrorKind::InputError(format!(
            "Занадто глибоке вкладення груп (максимум {MAX_GROUP_DEPTH})"
        ))
        .into());
    }
    Ok(segments.join("/"))
}

/// Папка групи на диску за логічним шляхом: кожен сегмент — `X` або
/// `X.disabled`, що існує. Повертає (шлях на диску, сегменти як на диску).
fn resolve_group_dir(mods_dir: &Path, group: &str) -> (PathBuf, Vec<String>) {
    let mut dir = mods_dir.to_path_buf();
    let mut segments = Vec::new();
    for name in group.split('/').filter(|s| !s.is_empty()) {
        let enabled = dir.join(name);
        let disabled = dir.join(format!("{name}{DISABLED_SUFFIX}"));
        let chosen = if !enabled.is_dir() && disabled.is_dir() {
            format!("{name}{DISABLED_SUFFIX}")
        } else {
            name.to_string()
        };
        dir = dir.join(&chosen);
        segments.push(chosen);
    }
    (dir, segments)
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

Підпапки в цій теці (наприклад mods/Оптимізація/ або mods/Оптимізація/Шейдери/)
— це групи модів, створені в Terrarium Launcher. Гра читає їх завдяки моду зі
складу збірки Terrarium, тож переносити нічого не треба.

Папка з суфіксом .disabled (наприклад mods/Шейдери.disabled/) — вимкнена група:
гра її не читає (разом з усім, що всередині), як і файли *.jar.disabled.

Якщо копіюєш цю теку в інший лаунчер БЕЗ збірки Terrarium — перенеси всі .jar
з підпапок прямо в mods/:
  Windows (PowerShell, у теці mods):
    Get-ChildItem -Recurse -Filter *.jar | Move-Item -Destination .
  Linux/macOS (у теці mods):
    find . -mindepth 2 -name '*.jar' -exec mv -t . {} +

---

Subfolders here (nested too) are mod groups made by Terrarium Launcher; a mod
bundled with the Terrarium pack loads them in-game. A folder ending in
.disabled is a disabled group (everything inside is off). Copying this folder
elsewhere without that mod? Move every .jar from the subfolders straight into
mods/ (commands above).
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

fn walk_groups(
    dir: &Path,
    logical_parent: Option<&str>,
    parent_enabled: bool,
    depth: usize,
    out: &mut Vec<ContentGroup>,
) {
    if depth > MAX_GROUP_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut dirs: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            (e.path().is_dir() && is_group_dir_name(&name))
                .then(|| (name, e.path()))
        })
        .collect();
    dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    for (dir_name, path) in dirs {
        let name = group_name_of_dir(&dir_name).to_string();
        let logical = match logical_parent {
            Some(parent) => format!("{parent}/{name}"),
            None => name.clone(),
        };
        let enabled = parent_enabled && !dir_name.ends_with(DISABLED_SUFFIX);
        let files = std::fs::read_dir(&path)
            .map(|d| d.flatten().filter(|e| e.path().is_file()).count())
            .unwrap_or(0);
        // Дублікати (`X` і `X.disabled` поруч) — обидві папки, один вузол
        if let Some(existing) = out.iter_mut().find(|g| g.path == logical) {
            existing.files += files;
            existing.enabled |= enabled;
        } else {
            out.push(ContentGroup {
                path: logical.clone(),
                name,
                parent: logical_parent.map(str::to_string),
                files,
                enabled,
            });
        }
        walk_groups(&path, Some(&logical), enabled, depth + 1, out);
    }
}

/// Усі групи примірника (дерево, розгорнуте в список; батьки перед дітьми).
pub(crate) async fn list_mod_groups(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<ContentGroup>> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    let mut groups = Vec::new();
    if mods_dir.is_dir() {
        walk_groups(&mods_dir, None, true, 1, &mut groups);
        // Групи з карти розкладання (оновлення збірки триває) — щоб не зникали
        if let Some(map) = read_flatten_map_sync(&mods_dir) {
            for group in map.files.values() {
                if let Some(existing) =
                    groups.iter_mut().find(|g| &g.path == group)
                {
                    existing.files += 1;
                } else {
                    let (parent, name) = match group.rsplit_once('/') {
                        Some((p, n)) => (Some(p.to_string()), n.to_string()),
                        None => (None, group.clone()),
                    };
                    groups.push(ContentGroup {
                        path: group.clone(),
                        name,
                        parent,
                        files: 1,
                        enabled: true,
                    });
                }
            }
        }
    }
    Ok(groups)
}

// ---------------------------------------------------------------------------
// Керування групами

async fn ensure_not_running(
    instance_id: &str,
    state: &State,
    what: &str,
) -> crate::Result<()> {
    if instance_has_running_process(instance_id, state).await? {
        return Err(crate::ErrorKind::OtherError(format!(
            "Закрий гру, щоб {what}"
        ))
        .into());
    }
    Ok(())
}

/// Перейменувати в БД усі файли з префіксом `old_prefix` (шлях папки на
/// диску, без завершального `/`) на `new_prefix`.
async fn rename_db_prefix(
    instance_id: &str,
    old_prefix: &str,
    new_prefix: &str,
    state: &State,
) -> crate::Result<()> {
    let old_prefix = format!("{old_prefix}/");
    let files =
        content_rows::get_instance_files(instance_id, &state.pool).await?;
    let mut tx = state.pool.begin().await?;
    for file in files
        .iter()
        .filter(|f| f.relative_path.starts_with(&old_prefix))
    {
        let rest = &file.relative_path[old_prefix.len()..];
        let new_path = format!("{new_prefix}/{rest}");
        let enabled = !file.file_name.ends_with(DISABLED_SUFFIX)
            && !group_disabled_in_path(&new_path);
        content_rows::rename_instance_file(
            instance_id,
            &file.relative_path,
            &new_path,
            &file.file_name,
            enabled,
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Створити групу за шляхом `A/B/C` (проміжні рівні створюються).
pub(crate) async fn create_mod_group(
    instance_id: &str,
    path: &str,
    state: &State,
) -> crate::Result<String> {
    let path = validate_group_path(path)?;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    let (dir, _) = resolve_group_dir(&mods_dir, &path);
    io::create_dir_all(&dir).await?;
    ensure_readme(&mods_dir).await?;
    Ok(path)
}

/// Увімкнути/вимкнути групу: перейменувати її папку `X` ↔ `X.disabled`
/// і оновити шляхи файлів у БД (включно з підгрупами).
pub(crate) async fn set_mod_group_enabled(
    instance_id: &str,
    path: &str,
    enabled: bool,
    state: &State,
) -> crate::Result<()> {
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    ensure_not_running(instance_id, state, "увімкнути чи вимкнути групу")
        .await?;
    let (current, segments) = resolve_group_dir(&mods_dir, path);
    if !current.is_dir() {
        return Err(
            crate::ErrorKind::FSError(format!("Групи «{path}» немає")).into()
        );
    }
    let Some(last) = segments.last() else {
        return Ok(());
    };
    let base_name = group_name_of_dir(last).to_string();
    let target_name = if enabled {
        base_name
    } else {
        format!("{base_name}{DISABLED_SUFFIX}")
    };
    if *last == target_name {
        return Ok(());
    }
    let target = current.with_file_name(&target_name);
    if target.exists() {
        return Err(crate::ErrorKind::FSError(format!(
            "Папка «{target_name}» уже існує"
        ))
        .into());
    }
    io::rename_or_move(&current, &target).await?;

    let old_prefix = format!("{MODS_FOLDER}/{}", segments.join("/"));
    let mut new_segments = segments.clone();
    *new_segments.last_mut().unwrap() = target_name;
    let new_prefix = format!("{MODS_FOLDER}/{}", new_segments.join("/"));
    rename_db_prefix(&scope.instance.id, &old_prefix, &new_prefix, state).await
}

/// Перемістити файл у групу (`None` — у корінь). Повертає новий відносний шлях.
pub(crate) async fn set_mod_group(
    instance_id: &str,
    project_path: &str,
    group: Option<&str>,
    state: &State,
) -> crate::Result<String> {
    let group = group.map(validate_group_path).transpose()?;
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    if !project_path.starts_with(&format!("{MODS_FOLDER}/")) {
        return Err(crate::ErrorKind::InputError(
            "Групи є лише для модів".to_string(),
        )
        .into());
    }
    ensure_not_running(instance_id, state, "переміщати моди між групами")
        .await?;
    let file_name = Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!("Bad path {project_path}"))
        })?
        .to_string();
    let mods_dir = base.join(MODS_FOLDER);
    // Цільова папка — як на диску (з можливими `.disabled` у сегментах)
    let group_dir_segments = group
        .as_deref()
        .map(|g| resolve_group_dir(&mods_dir, g).1.join("/"));
    let new_path = grouped_path(group_dir_segments.as_deref(), &file_name);
    if new_path == project_path {
        return Ok(new_path);
    }
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
    if let Some(segments) = &group_dir_segments {
        io::create_dir_all(mods_dir.join(segments)).await?;
    }
    io::rename_or_move(base.join(project_path), base.join(&new_path)).await?;
    ensure_readme(&mods_dir).await?;

    let enabled = !file_name.ends_with(DISABLED_SUFFIX)
        && !group_disabled_in_path(&new_path);
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

/// Перейменувати останній сегмент групи (`A/B` → `A/C`). Підгрупи їдуть разом.
pub(crate) async fn rename_mod_group(
    instance_id: &str,
    old_path: &str,
    new_name: &str,
    state: &State,
) -> crate::Result<String> {
    let new_name = validate_segment(new_name)?;
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let mods_dir = instance_full_path(state, &scope.instance).join(MODS_FOLDER);
    ensure_not_running(instance_id, state, "перейменувати групу").await?;
    let (current, segments) = resolve_group_dir(&mods_dir, old_path);
    let Some(last) = segments.last() else {
        return Ok(new_name);
    };
    if last.ends_with(DISABLED_SUFFIX) {
        return Err(crate::ErrorKind::OtherError(
            "Група вимкнена — спершу увімкни її".to_string(),
        )
        .into());
    }
    if *last == new_name {
        return Ok(old_path.to_string());
    }
    let parent = current.parent().unwrap_or(&mods_dir).to_path_buf();
    if parent.join(&new_name).exists()
        || parent.join(format!("{new_name}{DISABLED_SUFFIX}")).exists()
    {
        return Err(crate::ErrorKind::FSError(format!(
            "Група «{new_name}» уже існує"
        ))
        .into());
    }
    io::rename_or_move(&current, parent.join(&new_name)).await?;

    let old_prefix = format!("{MODS_FOLDER}/{}", segments.join("/"));
    let mut new_segments = segments.clone();
    *new_segments.last_mut().unwrap() = new_name.clone();
    let new_prefix = format!("{MODS_FOLDER}/{}", new_segments.join("/"));
    rename_db_prefix(&scope.instance.id, &old_prefix, &new_prefix, state)
        .await?;

    let new_logical = new_segments
        .iter()
        .map(|s| group_name_of_dir(s))
        .collect::<Vec<_>>()
        .join("/");
    Ok(new_logical)
}

/// Розформувати групу: її файли (і файли підгруп) — у батьківську групу
/// (або в корінь), порожні папки видалити.
pub(crate) async fn delete_mod_group(
    instance_id: &str,
    path: &str,
    state: &State,
) -> crate::Result<()> {
    let _content_lock = state.lock_instance_content(instance_id).await;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    let mods_dir = base.join(MODS_FOLDER);
    ensure_not_running(instance_id, state, "видалити групу").await?;
    let (group_dir, segments) = resolve_group_dir(&mods_dir, path);
    if !group_dir.is_dir() {
        return Ok(());
    }
    let parent_segments = &segments[..segments.len().saturating_sub(1)];
    let parent_rel = if parent_segments.is_empty() {
        MODS_FOLDER.to_string()
    } else {
        format!("{MODS_FOLDER}/{}", parent_segments.join("/"))
    };
    let prefix = format!("{MODS_FOLDER}/{}/", segments.join("/"));
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?;
    let mut tx = state.pool.begin().await?;
    for file in files
        .iter()
        .filter(|f| f.relative_path.starts_with(&prefix))
    {
        let source = base.join(&file.relative_path);
        if !source.is_file() {
            continue;
        }
        let target_rel = format!("{parent_rel}/{}", file.file_name);
        let target = base.join(&target_rel);
        if target.exists() {
            tracing::warn!(
                "Terrarium: {} уже є в «{parent_rel}» — лишаю в групі",
                file.file_name
            );
            continue;
        }
        io::rename_or_move(&source, &target).await?;
        let enabled = !file.file_name.ends_with(DISABLED_SUFFIX)
            && !group_disabled_in_path(&target_rel);
        content_rows::rename_instance_file(
            &scope.instance.id,
            &file.relative_path,
            &target_rel,
            &file.file_name,
            enabled,
            &mut tx,
        )
        .await?;
    }
    tx.commit().await?;
    // Папка (з порожніми підпапками) — геть; якщо щось лишилось, remove_dir_all
    // усе одно безпечний: файли, що лишились, ми свідомо не чіпали лише при
    // конфлікті імен, тож видаляємо лише порожні
    remove_empty_dirs(&group_dir);
    Ok(())
}

fn remove_empty_dirs(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    let mut empty = true;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !remove_empty_dirs(&path) {
                empty = false;
            }
        } else {
            empty = false;
        }
    }
    if empty {
        let _ = std::fs::remove_dir(dir);
    }
    empty
}

// ---------------------------------------------------------------------------
// Розкладання на час оновлення збірки (і самовідновлення після старих версій)

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
        let (group_dir, _) = resolve_group_dir(&mods_dir, &group);
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

/// Самовідновлення: карта від запуску гри лишилась (стара версія лаунчера),
/// а гра вже не працює — повернути моди по групах.
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

/// Застосувати групи з пакета збірки (`.terrarium/groups.json`: файл → група,
/// група може бути вкладеною `A/B`) до модів примірника. Повертає кількість
/// переміщених.
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
                && f.file_name.trim_end_matches(DISABLED_SUFFIX) == file_name
        });
        let from = match current {
            Some(file) => file.relative_path.clone(),
            None => grouped_path(None, file_name),
        };
        if group_of(&from).as_deref() == Some(group.as_str()) {
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
        assert_eq!(group_of("mods/Опт/x.jar").as_deref(), Some("Опт"));
        assert_eq!(group_of("mods/A/B/x.jar").as_deref(), Some("A/B"));
        assert_eq!(group_of("mods/A/B.disabled/x.jar").as_deref(), Some("A/B"));
        assert_eq!(group_of("mods/x.jar"), None);
        assert_eq!(group_of("config/mods/x.jar"), None);
        assert!(group_disabled_in_path("mods/A.disabled/B/x.jar"));
        assert!(!group_disabled_in_path("mods/A/B/x.jar"));
        assert_eq!(flat_path("mods/Опт/x.jar"), "mods/x.jar");
        assert_eq!(flat_path("mods/A/B/x.jar"), "mods/x.jar");
        assert_eq!(flat_path("mods/x.jar.disabled"), "mods/x.jar.disabled");
        assert_eq!(grouped_path(Some("A/B"), "y.jar"), "mods/A/B/y.jar");
        assert!(validate_segment("mods").is_err());
        assert!(validate_segment(".hidden").is_err());
        assert!(validate_segment("a/b").is_err());
        assert!(validate_segment("x.disabled").is_err());
        assert_eq!(validate_group_path("A/B").unwrap(), "A/B");
        assert!(validate_group_path("A//B").is_err());
    }
}
