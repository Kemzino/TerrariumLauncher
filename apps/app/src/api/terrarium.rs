use std::path::PathBuf;

use crate::api::Result;
use tauri::plugin::TauriPlugin;
use theseus::terrarium::{
    self, AdminInfo, Channel, ContentGroup, PackKind, PublishPreview,
    PublishRequest, PublishedRelease, TerrariumRelease, TerrariumState,
};
use theseus::terrarium_sync::{self, SyncPreview, SyncRequest, SyncResult};

pub fn init<R: tauri::Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("terrarium")
        .invoke_handler(tauri::generate_handler![
            terrarium_get_state,
            terrarium_set_state,
            terrarium_fetch_latest_release,
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
            terrarium_promote_release,
            terrarium_sync_preview,
            terrarium_sync_apply,
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
pub async fn terrarium_promote_release(
    pack: PackKind,
    release_id: u64,
) -> Result<TerrariumRelease> {
    Ok(terrarium::promote_release(pack, release_id).await?)
}

#[tauri::command]
pub async fn terrarium_sync_preview(
    source_instance_id: String,
    target_instance_id: String,
) -> Result<SyncPreview> {
    Ok(
        terrarium_sync::sync_preview(source_instance_id, target_instance_id)
            .await?,
    )
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
