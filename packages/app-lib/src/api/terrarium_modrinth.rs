//! Terrarium: спільна тека даних із Modrinth App.
//!
//! У гравця, який уже має Modrinth App, збірки/Java/кеш лежать у
//! `…/ModrinthApp`. Замість дублювати гігабайти лаунчер може перемкнути свою
//! теку даних туди (штатний механізм `custom_dir`) і підхопити примірники з
//! бази Modrinth. Бази даних лишаються окремими: схеми двох лаунчерів
//! розходяться, тож спільний `app.db` зламав би одного з них. Файли примірників
//! читаються обома; Modrinth App при цьому не блокується.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, SqliteConnection};

use crate::State;
use crate::state::{DirectoryInfo, ModLoader};

/// Ідентифікатор Modrinth App (тека в data_dir).
const MODRINTH_APP_ID: &str = "ModrinthApp";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthAppInfo {
    /// Тека даних Modrinth App (де `profiles/`, `meta/`, `app.db`).
    pub path: String,
    /// Скільки примірників у базі Modrinth.
    pub instances: usize,
    /// Скільки з них ще не імпортовано (за шляхом папки).
    pub importable: usize,
    /// Наша тека даних уже = тека Modrinth.
    pub already_shared: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthImportResult {
    pub imported: usize,
    pub skipped: usize,
}

/// Рядок примірника з бази Modrinth — лише те, що потрібно, щоб створити наш.
#[derive(sqlx::FromRow)]
struct ModrinthInstanceRow {
    path: String,
    name: String,
    icon_path: Option<String>,
    game_version: String,
    loader: String,
    loader_version: Option<String>,
    modrinth_project_id: Option<String>,
    modrinth_version_id: Option<String>,
    created: i64,
    modified: i64,
    last_played: Option<i64>,
}

fn modrinth_data_dir() -> Option<PathBuf> {
    DirectoryInfo::initial_settings_dir_path(MODRINTH_APP_ID)
        .filter(|dir| dir.join("app.db").is_file())
}

/// Модель Modrinth App читаємо напряму read-only: спільний код міграцій сюди не
/// застосовуємо, щоб випадково не «оновити» чужу базу.
async fn open_modrinth_db(dir: &Path) -> crate::Result<SqliteConnection> {
    let options = SqliteConnectOptions::new()
        .filename(dir.join("app.db"))
        .read_only(true)
        .immutable(false);
    Ok(SqliteConnection::connect_with(&options).await?)
}

async fn read_modrinth_instances(
    dir: &Path,
) -> crate::Result<Vec<ModrinthInstanceRow>> {
    let mut conn = open_modrinth_db(dir).await?;
    // Схема Modrinth ≥ 0.21: instances + instance_content_sets (+ instance_links).
    // Беремо лише встановлені; поля, яких може не бути у старіших/новіших
    // версіях, — через LEFT JOIN, щоб один відсутній стовпець не ламав усе.
    let rows = sqlx::query_as::<_, ModrinthInstanceRow>(
        "
        SELECT i.path, i.name, i.icon_path,
               cs.game_version, cs.loader, cs.loader_version,
               l.modrinth_project_id, l.modrinth_version_id,
               i.created, i.modified, i.last_played
        FROM instances i
        JOIN instance_content_sets cs ON cs.id = i.applied_content_set_id
        LEFT JOIN instance_links l ON l.instance_id = i.id
        WHERE i.install_stage = 'installed'
        ORDER BY i.modified DESC
        ",
    )
    .fetch_all(&mut conn)
    .await?;
    let _ = conn.close().await;
    Ok(rows)
}

/// Шляхи (папки в `profiles/`) примірників, які вже є в нашій базі.
async fn our_instance_paths(state: &State) -> crate::Result<Vec<String>> {
    Ok(
        sqlx::query_scalar::<_, String>("SELECT path FROM instances")
            .fetch_all(&state.pool)
            .await?,
    )
}

/// Чи є Modrinth App на цьому ПК, і що з нього можна підхопити.
#[tracing::instrument]
pub async fn detect_modrinth_app() -> crate::Result<Option<ModrinthAppInfo>> {
    let Some(dir) = modrinth_data_dir() else {
        return Ok(None);
    };
    let state = State::get().await?;
    let already_shared = same_dir(&state.directories.config_dir, &dir);
    let rows = match read_modrinth_instances(&dir).await {
        Ok(rows) => rows,
        Err(err) => {
            // База є, але схема інша/зайнята — показуємо теку без лічильників
            tracing::warn!(
                "Terrarium: не вдалося прочитати базу Modrinth App: {err}"
            );
            return Ok(Some(ModrinthAppInfo {
                path: dir.to_string_lossy().to_string(),
                instances: 0,
                importable: 0,
                already_shared,
            }));
        }
    };
    let ours = our_instance_paths(&state).await?;
    let importable = rows
        .iter()
        .filter(|r| {
            !ours.contains(&r.path)
                && dir.join("profiles").join(&r.path).is_dir()
        })
        .count();
    Ok(Some(ModrinthAppInfo {
        path: dir.to_string_lossy().to_string(),
        instances: rows.len(),
        importable,
        already_shared,
    }))
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// Перемкнути теку даних лаунчера на теку Modrinth App. Сама зміна
/// застосовується штатним механізмом (`custom_dir` → перенесення при
/// наступному старті), тому після виклику лаунчер треба перезапустити.
#[tracing::instrument]
pub async fn use_modrinth_directory() -> crate::Result<String> {
    let dir = modrinth_data_dir().ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Modrinth App на цьому ПК не знайдено".to_string(),
        )
    })?;
    let state = State::get().await?;
    // При наступному старті лаунчер ПЕРЕНОСИТЬ profiles/, meta/, caches/ зі
    // своєї теки в теку Modrinth пофайлово, поверх наявного. Якщо папка
    // примірника з такою ж назвою вже є в Modrinth — файли Modrinth-версії
    // були б перезаписані; такий випадок відхиляємо.
    let ours = state.directories.instances_dir();
    let theirs = dir.join("profiles");
    if ours.is_dir() && theirs.is_dir() {
        let mut clashes = Vec::new();
        for entry in std::fs::read_dir(&ours)
            .map_err(|e| crate::util::io::IOError::with_path(e, &ours))?
            .flatten()
        {
            let name = entry.file_name();
            if entry.path().is_dir() && theirs.join(&name).is_dir() {
                clashes.push(name.to_string_lossy().to_string());
            }
        }
        if !clashes.is_empty() {
            return Err(crate::ErrorKind::OtherError(format!(
                "У Modrinth App уже є примірники з такими ж папками: {}. Перейменуй або видали їх в одному з лаунчерів і спробуй знову.",
                clashes.join(", ")
            ))
            .into());
        }
    }
    let mut settings = crate::state::Settings::get(&state.pool).await?;
    settings.custom_dir = Some(dir.to_string_lossy().to_string());
    settings.update(&state.pool).await?;
    Ok(dir.to_string_lossy().to_string())
}

/// Підхопити примірники Modrinth у нашу базу. Працює лише коли тека даних уже
/// спільна (інакше шляхи `profiles/<папка>` не існуватимуть). Нічого не копіює.
#[tracing::instrument]
pub async fn import_modrinth_instances() -> crate::Result<ModrinthImportResult>
{
    let dir = modrinth_data_dir().ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Modrinth App на цьому ПК не знайдено".to_string(),
        )
    })?;
    let state = State::get().await?;
    if !same_dir(&state.directories.config_dir, &dir) {
        return Err(crate::ErrorKind::OtherError(
            "Спершу перемкни теку даних на теку Modrinth App і перезапусти лаунчер"
                .to_string(),
        )
        .into());
    }
    let rows = read_modrinth_instances(&dir).await?;
    let ours = our_instance_paths(&state).await?;
    let mut imported = 0;
    let mut skipped = 0;
    for row in rows {
        if ours.contains(&row.path)
            || !dir.join("profiles").join(&row.path).is_dir()
        {
            skipped += 1;
            continue;
        }
        let loader = match row.loader.as_str() {
            "forge" => ModLoader::Forge,
            "fabric" => ModLoader::Fabric,
            "quilt" => ModLoader::Quilt,
            "neoforge" => ModLoader::NeoForge,
            _ => ModLoader::Vanilla,
        };
        crate::state::legacy_converter::upsert_imported_instance(
            &state.pool,
            crate::state::legacy_converter::ImportedInstance {
                path: row.path,
                name: row.name,
                icon_path: row.icon_path,
                game_version: row.game_version,
                loader,
                loader_version: row.loader_version,
                modrinth_project_id: row.modrinth_project_id,
                modrinth_version_id: row.modrinth_version_id,
                created: row.created,
                modified: row.modified,
                last_played: row.last_played,
            },
        )
        .await?;
        imported += 1;
    }
    Ok(ModrinthImportResult { imported, skipped })
}
