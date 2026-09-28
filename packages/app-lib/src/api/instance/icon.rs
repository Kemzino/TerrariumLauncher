use crate::event::InstancePayloadType;
use crate::event::emit::emit_instance;
use crate::state::instances::adapters::sqlite::instance_rows;
use crate::state::{
    EditInstance, InstanceIconBackground, InstanceIconConfig, State,
};
use crate::util::fetch::{sha1_async, write};
use crate::util::io;
use bytes::Bytes;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader, Rgba, RgbaImage};
use std::fs::File as StdFile;
use std::io::{BufRead, BufReader, Cursor, Read, Seek};
use std::path::{Path, PathBuf};

const INSTANCE_ICON_MAX_BYTES: usize = 4 * 1024 * 1024;
const INSTANCE_ICON_MAX_DIMENSION: u32 = 512;
const INSTANCE_ICON_MAX_SOURCE_DIMENSION: u32 = 8_192;
const INSTANCE_ICON_MAX_DECODE_BYTES: u64 = 64 * 1024 * 1024;
const GENERATED_ICON_SIZE: u32 = 256;
const PNG_EXTENSION: &str = "png";
const GIF_EXTENSION: &str = "gif";
/// Скільки байтів початку файлу треба, щоб упізнати формат і полотно GIF
const HEAD_BYTES: usize = 64;
const MAX_ICON_CONFIG_ID_LENGTH: usize = 64;
const MAX_SYMBOL_BYTES: usize = 4 * 1024 * 1024;
const MAX_SYMBOL_DIMENSION: u32 = 4096;

enum LegacyIconAction {
    Keep,
    Normalize,
    Remove,
}

#[derive(Debug, Eq, PartialEq)]
enum ValidatedIconBackground {
    Color([u8; 3]),
    LinearTopDownGradient {
        top_color: [u8; 3],
        bottom_color: [u8; 3],
    },
}

pub async fn edit_icon(
    instance_id: &str,
    icon_path: Option<&Path>,
) -> crate::Result<()> {
    let state = State::get().await?;
    let icon_path = if let Some(icon_path) = icon_path {
        Some(
            cache_icon_from_path(icon_path, &state)
                .await?
                .to_string_lossy()
                .to_string(),
        )
    } else {
        None
    };

    apply_instance_icon(instance_id, icon_path, None, &state).await
}

pub async fn edit_generated_icon(
    instance_id: &str,
    config: InstanceIconConfig,
    symbol_bytes: Vec<u8>,
) -> crate::Result<String> {
    let state = State::get().await?;
    let icon_path =
        cache_generated_icon_with_state(config.clone(), symbol_bytes, &state)
            .await?;

    apply_instance_icon(
        instance_id,
        Some(icon_path.clone()),
        Some(config),
        &state,
    )
    .await?;

    Ok(icon_path)
}

pub async fn edit_generated_icon_if_empty(
    instance_id: &str,
    config: InstanceIconConfig,
    symbol_bytes: Vec<u8>,
) -> crate::Result<Option<String>> {
    let state = State::get().await?;
    let icon_path =
        cache_generated_icon_with_state(config.clone(), symbol_bytes, &state)
            .await?;
    let instance =
        instance_rows::get_instance_display_info(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError("Unknown instance".to_string())
            })?;

    let applied = instance_rows::update_instance_icon_if_empty(
        instance_id,
        &icon_path,
        &config,
        &state.pool,
    )
    .await?;
    if !applied {
        return Ok(None);
    }

    if let Err(error) = super::shared::sync_shared_instance_icon(
        instance_id,
        Some(&icon_path),
        &state,
    )
    .await
    {
        tracing::warn!(
            instance_id,
            error = %error,
            "Failed to sync shared instance icon"
        );
    }

    emit_instance(&instance.id, InstancePayloadType::Edited).await?;

    Ok(Some(icon_path))
}

pub async fn cache_generated_icon(
    config: InstanceIconConfig,
    symbol_bytes: Vec<u8>,
    add_to_recents: bool,
) -> crate::Result<String> {
    let state = State::get().await?;
    let icon_path =
        cache_generated_icon_with_state(config.clone(), symbol_bytes, &state)
            .await?;

    if add_to_recents {
        let mut tx = state.pool.begin().await?;
        instance_rows::update_recent_instance_icon_config(&config, &mut tx)
            .await?;
        tx.commit().await?;
    }

    Ok(icon_path)
}

pub async fn get_recent_icon_configs() -> crate::Result<Vec<InstanceIconConfig>>
{
    let state = State::get().await?;
    instance_rows::get_recent_instance_icon_configs(&state.pool).await
}

async fn cache_generated_icon_with_state(
    config: InstanceIconConfig,
    symbol_bytes: Vec<u8>,
    state: &State,
) -> crate::Result<String> {
    let background = validate_icon_config(&config)?;
    let icon_bytes = tokio::task::spawn_blocking(move || {
        render_generated_icon(background, &symbol_bytes)
    })
    .await??;
    let file = write_cached_icon(Bytes::from(icon_bytes), PNG_EXTENSION, state)
        .await?;

    Ok(file.to_string_lossy().to_string())
}

pub(crate) async fn cache_icon(
    bytes: Bytes,
    state: &State,
) -> crate::Result<PathBuf> {
    let (bytes, extension) = tokio::task::spawn_blocking(move || {
        if looks_like_svg(&bytes) {
            return Err(svg_not_supported_error());
        }

        if looks_like_gif(&bytes) {
            check_gif_dimensions(&bytes)?;
            return Ok((bytes, GIF_EXTENSION));
        }

        normalize_raster(Cursor::new(bytes)).map(|bytes| (bytes, PNG_EXTENSION))
    })
    .await??;

    write_cached_icon(bytes, extension, state).await
}

pub(crate) async fn cache_icon_from_path(
    icon_path: &Path,
    state: &State,
) -> crate::Result<PathBuf> {
    let icon_path = icon_path.to_path_buf();
    let (bytes, extension) = tokio::task::spawn_blocking(move || {
        let file = StdFile::open(&icon_path).map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not open instance icon {}: {error}",
                icon_path.display()
            ))
        })?;
        let mut reader = BufReader::new(file);
        let head = {
            let bytes = reader.fill_buf().map_err(|error| {
                crate::ErrorKind::InputError(format!(
                    "Could not inspect instance icon {}: {error}",
                    icon_path.display()
                ))
            })?;
            bytes[..bytes.len().min(HEAD_BYTES)].to_vec()
        };
        if has_svg_extension(&icon_path) || looks_like_svg(&head) {
            return Err(svg_not_supported_error());
        }

        // Анімована гіфка зберігається лише в первісних байтах: декодування
        // в RGBA й перезапис у PNG лишили б від неї один кадр
        if looks_like_gif(&head) {
            check_gif_dimensions(&head)?;
            reader.rewind().map_err(|error| {
                crate::ErrorKind::InputError(format!(
                    "Could not read instance icon {}: {error}",
                    icon_path.display()
                ))
            })?;
            let mut raw = Vec::new();
            reader.read_to_end(&mut raw).map_err(|error| {
                crate::ErrorKind::InputError(format!(
                    "Could not read instance icon {}: {error}",
                    icon_path.display()
                ))
            })?;
            return Ok((Bytes::from(raw), GIF_EXTENSION));
        }

        normalize_raster(reader).map(|bytes| (bytes, PNG_EXTENSION))
    })
    .await??;

    write_cached_icon(bytes, extension, state).await
}

pub(crate) async fn migrate_legacy_icons() -> crate::Result<()> {
    let state = State::get().await?;
    let instances = instance_rows::list_instances(&state.pool).await?;

    for instance in instances {
        let Some(icon_path) = instance.icon_path.as_deref() else {
            continue;
        };
        let action = match inspect_legacy_icon(Path::new(icon_path)) {
            Ok(action) => action,
            Err(error) => {
                tracing::warn!(
                    instance_id = instance.id,
                    icon_path,
                    error = %error,
                    "Failed to inspect legacy instance icon"
                );
                continue;
            }
        };

        match action {
            LegacyIconAction::Keep => {}
            LegacyIconAction::Normalize => {
                if let Err(error) =
                    edit_icon(&instance.id, Some(Path::new(icon_path))).await
                {
                    tracing::warn!(
                        instance_id = instance.id,
                        icon_path,
                        error = %error,
                        "Failed to normalize legacy instance icon"
                    );
                }
            }
            LegacyIconAction::Remove => {
                if let Err(error) =
                    apply_instance_icon(&instance.id, None, None, &state).await
                {
                    tracing::warn!(
                        instance_id = instance.id,
                        icon_path,
                        error = %error,
                        "Failed to remove legacy SVG instance icon"
                    );
                }
            }
        }
    }

    Ok(())
}

async fn apply_instance_icon(
    instance_id: &str,
    icon_path: Option<String>,
    icon_config: Option<InstanceIconConfig>,
    state: &State,
) -> crate::Result<()> {
    let instance =
        instance_rows::get_instance_display_info(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError("Unknown instance".to_string())
            })?;
    crate::state::edit_instance(
        instance_id,
        EditInstance {
            icon_path: Some(icon_path.clone()),
            icon_config: Some(icon_config),
            ..EditInstance::default()
        },
        &state.pool,
    )
    .await?;

    if let Err(error) = super::shared::sync_shared_instance_icon(
        instance_id,
        icon_path.as_deref(),
        state,
    )
    .await
    {
        tracing::warn!(
            instance_id,
            error = %error,
            "Failed to sync shared instance icon"
        );
    }

    emit_instance(&instance.id, InstancePayloadType::Edited).await?;

    Ok(())
}

async fn write_cached_icon(
    bytes: Bytes,
    extension: &str,
    state: &State,
) -> crate::Result<PathBuf> {
    if bytes.len() >= INSTANCE_ICON_MAX_BYTES {
        return Err(icon_too_large_error());
    }

    let hash = sha1_async(bytes.clone()).await?;
    let path = state
        .directories
        .caches_dir()
        .join("icons")
        .join(format!("{hash}.{extension}"));
    write(&path, &bytes, &state.io_semaphore).await?;

    Ok(io::canonicalize(path)?)
}

fn normalize_raster<R>(reader: R) -> crate::Result<Bytes>
where
    R: BufRead + Seek,
{
    let mut reader = image::ImageReader::new(reader)
        .with_guessed_format()
        .map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not identify instance icon format: {error}"
            ))
        })?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(INSTANCE_ICON_MAX_SOURCE_DIMENSION);
    limits.max_image_height = Some(INSTANCE_ICON_MAX_SOURCE_DIMENSION);
    limits.max_alloc = Some(INSTANCE_ICON_MAX_DECODE_BYTES);
    reader.limits(limits);

    let image = reader.decode().map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Could not decode instance icon: {error}"
        ))
    })?;
    let image = if image.width() > INSTANCE_ICON_MAX_DIMENSION
        || image.height() > INSTANCE_ICON_MAX_DIMENSION
    {
        image.resize(
            INSTANCE_ICON_MAX_DIMENSION,
            INSTANCE_ICON_MAX_DIMENSION,
            image::imageops::FilterType::Lanczos3,
        )
    } else {
        image
    };
    let mut normalized = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image.to_rgba8())
        .write_to(&mut normalized, image::ImageFormat::Png)
        .map_err(|error| {
            crate::ErrorKind::InputError(format!(
                "Could not encode instance icon as PNG: {error}"
            ))
        })?;

    validate_normalized_icon(normalized.into_inner())
}

fn inspect_legacy_icon(icon_path: &Path) -> crate::Result<LegacyIconAction> {
    let metadata = std::fs::metadata(icon_path).map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Could not inspect instance icon {}: {error}",
            icon_path.display()
        ))
    })?;
    let file = StdFile::open(icon_path).map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Could not open instance icon {}: {error}",
            icon_path.display()
        ))
    })?;
    let mut reader = BufReader::new(file);
    let bytes = reader.fill_buf().map_err(|error| {
        crate::ErrorKind::InputError(format!(
            "Could not inspect instance icon {}: {error}",
            icon_path.display()
        ))
    })?;

    if has_svg_extension(icon_path) || looks_like_svg(bytes) {
        return Ok(LegacyIconAction::Remove);
    }

    if metadata.len() < INSTANCE_ICON_MAX_BYTES as u64
        && matches!(
            image::guess_format(bytes).ok(),
            Some(image::ImageFormat::Png | image::ImageFormat::Gif)
        )
    {
        return Ok(LegacyIconAction::Keep);
    }

    Ok(LegacyIconAction::Normalize)
}

fn validate_normalized_icon(normalized: Vec<u8>) -> crate::Result<Bytes> {
    if normalized.len() >= INSTANCE_ICON_MAX_BYTES {
        return Err(icon_too_large_error());
    }

    Ok(Bytes::from(normalized))
}

fn validate_icon_config(
    config: &InstanceIconConfig,
) -> crate::Result<ValidatedIconBackground> {
    let background = match &config.background {
        InstanceIconBackground::Color { value } => {
            ValidatedIconBackground::Color(parse_background_color(value)?)
        }
        InstanceIconBackground::LinearTopDownGradient {
            top_color,
            bottom_color,
        } => ValidatedIconBackground::LinearTopDownGradient {
            top_color: parse_background_color(top_color)?,
            bottom_color: parse_background_color(bottom_color)?,
        },
    };
    validate_icon_config_id("symbol", &config.symbol)?;
    Ok(background)
}

pub(crate) fn validate_generated_icon_config(
    config: &InstanceIconConfig,
) -> crate::Result<()> {
    validate_icon_config(config).map(drop)
}

fn parse_background_color(value: &str) -> crate::Result<[u8; 3]> {
    if value.len() != 7 || !value.starts_with('#') {
        return Err(crate::ErrorKind::InputError(
            "Instance icon background must be a hexadecimal color".to_string(),
        )
        .into());
    }

    let color = u32::from_str_radix(&value[1..], 16).map_err(|_| {
        crate::ErrorKind::InputError(
            "Instance icon background must be a hexadecimal color".to_string(),
        )
    })?;

    Ok([
        ((color >> 16) & 0xff) as u8,
        ((color >> 8) & 0xff) as u8,
        (color & 0xff) as u8,
    ])
}

fn validate_icon_config_id(kind: &str, value: &str) -> crate::Result<()> {
    if value.is_empty()
        || value.len() > MAX_ICON_CONFIG_ID_LENGTH
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
        })
    {
        return Err(crate::ErrorKind::InputError(format!(
            "Instance icon {kind} ID is invalid"
        ))
        .into());
    }

    Ok(())
}

fn render_generated_icon(
    background: ValidatedIconBackground,
    symbol_bytes: &[u8],
) -> crate::Result<Vec<u8>> {
    if symbol_bytes.is_empty() || symbol_bytes.len() > MAX_SYMBOL_BYTES {
        return Err(crate::ErrorKind::InputError(
            "Instance icon symbol must be a PNG smaller than 4 MiB".to_string(),
        )
        .into());
    }

    let reader =
        ImageReader::with_format(Cursor::new(symbol_bytes), ImageFormat::Png);
    let (width, height) =
        reader.into_dimensions().map_err(image_input_error)?;
    if width == 0
        || height == 0
        || width > MAX_SYMBOL_DIMENSION
        || height > MAX_SYMBOL_DIMENSION
    {
        return Err(crate::ErrorKind::InputError(format!(
            "Instance icon symbol dimensions must be between 1 and {MAX_SYMBOL_DIMENSION} pixels"
        ))
        .into());
    }

    let symbol =
        image::load_from_memory_with_format(symbol_bytes, ImageFormat::Png)
            .map_err(image_input_error)?
            .resize_exact(
                GENERATED_ICON_SIZE,
                GENERATED_ICON_SIZE,
                FilterType::Lanczos3,
            )
            .to_rgba8();
    let mut icon = match background {
        ValidatedIconBackground::Color(color) => RgbaImage::from_pixel(
            GENERATED_ICON_SIZE,
            GENERATED_ICON_SIZE,
            Rgba([color[0], color[1], color[2], 255]),
        ),
        ValidatedIconBackground::LinearTopDownGradient {
            top_color,
            bottom_color,
        } => RgbaImage::from_fn(
            GENERATED_ICON_SIZE,
            GENERATED_ICON_SIZE,
            |_, y| {
                let interpolate = |top: u8, bottom: u8| {
                    let distance = i32::from(bottom) - i32::from(top);
                    (i32::from(top)
                        + distance * y as i32
                            / (GENERATED_ICON_SIZE - 1) as i32)
                        as u8
                };
                Rgba([
                    interpolate(top_color[0], bottom_color[0]),
                    interpolate(top_color[1], bottom_color[1]),
                    interpolate(top_color[2], bottom_color[2]),
                    255,
                ])
            },
        ),
    };
    image::imageops::overlay(&mut icon, &symbol, 0, 0);
    for pixel in icon.pixels_mut() {
        pixel[3] = 255;
    }

    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(icon)
        .write_to(&mut encoded, ImageFormat::Png)
        .map_err(image_input_error)?;

    Ok(encoded.into_inner())
}

fn image_input_error(error: image::ImageError) -> crate::Error {
    crate::ErrorKind::InputError(format!(
        "Invalid instance icon symbol: {error}"
    ))
    .into()
}

/// Гіфку впізнаємо за сигнатурою, а не за розширенням: збірки ставлять іконки
/// звідки завгодно, а анімацію треба зберегти саме за вмістом.
fn looks_like_gif(bytes: &[u8]) -> bool {
    bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")
}

/// Розмір полотна з заголовка GIF — щоб не декодувати всі кадри лише заради
/// перевірки, що іконка не надмірна.
fn check_gif_dimensions(bytes: &[u8]) -> crate::Result<()> {
    let header = bytes.get(..10).ok_or_else(|| {
        crate::ErrorKind::InputError(
            "Could not read instance icon: truncated GIF".to_string(),
        )
    })?;
    let width = u32::from(u16::from_le_bytes([header[6], header[7]]));
    let height = u32::from(u16::from_le_bytes([header[8], header[9]]));
    if width == 0
        || height == 0
        || width > INSTANCE_ICON_MAX_SOURCE_DIMENSION
        || height > INSTANCE_ICON_MAX_SOURCE_DIMENSION
    {
        return Err(crate::ErrorKind::InputError(format!(
            "Instance icon is {width}x{height}; GIF icons may be at most              {INSTANCE_ICON_MAX_SOURCE_DIMENSION}x{INSTANCE_ICON_MAX_SOURCE_DIMENSION}"
        ))
        .into());
    }

    Ok(())
}

fn looks_like_svg(bytes: &[u8]) -> bool {
    if image::guess_format(bytes).is_ok() {
        return false;
    }

    bytes[..bytes.len().min(1_024)]
        .windows(4)
        .any(|window| window.eq_ignore_ascii_case(b"<svg"))
}

fn has_svg_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
}

fn icon_too_large_error() -> crate::Error {
    crate::ErrorKind::InputError(format!(
        "Instance icons must be smaller than {INSTANCE_ICON_MAX_BYTES} bytes"
    ))
    .into()
}

fn svg_not_supported_error() -> crate::Error {
    crate::ErrorKind::InputError(
        "SVG instance icons are not supported".to_string(),
    )
    .into()
}

#[cfg(test)]
mod tests {
    use super::{
        GENERATED_ICON_SIZE, InstanceIconBackground, InstanceIconConfig,
        ValidatedIconBackground, render_generated_icon, validate_icon_config,
    };
    use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
    use std::io::Cursor;

    fn png(pixel: Rgba<u8>) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, pixel))
            .write_to(&mut bytes, ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn generated_icon_renders_background_and_symbol() {
        let bytes = render_generated_icon(
            ValidatedIconBackground::Color([10, 20, 30]),
            &png(Rgba([200, 100, 50, 128])),
        )
        .unwrap();
        let icon =
            image::load_from_memory_with_format(&bytes, ImageFormat::Png)
                .unwrap()
                .to_rgba8();

        assert_eq!(
            icon.dimensions(),
            (GENERATED_ICON_SIZE, GENERATED_ICON_SIZE)
        );
        assert_eq!(icon.get_pixel(0, 0), &Rgba([105, 60, 40, 255]));
    }

    #[test]
    fn generated_icon_rejects_invalid_symbol_data() {
        assert!(
            render_generated_icon(
                ValidatedIconBackground::Color([0, 0, 0]),
                b"not a png",
            )
            .is_err()
        );
    }

    #[test]
    fn generated_icon_renders_linear_top_down_gradient() {
        let bytes = render_generated_icon(
            ValidatedIconBackground::LinearTopDownGradient {
                top_color: [10, 20, 30],
                bottom_color: [110, 120, 130],
            },
            &png(Rgba([0, 0, 0, 0])),
        )
        .unwrap();
        let icon =
            image::load_from_memory_with_format(&bytes, ImageFormat::Png)
                .unwrap()
                .to_rgba8();

        assert_eq!(icon.get_pixel(0, 0), &Rgba([10, 20, 30, 255]));
        assert_eq!(
            icon.get_pixel(0, GENERATED_ICON_SIZE - 1),
            &Rgba([110, 120, 130, 255])
        );
    }

    #[test]
    fn generated_icon_config_validates_color_and_symbol_id() {
        assert_eq!(
            validate_icon_config(&InstanceIconConfig {
                background: InstanceIconBackground::Color {
                    value: "#c78aff".to_string(),
                },
                symbol: "dusk_block".to_string(),
            })
            .unwrap(),
            ValidatedIconBackground::Color([199, 138, 255])
        );
        assert!(
            validate_icon_config(&InstanceIconConfig {
                background: InstanceIconBackground::Color {
                    value: "purple".to_string(),
                },
                symbol: "dusk_block".to_string(),
            })
            .is_err()
        );
        assert!(
            validate_icon_config(&InstanceIconConfig {
                background: InstanceIconBackground::Color {
                    value: "#c78aff".to_string(),
                },
                symbol: "dusk-block".to_string(),
            })
            .is_err()
        );
    }
}
