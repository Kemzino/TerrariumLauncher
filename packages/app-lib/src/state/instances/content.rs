use super::ContentSourceKind;
use crate::state::{
    License, Project, ProjectType, Version, VersionEnvironment,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItem {
    pub file_name: String,
    pub file_path: String,
    pub id: String,
    pub size: u64,
    pub enabled: bool,
    pub locked: bool,
    pub project_type: ProjectType,
    pub project: Option<ContentItemProject>,
    pub version: Option<ContentItemVersion>,
    pub environment: Option<VersionEnvironment>,
    pub owner: Option<ContentItemOwner>,
    pub has_update: bool,
    pub update_version_id: Option<String>,
    pub date_added: Option<String>,
    pub source_kind: Option<ContentSourceKind>,
    pub embedded_metadata: Option<EmbeddedContentMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synced_pack: Option<SyncedPackInfo>,
    /// Terrarium: мод упізнано на CurseForge (лише для файлів, яких нема на
    /// Modrinth) — іконка, сторінка, автор, чи є новіший файл.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curseforge: Option<CurseForgeContent>,
}

/// Terrarium: картка мода з CurseForge для файлу примірника.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurseForgeContent {
    pub mod_id: u64,
    pub name: String,
    pub slug: String,
    /// Сторінка мода на curseforge.com
    pub url: String,
    pub icon_url: Option<String>,
    pub author: Option<String>,
    pub author_url: Option<String>,
    /// Встановлений файл
    pub file_id: u64,
    pub file_name: String,
    /// Назва файлу з CurseForge («Create Tick Controller 1.4.24»)
    pub display_name: String,
    /// Новіший файл для цієї версії гри й завантажувача, якщо є
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update: Option<CurseForgeUpdate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CurseForgeUpdate {
    pub file_id: u64,
    pub file_name: String,
    /// Сторінка файлу на curseforge.com — звідти його можна завантажити
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncedPackInfo {
    pub id: String,
    pub instance_ids: Vec<String>,
    pub update_pending: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EmbeddedContentMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub icon_path: Option<String>,
}

impl EmbeddedContentMetadata {
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.version.is_none()
            && self.icon_path.is_none()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItemProject {
    pub id: String,
    pub slug: Option<String>,
    pub title: String,
    pub icon_url: Option<String>,
    pub license: License,
    pub categories: Vec<String>,
    pub additional_categories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItemVersion {
    pub id: String,
    pub version_number: String,
    pub file_name: String,
    pub date_published: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContentItemOwner {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    #[serde(rename = "type")]
    pub owner_type: OwnerType,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OwnerType {
    User,
    Organization,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LinkedModpackInfo {
    pub project: Project,
    pub version: Option<Version>,
    pub owner: Option<ContentItemOwner>,
    pub has_update: bool,
    pub update_version_id: Option<String>,
    pub update_version: Option<Version>,
}
