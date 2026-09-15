//! Terrarium: наші збірки, що живуть у GitHub Releases.
//!
//! Збірок дві — [`PackKind::Client`] для гравців і [`PackKind::Server`] для
//! сервера. Кожна має свій репозиторій; актуальна версія — останній реліз
//! із прикріпленим `.mrpack`. Тут: запит релізу, завантаження пакета в кеш,
//! локальний стан (що встановлено, хто адмін) і публікація нових релізів.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::instances::commands;
pub use crate::state::instances::commands::ContentGroup;
use crate::state::{EditInstance, InstanceLink};
use crate::util::fetch::REQWEST_CLIENT;
use crate::util::io;

/// Репозиторій клієнтської збірки — те, що встановлюють гравці.
pub const CLIENT_REPO: &str = "Kemzino/TerrariumCreate";
/// Репозиторій серверної збірки — редагує лише адмін, гравцям не показується.
pub const SERVER_REPO: &str = "Kemzino/TerrariumCreateServer";

/// Токен лише на читання репозиторіїв — щоб гравці могли бачити релізи
/// приватного репозиторію. Задається у `.env` як `TERRARIUM_READ_TOKEN` під час
/// збірки лаунчера; без нього працюють лише публічні репозиторії та адмін-ключ.
const READ_TOKEN: Option<&str> = option_env!("TERRARIUM_READ_TOKEN");

const STATE_FILE: &str = "terrarium.json";
const CACHE_FOLDER: &str = "terrarium";
/// Службова папка всередині примірника: іконка збірки їде в пакеті як
/// `.terrarium/icon.<ext>`, бо формат .mrpack іконки не передбачає.
const BRANDING_DIR: &str = ".terrarium";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PackKind {
    #[default]
    Client,
    Server,
}

impl PackKind {
    pub fn repo(self) -> &'static str {
        match self {
            PackKind::Client => CLIENT_REPO,
            PackKind::Server => SERVER_REPO,
        }
    }

    fn cache_folder(self) -> &'static str {
        match self {
            PackKind::Client => "client",
            PackKind::Server => "server",
        }
    }
}

/// Канал релізів: стабільний (для всіх) або тестовий (pre-release на GitHub,
/// бачать лише адміни й тестери).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    #[default]
    Stable,
    Test,
}

/// Роль, яку дає ключ доступу: адмін (push у репозиторії) або тестер
/// (лише читання приватного серверного репозиторію → бачить тестові релізи).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessRole {
    Admin,
    Tester,
}

/// Що встановлено для однієї збірки.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PackState {
    /// Ідентифікатор інстансу, у який встановлено збірку.
    pub instance_id: Option<String>,
    /// Тег релізу GitHub, який зараз встановлено.
    pub installed_tag: Option<String>,
    /// Файли примірника, які адмін не публікує (особисті моди тощо).
    /// Шляхи відносно примірника, напр. `mods/xaeros_minimap.jar`.
    #[serde(default)]
    pub excluded_paths: Vec<String>,
}

/// Локальний стан лаунчера. Зберігається у `settings_dir/terrarium.json`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TerrariumState {
    /// Ключ адміністратора — GitHub-токен із правом запису в репозиторії збірок.
    /// Задається в налаштуваннях; без нього функції публікації недоступні.
    #[serde(default)]
    pub admin_token: Option<String>,
    /// Роль, визначена при перевірці ключа (кешується, щоб не ходити в GitHub).
    #[serde(default)]
    pub role: Option<AccessRole>,
    /// Яку збірку зараз показує головна (перемикає лише адмін).
    #[serde(default)]
    pub active_pack: PackKind,
    /// Який канал показує головна (перемикають адміни й тестери).
    #[serde(default)]
    pub active_channel: Channel,
    #[serde(default)]
    pub client: PackState,
    #[serde(default)]
    pub server: PackState,
    /// Окремі примірники для тестових версій — щоб тест не ламав основну гру.
    #[serde(default)]
    pub client_test: PackState,
    #[serde(default)]
    pub server_test: PackState,

    // Поля з часів однієї збірки — читаємо для міграції, не пишемо.
    #[serde(default, skip_serializing)]
    instance_id: Option<String>,
    #[serde(default, skip_serializing)]
    installed_tag: Option<String>,
    #[serde(default, skip_serializing)]
    excluded_paths: Vec<String>,
}

impl TerrariumState {
    pub fn pack(&self, kind: PackKind) -> &PackState {
        match kind {
            PackKind::Client => &self.client,
            PackKind::Server => &self.server,
        }
    }

    pub fn pack_mut(&mut self, kind: PackKind) -> &mut PackState {
        match kind {
            PackKind::Client => &mut self.client,
            PackKind::Server => &mut self.server,
        }
    }

    pub fn pack_for(&self, kind: PackKind, channel: Channel) -> &PackState {
        match (kind, channel) {
            (PackKind::Client, Channel::Stable) => &self.client,
            (PackKind::Server, Channel::Stable) => &self.server,
            (PackKind::Client, Channel::Test) => &self.client_test,
            (PackKind::Server, Channel::Test) => &self.server_test,
        }
    }

    pub fn pack_for_mut(
        &mut self,
        kind: PackKind,
        channel: Channel,
    ) -> &mut PackState {
        match (kind, channel) {
            (PackKind::Client, Channel::Stable) => &mut self.client,
            (PackKind::Server, Channel::Stable) => &mut self.server,
            (PackKind::Client, Channel::Test) => &mut self.client_test,
            (PackKind::Server, Channel::Test) => &mut self.server_test,
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == Some(AccessRole::Admin)
    }

    /// Переносить старі верхньорівневі поля в `client`, якщо там ще порожньо.
    fn migrate_legacy(mut self) -> Self {
        // Ключ, збережений до появи ролей, був адмінським
        if self.admin_token.is_some() && self.role.is_none() {
            self.role = Some(AccessRole::Admin);
        }
        if self.client.instance_id.is_none() && self.instance_id.is_some() {
            self.client.instance_id = self.instance_id.take();
            self.client.installed_tag = self.installed_tag.take();
            self.client.excluded_paths =
                std::mem::take(&mut self.excluded_paths);
        }
        self
    }
}

/// Останній реліз збірки з GitHub, зведений до того, що потрібно лаунчеру.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrariumRelease {
    pub pack: PackKind,
    /// Ідентифікатор релізу на GitHub (потрібен для «поширити для всіх»).
    pub id: u64,
    /// Тестовий реліз (pre-release) — його бачать лише адміни й тестери.
    pub prerelease: bool,
    pub html_url: String,
    pub tag: String,
    pub name: String,
    pub body: Option<String>,
    pub published_at: Option<String>,
    pub mrpack_name: String,
    pub mrpack_url: String,
    pub mrpack_size: u64,
    /// Ідентифікатор asset-а — через нього качаємо з приватного репозиторію.
    pub mrpack_asset_id: u64,
}

#[derive(Deserialize)]
struct GithubRelease {
    id: u64,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    html_url: String,
    tag_name: String,
    name: Option<String>,
    body: Option<String>,
    published_at: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Deserialize)]
struct GithubAsset {
    id: u64,
    name: String,
    browser_download_url: String,
    size: u64,
}

fn state_path(state: &State) -> PathBuf {
    state.directories.settings_dir.join(STATE_FILE)
}

#[tracing::instrument]
pub async fn get_state() -> crate::Result<TerrariumState> {
    let state = State::get().await?;
    let path = state_path(&state);
    if !path.exists() {
        return Ok(TerrariumState::default());
    }
    let raw = io::read(&path).await?;
    let parsed: TerrariumState = serde_json::from_slice(&raw)?;
    Ok(parsed.migrate_legacy())
}

#[tracing::instrument]
pub async fn set_state(new_state: TerrariumState) -> crate::Result<()> {
    let state = State::get().await?;
    io::write(state_path(&state), serde_json::to_vec_pretty(&new_state)?)
        .await?;
    Ok(())
}

/// Токен для читання: адмін-ключ, якщо є, інакше вбудований токен гравця.
fn read_token(state: &TerrariumState) -> Option<String> {
    state
        .admin_token
        .clone()
        .or_else(|| READ_TOKEN.map(str::to_string))
}

fn with_token(
    request: reqwest::RequestBuilder,
    token: Option<&str>,
) -> reqwest::RequestBuilder {
    match token {
        Some(token) => request.bearer_auth(token),
        None => request,
    }
}

fn github_error(
    context: &str,
    status: reqwest::StatusCode,
    body: &str,
) -> crate::Error {
    crate::ErrorKind::OtherError(format!(
        "{context}: GitHub відповів {status} — {body}"
    ))
    .into()
}

async fn github_json<T: serde::de::DeserializeOwned>(
    request: reqwest::RequestBuilder,
    context: &str,
) -> crate::Result<T> {
    let response = request
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(github_error(context, status, &text));
    }
    Ok(serde_json::from_str(&text)?)
}

/// GET із токеном; якщо GitHub відповів 401 (ключ відкликано/перевидано) —
/// повторюємо без токена: публічний репозиторій читається і так.
async fn github_get_json<T: serde::de::DeserializeOwned>(
    url: &str,
    token: Option<&str>,
    context: &str,
) -> crate::Result<T> {
    let first: crate::Result<T> =
        github_json(with_token(REQWEST_CLIENT.get(url), token), context).await;
    match first {
        Err(err) if token.is_some() && err.to_string().contains("401") => {
            tracing::warn!(
                "Terrarium: ключ доступу відхилено (401), пробую без нього"
            );
            github_json(REQWEST_CLIENT.get(url), context).await
        }
        other => other,
    }
}

/// Запитує останній реліз збірки і знаходить у ньому `.mrpack`.
/// `Stable` — останній звичайний реліз (`/releases/latest`, pre-release
/// GitHub сюди не включає); `Test` — найновіший реліз узагалі, включно з
/// тестовими.
#[tracing::instrument]
pub async fn fetch_latest_release(
    pack: PackKind,
    channel: Channel,
) -> crate::Result<TerrariumRelease> {
    let repo = pack.repo();
    let token = read_token(&get_state().await?);

    let release: GithubRelease = match channel {
        Channel::Stable => {
            let url =
                format!("https://api.github.com/repos/{repo}/releases/latest");
            github_get_json(&url, token.as_deref(), "Перевірка оновлень")
                .await?
        }
        Channel::Test => {
            let url = format!(
                "https://api.github.com/repos/{repo}/releases?per_page=30"
            );
            let releases: Vec<GithubRelease> = github_get_json(
                &url,
                token.as_deref(),
                "Перевірка тестових версій",
            )
            .await?;
            // GitHub віддає від найновішого; чернетки пропускаємо
            releases.into_iter().find(|r| !r.draft).ok_or_else(|| {
                crate::ErrorKind::OtherError(format!(
                    "У репозиторії {repo} ще немає релізів"
                ))
            })?
        }
    };

    let asset = release
        .assets
        .into_iter()
        .find(|asset| asset.name.ends_with(".mrpack"))
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "У релізі {} репозиторію {repo} немає файлу .mrpack",
                release.tag_name
            ))
        })?;

    Ok(TerrariumRelease {
        pack,
        id: release.id,
        prerelease: release.prerelease,
        html_url: release.html_url,
        name: release.name.unwrap_or_else(|| release.tag_name.clone()),
        tag: release.tag_name,
        body: release.body,
        published_at: release.published_at,
        mrpack_name: asset.name,
        mrpack_url: asset.browser_download_url,
        mrpack_size: asset.size,
        mrpack_asset_id: asset.id,
    })
}

/// Завантажує `.mrpack` релізу в кеш лаунчера і повертає шлях до файлу.
/// Повторний виклик для того самого тегу нічого не качає.
/// Качаємо через API asset-ів, а не `browser_download_url`, — так працює і
/// для приватного репозиторію (GitHub віддає редирект на сховище).
#[tracing::instrument]
pub async fn download_release(
    release: TerrariumRelease,
) -> crate::Result<PathBuf> {
    let state = State::get().await?;
    let dir = state
        .directories
        .caches_dir()
        .join(CACHE_FOLDER)
        .join(release.pack.cache_folder());
    let path = dir.join(format!("{}-{}", release.tag, release.mrpack_name));

    if path.exists() {
        return Ok(path);
    }

    let token = read_token(&get_state().await?);
    let url = format!(
        "https://api.github.com/repos/{}/releases/assets/{}",
        release.pack.repo(),
        release.mrpack_asset_id
    );
    let asset_request = |token: Option<&str>| {
        with_token(REQWEST_CLIENT.get(&url), token)
            .header("Accept", "application/octet-stream")
            .header("X-GitHub-Api-Version", "2022-11-28")
    };
    let mut response = asset_request(token.as_deref()).send().await?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED && token.is_some()
    {
        // Ключ відкликано — публічний репозиторій віддасть файл і без нього
        response = asset_request(None).send().await?;
    }
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(github_error("Завантаження збірки", status, &text));
    }
    let bytes = response.bytes().await?;

    io::create_dir_all(&dir).await?;
    io::write(&path, &bytes).await?;
    Ok(path)
}

// ---------------------------------------------------------------------------
// Адмін-режим: перевірка ключа та публікація нової версії збірки.
// ---------------------------------------------------------------------------

/// Хто стоїть за адмін-ключем і в які репозиторії він може пушити.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminInfo {
    pub login: String,
    pub client_repo: String,
    pub can_push_client: bool,
    pub server_repo: String,
    pub can_push_server: bool,
    /// Ключ читає приватний серверний репозиторій → достатньо для тестера.
    pub can_read_server: bool,
    /// Роль за цим ключем; `None` — ключ дійсний, але доступу до збірок не дає.
    pub role: Option<AccessRole>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    pub pack: PackKind,
    pub instance_id: String,
    /// Тег релізу, напр. `v1.2.0`.
    pub tag: String,
    pub name: String,
    pub body: Option<String>,
    /// Спершу в тест (pre-release; бачать лише адміни й тестери).
    #[serde(default)]
    pub prerelease: bool,
    /// Що НЕ включати в цей реліз (і запам'ятати на майбутнє).
    #[serde(default)]
    pub excluded: Vec<String>,
}

/// Один файл примірника і його стосунок до останнього релізу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishCandidate {
    /// Шлях у примірнику (для правил експорту; моди можуть бути в групі).
    pub path: String,
    /// Шлях у пакеті — плоский `mods/x.jar`; за ним порівнюємо з релізом.
    pub pack_path: String,
    /// Група мода (`mods/<Група>/`), якщо є.
    pub group: Option<String>,
    pub file_name: String,
    /// Перший сегмент шляху: `mods`, `config`, `kubejs`…
    pub folder: String,
    pub size: Option<u64>,
    /// Є в останньому опублікованому релізі.
    pub in_release: bool,
    /// Є в релізі, але вміст відрізняється (лише для файлів, що лежать у пакеті як є).
    pub changed: bool,
    /// Адмін раніше свідомо виключив цей файл.
    pub excluded: bool,
    /// Файл вимкнено (`.disabled`) — за замовчуванням не публікуємо.
    pub disabled: bool,
}

/// «Ядро» збірки: версія гри та завантажувача.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackCore {
    pub game_version: String,
    /// `neoforge`, `forge`, `fabric`, `quilt` або `vanilla`.
    pub loader: String,
    pub loader_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishPreview {
    /// Тег релізу, з яким порівнюємо (None — релізів ще нема).
    pub release_tag: Option<String>,
    /// База порівняння — тестовий реліз (ще не поширений для всіх).
    pub release_prerelease: bool,
    /// Ядро примірника — те, що піде в реліз.
    pub core: PackCore,
    /// Ядро останнього релізу (None — релізів ще нема).
    pub release_core: Option<PackCore>,
    /// Назва примірника — стане назвою збірки у гравців.
    pub name: String,
    pub release_name: Option<String>,
    /// Іконка примірника відрізняється від тієї, що в релізі (або її там нема).
    pub icon_changed: bool,
    pub candidates: Vec<PublishCandidate>,
    /// Були в релізі, але в примірнику їх уже немає — випадуть зі збірки.
    pub removed_from_instance: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRelease {
    pub pack: PackKind,
    pub prerelease: bool,
    pub tag: String,
    pub html_url: String,
    pub asset_name: String,
}

#[derive(Deserialize)]
struct GithubUser {
    login: String,
}

#[derive(Deserialize)]
struct GithubRepo {
    permissions: Option<GithubRepoPermissions>,
}

#[derive(Deserialize)]
struct GithubRepoPermissions {
    push: bool,
}

/// Чи читає токен репозиторій (для приватного це і є перевірка доступу).
async fn can_read(token: &str, repo: &str) -> bool {
    let info: crate::Result<GithubRepo> = github_json(
        REQWEST_CLIENT
            .get(format!("https://api.github.com/repos/{repo}"))
            .bearer_auth(token),
        "Доступ до репозиторію",
    )
    .await;
    info.is_ok()
}

#[derive(Deserialize)]
struct GithubCreatedRelease {
    id: u64,
    html_url: String,
}

/// Чи може САМЕ ЦЕЙ токен писати в репозиторій.
///
/// `permissions.push` з `GET /repos/{repo}` показує роль акаунта, а не права
/// токена: власник із read-only fine-grained PAT усе одно бачить `push: true`.
/// Тому робимо «сухий» запис: `PATCH` останнього релізу з порожнім тілом —
/// нічого не змінює, але токен без права запису отримує 403, а з правом — 200.
/// (Створення релізу з порожнім тілом не годиться: GitHub валідує тіло раніше,
/// ніж перевіряє рівень доступу, і віддає 422 навіть read-only токену.)
async fn can_push(token: &str, repo: &str) -> bool {
    let account_can_push = github_json::<GithubRepo>(
        REQWEST_CLIENT
            .get(format!("https://api.github.com/repos/{repo}"))
            .bearer_auth(token),
        "Доступ до репозиторію",
    )
    .await
    .ok()
    .and_then(|repo| repo.permissions)
    .is_some_and(|p| p.push);
    if !account_can_push {
        return false;
    }
    let latest: Option<GithubRelease> = github_json(
        REQWEST_CLIENT
            .get(format!(
                "https://api.github.com/repos/{repo}/releases/latest"
            ))
            .bearer_auth(token),
        "Останній реліз",
    )
    .await
    .ok();
    let Some(latest) = latest else {
        // Релізів ще нема — покладаємось на роль акаунта
        return true;
    };
    let probe = REQWEST_CLIENT
        .patch(format!(
            "https://api.github.com/repos/{repo}/releases/{}",
            latest.id
        ))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&serde_json::json!({}))
        .send()
        .await;
    match probe {
        Ok(response) => response.status().is_success(),
        Err(_) => false,
    }
}

/// Перевіряє токен: хто це і в які репозиторії збірок він може писати.
#[tracing::instrument(skip(token))]
pub async fn verify_admin_token(token: String) -> crate::Result<AdminInfo> {
    let user: GithubUser = github_json(
        REQWEST_CLIENT
            .get("https://api.github.com/user")
            .bearer_auth(&token),
        "Перевірка ключа",
    )
    .await?;

    let can_push_client = can_push(&token, CLIENT_REPO).await;
    let can_push_server = can_push(&token, SERVER_REPO).await;
    let can_read_server =
        can_push_server || can_read(&token, SERVER_REPO).await;
    let role = if can_push_client || can_push_server {
        Some(AccessRole::Admin)
    } else if can_read_server {
        Some(AccessRole::Tester)
    } else {
        None
    };

    Ok(AdminInfo {
        login: user.login,
        client_repo: CLIENT_REPO.to_string(),
        can_push_client,
        server_repo: SERVER_REPO.to_string(),
        can_push_server,
        can_read_server,
        role,
    })
}

/// «Поширити для всіх»: зняти з тестового релізу позначку pre-release —
/// він стає `latest`, і звичайні гравці отримують оновлення.
#[tracing::instrument]
pub async fn promote_release(
    pack: PackKind,
    release_id: u64,
) -> crate::Result<TerrariumRelease> {
    let terrarium = get_state().await?;
    let token = terrarium.admin_token.clone().ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Адмін-ключ не задано — додай його в налаштуваннях".to_string(),
        )
    })?;
    let repo = pack.repo();
    let _: serde_json::Value = github_json(
        REQWEST_CLIENT
            .patch(format!(
                "https://api.github.com/repos/{repo}/releases/{release_id}"
            ))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "prerelease": false,
                "make_latest": "true",
            })),
        "Поширення релізу",
    )
    .await?;
    fetch_latest_release(pack, Channel::Stable).await
}

/// Експортує інстанс у `.mrpack` і публікує його як новий реліз GitHub.
/// Після успіху локальний стан вважає цю версію встановленою.
#[tracing::instrument]
pub async fn publish_release(
    request: PublishRequest,
) -> crate::Result<PublishedRelease> {
    let state = State::get().await?;
    let mut terrarium = get_state().await?;
    let token = terrarium.admin_token.clone().ok_or_else(|| {
        crate::ErrorKind::OtherError(
            "Адмін-ключ не задано — додай його в налаштуваннях".to_string(),
        )
    })?;
    let repo = request.pack.repo();
    let tag = request.tag.trim().to_string();
    if tag.is_empty() {
        return Err(crate::ErrorKind::OtherError(
            "Вкажи тег версії, напр. v1.0.1".to_string(),
        )
        .into());
    }

    if commands::flatten_reason(
        &crate::api::instance::get_full_path(&request.instance_id).await?,
    )
    .is_some()
    {
        return Err(crate::ErrorKind::OtherError(
            "Закрий гру (або дочекайся оновлення) перед публікацією"
                .to_string(),
        )
        .into());
    }

    // 1. Експорт — той самий набір файлів, що й у ручному експорті за замовчуванням.
    let candidates =
        crate::api::instance::get_pack_export_candidates(&request.instance_id)
            .await?;
    let included = candidates
        .into_iter()
        .filter(|c| c.default_selected && !c.disabled)
        .map(|c| c.path.as_str().to_string())
        .collect::<Vec<_>>();

    // Іконка примірника — у пакет як `.terrarium/icon.<ext>`; після експорту прибираємо.
    let instance_dir =
        crate::api::instance::get_full_path(&request.instance_id).await?;
    let branding_dir = instance_dir.join(BRANDING_DIR);
    let mut included = included;
    if let Some(icon) = crate::api::instance::get(&request.instance_id)
        .await?
        .and_then(|m| m.instance.icon_path)
    {
        let icon = PathBuf::from(icon);
        if icon.is_file() {
            let ext = icon
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_ascii_lowercase();
            io::create_dir_all(&branding_dir).await?;
            io::copy(&icon, branding_dir.join(format!("icon.{ext}"))).await?;
            included.push(format!("{BRANDING_DIR}/icon.{ext}"));
        }
    }

    let asset_name =
        format!("terrarium-{}-{tag}.mrpack", request.pack.cache_folder());
    let dir = state
        .directories
        .caches_dir()
        .join(CACHE_FOLDER)
        .join("publish");
    io::create_dir_all(&dir).await?;
    let path = dir.join(&asset_name);
    let exported = crate::api::instance::export_mrpack(
        &request.instance_id,
        path.clone(),
        included,
        request.excluded.clone(),
        Some(tag.trim_start_matches('v').to_string()),
        request.body.clone(),
        Some(request.name.clone()),
    )
    .await;
    if branding_dir.exists() {
        let _ = io::remove_dir_all(&branding_dir).await;
    }
    exported?;

    // 2. Реліз. GitHub сам створить тег на гілці за замовчуванням, якщо його ще нема.
    let created: GithubCreatedRelease = github_json(
        REQWEST_CLIENT
            .post(format!("https://api.github.com/repos/{repo}/releases"))
            .bearer_auth(&token)
            .json(&serde_json::json!({
                "tag_name": tag,
                "name": request.name,
                "body": request.body.clone().unwrap_or_default(),
                "prerelease": request.prerelease,
            })),
        "Створення релізу",
    )
    .await?;

    // 3. Файл збірки як asset релізу.
    let bytes = io::read(&path).await?;
    let response = REQWEST_CLIENT
        .post(format!(
            "https://uploads.github.com/repos/{repo}/releases/{}/assets?name={}",
            created.id,
            urlencoding::encode(&asset_name)
        ))
        .bearer_auth(&token)
        .header("Accept", "application/vnd.github+json")
        .header("Content-Type", "application/octet-stream")
        .body(bytes)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(github_error("Завантаження .mrpack", status, &text));
    }

    // Примірник тепер = опублікована версія. Записуємо це в його прив'язку
    // (imported_modpack), бо саме звідти лаунчер бере «встановлену» версію
    // для картки «Надається» і для порівняння з останнім релізом.
    if let Some(metadata) =
        crate::api::instance::get(&request.instance_id).await?
    {
        let (name, project_id, version_id) = match metadata.link {
            InstanceLink::ImportedModpack {
                name,
                project_id,
                version_id,
                ..
            } => (name, project_id, version_id),
            _ => (None, None, None),
        };
        crate::api::instance::edit(
            &request.instance_id,
            EditInstance {
                link: Some(InstanceLink::ImportedModpack {
                    project_id,
                    version_id,
                    name: name.or_else(|| Some(request.name.clone())),
                    version_number: Some(
                        tag.trim_start_matches('v').to_string(),
                    ),
                    filename: Some(asset_name.clone()),
                }),
                ..EditInstance::default()
            },
        )
        .await?;
    }

    // Адмін щойно опублікував саме те, що в нього встановлено; виключення
    // запам'ятовуємо, щоб наступного разу не пропонувати особисті моди знову.
    // Виключення спільні для каналів; примірник/тег — у каналі, куди публікували.
    terrarium.pack_mut(request.pack).excluded_paths = request
        .excluded
        .iter()
        .map(|p| commands::flat_path(p))
        .collect();
    let channel = if request.prerelease {
        Channel::Test
    } else {
        Channel::Stable
    };
    let pack_state = terrarium.pack_for_mut(request.pack, channel);
    pack_state.instance_id = Some(request.instance_id);
    pack_state.installed_tag = Some(tag.clone());
    set_state(terrarium).await?;

    Ok(PublishedRelease {
        pack: request.pack,
        prerelease: request.prerelease,
        tag,
        html_url: created.html_url,
        asset_name,
    })
}

/// Усі файли примірника, що йдуть у пакет, з позначками: чи є в останньому
/// релізі, чи змінились, чи виключені адміном раніше. Основа для вибору
/// «що саме оновлюємо».
#[tracing::instrument]
pub async fn publish_preview(
    pack: PackKind,
    instance_id: String,
) -> crate::Result<PublishPreview> {
    let terrarium = get_state().await?;
    let excluded: HashSet<String> = terrarium
        .pack(pack)
        .excluded_paths
        .iter()
        .cloned()
        .collect();

    // Файли останнього релізу: посилання з modrinth.index.json (без вмісту)
    // та overrides (з sha1 вмісту — щоб бачити змінені конфіги).
    let mut release_tag = None;
    let mut release_core = None;
    let mut release_name = None;
    let mut release_files = HashMap::<String, Option<String>>::new();
    let mut release_prerelease = false;
    // База порівняння — найновіший реліз, включно з тестовим
    if let Ok(release) = fetch_latest_release(pack, Channel::Test).await {
        release_prerelease = release.prerelease;
        let path = download_release(release.clone()).await?;
        release_tag = Some(release.tag);
        let contents =
            tokio::task::spawn_blocking(move || read_release_files(&path))
                .await??;
        release_files = contents.files;
        release_core = contents.core;
        release_name = contents.name;
    }

    let metadata =
        crate::api::instance::get(&instance_id)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::OtherError(format!(
                    "Примірник {instance_id} не знайдено"
                ))
            })?;
    let core = PackCore {
        game_version: metadata.applied_content_set.game_version.clone(),
        loader: metadata.applied_content_set.loader.as_str().to_string(),
        loader_version: metadata.applied_content_set.loader_version.clone(),
    };

    // Іконка: порівнюємо sha1 файлу іконки примірника з `.terrarium/icon.*` у релізі.
    let release_icon_sha1 = release_files
        .iter()
        .find(|(k, _)| k.starts_with(&format!("{BRANDING_DIR}/icon")))
        .and_then(|(_, v)| v.clone());
    let icon_changed = match metadata.instance.icon_path.as_deref() {
        Some(icon) => {
            let icon = PathBuf::from(icon);
            let local = tokio::task::spawn_blocking(move || sha1_of(&icon))
                .await?
                .ok();
            local.is_some() && local != release_icon_sha1
        }
        None => false,
    };

    // Обходимо ті самі папки, що й експорт за замовчуванням.
    let roots = crate::api::instance::get_pack_export_candidates(&instance_id)
        .await?
        .into_iter()
        .filter(|c| c.default_selected && !c.disabled)
        .collect::<Vec<_>>();
    let mut entries = Vec::new();
    let mut queue: Vec<crate::instance::PackExportCandidate> = roots;
    while let Some(entry) = queue.pop() {
        match entry.kind {
            crate::instance::PackExportCandidateType::Directory => {
                let children =
                    crate::api::instance::get_pack_export_candidates_for_parent(
                        &instance_id,
                        Some(entry.path.clone()),
                    )
                    .await?;
                queue.extend(children);
            }
            crate::instance::PackExportCandidateType::File => {
                entries.push(entry)
            }
        }
    }

    let instance_dir =
        crate::api::instance::get_full_path(&instance_id).await?;
    let mut seen = HashSet::<String>::new();
    let mut candidates = Vec::new();
    for entry in entries {
        let path = entry.path.as_str().to_string();
        if path.starts_with(&format!("{BRANDING_DIR}/")) {
            continue;
        }
        if path == format!("mods/{}", commands::README_FILE)
            || path == format!("mods/{}", commands::FLATTEN_FILE)
        {
            continue;
        }
        let file_name = path.rsplit('/').next().unwrap_or(&path).to_string();
        let folder = path.split('/').next().unwrap_or("").to_string();
        let pack_path = commands::flat_path(&path);
        let group = commands::group_of(&path).map(str::to_string);
        seen.insert(pack_path.clone());
        let in_release = release_files.contains_key(&pack_path);
        // Порівнюємо вміст лише з тим, що в релізі лежить файлом (overrides);
        // моди за посиланням не мають вмісту в пакеті — для них changed = false.
        let changed = match release_files.get(&pack_path) {
            Some(Some(release_sha1)) => {
                let full = instance_dir.join(&path);
                let local = tokio::task::spawn_blocking(move || sha1_of(&full))
                    .await?
                    .ok();
                local.as_deref() != Some(release_sha1.as_str())
            }
            _ => false,
        };
        candidates.push(PublishCandidate {
            in_release,
            changed,
            excluded: excluded.contains(&pack_path),
            disabled: file_name.ends_with(".disabled"),
            size: entry.size,
            file_name,
            folder,
            path,
            pack_path,
            group,
        });
    }
    candidates
        .sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));

    let mut removed_from_instance: Vec<String> = release_files
        .into_keys()
        .filter(|p| {
            !seen.contains(p) && !p.starts_with(&format!("{BRANDING_DIR}/"))
        })
        .collect();
    removed_from_instance.sort();

    Ok(PublishPreview {
        release_tag,
        release_prerelease,
        core,
        release_core,
        name: metadata.instance.name.clone(),
        release_name,
        icon_changed,
        candidates,
        removed_from_instance,
    })
}

fn sha1_of(path: &Path) -> std::io::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = sha1_smol::Sha1::new();
    let mut buf = [0u8; 1 << 16];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.digest().to_string())
}

fn zip_error(error: zip::result::ZipError) -> crate::Error {
    crate::ErrorKind::OtherError(format!("Пошкоджений .mrpack: {error}")).into()
}

struct ReleaseContents {
    files: HashMap<String, Option<String>>,
    core: Option<PackCore>,
    name: Option<String>,
}

/// Файли всередині `.mrpack` (overrides — із sha1 вмісту, файли з маніфесту —
/// без нього), ядро з `dependencies` і назва збірки.
fn read_release_files(path: &Path) -> crate::Result<ReleaseContents> {
    use std::io::Read;

    #[derive(Deserialize)]
    struct Index {
        files: Vec<IndexFile>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        dependencies: HashMap<String, String>,
    }
    #[derive(Deserialize)]
    struct IndexFile {
        path: String,
    }

    let file = std::fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_error)?;
    let mut files = HashMap::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_error)?;
        let name = entry.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        for prefix in ["overrides/", "client-overrides/", "server-overrides/"] {
            if let Some(rest) = name.strip_prefix(prefix) {
                let mut hasher = sha1_smol::Sha1::new();
                let mut buf = [0u8; 1 << 16];
                loop {
                    let n = entry.read(&mut buf)?;
                    if n == 0 {
                        break;
                    }
                    hasher.update(&buf[..n]);
                }
                files.insert(
                    rest.to_string(),
                    Some(hasher.digest().to_string()),
                );
                break;
            }
        }
    }
    let mut core = None;
    let mut name = None;
    if let Ok(mut entry) = archive.by_name("modrinth.index.json") {
        let mut raw = String::new();
        entry.read_to_string(&mut raw)?;
        let index: Index = serde_json::from_str(&raw)?;
        name = index.name.clone();
        for f in index.files {
            files.entry(f.path).or_insert(None);
        }
        let deps = &index.dependencies;
        let loader = ["neoforge", "forge", "fabric-loader", "quilt-loader"]
            .into_iter()
            .find(|k| deps.contains_key(*k));
        core = deps.get("minecraft").map(|game_version| PackCore {
            game_version: game_version.clone(),
            loader: match loader {
                Some("fabric-loader") => "fabric".to_string(),
                Some("quilt-loader") => "quilt".to_string(),
                Some(other) => other.to_string(),
                None => "vanilla".to_string(),
            },
            loader_version: loader.and_then(|k| deps.get(k).cloned()),
        });
    }
    Ok(ReleaseContents { files, core, name })
}

/// Після встановлення/оновлення збірки: якщо в пакеті приїхала іконка
/// (`.terrarium/icon.*`) — ставимо її примірнику й прибираємо службову папку.
/// Повертає `true`, якщо іконку застосовано.
#[tracing::instrument]
pub async fn apply_pack_branding(instance_id: String) -> crate::Result<bool> {
    let state = State::get().await?;
    let instance_dir =
        crate::api::instance::get_full_path(&instance_id).await?;

    // Групи модів: спершу повертаємо особисті моди туди, де вони були до
    // оновлення (див. prepare_pack_update), потім розкладаємо моди збірки
    // за групами адміна з пакета.
    commands::restore_after_pack_update(&instance_id, &state).await?;
    let groups_file = instance_dir.join(commands::PACK_GROUPS_FILE);
    if groups_file.is_file() {
        let raw = io::read(&groups_file).await?;
        let groups: std::collections::BTreeMap<String, String> =
            serde_json::from_slice(&raw).unwrap_or_default();
        crate::state::sync_content_files(&instance_id, &state).await?;
        commands::apply_pack_groups(&instance_id, &groups, &state).await?;
        let _ = io::remove_file(&groups_file).await;
    }

    let branding_dir = instance_dir.join(BRANDING_DIR);
    if !branding_dir.is_dir() {
        return Ok(false);
    }
    let icon = std::fs::read_dir(&branding_dir)?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_file()
                && p.file_stem().and_then(|s| s.to_str()) == Some("icon")
        });
    let applied = match icon {
        Some(icon) => {
            crate::api::instance::edit_icon(&instance_id, Some(&icon)).await?;
            true
        }
        None => false,
    };
    let _ = io::remove_dir_all(&branding_dir).await;
    Ok(applied)
}

// ---------------------------------------------------------------------------
// Групи модів (підпапки `mods/<Група>/`)

/// Перед оновленням збірки: моди з груп — у корінь, щоб theseus бачив їх за
/// тими шляхами, що в попередньому пакеті. Назад — у `apply_pack_branding`.
#[tracing::instrument]
pub async fn prepare_pack_update(instance_id: String) -> crate::Result<()> {
    let state = State::get().await?;
    commands::flatten_for_pack_update(&instance_id, &state).await
}

#[tracing::instrument]
pub async fn list_mod_groups(
    instance_id: String,
) -> crate::Result<Vec<commands::ContentGroup>> {
    let state = State::get().await?;
    commands::list_mod_groups(&instance_id, &state).await
}

#[tracing::instrument]
pub async fn create_mod_group(
    instance_id: String,
    name: String,
) -> crate::Result<String> {
    let state = State::get().await?;
    commands::create_mod_group(&instance_id, &name, &state).await
}

#[tracing::instrument]
pub async fn rename_mod_group(
    instance_id: String,
    old_name: String,
    new_name: String,
) -> crate::Result<String> {
    let state = State::get().await?;
    let name =
        commands::rename_mod_group(&instance_id, &old_name, &new_name, &state)
            .await?;
    crate::state::sync_content_files(&instance_id, &state).await?;
    Ok(name)
}

#[tracing::instrument]
pub async fn delete_mod_group(
    instance_id: String,
    name: String,
) -> crate::Result<()> {
    let state = State::get().await?;
    commands::delete_mod_group(&instance_id, &name, &state).await?;
    crate::state::sync_content_files(&instance_id, &state).await?;
    Ok(())
}

/// Перемістити моди в групу (`None` — прибрати з групи). Повертає нові шляхи.
#[tracing::instrument]
pub async fn set_mod_group(
    instance_id: String,
    paths: Vec<String>,
    group: Option<String>,
) -> crate::Result<Vec<String>> {
    let state = State::get().await?;
    let mut result = Vec::with_capacity(paths.len());
    for path in &paths {
        result.push(
            commands::set_mod_group(
                &instance_id,
                path,
                group.as_deref(),
                &state,
            )
            .await?,
        );
    }
    crate::state::sync_content_files(&instance_id, &state).await?;
    Ok(result)
}
