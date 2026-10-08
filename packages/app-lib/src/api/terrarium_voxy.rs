//! Terrarium: Voxy (далека прорисовка LoD), NeoForge-порт `j-shelfwood/voxy-neoforge` — наш форк
//! `Kemzino/voxy-neoforge`, гілка `terrarium`: портований на Sodium 0.8.13, який стоїть у збірці
//! (апстрім зібраний під Sodium 0.6 і крашиться на старті).
//!
//! Ліцензія Voxy — «All rights reserved, do not redistribute», тож у збірку його не кладемо: кожен гравець
//! збирає мод у себе з вихідного коду (останній коміт гілки [`BRANCH`]), і jar іде лише в його власну теку `mods/`.
//!
//! Без git і без встановленої Java: код — zip-архів коміту з GitHub, JDK — Azul Zulu (качається один раз
//! у `meta/java_versions`), Gradle запускається напряму через `gradle-wrapper.jar`. Перевірки міксинів
//! апстріму (python/bash) пропускаються. Кеш збірки (~1 ГБ) після успіху видаляється.

use crate::State;
use crate::state::ModLoader;
use crate::util::fetch::REQWEST_CLIENT;
use crate::util::io;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Репозиторій порту і гілка, з останнього коміту якої збираємо.
pub const REPO: &str = "Kemzino/voxy-neoforge";
pub const BRANCH: &str = "terrarium";
/// Коміт на випадок, коли GitHub API недоступний (немає мережі, ліміт запитів).
const FALLBACK_COMMIT: &str = "c0fa6e5ade99aa5bbbf355a838a06a4201e0c99b";
/// Скільки тримаємо відповідь GitHub: статус кнопки питають часто, а без токена API дає 60 запитів на годину.
const LATEST_TTL: std::time::Duration = std::time::Duration::from_secs(10 * 60);
const GAME_VERSION: &str = "1.21.1";
const JDK_MAJOR: u32 = 21;
/// Наші jar-и: `voxy-<версія>+<коміт>.jar`
const JAR_PREFIX: &str = "voxy-";

fn short_commit(commit: &str) -> &str {
    &commit[..7.min(commit.len())]
}

static LATEST: Mutex<Option<(String, std::time::Instant)>> = Mutex::new(None);

/// Останній коміт гілки [`BRANCH`] (кешований на [`LATEST_TTL`]; `fresh` — питати GitHub попри кеш).
/// Якщо GitHub не відповів — останній відомий коміт або [`FALLBACK_COMMIT`].
async fn latest_commit(fresh: bool) -> String {
    let cached = LATEST.lock().ok().and_then(|l| l.clone());
    if let Some((commit, at)) = &cached
        && !fresh
        && at.elapsed() < LATEST_TTL
    {
        return commit.clone();
    }
    let url = format!("https://api.github.com/repos/{REPO}/commits/{BRANCH}");
    let fetched = async {
        REQWEST_CLIENT
            .get(&url)
            .header("Accept", "application/vnd.github.sha")
            .send()
            .await?
            .error_for_status()?
            .text()
            .await
    }
    .await;
    match fetched {
        Ok(sha)
            if sha.trim().len() == 40
                && sha.trim().chars().all(|c| c.is_ascii_hexdigit()) =>
        {
            let sha = sha.trim().to_ascii_lowercase();
            if let Ok(mut l) = LATEST.lock() {
                *l = Some((sha.clone(), std::time::Instant::now()));
            }
            sha
        }
        other => {
            tracing::warn!(
                "Voxy: не вдалося дізнатися останній коміт {REPO}@{BRANCH}: {:?}",
                other.err()
            );
            cached
                .map(|(c, _)| c)
                .unwrap_or_else(|| FALLBACK_COMMIT.to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoxyStatus {
    /// Примірник — NeoForge 1.21.1
    pub compatible: bool,
    /// Назва встановленого jar Voxy, якщо є
    pub installed: Option<String>,
    /// Встановлений jar зібрано з останнього коміту гілки
    pub up_to_date: bool,
    pub commit: String,
    pub repo: String,
    /// Збірка йде просто зараз
    pub building: bool,
    /// Що зараз робиться (для кнопки)
    pub stage: Option<String>,
    /// 0..1
    pub progress: f32,
}

#[derive(Clone)]
struct Progress {
    instance_id: String,
    stage: String,
    progress: f32,
}

static PROGRESS: Mutex<Option<Progress>> = Mutex::new(None);

fn set_progress(instance_id: &str, stage: &str, progress: f32) {
    if let Ok(mut p) = PROGRESS.lock() {
        *p = Some(Progress {
            instance_id: instance_id.to_string(),
            stage: stage.to_string(),
            progress: progress.clamp(0.0, 1.0),
        });
    }
}

fn current_progress() -> Option<Progress> {
    PROGRESS.lock().ok().and_then(|p| p.clone())
}

/// Знімає позначку «збирається», навіть якщо збірка впала.
struct ProgressGuard;
impl Drop for ProgressGuard {
    fn drop(&mut self) {
        if let Ok(mut c) = CANCEL.lock() {
            *c = None;
        }
        if let Ok(mut p) = PROGRESS.lock() {
            *p = None;
        }
    }
}

/// Скасування поточної збірки: [`cancel`] шле сигнал, [`build_and_install`] чекає на нього поряд зі збіркою.
static CANCEL: Mutex<Option<tokio::sync::oneshot::Sender<()>>> =
    Mutex::new(None);
/// PID запущеного Gradle: при скасуванні вбиваємо все дерево (Gradle форкає свої JVM).
static GRADLE_PID: Mutex<Option<u32>> = Mutex::new(None);

fn kill_gradle_tree() {
    let Some(pid) = GRADLE_PID.lock().ok().and_then(|mut p| p.take()) else {
        return;
    };
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

/// Скасовує збірку Voxy, якщо вона йде. Повертає, чи було що скасовувати.
pub fn cancel() -> bool {
    let sender = CANCEL.lock().ok().and_then(|mut c| c.take());
    match sender {
        Some(sender) => {
            kill_gradle_tree();
            sender.send(()).is_ok()
        }
        None => false,
    }
}

async fn work_root() -> crate::Result<PathBuf> {
    let state = State::get().await?;
    // Коротко: на Windows Gradle/NeoForm не запускає процеси з теки довшої за 260 символів
    Ok(state
        .directories
        .caches_dir()
        .join("terrarium")
        .join("voxy"))
}

async fn jdk_root() -> crate::Result<PathBuf> {
    let state = State::get().await?;
    Ok(state
        .directories
        .java_versions_dir()
        .join(format!("zulu-jdk-{JDK_MAJOR}")))
}

/// Усі jar-и Voxy у `mods/` і в підтеках груп (`mods/<Група>/`).
fn voxy_jars(mods: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut dirs = vec![mods.to_path_buf()];
    if let Ok(entries) = std::fs::read_dir(mods) {
        dirs.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
    }
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries.flatten().map(|e| e.path()) {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            if path.is_file()
                && lower.starts_with(JAR_PREFIX)
                && (lower.ends_with(".jar") || lower.ends_with(".jar.disabled"))
            {
                found.push(path);
            }
        }
    }
    found
}

pub async fn status(instance_id: &str) -> crate::Result<VoxyStatus> {
    let metadata = crate::api::instance::get(instance_id).await?;
    let compatible = metadata.as_ref().is_some_and(|m| {
        m.applied_content_set.loader == ModLoader::NeoForge
            && m.applied_content_set.game_version == GAME_VERSION
    });
    let mods = crate::api::instance::get_full_path(instance_id)
        .await?
        .join("mods");
    let installed = tokio::task::spawn_blocking(move || voxy_jars(&mods))
        .await?
        .into_iter()
        .next()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));
    let commit = latest_commit(false).await;
    let up_to_date = installed
        .as_deref()
        .is_some_and(|n| n.contains(&format!("+{}", short_commit(&commit))));
    let progress = current_progress().filter(|p| p.instance_id == instance_id);
    Ok(VoxyStatus {
        compatible,
        installed,
        up_to_date,
        commit,
        repo: REPO.to_string(),
        building: progress.is_some(),
        stage: progress.as_ref().map(|p| p.stage.clone()),
        progress: progress.map(|p| p.progress).unwrap_or(0.0),
    })
}

/// Стрімить файл на диск; `on_progress(частка)`.
async fn download(
    url: &str,
    to: &Path,
    mut on_progress: impl FnMut(f32),
) -> crate::Result<()> {
    let response = REQWEST_CLIENT.get(url).send().await?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(to).await?;
    let mut stream = response.bytes_stream();
    let mut done = 0_u64;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        if total > 0 {
            on_progress(done as f32 / total as f32);
        }
    }
    file.flush().await?;
    Ok(())
}

async fn extract_zip(archive: &Path, to: &Path) -> crate::Result<()> {
    let archive = archive.to_path_buf();
    let to = to.to_path_buf();
    tokio::task::spawn_blocking(move || -> crate::Result<()> {
        std::fs::create_dir_all(&to)?;
        let reader = std::fs::File::open(&archive)?;
        let mut zip = zip::ZipArchive::new(reader).map_err(|e| {
            crate::ErrorKind::InputError(format!(
                "Не вдалося прочитати архів: {e}"
            ))
        })?;
        zip.extract(&to).map_err(|e| {
            crate::ErrorKind::InputError(format!(
                "Не вдалося розпакувати архів: {e}"
            ))
        })?;
        Ok(())
    })
    .await?
}

/// Тека JDK (JAVA_HOME) усередині розпакованого архіву: там, де лежить `bin/javac`.
fn find_jdk_home(root: &Path, depth: u32) -> Option<PathBuf> {
    let javac = if cfg!(windows) { "javac.exe" } else { "javac" };
    if root.join("bin").join(javac).is_file() {
        return Some(root.to_path_buf());
    }
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.is_symlink())
        .find_map(|p| find_jdk_home(&p, depth - 1))
}

/// JDK для збірки (у лаунчера лише JRE — без javac). Качає Zulu JDK один раз.
async fn install_jdk(instance_id: &str) -> crate::Result<PathBuf> {
    let root = jdk_root().await?;
    if let Some(home) = {
        let r = root.clone();
        tokio::task::spawn_blocking(move || find_jdk_home(&r, 4)).await?
    } {
        return Ok(home);
    }

    #[derive(Deserialize)]
    struct Package {
        download_url: String,
    }
    set_progress(instance_id, "Пошук JDK", 0.02);
    let url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages?arch={}&java_version={JDK_MAJOR}&os={}&archive_type=zip&javafx_bundled=false&crac_supported=false&release_status=ga&java_package_type=jdk&latest=true&page_size=1",
        std::env::consts::ARCH,
        std::env::consts::OS
    );
    let packages: Vec<Package> = REQWEST_CLIENT
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let package = packages.into_iter().next().ok_or_else(|| {
        crate::ErrorKind::LauncherError(format!(
            "JDK {JDK_MAJOR} недоступний для {} {}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    })?;

    let staging = root.with_extension("partial");
    if staging.exists() {
        io::remove_dir_all(&staging).await?;
    }
    io::create_dir_all(&staging).await?;
    let archive = staging.join("jdk.zip");
    download(&package.download_url, &archive, |f| {
        set_progress(instance_id, "Завантаження JDK", 0.02 + 0.18 * f)
    })
    .await?;
    set_progress(instance_id, "Розпакування JDK", 0.2);
    let extracted = staging.join("jdk");
    extract_zip(&archive, &extracted).await?;
    io::remove_file(&archive).await?;

    if root.exists() {
        io::remove_dir_all(&root).await?;
    }
    io::rename_or_move(&extracted, &root).await?;
    let _ = io::remove_dir_all(&staging).await;

    let r = root.clone();
    tokio::task::spawn_blocking(move || find_jdk_home(&r, 4))
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "В архіві JDK немає javac".to_string(),
            )
            .into()
        })
}

/// Вихідний код на коміті `commit` (zip з GitHub, git не потрібен).
async fn fetch_source(
    instance_id: &str,
    work: &Path,
    commit: &str,
) -> crate::Result<PathBuf> {
    let src = work.join("s");
    if src.exists() {
        io::remove_dir_all(&src).await?;
    }
    io::create_dir_all(work).await?;
    let archive = work.join("src.zip");
    let url = format!("https://codeload.github.com/{REPO}/zip/{commit}");
    set_progress(instance_id, "Завантаження коду Voxy", 0.22);
    download(&url, &archive, |_| {}).await?;
    let unpacked = work.join("s-unpack");
    if unpacked.exists() {
        io::remove_dir_all(&unpacked).await?;
    }
    extract_zip(&archive, &unpacked).await?;
    io::remove_file(&archive).await?;
    // У архіві одна тека `voxy-neoforge-<коміт>`
    let top = std::fs::read_dir(&unpacked)?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.join("gradle")
                .join("wrapper")
                .join("gradle-wrapper.jar")
                .is_file()
        })
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "В архіві Voxy немає gradle-wrapper.jar".to_string(),
            )
        })?;
    io::rename_or_move(&top, &src).await?;
    let _ = io::remove_dir_all(&unpacked).await;
    Ok(src)
}

/// Останні змістовні рядки логу Gradle (без стек-трейсів) — для повідомлення про помилку.
fn log_tail(log: &str) -> String {
    let lines: Vec<&str> = log
        .lines()
        .map(str::trim_end)
        .filter(|l| {
            let t = l.trim_start();
            !t.is_empty() && !t.starts_with("at ") && !t.starts_with("...")
        })
        .collect();
    let start = lines
        .iter()
        .rposition(|l| l.contains("What went wrong"))
        .unwrap_or(lines.len().saturating_sub(12));
    lines[start..]
        .iter()
        .take(12)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

async fn run_gradle(
    instance_id: &str,
    jdk_home: &Path,
    src: &Path,
    gradle_home: &Path,
    log_path: &Path,
) -> crate::Result<()> {
    let java = jdk_home.join("bin").join(if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    });
    let mut command = tokio::process::Command::new(java);
    command
        .args([
            "-Xmx128m",
            "-cp",
            "gradle/wrapper/gradle-wrapper.jar",
            "org.gradle.wrapper.GradleWrapperMain",
            "jar",
            // Перевірки апстріму запускають python3 / bash — їх у гравців немає
            "-x",
            "validateMixinConfig",
            "-x",
            "validateMixinSignatures",
            "--no-daemon",
            "--console=plain",
        ])
        .current_dir(src)
        .env("JAVA_HOME", jdk_home)
        .env("GRADLE_USER_HOME", gradle_home)
        .env_remove("GRADLE_OPTS")
        .env_remove("JAVA_TOOL_OPTIONS")
        .env_remove("_JAVA_OPTIONS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = command.spawn()?;
    if let Ok(mut pid) = GRADLE_PID.lock() {
        *pid = child.id();
    }

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    for pipe in [
        child.stdout.take().map(|s| {
            Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>
        }),
        child.stderr.take().map(|s| {
            Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>
        }),
    ]
    .into_iter()
    .flatten()
    {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(pipe).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
    }
    drop(tx);

    let mut log = String::new();
    let mut tasks = 0_u32;
    while let Some(line) = rx.recv().await {
        if line.starts_with("Downloading https://services.gradle.org") {
            set_progress(instance_id, "Завантаження Gradle", 0.25);
        } else if line.starts_with("> Configure project") {
            set_progress(
                instance_id,
                "Підготовка Minecraft (кілька хвилин)",
                0.3,
            );
        } else if let Some(task) = line.strip_prefix("> Task :") {
            tasks += 1;
            let task = task.split_whitespace().next().unwrap_or(task);
            let stage = match task {
                "createMinecraftArtifacts" => {
                    "Підготовка Minecraft (кілька хвилин)"
                }
                "compileJava" => "Компіляція Voxy",
                "jar" | "jarJar" => "Пакування jar",
                _ => "Збірка Voxy",
            };
            set_progress(
                instance_id,
                stage,
                0.35 + 0.5 * (tasks as f32 / 12.0).min(1.0),
            );
        }
        log.push_str(&line);
        log.push('\n');
    }
    let exit = child.wait().await?;
    if let Ok(mut pid) = GRADLE_PID.lock() {
        *pid = None;
    }
    io::write(log_path, log.as_bytes()).await?;
    if !exit.success() {
        return Err(crate::ErrorKind::LauncherError(format!(
            "Збірка Voxy не вдалася ({exit}). Лог: {}\n{}",
            log_path.display(),
            log_tail(&log)
        ))
        .into());
    }
    Ok(())
}

/// Качає код Voxy, збирає його і кладе jar у `mods/` примірника (старий Voxy прибирає).
pub async fn build_and_install(instance_id: &str) -> crate::Result<VoxyStatus> {
    {
        let mut p = PROGRESS.lock().map_err(|_| {
            crate::ErrorKind::LauncherError(
                "Voxy: стан збірки зламано".to_string(),
            )
        })?;
        if p.is_some() {
            return Err(crate::ErrorKind::LauncherError(
                "Voxy вже збирається".to_string(),
            )
            .into());
        }
        *p = Some(Progress {
            instance_id: instance_id.to_string(),
            stage: "Початок".to_string(),
            progress: 0.0,
        });
    }
    let _guard = ProgressGuard;
    let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
    if let Ok(mut c) = CANCEL.lock() {
        *c = Some(cancel_tx);
    }

    // Скасування кидає незавершену збірку посеред будь-якого кроку (завантаження, Gradle): її future
    // дропається, процес Gradle уже вбито в [`cancel`].
    let result = tokio::select! {
        result = build_inner(instance_id) => result,
        Ok(()) = cancel_rx => {
            if let Ok(work) = work_root().await {
                let _ = io::remove_dir_all(&work).await;
            }
            tracing::info!("Збірку Voxy скасовано");
            Err(crate::ErrorKind::LauncherError(CANCELLED.to_string()).into())
        }
    };
    drop(_guard);
    result?;
    status(instance_id).await
}

/// Текст помилки скасованої збірки; фронтенд показує його як повідомлення, а не як збій.
pub const CANCELLED: &str = "Збірку Voxy скасовано";

async fn build_inner(instance_id: &str) -> crate::Result<()> {
    if !status(instance_id).await?.compatible {
        return Err(crate::ErrorKind::InputError(format!(
            "Voxy збирається лише для NeoForge {GAME_VERSION}"
        ))
        .into());
    }

    let commit = latest_commit(true).await;
    let jdk_home = install_jdk(instance_id).await?;
    let work = work_root().await?;
    let src = fetch_source(instance_id, &work, &commit).await?;
    let gradle_home = work.join("g");
    let log_path = work.join("build.log");
    set_progress(instance_id, "Запуск Gradle", 0.24);
    run_gradle(instance_id, &jdk_home, &src, &gradle_home, &log_path).await?;

    set_progress(instance_id, "Встановлення в mods", 0.9);
    let libs = src.join("build").join("libs");
    let built = std::fs::read_dir(&libs)?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.extension().is_some_and(|e| e == "jar")
                && !p.to_string_lossy().ends_with("-sources.jar")
        })
        .ok_or_else(|| {
            crate::ErrorKind::LauncherError(
                "Gradle завершився, але jar Voxy не знайдено".to_string(),
            )
        })?;
    let version = built
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.strip_prefix(JAR_PREFIX))
        .unwrap_or("dev")
        .to_string();

    let mods = crate::api::instance::get_full_path(instance_id)
        .await?
        .join("mods");
    io::create_dir_all(&mods).await?;
    let old = {
        let m = mods.clone();
        tokio::task::spawn_blocking(move || voxy_jars(&m)).await?
    };
    for path in old {
        io::remove_file(&path).await?;
    }
    let target = mods.join(format!(
        "{JAR_PREFIX}{version}+{}.jar",
        short_commit(&commit)
    ));
    io::copy(&built, &target).await?;

    let state = State::get().await?;
    crate::state::sync_content_files(instance_id, &state).await?;

    // Кеш збірки (Gradle, Minecraft, код) ~1 ГБ — гравцю він більше не потрібен. JDK лишаємо.
    set_progress(instance_id, "Прибирання", 0.97);
    let _ = io::remove_dir_all(&work).await;
    tracing::info!(
        "Voxy {version} ({commit}) встановлено: {}",
        target.display()
    );

    Ok(())
}

/// Прибирає Voxy з примірника.
pub async fn remove(instance_id: &str) -> crate::Result<VoxyStatus> {
    let mods = crate::api::instance::get_full_path(instance_id)
        .await?
        .join("mods");
    let old = tokio::task::spawn_blocking(move || voxy_jars(&mods)).await?;
    for path in old {
        io::remove_file(&path).await?;
    }
    let state = State::get().await?;
    crate::state::sync_content_files(instance_id, &state).await?;
    status(instance_id).await
}
