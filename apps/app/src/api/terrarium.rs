use std::path::PathBuf;

use crate::api::Result;
use tauri::plugin::TauriPlugin;
use theseus::terrarium::{
    self, AdminInfo, Channel, ContentGroup, PackKind, PublishPreview,
    PublishRequest, PublishedRelease, TerrariumRelease, TerrariumState,
};
use theseus::terrarium_modrinth::{
    self, ModrinthAppInfo, ModrinthImportResult,
};
use theseus::terrarium_sync::{self, SyncPreview, SyncRequest, SyncResult};

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("terrarium")
        .invoke_handler(tauri::generate_handler![
            terrarium_get_state,
            terrarium_set_state,
            terrarium_fetch_latest_release,
            terrarium_list_releases,
            terrarium_download_release,
            terrarium_verify_admin_token,
            terrarium_publish_release,
            terrarium_publish_preview,
            terrarium_apply_pack_branding,
            terrarium_prepare_pack_update,
            terrarium_list_mod_groups,
            terrarium_create_mod_group,
            terrarium_rename_mod_group,
            terrarium_delete_mod_group,
            terrarium_set_mod_group,
            terrarium_set_mod_group_enabled,
            terrarium_promote_release,
            terrarium_sync_preview,
            terrarium_sync_apply,
            terrarium_detect_modrinth_app,
            terrarium_use_modrinth_directory,
            terrarium_import_modrinth_instances,
            terrarium_open_discord,
            terrarium_curseforge_has_key,
            terrarium_curseforge_list_files,
            terrarium_curseforge_switch_file,
            terrarium_curseforge_get_mod,
            terrarium_curseforge_get_description,
            terrarium_curseforge_get_changelog,
            terrarium_curseforge_get_files,
            terrarium_curseforge_get_file,
            terrarium_curseforge_get_categories,
            terrarium_curseforge_search,
            terrarium_curseforge_install,
            terrarium_curseforge_prepare_modpack,
        ])
        .build()
}

#[tauri::command]
pub async fn terrarium_get_state() -> Result<TerrariumState> {
    Ok(terrarium::get_state().await?)
}

#[tauri::command]
pub async fn terrarium_set_state(state: TerrariumState) -> Result<()> {
    Ok(terrarium::set_state(state).await?)
}

#[tauri::command]
pub async fn terrarium_fetch_latest_release(
    pack: PackKind,
    channel: Option<Channel>,
) -> Result<TerrariumRelease> {
    Ok(
        terrarium::fetch_latest_release(pack, channel.unwrap_or_default())
            .await?,
    )
}

#[tauri::command]
pub async fn terrarium_set_mod_group_enabled(
    instance_id: String,
    name: String,
    enabled: bool,
) -> Result<()> {
    Ok(terrarium::set_mod_group_enabled(instance_id, name, enabled).await?)
}

#[tauri::command]
pub async fn terrarium_promote_release(
    pack: PackKind,
    release_id: u64,
) -> Result<TerrariumRelease> {
    Ok(terrarium::promote_release(pack, release_id).await?)
}

#[tauri::command]
pub async fn terrarium_sync_preview(
    client_instance_id: String,
    server_instance_id: String,
) -> Result<SyncPreview> {
    Ok(
        terrarium_sync::sync_preview(client_instance_id, server_instance_id)
            .await?,
    )
}

#[tauri::command]
pub async fn terrarium_detect_modrinth_app() -> Result<Option<ModrinthAppInfo>>
{
    Ok(terrarium_modrinth::detect_modrinth_app().await?)
}

#[tauri::command]
pub async fn terrarium_use_modrinth_directory() -> Result<String> {
    Ok(terrarium_modrinth::use_modrinth_directory().await?)
}

#[tauri::command]
pub async fn terrarium_import_modrinth_instances()
-> Result<ModrinthImportResult> {
    Ok(terrarium_modrinth::import_modrinth_instances().await?)
}

#[tauri::command]
pub async fn terrarium_sync_apply(request: SyncRequest) -> Result<SyncResult> {
    Ok(terrarium_sync::sync_apply(request).await?)
}

#[tauri::command]
pub async fn terrarium_download_release(
    release: TerrariumRelease,
) -> Result<PathBuf> {
    Ok(terrarium::download_release(release).await?)
}

#[tauri::command]
pub async fn terrarium_verify_admin_token(token: String) -> Result<AdminInfo> {
    Ok(terrarium::verify_admin_token(token).await?)
}

#[tauri::command]
pub async fn terrarium_publish_release(
    request: PublishRequest,
) -> Result<PublishedRelease> {
    Ok(terrarium::publish_release(request).await?)
}

#[tauri::command]
pub async fn terrarium_publish_preview(
    pack: PackKind,
    instance_id: String,
) -> Result<PublishPreview> {
    Ok(terrarium::publish_preview(pack, instance_id).await?)
}

#[tauri::command]
pub async fn terrarium_apply_pack_branding(
    instance_id: String,
) -> Result<bool> {
    Ok(terrarium::apply_pack_branding(instance_id).await?)
}

#[tauri::command]
pub async fn terrarium_prepare_pack_update(instance_id: String) -> Result<()> {
    Ok(terrarium::prepare_pack_update(instance_id).await?)
}

#[tauri::command]
pub async fn terrarium_list_mod_groups(
    instance_id: String,
) -> Result<Vec<ContentGroup>> {
    Ok(terrarium::list_mod_groups(instance_id).await?)
}

#[tauri::command]
pub async fn terrarium_create_mod_group(
    instance_id: String,
    name: String,
) -> Result<String> {
    Ok(terrarium::create_mod_group(instance_id, name).await?)
}

#[tauri::command]
pub async fn terrarium_rename_mod_group(
    instance_id: String,
    old_name: String,
    new_name: String,
) -> Result<String> {
    Ok(terrarium::rename_mod_group(instance_id, old_name, new_name).await?)
}

#[tauri::command]
pub async fn terrarium_delete_mod_group(
    instance_id: String,
    name: String,
) -> Result<()> {
    Ok(terrarium::delete_mod_group(instance_id, name).await?)
}

#[tauri::command]
pub async fn terrarium_set_mod_group(
    instance_id: String,
    paths: Vec<String>,
    group: Option<String>,
) -> Result<Vec<String>> {
    Ok(terrarium::set_mod_group(instance_id, paths, group).await?)
}

/// Чи зареєстровано в системі обробник протоколу `discord://` (десктоп-застосунок).
fn discord_app_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CLASSES_ROOT;
        RegKey::predef(HKEY_CLASSES_ROOT)
            .open_subkey(r"discord\shell\open\command")
            .is_ok()
    }
    #[cfg(target_os = "macos")]
    {
        [
            "/Applications/Discord.app",
            "/Applications/Discord PTB.app",
            "/Applications/Discord Canary.app",
        ]
        .iter()
        .any(|p| std::path::Path::new(p).exists())
            || dirs::home_dir()
                .map(|h| h.join("Applications/Discord.app").exists())
                .unwrap_or(false)
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-mime")
            .args(["query", "default", "x-scheme-handler/discord"])
            .output()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false)
    }
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    )))]
    {
        false
    }
}

/// Відкрити посилання Discord у застосунку (діплінк `discord://-/…`), а якщо
/// застосунку немає — у браузері. Повертає `true`, якщо пішло в застосунок.
#[tauri::command]
pub async fn terrarium_open_discord<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    url: String,
) -> Result<bool> {
    use tauri_plugin_opener::OpenerExt;
    let https = url.clone();
    let deep = url
        .strip_prefix("https://discord.com/")
        .or_else(|| url.strip_prefix("https://discordapp.com/"))
        .map(|rest| format!("discord://-/{rest}"));
    let use_app = deep.is_some()
        && tauri::async_runtime::spawn_blocking(discord_app_installed)
            .await
            .unwrap_or(false);
    let target = if use_app { deep.unwrap() } else { https };
    app.opener().open_url(&target, None::<&str>).map_err(|e| {
        theseus::ErrorKind::OtherError(format!(
            "Не вдалося відкрити {target}: {e}"
        ))
        .as_error()
    })?;
    Ok(use_app)
}

/// Чи є ключ CurseForge API (з налаштувань або вбудований) — без нього
/// моди не з Modrinth показуються без іконок і оновлень.
#[tauri::command]
pub async fn terrarium_curseforge_has_key() -> Result<bool> {
    Ok(theseus::terrarium_curseforge::has_api_key().await)
}

/// Файли мода з CurseForge для версії гри/завантажувача (вибір версії).
#[tauri::command]
pub async fn terrarium_curseforge_list_files(
    mod_id: u64,
    slug: String,
    game_version: String,
    loader: Option<theseus::prelude::ModLoader>,
) -> Result<Vec<theseus::terrarium_curseforge::CurseForgeFile>> {
    Ok(theseus::terrarium_curseforge::list_mod_files(
        mod_id,
        slug,
        game_version,
        loader,
    )
    .await?)
}

/// Замінити файл мода з CurseForge на інший (зміна версії / оновлення).
#[tauri::command]
pub async fn terrarium_curseforge_switch_file(
    instance_id: String,
    project_path: String,
    mod_id: u64,
    file_id: u64,
) -> Result<String> {
    Ok(theseus::terrarium_curseforge::switch_mod_file(
        instance_id,
        project_path,
        mod_id,
        file_id,
    )
    .await?)
}

// --- CurseForge як повноцінне джерело: сторінка проєкту, пошук, установка

#[tauri::command]
pub async fn terrarium_curseforge_get_mod(
    mod_id: u64,
    force: Option<bool>,
) -> Result<theseus::terrarium_curseforge::CurseForgeMod> {
    Ok(theseus::terrarium_curseforge::get_mod(mod_id, force.unwrap_or(false))
        .await?)
}

#[tauri::command]
pub async fn terrarium_curseforge_get_description(mod_id: u64) -> Result<String> {
    Ok(theseus::terrarium_curseforge::get_mod_description(mod_id).await?)
}

#[tauri::command]
pub async fn terrarium_curseforge_get_changelog(
    mod_id: u64,
    file_id: u64,
) -> Result<String> {
    Ok(theseus::terrarium_curseforge::get_file_changelog(mod_id, file_id).await?)
}

#[derive(serde::Serialize)]
pub struct CurseForgeFilesPage {
    pub files: Vec<theseus::terrarium_curseforge::CurseForgeFileDetails>,
    pub total: u64,
}

#[tauri::command]
pub async fn terrarium_curseforge_get_files(
    mod_id: u64,
    slug: String,
    game_version: Option<String>,
    loader: Option<theseus::prelude::ModLoader>,
    index: Option<u64>,
) -> Result<CurseForgeFilesPage> {
    let (files, total) = theseus::terrarium_curseforge::get_mod_files(
        mod_id,
        slug,
        game_version,
        loader,
        index.unwrap_or(0),
    )
    .await?;
    Ok(CurseForgeFilesPage { files, total })
}

#[tauri::command]
pub async fn terrarium_curseforge_get_file(
    mod_id: u64,
    file_id: u64,
    slug: String,
) -> Result<theseus::terrarium_curseforge::CurseForgeFileDetails> {
    Ok(theseus::terrarium_curseforge::get_file(mod_id, file_id, slug).await?)
}

#[tauri::command]
pub async fn terrarium_curseforge_get_categories(
    project_type: Option<theseus::prelude::ProjectType>,
) -> Result<Vec<theseus::terrarium_curseforge::CurseForgeCategory>> {
    Ok(theseus::terrarium_curseforge::get_categories(project_type).await?)
}

#[tauri::command]
pub async fn terrarium_curseforge_search(
    params: theseus::terrarium_curseforge::CurseForgeSearchParams,
) -> Result<theseus::terrarium_curseforge::CurseForgeSearchResult> {
    Ok(theseus::terrarium_curseforge::search(params).await?)
}

#[tauri::command]
pub async fn terrarium_curseforge_install(
    instance_id: String,
    mod_id: u64,
    file_id: Option<u64>,
    with_dependencies: Option<bool>,
) -> Result<theseus::terrarium_curseforge::CurseForgeInstallResult> {
    Ok(theseus::terrarium_curseforge::install_mod(
        instance_id,
        mod_id,
        file_id,
        with_dependencies.unwrap_or(true),
    )
    .await?)
}

/// Збірка з CurseForge → .mrpack у кеші (далі — звичайна установка збірки).
#[tauri::command]
pub async fn terrarium_curseforge_prepare_modpack(
    mod_id: u64,
    file_id: Option<u64>,
) -> Result<theseus::terrarium_curseforge::CurseForgeModpackPrepared> {
    Ok(theseus::terrarium_curseforge::prepare_modpack(mod_id, file_id).await?)
}

/// Список змін збірки (останні релізи) для головної.
#[tauri::command]
pub async fn terrarium_list_releases(
    pack: PackKind,
    channel: Option<Channel>,
    limit: Option<u32>,
) -> Result<Vec<terrarium::TerrariumChangelogEntry>> {
    Ok(terrarium::list_releases(
        pack,
        channel.unwrap_or_default(),
        limit.unwrap_or(20),
    )
    .await?)
}
