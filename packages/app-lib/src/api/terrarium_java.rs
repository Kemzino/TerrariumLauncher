//! Terrarium: per-instance switch between the launcher's default Java and Oracle GraalVM.
//!
//! GraalVM's JIT compiler (Graal) inlines and removes allocations far more aggressively than HotSpot's C2, which
//! pays off in heavily modded packs: on the Terrarium pack it measured +6% FPS standing, +17% turning, +23% moving
//! and a 32% better 1% low. The archive is fetched once into `meta/java_versions/graalvm-<major>` and an instance
//! opts in through its launch override `java_path`; switching back clears the override. If the GraalVM files ever
//! go missing, the launcher's own Java is used (an invalid override is ignored at launch).

use crate::event::emit::{emit_loading, init_loading};
use crate::state::instances::commands::{EditInstance, InstanceLaunchOverridesPatch};
use crate::util::fetch::REQWEST_CLIENT;
use crate::util::io;
use crate::{LoadingBarType, State};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

/// GraalVM major version offered to instances.
pub const GRAALVM_MAJOR: u32 = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraalvmStatus {
    /// Supported on this OS/architecture at all.
    pub available: bool,
    /// Already downloaded.
    pub installed: bool,
    /// This instance launches with it.
    pub enabled: bool,
    pub major_version: u32,
}

fn download_url() -> Option<(String, bool)> {
    let (os, ext, zip) = match std::env::consts::OS {
        "windows" => ("windows", "zip", true),
        "macos" => ("macos", "tar.gz", false),
        "linux" => ("linux", "tar.gz", false),
        _ => return None,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "aarch64",
        _ => return None,
    };
    Some((
        format!(
            "https://download.oracle.com/graalvm/{GRAALVM_MAJOR}/latest/graalvm-jdk-{GRAALVM_MAJOR}_{os}-{arch}_bin.{ext}"
        ),
        zip,
    ))
}

async fn install_root() -> crate::Result<PathBuf> {
    let state = State::get().await?;
    Ok(state
        .directories
        .java_versions_dir()
        .join(format!("graalvm-{GRAALVM_MAJOR}")))
}

/// The java binary inside an extracted GraalVM, wherever the archive's top folder put it.
fn find_java(root: &Path) -> Option<PathBuf> {
    let bin = if cfg!(windows) { "javaw.exe" } else { "java" };
    let entries = std::fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        for candidate in [
            dir.join("bin").join(bin),
            dir.join("Contents").join("Home").join("bin").join(bin),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Path of the installed GraalVM java binary, if it is there.
pub async fn installed_path() -> crate::Result<Option<PathBuf>> {
    let root = install_root().await?;
    Ok(tokio::task::spawn_blocking(move || find_java(&root)).await?)
}

/// Downloads and extracts GraalVM (once); returns its java binary.
pub async fn install() -> crate::Result<PathBuf> {
    if let Some(path) = installed_path().await? {
        return Ok(path);
    }
    let (url, is_zip) = download_url().ok_or_else(|| {
        crate::ErrorKind::LauncherError(format!(
            "GraalVM {GRAALVM_MAJOR} is not available for {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;

    let loading_bar = init_loading(
        LoadingBarType::JavaDownload {
            version: GRAALVM_MAJOR,
        },
        100.0,
        "Downloading GraalVM",
    )
    .await?;

    let root = install_root().await?;
    let staging = root.with_extension("partial");
    if staging.exists() {
        io::remove_dir_all(&staging).await?;
    }
    io::create_dir_all(&staging).await?;
    let archive = staging.join(if is_zip { "graalvm.zip" } else { "graalvm.tar.gz" });

    // Stream to disk: the archive is ~350 MB
    let response = REQWEST_CLIENT.get(&url).send().await?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&archive).await?;
    let mut stream = response.bytes_stream();
    let mut done = 0_u64;
    let mut last_percent = 0.0_f64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        if total > 0 {
            let percent = done as f64 / total as f64 * 85.0;
            if percent - last_percent >= 1.0 {
                emit_loading(&loading_bar, percent - last_percent, Some("Downloading GraalVM"))?;
                last_percent = percent;
            }
        }
    }
    file.flush().await?;
    drop(file);

    emit_loading(&loading_bar, (85.0 - last_percent).max(0.0), Some("Extracting GraalVM"))?;
    let extract_to = staging.join("jdk");
    let archive_path = archive.clone();
    let extract_dir = extract_to.clone();
    tokio::task::spawn_blocking(move || -> crate::Result<()> {
        std::fs::create_dir_all(&extract_dir)?;
        let reader = std::fs::File::open(&archive_path)?;
        if is_zip {
            let mut zip = zip::ZipArchive::new(reader).map_err(|e| {
                crate::ErrorKind::InputError(format!("Failed to read GraalVM zip: {e}"))
            })?;
            zip.extract(&extract_dir).map_err(|e| {
                crate::ErrorKind::InputError(format!("Failed to extract GraalVM: {e}"))
            })?;
        } else {
            let gz = flate2::read::GzDecoder::new(reader);
            tar::Archive::new(gz).unpack(&extract_dir)?;
        }
        Ok(())
    })
    .await??;
    io::remove_file(&archive).await?;

    let java = tokio::task::spawn_blocking({
        let dir = extract_to.clone();
        move || find_java(&dir)
    })
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::LauncherError("GraalVM archive has no java binary".to_string())
    })?;
    // Validate before making it visible
    crate::api::jre::check_jre(java.clone()).await?;

    if root.exists() {
        io::remove_dir_all(&root).await?;
    }
    io::rename_or_move(&extract_to, &root).await?;
    let _ = io::remove_dir_all(&staging).await;
    emit_loading(&loading_bar, 15.0, Some("GraalVM ready"))?;

    installed_path().await?.ok_or_else(|| {
        crate::ErrorKind::LauncherError("GraalVM install vanished".to_string()).into()
    })
}

pub async fn status(instance_id: &str) -> crate::Result<GraalvmStatus> {
    let state = State::get().await?;
    let installed = installed_path().await?;
    let context =
        crate::state::instances::commands::get_instance_launch_context(instance_id, &state.pool)
            .await?;
    let current = context.and_then(|c| c.launch_overrides.java_path);
    let enabled = match (&installed, &current) {
        (Some(graal), Some(current)) => Path::new(current) == graal.as_path(),
        // An override pointing at some other GraalVM (set by hand) still counts as on
        (None, Some(current)) => current.to_ascii_lowercase().contains("graalvm"),
        _ => false,
    };
    Ok(GraalvmStatus {
        available: download_url().is_some(),
        installed: installed.is_some(),
        enabled,
        major_version: GRAALVM_MAJOR,
    })
}

/// Switches the instance to GraalVM (downloading it first if needed) or back to the launcher's Java.
pub async fn set_enabled(instance_id: &str, enabled: bool) -> crate::Result<GraalvmStatus> {
    let java_path = if enabled {
        Some(install().await?.to_string_lossy().to_string())
    } else {
        None
    };
    crate::api::instance::edit(
        instance_id,
        EditInstance {
            launch_overrides: Some(InstanceLaunchOverridesPatch {
                java_path: Some(java_path),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .await?;
    status(instance_id).await
}
