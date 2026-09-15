mod create_instance;
pub use self::create_instance::CreateInstance;
pub(crate) use self::create_instance::{create_instance, resolve_icon_path};

mod edit_instance;
pub(crate) use self::edit_instance::edit_instance;
pub use self::edit_instance::{
    AppliedContentSetPatch, EditInstance, InstanceLaunchOverridesPatch,
};

mod get_instance;
pub use self::get_instance::InstanceMetadata;
pub(crate) use self::get_instance::{
    get_instance, get_instance_metadata, get_instances_metadata, list_instances,
};

mod game_options;
pub(crate) use self::game_options::*;

mod list_content;
pub(crate) use self::list_content::{
    dependencies_to_content_items, get_content_projects,
    get_installed_project_ids_for_instance, get_instance_install_candidates,
    get_linked_modpack_info, list_content, list_content_sets,
    list_linked_modpack_content, list_pack_content,
};

mod embedded_content_metadata;

mod remove_instance;
pub(crate) use self::remove_instance::*;

mod refresh_instances;
pub(crate) use self::refresh_instances::*;

mod sync_content_files;
pub(crate) use self::sync_content_files::sync_content_files;

mod launch_context;
pub(crate) use self::launch_context::*;

mod apply_content_install;
pub(crate) use self::apply_content_install::*;

mod check_content_updates;
pub(crate) use self::check_content_updates::refresh_content_updates;

mod apply_content_update;
pub(crate) use self::apply_content_update::*;

mod shared_instance;
pub(crate) use self::shared_instance::{
    attach_shared_instance, clear_shared_instance, mark_shared_instance_stale,
    quarantine_shared_instance, set_shared_instance_sync_status,
};

pub(crate) mod mod_groups;
pub use self::mod_groups::{ContentGroup, FlattenReason, flat_path, group_of};
pub(crate) use self::mod_groups::{
    apply_pack_groups, create_mod_group, delete_mod_group, flatten_mods,
    FLATTEN_FILE, MODS_FOLDER, PACK_GROUPS_FILE, README_FILE,
    flatten_for_pack_update, flatten_reason, is_group_dir_name,
    list_mod_groups, rename_mod_group,
    restore_after_pack_update, restore_mods, set_mod_group,
};
