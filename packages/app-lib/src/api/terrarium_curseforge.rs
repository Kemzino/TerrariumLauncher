//! Terrarium: CurseForge для модів, яких нема на Modrinth.
//!
//! Файл упізнається за відбитком CurseForge (MurmurHash2 без пробільних
//! байтів) через `/v1/fingerprints/432`, далі `/v1/mods` дає назву, іконку,
//! автора, сторінку й перелік останніх файлів — з нього видно, чи є новіший
//! файл для версії гри та завантажувача примірника. Автоматично файли не
//! ставимо (більшість авторів на CF забороняють сторонні завантаження) —
//! лише показуємо оновлення й ведемо на сторінку файлу.
//!
//! Потрібен API-ключ (console.curseforge.com): вбудований під час збірки з
//! `.env` (`CURSEFORGE_API_KEY`) або заданий у налаштуваннях Terrarium
//! (`terrarium.json` → `curseforge_api_key`). Без ключа модуль мовчки
//! нічого не робить. Усі помилки мережі — лише попередження в лог: список
//! умісту має показуватись і без CurseForge.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::State;
use crate::state::{
    CacheBehaviour, ContentFile, CurseForgeContent, CurseForgeUpdate,
    ModLoader, ProjectType,
};

const API_BASE: &str = "https://api.curseforge.com/v1";
/// ID гри Minecraft у CurseForge.
const GAME_ID: u32 = 432;
const BUILT_IN_KEY: Option<&str> = option_env!("CURSEFORGE_API_KEY");
/// Скільки відбитків/модів в одному запиті.
const BATCH: usize = 200;

const TTL_FINGERPRINT: i64 = 100 * 365 * 24 * 60 * 60; // відбиток файлу незмінний
const TTL_MATCH: i64 = 30 * 24 * 60 * 60; // збіг файл → мод теж не змінюється
const TTL_NO_MATCH: i64 = 24 * 60 * 60; // файл могли додати на CF пізніше
const TTL_MOD: i64 = 6 * 60 * 60; // картка мода — звідси перевірка оновлень

const KIND_FINGERPRINT: &str = "fingerprint";
const KIND_MATCH: &str = "match";
const KIND_MOD: &str = "mod";

/// Окремий клієнт: CurseForge відповідає 401 на запити з User-Agent
/// Modrinth-лаунчера (`modrinth/theseus/…`), навіть із дійсним ключем.
static CLIENT: std::sync::LazyLock<reqwest::Client> =
    std::sync::LazyLock::new(|| {
        reqwest::Client::builder()
            .user_agent(concat!(
                "TerrariumLauncher/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/Kemzino)"
            ))
            .connect_timeout(std::time::Duration::from_secs(15))
            .read_timeout(std::time::Duration::from_secs(30))
            .https_only(true)
            .build()
            .expect("client configuration should be valid")
    });

// ---------------------------------------------------------------------------
// Відбиток

/// MurmurHash2 (32-біт, seed 1) як у CurseForge; пробільні байти (9, 10, 13,
/// 32) не враховуються.
pub fn curseforge_fingerprint(bytes: &[u8]) -> u32 {
    const M: u32 = 0x5bd1_e995;
    const R: u32 = 24;
    let data: Vec<u8> = bytes
        .iter()
        .copied()
        .filter(|b| !matches!(b, 9 | 10 | 13 | 32))
        .collect();
    let len = data.len() as u32;
    let mut h: u32 = 1 ^ len;
    let mut chunks = data.chunks_exact(4);
    for chunk in &mut chunks {
        let mut k =
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        if rest.len() >= 3 {
            h ^= u32::from(rest[2]) << 16;
        }
        if rest.len() >= 2 {
            h ^= u32::from(rest[1]) << 8;
        }
        h ^= u32::from(rest[0]);
        h = h.wrapping_mul(M);
    }
    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    h
}

// ---------------------------------------------------------------------------
// Кеш (таблиця terrarium_curseforge_cache)

struct CachedRow {
    data: String,
    expired: bool,
}

async fn cache_get(
    pool: &SqlitePool,
    kind: &str,
    keys: &[String],
) -> HashMap<String, CachedRow> {
    if keys.is_empty() {
        return HashMap::new();
    }
    let Ok(keys_json) = serde_json::to_string(keys) else {
        return HashMap::new();
    };
    let rows = sqlx::query(
        "SELECT key, data, expires FROM terrarium_curseforge_cache
         WHERE kind = ? AND key IN (SELECT value FROM json_each(?))",
    )
    .bind(kind)
    .bind(keys_json)
    .fetch_all(pool)
    .await;
    let now = Utc::now().timestamp();
    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|row| {
                let key: String = row.get("key");
                let data: String = row.get("data");
                let expires: i64 = row.get("expires");
                (
                    key,
                    CachedRow {
                        data,
                        expired: expires <= now,
                    },
                )
            })
            .collect(),
        Err(err) => {
            tracing::warn!("Terrarium/CurseForge: кеш не читається: {err}");
            HashMap::new()
        }
    }
}

async fn cache_put(
    pool: &SqlitePool,
    kind: &str,
    key: &str,
    data: &str,
    ttl: i64,
) {
    let expires = Utc::now().timestamp() + ttl;
    if let Err(err) = sqlx::query(
        "INSERT INTO terrarium_curseforge_cache (kind, key, data, expires)
         VALUES (?, ?, ?, ?)
         ON CONFLICT (kind, key) DO UPDATE SET data = excluded.data, expires = excluded.expires",
    )
    .bind(kind)
    .bind(key)
    .bind(data)
    .bind(expires)
    .execute(pool)
    .await
    {
        tracing::warn!("Terrarium/CurseForge: кеш не пишеться: {err}");
    }
}

async fn cache_delete(pool: &SqlitePool, kind: &str, key: &str) {
    if let Err(err) = sqlx::query(
        "DELETE FROM terrarium_curseforge_cache WHERE kind = ? AND key = ?",
    )
    .bind(kind)
    .bind(key)
    .execute(pool)
    .await
    {
        tracing::warn!("Terrarium/CurseForge: кеш не чиститься: {err}");
    }
}

// ---------------------------------------------------------------------------
// API

/// Ключ із налаштувань має пріоритет над вбудованим.
async fn api_key() -> Option<String> {
    let from_settings = super::terrarium::get_state()
        .await
        .ok()
        .and_then(|state| state.curseforge_api_key)
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty());
    from_settings.or_else(|| BUILT_IN_KEY.map(str::to_string))
}

/// Чи є ключ CurseForge (для UI — щоб пояснити, чому нема іконок).
pub async fn has_api_key() -> bool {
    api_key().await.is_some()
}

async fn post_json<T: serde::de::DeserializeOwned>(
    key: &str,
    path: &str,
    body: &serde_json::Value,
) -> crate::Result<T> {
    let response = CLIENT
        .post(format!("{API_BASE}{path}"))
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .json(body)
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "CurseForge відповів {status} на {path}: {}",
            text.chars().take(200).collect::<String>()
        ))
        .into());
    }
    Ok(serde_json::from_str(&text)?)
}

#[derive(Deserialize)]
struct Envelope<T> {
    data: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintsData {
    #[serde(default)]
    exact_matches: Vec<FingerprintMatch>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintMatch {
    /// ID мода
    id: u64,
    file: CfFile,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFile {
    id: u64,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    file_name: String,
    /// 1 — реліз, 2 — бета, 3 — альфа
    #[serde(default = "default_release_type")]
    release_type: u8,
    #[serde(default)]
    file_fingerprint: u64,
}

fn default_release_type() -> u8 {
    1
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfMod {
    id: u64,
    name: String,
    slug: String,
    #[serde(default)]
    links: CfLinks,
    #[serde(default)]
    logo: Option<CfLogo>,
    #[serde(default)]
    authors: Vec<CfAuthor>,
    #[serde(default)]
    latest_files_indexes: Vec<CfFileIndex>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CfLinks {
    #[serde(default)]
    website_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfLogo {
    #[serde(default)]
    thumbnail_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
struct CfAuthor {
    name: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileIndex {
    game_version: String,
    file_id: u64,
    filename: String,
    #[serde(default = "default_release_type")]
    release_type: u8,
    /// 1 Forge, 4 Fabric, 5 Quilt, 6 NeoForge; відсутній — універсальний
    #[serde(default)]
    mod_loader: Option<u8>,
}

/// Що зберігаємо в кеші про збіг файлу (`null` — на CF файлу нема).
#[derive(Serialize, Deserialize, Clone)]
struct MatchInfo {
    mod_id: u64,
    file_id: u64,
    file_name: String,
    display_name: String,
    release_type: u8,
}

/// Що зберігаємо в кеші про мод.
#[derive(Serialize, Deserialize, Clone)]
struct ModInfo {
    id: u64,
    name: String,
    slug: String,
    url: String,
    icon_url: Option<String>,
    author: Option<String>,
    author_url: Option<String>,
    files: Vec<FileIndex>,
}

#[derive(Serialize, Deserialize, Clone)]
struct FileIndex {
    game_version: String,
    file_id: u64,
    file_name: String,
    release_type: u8,
    mod_loader: Option<u8>,
}

impl From<CfMod> for ModInfo {
    fn from(m: CfMod) -> Self {
        let url = m.links.website_url.unwrap_or_else(|| {
            format!("https://www.curseforge.com/minecraft/mc-mods/{}", m.slug)
        });
        let author = m.authors.first();
        ModInfo {
            id: m.id,
            name: m.name,
            slug: m.slug,
            url,
            icon_url: m.logo.and_then(|logo| logo.thumbnail_url.or(logo.url)),
            author: author.map(|a| a.name.clone()),
            author_url: author.and_then(|a| a.url.clone()),
            files: m
                .latest_files_indexes
                .into_iter()
                .map(|f| FileIndex {
                    game_version: f.game_version,
                    file_id: f.file_id,
                    file_name: f.filename,
                    release_type: f.release_type,
                    mod_loader: f.mod_loader,
                })
                .collect(),
        }
    }
}

fn loader_accepts(loader: ModLoader, cf_loader: Option<u8>) -> bool {
    match (loader, cf_loader) {
        (_, None) => true,
        (ModLoader::Forge, Some(1)) => true,
        (ModLoader::Fabric, Some(4)) => true,
        // Quilt читає й Fabric-моди
        (ModLoader::Quilt, Some(4 | 5)) => true,
        (ModLoader::NeoForge, Some(6)) => true,
        (ModLoader::Vanilla, Some(_)) => true,
        _ => false,
    }
}

/// Новіший файл для версії гри/завантажувача. Релізи мають пріоритет; бета чи
/// альфа пропонуються лише якщо встановлений файл теж не реліз.
fn find_update(
    m: &ModInfo,
    installed: &MatchInfo,
    game_version: &str,
    loader: ModLoader,
) -> Option<CurseForgeUpdate> {
    let candidates = m
        .files
        .iter()
        .filter(|f| f.game_version == game_version)
        .filter(|f| loader_accepts(loader, f.mod_loader))
        .filter(|f| f.file_id > installed.file_id)
        .filter(|f| installed.release_type != 1 || f.release_type == 1);
    let best = candidates.max_by_key(|f| (f.release_type == 1, f.file_id))?;
    Some(CurseForgeUpdate {
        file_id: best.file_id,
        file_name: best.file_name.clone(),
        url: format!(
            "https://www.curseforge.com/minecraft/mc-mods/{}/files/{}",
            m.slug, best.file_id
        ),
    })
}

// ---------------------------------------------------------------------------
// Основний крок

/// Для файлів, яких нема на Modrinth, — картка з CurseForge за sha1 файлу.
/// Ніколи не падає: без ключа чи мережі повертає порожню мапу.
/// `known_project_ids` — проєкти Modrinth, які справді знайшлись за
/// метаданими файлів. Файл, чий кешований збіг хеш→проєкт веде на проєкт,
/// якого на Modrinth уже нема (знято з публікації, відхилено модерацією),
/// інакше показувався б як «просто файл», хоч на CurseForge він є.
pub(crate) async fn resolve_curseforge_content(
    instance_dir: &Path,
    game_version: &str,
    loader: ModLoader,
    files: &[(String, ContentFile)],
    known_project_ids: &HashSet<String>,
    cache_behaviour: Option<CacheBehaviour>,
    state: &State,
) -> HashMap<String, CurseForgeContent> {
    let candidates: Vec<(&str, &str)> = files
        .iter()
        .filter(|(_, file)| {
            file.metadata
                .as_ref()
                .is_none_or(|m| !known_project_ids.contains(&m.project_id))
                && matches!(
                    file.project_type,
                    ProjectType::Mod
                        | ProjectType::ResourcePack
                        | ProjectType::ShaderPack
                        | ProjectType::DataPack
                )
        })
        .map(|(path, file)| (file.hash.as_str(), path.as_str()))
        .collect();
    if candidates.is_empty() {
        return HashMap::new();
    }
    let Some(key) = api_key().await else {
        return HashMap::new();
    };
    let pool = &state.pool;
    // Лише Bypass оминає кеш карток модів. MustRevalidate означає «перевір
    // прострочене», а не «питай API щоразу»: список умісту перечитується після
    // кожної дії (перемкнув мод, переніс у групу), і мережевий похід за
    // незміненими картками на кожну таку дію відчувався як підлагування.
    let revalidate = cache_behaviour == Some(CacheBehaviour::Bypass);

    // 1. Відбитки (кеш за sha1; інакше читаємо файл)
    let hashes: Vec<String> = candidates
        .iter()
        .map(|(hash, _)| hash.to_string())
        .collect();
    let cached_fp = cache_get(pool, KIND_FINGERPRINT, &hashes).await;
    let mut fingerprint_by_hash: HashMap<String, u32> = HashMap::new();
    for (hash, path) in &candidates {
        if let Some(row) = cached_fp.get(*hash)
            && let Ok(fp) = row.data.parse::<u32>()
        {
            fingerprint_by_hash.insert(hash.to_string(), fp);
            continue;
        }
        let full_path = instance_dir.join(path);
        let bytes = match tokio::fs::read(&full_path).await {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::debug!(
                    "Terrarium/CurseForge: не прочитати {}: {err}",
                    full_path.display()
                );
                continue;
            }
        };
        let fp =
            tokio::task::spawn_blocking(move || curseforge_fingerprint(&bytes))
                .await
                .ok();
        if let Some(fp) = fp {
            cache_put(
                pool,
                KIND_FINGERPRINT,
                hash,
                &fp.to_string(),
                TTL_FINGERPRINT,
            )
            .await;
            fingerprint_by_hash.insert(hash.to_string(), fp);
        }
    }

    // 2. Збіги відбиток → файл мода (кеш; решту — на API пачками)
    let fp_keys: Vec<String> = fingerprint_by_hash
        .values()
        .map(|fp| fp.to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let cached_matches = cache_get(pool, KIND_MATCH, &fp_keys).await;
    let mut match_by_fp: HashMap<u32, Option<MatchInfo>> = HashMap::new();
    let mut to_fetch: Vec<u32> = Vec::new();
    for fp_key in &fp_keys {
        let Ok(fp) = fp_key.parse::<u32>() else {
            continue;
        };
        match cached_matches.get(fp_key) {
            Some(row) if !row.expired => {
                match_by_fp
                    .insert(fp, serde_json::from_str(&row.data).ok().flatten());
            }
            _ => to_fetch.push(fp),
        }
    }
    for chunk in to_fetch.chunks(BATCH) {
        let body = serde_json::json!({ "fingerprints": chunk });
        let result: crate::Result<Envelope<FingerprintsData>> =
            post_json(&key, &format!("/fingerprints/{GAME_ID}"), &body).await;
        match result {
            Ok(envelope) => {
                let mut found: HashMap<u32, MatchInfo> = HashMap::new();
                for m in envelope.data.exact_matches {
                    let fp = m.file.file_fingerprint as u32;
                    found.insert(
                        fp,
                        MatchInfo {
                            mod_id: m.id,
                            file_id: m.file.id,
                            file_name: m.file.file_name,
                            display_name: m.file.display_name,
                            release_type: m.file.release_type,
                        },
                    );
                }
                for fp in chunk {
                    let info = found.remove(fp);
                    let (data, ttl) = match &info {
                        Some(info) => (
                            serde_json::to_string(info).unwrap_or_default(),
                            TTL_MATCH,
                        ),
                        None => ("null".to_string(), TTL_NO_MATCH),
                    };
                    cache_put(pool, KIND_MATCH, &fp.to_string(), &data, ttl)
                        .await;
                    match_by_fp.insert(*fp, info);
                }
            }
            Err(err) => {
                tracing::warn!(
                    "Terrarium/CurseForge: пошук за відбитками: {err}"
                );
                // Хоч застаріле з кешу, якщо було
                for fp in chunk {
                    if let Some(row) = cached_matches.get(&fp.to_string()) {
                        match_by_fp.insert(
                            *fp,
                            serde_json::from_str(&row.data).ok().flatten(),
                        );
                    }
                }
            }
        }
    }

    // 3. Картки модів (кеш 6 год; примусове оновлення — з API)
    let mod_ids: Vec<String> = match_by_fp
        .values()
        .flatten()
        .map(|m| m.mod_id.to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let cached_mods = cache_get(pool, KIND_MOD, &mod_ids).await;
    let mut mod_by_id: HashMap<u64, ModInfo> = HashMap::new();
    let mut mods_to_fetch: Vec<u64> = Vec::new();
    for id_key in &mod_ids {
        let Ok(id) = id_key.parse::<u64>() else {
            continue;
        };
        match cached_mods.get(id_key) {
            Some(row) if !row.expired && !revalidate => {
                if let Ok(info) = serde_json::from_str::<ModInfo>(&row.data) {
                    mod_by_id.insert(id, info);
                    continue;
                }
                mods_to_fetch.push(id);
            }
            _ => mods_to_fetch.push(id),
        }
    }
    for chunk in mods_to_fetch.chunks(BATCH) {
        let body = serde_json::json!({ "modIds": chunk, "filterPcOnly": true });
        let result: crate::Result<Envelope<Vec<CfMod>>> =
            post_json(&key, "/mods", &body).await;
        match result {
            Ok(envelope) => {
                for m in envelope.data {
                    let info = ModInfo::from(m);
                    cache_put(
                        pool,
                        KIND_MOD,
                        &info.id.to_string(),
                        &serde_json::to_string(&info).unwrap_or_default(),
                        TTL_MOD,
                    )
                    .await;
                    mod_by_id.insert(info.id, info);
                }
            }
            Err(err) => {
                tracing::warn!("Terrarium/CurseForge: картки модів: {err}");
                for id in chunk {
                    if let Some(row) = cached_mods.get(&id.to_string())
                        && let Ok(info) =
                            serde_json::from_str::<ModInfo>(&row.data)
                    {
                        mod_by_id.insert(*id, info);
                    }
                }
            }
        }
    }

    // 4. Збираємо картки за sha1
    let mut result = HashMap::new();
    for (hash, fp) in &fingerprint_by_hash {
        let Some(Some(installed)) = match_by_fp.get(fp) else {
            continue;
        };
        let Some(m) = mod_by_id.get(&installed.mod_id) else {
            continue;
        };
        result.insert(
            hash.clone(),
            CurseForgeContent {
                mod_id: m.id,
                name: m.name.clone(),
                slug: m.slug.clone(),
                url: m.url.clone(),
                icon_url: m.icon_url.clone(),
                author: m.author.clone(),
                author_url: m.author_url.clone(),
                file_id: installed.file_id,
                file_name: installed.file_name.clone(),
                display_name: installed.display_name.clone(),
                update: find_update(m, installed, game_version, loader),
            },
        );
    }
    result
}

#[cfg(test)]
mod tests {
    use super::curseforge_fingerprint;

    #[test]
    fn fingerprint_ignores_whitespace() {
        assert_eq!(
            curseforge_fingerprint(b"hello world"),
            curseforge_fingerprint(b"helloworld")
        );
        assert_ne!(
            curseforge_fingerprint(b"helloworld"),
            curseforge_fingerprint(b"helloworlD")
        );
    }
}

// ---------------------------------------------------------------------------
// Зміна версії (файлу) мода з CurseForge

/// Файл мода на CurseForge — для вибору версії в лаунчері.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeFile {
    pub id: u64,
    pub display_name: String,
    pub file_name: String,
    /// 1 — реліз, 2 — бета, 3 — альфа
    pub release_type: u8,
    pub file_date: String,
    pub game_versions: Vec<String>,
    /// `false` — автор заборонив сторонні завантаження: лише зі сторінки CF
    pub downloadable: bool,
    /// Сторінка файлу на curseforge.com
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileFull {
    id: u64,
    #[serde(default)]
    display_name: String,
    file_name: String,
    #[serde(default = "default_release_type")]
    release_type: u8,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<CfHash>,
}

#[derive(Deserialize)]
struct CfHash {
    value: String,
    /// 1 — sha1, 2 — md5
    algo: u8,
}

fn cf_loader_id(loader: ModLoader) -> Option<u8> {
    match loader {
        ModLoader::Forge => Some(1),
        ModLoader::Fabric => Some(4),
        ModLoader::Quilt => Some(5),
        ModLoader::NeoForge => Some(6),
        ModLoader::Vanilla => None,
    }
}

async fn get_json<T: serde::de::DeserializeOwned>(
    key: &str,
    path: &str,
) -> crate::Result<T> {
    let response = CLIENT
        .get(format!("{API_BASE}{path}"))
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "CurseForge відповів {status} на {path}"
        ))
        .into());
    }
    Ok(serde_json::from_str(&text)?)
}

async fn require_key() -> crate::Result<String> {
    api_key().await.ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Немає ключа CurseForge API — задай його в налаштуваннях Terrarium"
                .to_string(),
        )
        .into()
    })
}

fn file_page_url(slug: &str, file_id: u64) -> String {
    format!(
        "https://www.curseforge.com/minecraft/mc-mods/{slug}/files/{file_id}"
    )
}

/// Файли мода для версії гри й завантажувача примірника (новіші перші).
/// `loader: None` — для ресурспаків/шейдерів/датапаків, у яких його нема.
#[tracing::instrument]
pub async fn list_mod_files(
    mod_id: u64,
    slug: String,
    game_version: String,
    loader: Option<ModLoader>,
) -> crate::Result<Vec<CurseForgeFile>> {
    let key = require_key().await?;
    let mut path = format!(
        "/mods/{mod_id}/files?gameVersion={}&pageSize=50",
        urlencoding::encode(&game_version)
    );
    if let Some(id) = loader.and_then(cf_loader_id) {
        let _ = write!(path, "&modLoaderType={id}");
    }
    let envelope: Envelope<Vec<CfFileFull>> = get_json(&key, &path).await?;
    let mut files: Vec<CurseForgeFile> = envelope
        .data
        .into_iter()
        .map(|f| CurseForgeFile {
            url: file_page_url(&slug, f.id),
            id: f.id,
            display_name: f.display_name,
            file_name: f.file_name,
            release_type: f.release_type,
            file_date: f.file_date,
            game_versions: f.game_versions,
            downloadable: f.download_url.is_some(),
        })
        .collect();
    files.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(files)
}

/// Замінити встановлений файл мода на інший файл із CurseForge: завантажити,
/// прибрати старий, покласти новий у ту саму групу з тим самим станом
/// увімкнення. Повертає новий відносний шлях.
#[tracing::instrument]
pub async fn switch_mod_file(
    instance_id: String,
    project_path: String,
    mod_id: u64,
    file_id: u64,
) -> crate::Result<String> {
    use crate::state::instance_has_running_process;
    use crate::state::instances::commands;

    let state = State::get().await?;
    if instance_has_running_process(&instance_id, &state).await? {
        return Err(crate::ErrorKind::OtherError(
            "Закрий гру, щоб змінити версію мода".to_string(),
        )
        .into());
    }
    let key = require_key().await?;
    let envelope: Envelope<CfFileFull> =
        get_json(&key, &format!("/mods/{mod_id}/files/{file_id}")).await?;
    let file = envelope.data;
    let Some(download_url) = file.download_url.clone() else {
        return Err(crate::ErrorKind::OtherError(format!(
            "Автор мода заборонив сторонні завантаження — завантаж «{}» вручну зі сторінки CurseForge і поклади в mods/",
            file.file_name
        ))
        .into());
    };
    if !path_util::is_safe_file_name(&file.file_name) {
        return Err(crate::ErrorKind::InputError(format!(
            "Підозріле ім'я файлу з CurseForge: {}",
            file.file_name
        ))
        .into());
    }

    // Завантаження (окремим клієнтом — CDN CurseForge теж не любить UA Modrinth)
    let response = CLIENT.get(&download_url).send().await?;
    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "CurseForge CDN відповів {} на завантаження {}",
            response.status(),
            file.file_name
        ))
        .into());
    }
    let bytes = response.bytes().await?;
    let sha1 = crate::util::fetch::sha1_async(bytes.clone()).await?;
    if let Some(expected) = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.to_lowercase())
        && expected != sha1
    {
        return Err(crate::ErrorKind::OtherError(format!(
            "Хеш завантаженого {} не збігається з CurseForge",
            file.file_name
        ))
        .into());
    }

    // Куди класти: та сама група й той самий стан увімкнення, що в старого
    let was_disabled = project_path.ends_with(".disabled");
    let group = commands::group_of(&project_path);
    let project_type = ProjectType::get_from_parent_folder(&project_path);

    commands::remove_project(&instance_id, &project_path, &state).await?;
    let mut new_path = commands::add_project_bytes(
        &instance_id,
        &file.file_name,
        bytes,
        Some(&sha1),
        project_type,
        crate::state::instances::ContentSourceKind::Local,
        None,
        None,
        &state,
    )
    .await?;
    if let Some(group) = group.as_deref() {
        new_path = commands::set_mod_group(
            &instance_id,
            &new_path,
            Some(group),
            &state,
        )
        .await?;
    }
    if was_disabled {
        new_path = commands::toggle_disable_project(
            &instance_id,
            &new_path,
            Some(false),
            &state,
        )
        .await?;
    }
    // Картку мода — з API наступного разу (файл змінився, оновлення інші)
    cache_delete(&state.pool, KIND_MOD, &mod_id.to_string()).await;
    crate::state::sync_content_files(&instance_id, &state).await?;
    Ok(new_path)
}

// ---------------------------------------------------------------------------
// Провайдер для сторінки проєкту, пошуку й установки — щоб CF-моди в лаунчері
// виглядали як Modrinth-моди. Фронтенд мапить ці структури у формат Modrinth.

const TTL_MOD_FULL: i64 = 30 * 60;
const TTL_DESCRIPTION: i64 = 60 * 60;
const TTL_CHANGELOG: i64 = 30 * 24 * 60 * 60;
const TTL_CATEGORIES: i64 = 24 * 60 * 60;
const KIND_MOD_FULL: &str = "mod_full";
const KIND_DESCRIPTION: &str = "description";
const KIND_CHANGELOG: &str = "changelog";
const KIND_CATEGORIES: &str = "categories";

/// ID класу CurseForge за типом умісту.
pub fn class_id(project_type: ProjectType) -> u32 {
    match project_type {
        ProjectType::Mod => 6,
        ProjectType::ResourcePack => 12,
        ProjectType::ShaderPack => 6552,
        ProjectType::DataPack => 6945,
    }
}

/// Клас збірок CurseForge (у `ProjectType` лаунчера збірок нема).
pub const MODPACK_CLASS_ID: u32 = 4471;

fn project_type_of_class(class: Option<u32>) -> Option<ProjectType> {
    match class {
        Some(6) => Some(ProjectType::Mod),
        Some(12) => Some(ProjectType::ResourcePack),
        Some(6552) => Some(ProjectType::ShaderPack),
        Some(6945) => Some(ProjectType::DataPack),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeCategory {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub icon_url: Option<String>,
    pub class_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeAuthor {
    pub id: u64,
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeScreenshot {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub url: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeLinks {
    pub website_url: Option<String>,
    pub wiki_url: Option<String>,
    pub issues_url: Option<String>,
    pub source_url: Option<String>,
}

/// Повна картка мода (сторінка проєкту / картка пошуку).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeMod {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub class_id: Option<u32>,
    pub links: CurseForgeLinks,
    pub categories: Vec<CurseForgeCategory>,
    pub authors: Vec<CurseForgeAuthor>,
    pub icon_url: Option<String>,
    pub screenshots: Vec<CurseForgeScreenshot>,
    pub download_count: u64,
    pub thumbs_up_count: u64,
    pub date_created: String,
    pub date_modified: String,
    pub date_released: String,
    /// Останні файли за версією гри/завантажувачем (для «Установити»)
    pub latest_files_indexes: Vec<CurseForgeFileIndex>,
    /// Усі версії гри, для яких є файли
    pub game_versions: Vec<String>,
    /// Завантажувачі, для яких є файли (`neoforge`, `forge`, `fabric`, `quilt`)
    pub loaders: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeFileIndex {
    pub game_version: String,
    pub file_id: u64,
    pub file_name: String,
    pub release_type: u8,
    pub mod_loader: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeDependency {
    pub mod_id: u64,
    /// 1 embedded, 2 optional, 3 required, 4 tool, 5 incompatible, 6 include
    pub relation_type: u8,
}

/// Файл мода з усім, що треба для вкладки «Версії».
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeFileDetails {
    pub id: u64,
    pub mod_id: u64,
    pub display_name: String,
    pub file_name: String,
    pub release_type: u8,
    pub file_date: String,
    pub file_length: u64,
    pub download_count: u64,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub downloadable: bool,
    pub sha1: Option<String>,
    pub dependencies: Vec<CurseForgeDependency>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurseForgeSearchResult {
    pub hits: Vec<CurseForgeMod>,
    pub total: u64,
    pub index: u64,
    pub page_size: u64,
}

// --- сирі відповіді API

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfCategoryRaw {
    id: u32,
    name: String,
    slug: String,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    class_id: Option<u32>,
    #[serde(default)]
    is_class: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CfLinksRaw {
    #[serde(default)]
    website_url: Option<String>,
    #[serde(default)]
    wiki_url: Option<String>,
    #[serde(default)]
    issues_url: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfScreenshotRaw {
    id: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    url: String,
    #[serde(default)]
    thumbnail_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfAuthorRaw {
    id: u64,
    name: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfModFullRaw {
    id: u64,
    name: String,
    slug: String,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    class_id: Option<u32>,
    #[serde(default)]
    links: Option<CfLinksRaw>,
    #[serde(default)]
    categories: Vec<CfCategoryRaw>,
    #[serde(default)]
    authors: Vec<CfAuthorRaw>,
    #[serde(default)]
    logo: Option<CfLogo>,
    #[serde(default)]
    screenshots: Vec<CfScreenshotRaw>,
    #[serde(default)]
    download_count: f64,
    #[serde(default)]
    thumbs_up_count: u64,
    #[serde(default)]
    date_created: String,
    #[serde(default)]
    date_modified: String,
    #[serde(default)]
    date_released: String,
    #[serde(default)]
    latest_files_indexes: Vec<CfFileIndex>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfDependencyRaw {
    mod_id: u64,
    relation_type: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfFileDetailsRaw {
    id: u64,
    mod_id: u64,
    #[serde(default)]
    display_name: String,
    file_name: String,
    #[serde(default = "default_release_type")]
    release_type: u8,
    #[serde(default)]
    file_date: String,
    #[serde(default)]
    file_length: u64,
    #[serde(default)]
    download_count: u64,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    game_versions: Vec<String>,
    #[serde(default)]
    hashes: Vec<CfHash>,
    #[serde(default)]
    dependencies: Vec<CfDependencyRaw>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfPagination {
    #[serde(default)]
    index: u64,
    #[serde(default)]
    page_size: u64,
    #[serde(default)]
    total_count: u64,
}

#[derive(Deserialize)]
struct PagedEnvelope<T> {
    data: T,
    #[serde(default)]
    pagination: Option<CfPagination>,
}

const LOADER_NAMES: [(&str, &str); 4] = [
    ("NeoForge", "neoforge"),
    ("Forge", "forge"),
    ("Fabric", "fabric"),
    ("Quilt", "quilt"),
];

/// `gameVersions` CF змішує версії гри і назви завантажувачів — розділяємо.
fn split_game_versions(raw: &[String]) -> (Vec<String>, Vec<String>) {
    let mut versions = Vec::new();
    let mut loaders = Vec::new();
    for value in raw {
        if let Some((_, slug)) =
            LOADER_NAMES.iter().find(|(name, _)| name == value)
        {
            loaders.push((*slug).to_string());
        } else if value.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            versions.push(value.clone());
        }
        // «Client», «Server», «Java 21» тощо пропускаємо
    }
    (versions, loaders)
}

fn loader_slug(id: Option<u8>) -> Option<&'static str> {
    match id {
        Some(1) => Some("forge"),
        Some(4) => Some("fabric"),
        Some(5) => Some("quilt"),
        Some(6) => Some("neoforge"),
        _ => None,
    }
}

impl From<CfModFullRaw> for CurseForgeMod {
    fn from(m: CfModFullRaw) -> Self {
        let links = m.links.unwrap_or_default();
        let mut game_versions: Vec<String> = Vec::new();
        let mut loaders: Vec<String> = Vec::new();
        for index in &m.latest_files_indexes {
            if !game_versions.contains(&index.game_version) {
                game_versions.push(index.game_version.clone());
            }
            if let Some(slug) = loader_slug(index.mod_loader)
                && !loaders.iter().any(|l| l == slug)
            {
                loaders.push(slug.to_string());
            }
        }
        CurseForgeMod {
            id: m.id,
            name: m.name,
            slug: m.slug,
            summary: m.summary,
            class_id: m.class_id,
            links: CurseForgeLinks {
                website_url: links.website_url,
                wiki_url: links.wiki_url,
                issues_url: links.issues_url,
                source_url: links.source_url,
            },
            categories: m
                .categories
                .into_iter()
                .filter(|c| c.is_class != Some(true))
                .map(|c| CurseForgeCategory {
                    id: c.id,
                    name: c.name,
                    slug: c.slug,
                    icon_url: c.icon_url,
                    class_id: c.class_id,
                })
                .collect(),
            authors: m
                .authors
                .into_iter()
                .map(|a| CurseForgeAuthor {
                    id: a.id,
                    name: a.name,
                    url: a.url,
                })
                .collect(),
            icon_url: m.logo.and_then(|l| l.url.or(l.thumbnail_url)),
            screenshots: m
                .screenshots
                .into_iter()
                .map(|s| CurseForgeScreenshot {
                    id: s.id,
                    title: s.title,
                    description: s.description,
                    thumbnail_url: if s.thumbnail_url.is_empty() {
                        s.url.clone()
                    } else {
                        s.thumbnail_url
                    },
                    url: s.url,
                })
                .collect(),
            download_count: m.download_count as u64,
            thumbs_up_count: m.thumbs_up_count,
            date_created: m.date_created,
            date_modified: m.date_modified,
            date_released: m.date_released,
            latest_files_indexes: m
                .latest_files_indexes
                .into_iter()
                .map(|f| CurseForgeFileIndex {
                    game_version: f.game_version,
                    file_id: f.file_id,
                    file_name: f.filename,
                    release_type: f.release_type,
                    mod_loader: f.mod_loader,
                })
                .collect(),
            game_versions,
            loaders,
        }
    }
}

fn file_details(f: CfFileDetailsRaw, slug: &str) -> CurseForgeFileDetails {
    let (game_versions, loaders) = split_game_versions(&f.game_versions);
    CurseForgeFileDetails {
        url: file_page_url(slug, f.id),
        id: f.id,
        mod_id: f.mod_id,
        display_name: f.display_name,
        file_name: f.file_name,
        release_type: f.release_type,
        file_date: f.file_date,
        file_length: f.file_length,
        download_count: f.download_count,
        game_versions,
        loaders,
        downloadable: f.download_url.is_some(),
        sha1: f
            .hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.to_lowercase()),
        dependencies: f
            .dependencies
            .into_iter()
            .map(|d| CurseForgeDependency {
                mod_id: d.mod_id,
                relation_type: d.relation_type,
            })
            .collect(),
    }
}

async fn cached_json<T, F>(
    pool: &SqlitePool,
    kind: &str,
    key: &str,
    ttl: i64,
    force: bool,
    fetch: F,
) -> crate::Result<T>
where
    T: Serialize + serde::de::DeserializeOwned,
    F: std::future::Future<Output = crate::Result<T>>,
{
    if !force
        && let Some(row) =
            cache_get(pool, kind, &[key.to_string()]).await.remove(key)
        && !row.expired
        && let Ok(value) = serde_json::from_str::<T>(&row.data)
    {
        return Ok(value);
    }
    let value = fetch.await?;
    cache_put(
        pool,
        kind,
        key,
        &serde_json::to_string(&value).unwrap_or_default(),
        ttl,
    )
    .await;
    Ok(value)
}

/// Повна картка мода.
#[tracing::instrument]
pub async fn get_mod(mod_id: u64, force: bool) -> crate::Result<CurseForgeMod> {
    let state = State::get().await?;
    let key = require_key().await?;
    cached_json(
        &state.pool,
        KIND_MOD_FULL,
        &mod_id.to_string(),
        TTL_MOD_FULL,
        force,
        async {
            let envelope: Envelope<CfModFullRaw> =
                get_json(&key, &format!("/mods/{mod_id}")).await?;
            Ok(CurseForgeMod::from(envelope.data))
        },
    )
    .await
}

/// Опис мода (HTML як на curseforge.com).
#[tracing::instrument]
pub async fn get_mod_description(mod_id: u64) -> crate::Result<String> {
    let state = State::get().await?;
    let key = require_key().await?;
    cached_json(
        &state.pool,
        KIND_DESCRIPTION,
        &mod_id.to_string(),
        TTL_DESCRIPTION,
        false,
        async {
            let envelope: Envelope<String> =
                get_json(&key, &format!("/mods/{mod_id}/description")).await?;
            Ok(envelope.data)
        },
    )
    .await
}

/// Чейнджлог файлу (HTML).
#[tracing::instrument]
pub async fn get_file_changelog(
    mod_id: u64,
    file_id: u64,
) -> crate::Result<String> {
    let state = State::get().await?;
    let key = require_key().await?;
    cached_json(
        &state.pool,
        KIND_CHANGELOG,
        &format!("{mod_id}:{file_id}"),
        TTL_CHANGELOG,
        false,
        async {
            let envelope: Envelope<String> = get_json(
                &key,
                &format!("/mods/{mod_id}/files/{file_id}/changelog"),
            )
            .await?;
            Ok(envelope.data)
        },
    )
    .await
}

/// Файли мода (до 50 на сторінку, новіші перші) — для вкладки «Версії».
/// `game_version`/`loader` звужують перелік.
#[tracing::instrument]
pub async fn get_mod_files(
    mod_id: u64,
    slug: String,
    game_version: Option<String>,
    loader: Option<ModLoader>,
    index: u64,
) -> crate::Result<(Vec<CurseForgeFileDetails>, u64)> {
    let key = require_key().await?;
    let mut path = format!("/mods/{mod_id}/files?pageSize=50&index={index}");
    if let Some(gv) = game_version.as_deref().filter(|s| !s.is_empty()) {
        let _ = write!(path, "&gameVersion={}", urlencoding::encode(gv));
    }
    if let Some(id) = loader.and_then(cf_loader_id) {
        let _ = write!(path, "&modLoaderType={id}");
    }
    let envelope: PagedEnvelope<Vec<CfFileDetailsRaw>> =
        get_json(&key, &path).await?;
    let total = envelope
        .pagination
        .map(|p| p.total_count)
        .unwrap_or(envelope.data.len() as u64);
    let mut files: Vec<CurseForgeFileDetails> = envelope
        .data
        .into_iter()
        .map(|f| file_details(f, &slug))
        .collect();
    files.sort_by(|a, b| b.id.cmp(&a.id));
    Ok((files, total))
}

/// Один файл мода з деталями.
#[tracing::instrument]
pub async fn get_file(
    mod_id: u64,
    file_id: u64,
    slug: String,
) -> crate::Result<CurseForgeFileDetails> {
    let key = require_key().await?;
    let envelope: Envelope<CfFileDetailsRaw> =
        get_json(&key, &format!("/mods/{mod_id}/files/{file_id}")).await?;
    Ok(file_details(envelope.data, &slug))
}

/// Категорії CF для класу (модів, ресурспаків…).
#[tracing::instrument]
pub async fn get_categories(
    project_type: Option<ProjectType>,
) -> crate::Result<Vec<CurseForgeCategory>> {
    let state = State::get().await?;
    let key = require_key().await?;
    let class = project_type.map(class_id).unwrap_or(MODPACK_CLASS_ID);
    cached_json(
        &state.pool,
        KIND_CATEGORIES,
        &class.to_string(),
        TTL_CATEGORIES,
        false,
        async {
            let envelope: Envelope<Vec<CfCategoryRaw>> = get_json(
                &key,
                &format!("/categories?gameId={GAME_ID}&classId={class}"),
            )
            .await?;
            let mut categories: Vec<CurseForgeCategory> = envelope
                .data
                .into_iter()
                .filter(|c| c.is_class != Some(true))
                .map(|c| CurseForgeCategory {
                    id: c.id,
                    name: c.name,
                    slug: c.slug,
                    icon_url: c.icon_url,
                    class_id: c.class_id,
                })
                .collect();
            categories.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(categories)
        },
    )
    .await
}

/// Параметри пошуку — як у бічній панелі лаунчера.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurseForgeSearchParams {
    pub project_type: Option<ProjectType>,
    /// Шукати збірки (клас 4471) — `project_type` тоді ігнорується
    #[serde(default)]
    pub modpacks: bool,
    pub query: Option<String>,
    /// popularity | downloads | updated | newest | name
    pub sort: Option<String>,
    #[serde(default)]
    pub category_ids: Vec<u32>,
    pub game_version: Option<String>,
    pub loader: Option<ModLoader>,
    #[serde(default)]
    pub index: u64,
    #[serde(default)]
    pub page_size: u64,
}

/// Пошук модів на CurseForge.
#[tracing::instrument]
pub async fn search(
    params: CurseForgeSearchParams,
) -> crate::Result<CurseForgeSearchResult> {
    let key = require_key().await?;
    let class = if params.modpacks {
        MODPACK_CLASS_ID
    } else {
        class_id(params.project_type.unwrap_or(ProjectType::Mod))
    };
    // Поля сортування CF: 1 Featured, 2 Popularity, 3 LastUpdated, 4 Name,
    // 5 Author, 6 TotalDownloads, 11 ReleasedDate
    let (sort_field, sort_order) = match params.sort.as_deref() {
        Some("downloads") => (6, "desc"),
        Some("updated") => (3, "desc"),
        Some("newest") => (11, "desc"),
        Some("name") => (4, "asc"),
        _ => (2, "desc"),
    };
    let page_size = params.page_size.clamp(1, 50);
    let mut path = format!(
        "/mods/search?gameId={GAME_ID}&classId={class}&sortField={sort_field}&sortOrder={sort_order}&index={}&pageSize={page_size}",
        params.index
    );
    if let Some(q) = params
        .query
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
    {
        let _ = write!(path, "&searchFilter={}", urlencoding::encode(q));
    }
    if let Some(gv) = params.game_version.as_deref().filter(|s| !s.is_empty()) {
        let _ = write!(path, "&gameVersion={}", urlencoding::encode(gv));
    }
    if let Some(id) = params.loader.and_then(cf_loader_id) {
        let _ = write!(path, "&modLoaderType={id}");
    }
    // Одна категорія — `categoryId`; кілька — `categoryIds=[..]` (JSON-масив,
    // не більше 10 за документацією CF)
    match params.category_ids.as_slice() {
        [] => {}
        [id] => {
            let _ = write!(path, "&categoryId={id}");
        }
        ids => {
            let ids = serde_json::to_string(&ids[..ids.len().min(10)])
                .unwrap_or_default();
            let _ = write!(path, "&categoryIds={}", urlencoding::encode(&ids));
        }
    }
    let envelope: PagedEnvelope<Vec<CfModFullRaw>> =
        get_json(&key, &path).await?;
    let (index, size, total) = envelope
        .pagination
        .map(|p| (p.index, p.page_size, p.total_count))
        .unwrap_or((params.index, page_size, envelope.data.len() as u64));
    Ok(CurseForgeSearchResult {
        hits: envelope.data.into_iter().map(CurseForgeMod::from).collect(),
        total,
        index,
        page_size: size,
    })
}

/// Найкращий файл мода для версії гри/завантажувача: реліз, інакше будь-який.
fn pick_file_for<'a>(
    m: &'a CurseForgeMod,
    game_version: &str,
    loader: ModLoader,
) -> Option<&'a CurseForgeFileIndex> {
    let candidates: Vec<&CurseForgeFileIndex> = m
        .latest_files_indexes
        .iter()
        .filter(|f| f.game_version == game_version)
        .filter(|f| loader_accepts(loader, f.mod_loader))
        .collect();
    candidates
        .iter()
        .copied()
        .filter(|f| f.release_type == 1)
        .max_by_key(|f| f.file_id)
        .or_else(|| candidates.iter().copied().max_by_key(|f| f.file_id))
}

/// Що зробила установка — для повідомлення гравцю.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurseForgeInstallResult {
    /// Встановлені файли (шляхи в примірнику), головний перший
    pub installed: Vec<String>,
    /// Назви модів, які вже були в примірнику
    pub skipped: Vec<String>,
    /// Обов'язкові залежності, які не вдалося поставити (нема файлу для
    /// версії/завантажувача або автор заборонив завантаження)
    pub failed: Vec<String>,
}

async fn download_file_bytes(
    key: &str,
    mod_id: u64,
    file_id: u64,
) -> crate::Result<(CfFileDetailsRaw, bytes::Bytes, String)> {
    let envelope: Envelope<CfFileDetailsRaw> =
        get_json(key, &format!("/mods/{mod_id}/files/{file_id}")).await?;
    let file = envelope.data;
    let Some(download_url) = file.download_url.clone() else {
        return Err(crate::ErrorKind::OtherError(format!(
            "Автор мода заборонив сторонні завантаження — «{}» можна взяти лише зі сторінки CurseForge",
            file.file_name
        ))
        .into());
    };
    if !path_util::is_safe_file_name(&file.file_name) {
        return Err(crate::ErrorKind::InputError(format!(
            "Підозріле ім'я файлу з CurseForge: {}",
            file.file_name
        ))
        .into());
    }
    let response = CLIENT.get(&download_url).send().await?;
    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "CurseForge CDN відповів {} на завантаження {}",
            response.status(),
            file.file_name
        ))
        .into());
    }
    let bytes = response.bytes().await?;
    let sha1 = crate::util::fetch::sha1_async(bytes.clone()).await?;
    if let Some(expected) = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.to_lowercase())
        && expected != sha1
    {
        return Err(crate::ErrorKind::OtherError(format!(
            "Хеш завантаженого {} не збігається з CurseForge",
            file.file_name
        ))
        .into());
    }
    Ok((file, bytes, sha1))
}

/// Установити файл мода в примірник разом з обов'язковими залежностями
/// (`file_id: None` — найкращий файл для версії гри/завантажувача).
/// Уже наявні моди (за ID CurseForge або за slug проєкту Modrinth) пропускаються.
#[tracing::instrument]
pub async fn install_mod(
    instance_id: String,
    mod_id: u64,
    file_id: Option<u64>,
    with_dependencies: bool,
) -> crate::Result<CurseForgeInstallResult> {
    use crate::state::instances::commands;

    let state = State::get().await?;
    let key = require_key().await?;
    let scope =
        commands::resolve_content_scope(&instance_id, None, &state).await?;
    let content_set =
        crate::state::instances::adapters::sqlite::content_rows::get_content_set(
            &scope.content_set_id,
            &state.pool,
        )
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(
                "Примірник без набору вмісту".to_string(),
            )
        })?;
    let game_version = content_set.game_version.clone();
    let loader = content_set.loader;

    // Що вже стоїть (власний уміст і моди збірки): ID CF-модів і slug-и
    // Modrinth-проєктів
    let mut items = super::instance::get_content_items(&instance_id, None)
        .await
        .unwrap_or_default();
    items.extend(
        super::instance::get_linked_modpack_content(&instance_id, None)
            .await
            .unwrap_or_default(),
    );
    let mut installed_cf: HashSet<u64> = items
        .iter()
        .filter_map(|i| i.curseforge.as_ref().map(|c| c.mod_id))
        .collect();
    let installed_slugs: HashSet<String> = items
        .iter()
        .filter_map(|i| i.project.as_ref().and_then(|p| p.slug.clone()))
        .collect();

    let mut result = CurseForgeInstallResult::default();
    let mut queue: Vec<(u64, Option<u64>, bool)> =
        vec![(mod_id, file_id, true)];
    let mut visited: HashSet<u64> = HashSet::new();

    while let Some((current_mod, wanted_file, is_root)) = queue.pop() {
        if !visited.insert(current_mod) {
            continue;
        }
        let m = get_mod(current_mod, false).await?;
        if !is_root
            && (installed_cf.contains(&m.id)
                || installed_slugs.contains(&m.slug))
        {
            result.skipped.push(m.name.clone());
            continue;
        }
        let chosen = match wanted_file {
            Some(id) => Some(id),
            None => pick_file_for(&m, &game_version, loader).map(|f| f.file_id),
        };
        let Some(chosen) = chosen else {
            if is_root {
                return Err(crate::ErrorKind::OtherError(format!(
                    "На CurseForge немає файлу «{}» для {game_version} / {}",
                    m.name,
                    loader.as_str()
                ))
                .into());
            }
            result.failed.push(m.name.clone());
            continue;
        };
        let (file, bytes, sha1) =
            match download_file_bytes(&key, m.id, chosen).await {
                Ok(v) => v,
                Err(err) if !is_root => {
                    tracing::warn!(
                        "Terrarium/CurseForge: залежність «{}»: {err}",
                        m.name
                    );
                    result.failed.push(m.name.clone());
                    continue;
                }
                Err(err) => return Err(err),
            };
        // Той самий мод уже є (з CurseForge — за ID, з Modrinth — за slug):
        // для головного мода це зміна версії — старий файл прибираємо, новий
        // кладемо в ту саму групу з тим самим станом; залежність пропускаємо
        let existing: Vec<&crate::state::ContentItem> = items
            .iter()
            .filter(|i| {
                i.curseforge.as_ref().is_some_and(|c| c.mod_id == m.id)
                    || i.project.as_ref().and_then(|p| p.slug.as_deref())
                        == Some(m.slug.as_str())
                    || i.file_name == file.file_name
            })
            .collect();
        if !existing.is_empty() && !is_root {
            result.skipped.push(m.name.clone());
            continue;
        }
        let mut group: Option<String> = None;
        let mut was_disabled = false;
        for item in &existing {
            group = group.or_else(|| commands::group_of(&item.file_path));
            was_disabled |= item.file_path.ends_with(".disabled");
            commands::remove_project(&instance_id, &item.file_path, &state)
                .await?;
        }
        let mut path = commands::add_project_bytes(
            &instance_id,
            &file.file_name,
            bytes,
            Some(&sha1),
            project_type_of_class(m.class_id),
            crate::state::instances::ContentSourceKind::Local,
            None,
            None,
            &state,
        )
        .await?;
        if let Some(group) = group.as_deref() {
            path = commands::set_mod_group(
                &instance_id,
                &path,
                Some(group),
                &state,
            )
            .await?;
        }
        if was_disabled {
            path = commands::toggle_disable_project(
                &instance_id,
                &path,
                Some(false),
                &state,
            )
            .await?;
        }
        installed_cf.insert(m.id);
        result.installed.push(path);
        if with_dependencies {
            for dep in file.dependencies.iter().filter(|d| d.relation_type == 3)
            {
                queue.push((dep.mod_id, None, false));
            }
        }
    }
    // Щоб список умісту одразу впізнав нові файли
    crate::state::sync_content_files(&instance_id, &state).await?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Збірки CurseForge. Архів збірки (manifest.json + overrides/) перетворюється
// в .mrpack (modrinth.index.json з прямими URL файлів з CF API + ті самі
// overrides) і йде в звичайний установник збірок лаунчера — тому примірник,
// оновлення й «Уміст збірки» працюють як для Modrinth-збірки.

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifest {
    minecraft: CfManifestMinecraft,
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    files: Vec<CfManifestFile>,
    #[serde(default)]
    overrides: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifestMinecraft {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<CfManifestLoader>,
}

#[derive(Deserialize)]
struct CfManifestLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    #[serde(default = "default_true")]
    required: bool,
}

fn default_true() -> bool {
    true
}

/// `forge-47.2.0` / `neoforge-21.1.72` / `fabric-0.15.11` → (ключ mrpack, версія)
fn loader_dependency(id: &str) -> Option<(&'static str, String)> {
    let (name, version) = id.split_once('-')?;
    let key = match name {
        "forge" => "forge",
        "neoforge" => "neoforge",
        "fabric" => "fabric-loader",
        "quilt" => "quilt-loader",
        _ => return None,
    };
    Some((key, version.to_string()))
}

/// Папка для файлу за класом мода (класи CF відомі з картки мода).
fn folder_for_class(class_id: Option<u32>) -> &'static str {
    match class_id {
        Some(12) => "resourcepacks",
        Some(6552) => "shaderpacks",
        Some(6945) => "datapacks",
        _ => "mods",
    }
}

/// Файли, які автори заборонили завантажувати стороннім (нема downloadUrl):
/// збірка ставиться без них, гравцю показуємо список.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurseForgeModpackPrepared {
    /// Готовий .mrpack у кеші лаунчера
    pub mrpack_path: String,
    pub name: String,
    pub version: String,
    pub game_version: String,
    pub loader: Option<String>,
    pub file_count: usize,
    /// Назви модів, для яких автор заборонив сторонні завантаження
    pub missing: Vec<String>,
    /// Іконка збірки, завантажена в кеш (для іконки примірника)
    pub icon_path: Option<String>,
}

/// Завантажити архів збірки з CurseForge і перетворити на .mrpack.
#[tracing::instrument]
pub async fn prepare_modpack(
    mod_id: u64,
    file_id: Option<u64>,
) -> crate::Result<CurseForgeModpackPrepared> {
    use std::io::{Read, Write};

    let state = State::get().await?;
    let key = require_key().await?;
    let m = get_mod(mod_id, false).await?;

    // Який файл збірки: заданий або найновіший реліз
    let file_id = match file_id {
        Some(id) => id,
        None => m
            .latest_files_indexes
            .iter()
            .filter(|f| f.release_type == 1)
            .map(|f| f.file_id)
            .max()
            .or_else(|| m.latest_files_indexes.iter().map(|f| f.file_id).max())
            .ok_or_else(|| {
                crate::ErrorKind::OtherError(format!(
                    "У збірки «{}» немає файлів на CurseForge",
                    m.name
                ))
            })?,
    };
    let (pack_file, archive_bytes, _) =
        download_file_bytes(&key, mod_id, file_id).await?;

    // manifest.json + overrides
    let (manifest, overrides): (CfManifest, Vec<(String, Vec<u8>)>) =
        tokio::task::spawn_blocking(move || -> crate::Result<_> {
            let cursor = std::io::Cursor::new(&archive_bytes[..]);
            let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
                crate::ErrorKind::InputError(format!(
                    "Архів збірки CurseForge пошкоджено: {e}"
                ))
            })?;
            let mut manifest: Option<CfManifest> = None;
            let mut raw_entries: Vec<(String, Vec<u8>)> = Vec::new();
            for i in 0..archive.len() {
                let mut entry = archive.by_index(i).map_err(|e| {
                    crate::ErrorKind::InputError(format!(
                        "Архів збірки CurseForge пошкоджено: {e}"
                    ))
                })?;
                if entry.is_dir() {
                    continue;
                }
                let name = entry.name().to_string();
                let mut bytes = Vec::with_capacity(entry.size() as usize);
                entry.read_to_end(&mut bytes)?;
                if name == "manifest.json" {
                    manifest = Some(serde_json::from_slice(&bytes)?);
                } else {
                    raw_entries.push((name, bytes));
                }
            }
            let manifest = manifest.ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "У архіві збірки CurseForge немає manifest.json"
                        .to_string(),
                )
            })?;
            let prefix = format!(
                "{}/",
                manifest.overrides.as_deref().unwrap_or("overrides")
            );
            let overrides = raw_entries
                .into_iter()
                .filter_map(|(name, bytes)| {
                    name.strip_prefix(&prefix)
                        .filter(|rest| !rest.is_empty())
                        .map(|rest| (rest.to_string(), bytes))
                })
                .collect();
            Ok((manifest, overrides))
        })
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()))??;

    // Файли: картки модів пачками (папка за класом), файли — по одному (URL, sha1, розмір)
    let mod_ids: Vec<String> = manifest
        .files
        .iter()
        .map(|f| f.project_id.to_string())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let mut class_by_mod: HashMap<u64, Option<u32>> = HashMap::new();
    let mut name_by_mod: HashMap<u64, String> = HashMap::new();
    for chunk in mod_ids.chunks(BATCH) {
        let ids: Vec<u64> =
            chunk.iter().filter_map(|s| s.parse().ok()).collect();
        let body = serde_json::json!({ "modIds": ids });
        let envelope: Envelope<Vec<CfModFullRaw>> =
            post_json(&key, "/mods", &body).await?;
        for raw in envelope.data {
            class_by_mod.insert(raw.id, raw.class_id);
            name_by_mod.insert(raw.id, raw.name);
        }
    }
    let file_ids: Vec<u64> = manifest.files.iter().map(|f| f.file_id).collect();
    let mut details_by_id: HashMap<u64, CfFileDetailsRaw> = HashMap::new();
    for chunk in file_ids.chunks(BATCH) {
        let body = serde_json::json!({ "fileIds": chunk });
        let envelope: Envelope<Vec<CfFileDetailsRaw>> =
            post_json(&key, "/mods/files", &body).await?;
        for f in envelope.data {
            details_by_id.insert(f.id, f);
        }
    }

    // Файли, які автор заборонив завантажувати через CF API, часто є на
    // Modrinth — шукаємо за sha1 і беремо посилання звідти
    let blocked_hashes: Vec<String> = manifest
        .files
        .iter()
        .filter_map(|e| details_by_id.get(&e.file_id))
        .filter(|d| d.download_url.is_none())
        .filter_map(|d| d.hashes.iter().find(|h| h.algo == 1))
        .map(|h| h.value.to_lowercase())
        .collect();
    let mut modrinth_url_by_hash: HashMap<String, String> = HashMap::new();
    if !blocked_hashes.is_empty() {
        let refs: Vec<&str> =
            blocked_hashes.iter().map(String::as_str).collect();
        if let Ok(found) = crate::state::CachedEntry::get_file_many(
            &refs,
            None,
            &state.pool,
            &state.api_semaphore,
        )
        .await
        {
            let version_ids: Vec<String> =
                found.iter().map(|f| f.version_id.clone()).collect();
            let version_refs: Vec<&str> =
                version_ids.iter().map(String::as_str).collect();
            if let Ok(versions) = crate::state::CachedEntry::get_version_many(
                &version_refs,
                None,
                &state.pool,
                &state.api_semaphore,
            )
            .await
            {
                for version in versions {
                    for file in version.files {
                        if let Some(sha1) = file.hashes.get("sha1") {
                            modrinth_url_by_hash
                                .insert(sha1.to_lowercase(), file.url.clone());
                        }
                    }
                }
            }
        }
    }

    let mut index_files = Vec::new();
    let mut missing = Vec::new();
    for entry in &manifest.files {
        let name = name_by_mod
            .get(&entry.project_id)
            .cloned()
            .unwrap_or_else(|| format!("CurseForge #{}", entry.project_id));
        let Some(details) = details_by_id.get(&entry.file_id) else {
            missing.push(name);
            continue;
        };
        let sha1 = details
            .hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.to_lowercase());
        let url = details.download_url.clone().or_else(|| {
            sha1.as_deref()
                .and_then(|h| modrinth_url_by_hash.get(h).cloned())
        });
        let Some(url) = url else {
            if entry.required {
                missing.push(name);
            }
            continue;
        };
        if !path_util::is_safe_file_name(&details.file_name) {
            missing.push(name);
            continue;
        }
        let folder = folder_for_class(
            class_by_mod.get(&entry.project_id).copied().flatten(),
        );
        let mut hashes = serde_json::Map::new();
        if let Some(sha1) = &sha1 {
            hashes
                .insert("sha1".into(), serde_json::Value::String(sha1.clone()));
        }
        index_files.push(serde_json::json!({
            "path": format!("{folder}/{}", details.file_name),
            "hashes": hashes,
            "downloads": [url],
            "fileSize": details.file_length,
        }));
    }

    let mut dependencies = serde_json::Map::new();
    dependencies.insert(
        "minecraft".into(),
        serde_json::Value::String(manifest.minecraft.version.clone()),
    );
    let loader = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .and_then(|l| loader_dependency(&l.id));
    if let Some((key_name, version)) = &loader {
        dependencies.insert(
            (*key_name).into(),
            serde_json::Value::String(version.clone()),
        );
    }
    let pack_name = if manifest.name.trim().is_empty() {
        m.name.clone()
    } else {
        manifest.name.clone()
    };
    let index = serde_json::json!({
        "game": "minecraft",
        "formatVersion": 1,
        "versionId": manifest.version,
        "name": pack_name,
        "summary": m.summary,
        "files": index_files,
        "dependencies": dependencies,
    });

    // .mrpack у кеші
    let dir = state.directories.caches_dir().join("terrarium-curseforge");
    crate::util::io::create_dir_all(&dir).await?;
    let stem = pack_file.file_name.trim_end_matches(".zip").to_string();
    let mrpack_path = dir.join(format!("{mod_id}-{file_id}-{stem}.mrpack"));
    let index_bytes = serde_json::to_vec_pretty(&index)?;
    let write_path = mrpack_path.clone();
    tokio::task::spawn_blocking(move || -> crate::Result<()> {
        let file = std::fs::File::create(&write_path)?;
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        writer
            .start_file("modrinth.index.json", options)
            .map_err(std::io::Error::from)?;
        writer.write_all(&index_bytes)?;
        for (rel, bytes) in overrides {
            writer
                .start_file(format!("overrides/{rel}"), options)
                .map_err(std::io::Error::from)?;
            writer.write_all(&bytes)?;
        }
        writer.finish().map_err(std::io::Error::from)?;
        Ok(())
    })
    .await
    .map_err(|e| crate::ErrorKind::OtherError(e.to_string()))??;

    // Іконка збірки — у кеш, щоб примірник мав її як Modrinth-збірка
    let mut icon_path = None;
    if let Some(url) = m.icon_url.as_deref() {
        let ext = url
            .rsplit('.')
            .next()
            .filter(|e| {
                e.len() <= 4 && e.chars().all(|c| c.is_ascii_alphanumeric())
            })
            .unwrap_or("png");
        let target = dir.join(format!("{mod_id}-icon.{ext}"));
        if let Ok(response) = CLIENT.get(url).send().await
            && response.status().is_success()
            && let Ok(bytes) = response.bytes().await
            && crate::util::io::write(&target, &bytes).await.is_ok()
        {
            icon_path = Some(target.to_string_lossy().to_string());
        }
    }

    Ok(CurseForgeModpackPrepared {
        mrpack_path: mrpack_path.to_string_lossy().to_string(),
        icon_path,
        name: pack_name,
        version: manifest.version,
        game_version: manifest.minecraft.version,
        loader: loader.map(|(k, _)| k.to_string()),
        file_count: manifest.files.len(),
        missing,
    })
}
