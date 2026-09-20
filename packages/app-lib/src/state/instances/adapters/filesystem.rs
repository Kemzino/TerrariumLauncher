use crate::state::instances::commands::mod_groups;
use crate::state::{ProjectType, file_hash_cache_key, file_modified_at_ns};
use crate::util::io::{self, IOError};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub(crate) struct ScannedContentFile {
    pub relative_path: String,
    pub file_name: String,
    pub enabled: bool,
    pub size: u64,
    pub hash_cache_key: String,
}

pub(crate) fn scan_content_files(
    instances_dir: &Path,
    instance_path: &str,
) -> crate::Result<Vec<ScannedContentFile>> {
    let instance_dir = io::canonicalize(instances_dir.join(instance_path))?;
    let mut files = Vec::new();

    for project_type in ProjectType::iterator() {
        let folder = project_type.get_folder();
        let folder_path = instance_dir.join(folder);

        if !folder_path.exists() {
            continue;
        }

        // Terrarium: моди можуть лежати в групах `mods/<Група>/`; поки гра
        // запущена, вони фізично в корені, а група — у карті розкладання.
        let flatten_map = if project_type == ProjectType::Mod {
            mod_groups::read_flatten_map_sync(&folder_path).filter(|map| {
                map.reason == Some(mod_groups::FlattenReason::Launch)
            })
        } else {
            None
        };
        let mut scan_targets: Vec<(PathBuf, Option<String>)> =
            vec![(folder_path.clone(), None)];
        if project_type == ProjectType::Mod {
            for entry in std::fs::read_dir(&folder_path)
                .map_err(|err| IOError::with_path(err, &folder_path))?
            {
                let path = entry.map_err(IOError::from)?.path();
                let Some(name) = path.file_name().and_then(|n| n.to_str())
                else {
                    continue;
                };
                if path.is_dir() && mod_groups::is_group_dir_name(name) {
                    scan_targets.push((path.clone(), Some(name.to_string())));
                }
            }
        }

        for (dir, group) in scan_targets {
            for entry in std::fs::read_dir(&dir)
                .map_err(|err| IOError::with_path(err, &dir))?
            {
                let path = entry.map_err(IOError::from)?.path();
                if !path.is_file() {
                    continue;
                }

                let Some(file_name) =
                    path.file_name().and_then(|value| value.to_str())
                else {
                    continue;
                };

                if !is_scannable_project_file(project_type, file_name) {
                    continue;
                }

                let metadata = path.metadata().map_err(IOError::from)?;
                let size = metadata.len();
                let modified_at_ns =
                    file_modified_at_ns(&metadata).map_err(IOError::from)?;
                // Папка групи як є (`Група` чи `Група.disabled`) — шлях у БД має
                // збігатися з диском; назву групи звідси дістає `group_of`
                let group = group.clone().or_else(|| {
                    flatten_map
                        .as_ref()
                        .and_then(|m| m.files.get(file_name).cloned())
                });
                let relative_path = match &group {
                    Some(group) => format!("{folder}/{group}/{file_name}"),
                    None => format!("{folder}/{file_name}"),
                };
                let group_disabled = group
                    .as_deref()
                    .is_some_and(|g| g.ends_with(mod_groups::DISABLED_SUFFIX));
                let hash_cache_key = file_hash_cache_key(
                    size,
                    modified_at_ns,
                    &format!("{instance_path}/{relative_path}"),
                );

                files.push(ScannedContentFile {
                    relative_path,
                    file_name: file_name.to_string(),
                    enabled: !file_name.ends_with(".disabled")
                        && !group_disabled,
                    size,
                    hash_cache_key,
                });
            }
        }
    }

    Ok(files)
}

pub(crate) fn project_type_from_relative_path(
    relative_path: &str,
) -> Option<ProjectType> {
    ProjectType::get_from_parent_folder(PathBuf::from(relative_path))
}

fn is_scannable_project_file(
    project_type: ProjectType,
    file_name: &str,
) -> bool {
    let Some(extension) = Path::new(file_name.trim_end_matches(".disabled"))
        .extension()
        .and_then(|ext| ext.to_str())
    else {
        return false;
    };

    match project_type {
        ProjectType::Mod => extension.eq_ignore_ascii_case("jar"),
        ProjectType::DataPack
        | ProjectType::ResourcePack
        | ProjectType::ShaderPack => {
            extension.eq_ignore_ascii_case("zip")
                || extension.eq_ignore_ascii_case("jar")
        }
    }
}
