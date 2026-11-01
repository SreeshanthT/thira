//! Golden-frame testing: compare rendered RGBA frames with reference PNGs.
//!
//! The PRD requires identical output on every OS "within a defined tolerance".
//! [`Tolerance`] is that definition: how far a channel may differ, and what
//! fraction of pixels may exceed it.
//!
//! - Set `THIRA_UPDATE_GOLDEN=1` to write the current frames as the new goldens.
//! - On a mismatch, the actual frame and a diff image (differing pixels in red)
//!   are written to `THIRA_GOLDEN_OUT` (default `target/golden-failures`).

use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

/// How different a frame may be from its golden and still pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tolerance {
    /// Largest allowed absolute difference in any channel (0–255).
    pub max_channel_diff: u8,
    /// Largest allowed fraction of pixels that exceed `max_channel_diff`.
    pub max_bad_fraction: f64,
}

impl Tolerance {
    /// Exact match.
    pub const EXACT: Self = Self {
        max_channel_diff: 0,
        max_bad_fraction: 0.0,
    };

    /// Rounding differences between GPUs and software rasterizers: each channel
    /// within 2 levels, and no pixel beyond that.
    pub const ROUNDING: Self = Self {
        max_channel_diff: 2,
        max_bad_fraction: 0.0,
    };
}

/// An RGBA8 image, rows top to bottom, tightly packed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Result of comparing two images of the same size.
#[derive(Debug, Clone, PartialEq)]
pub struct Comparison {
    /// Pixels where some channel differs by more than the tolerance allows.
    pub bad_pixels: usize,
    pub total_pixels: usize,
    /// Largest channel difference seen anywhere.
    pub max_diff: u8,
    /// Red where a pixel is bad, a dimmed copy of the golden elsewhere.
    pub diff: Image,
}

impl Comparison {
    pub fn bad_fraction(&self) -> f64 {
        #[allow(clippy::cast_precision_loss)]
        let fraction = self.bad_pixels as f64 / self.total_pixels.max(1) as f64;
        fraction
    }

    pub fn passes(&self, tolerance: Tolerance) -> bool {
        self.bad_fraction() <= tolerance.max_bad_fraction
    }
}

/// Compares `actual` with `golden`. Returns `None` if the sizes differ.
pub fn compare(actual: &Image, golden: &Image, tolerance: Tolerance) -> Option<Comparison> {
    if (actual.width, actual.height) != (golden.width, golden.height)
        || actual.rgba.len() != golden.rgba.len()
    {
        return None;
    }
    let mut bad_pixels = 0;
    let mut max_diff = 0;
    let mut diff = Vec::with_capacity(golden.rgba.len());
    let pairs = actual
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .zip(golden.rgba.as_chunks::<4>().0);
    for (a, g) in pairs {
        let worst = a
            .iter()
            .zip(g)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        max_diff = max_diff.max(worst);
        if worst > tolerance.max_channel_diff {
            bad_pixels += 1;
            diff.extend_from_slice(&[255, 0, 0, 255]);
        } else {
            diff.extend(g[..3].iter().map(|c| c / 3));
            diff.push(255);
        }
    }
    Some(Comparison {
        bad_pixels,
        total_pixels: golden.rgba.len() / 4,
        max_diff,
        diff: Image {
            width: golden.width,
            height: golden.height,
            rgba: diff,
        },
    })
}

/// Checks `actual` against `<dir>/<name>.png`, panicking with a readable message
/// (and writing the actual and diff images) if it does not match.
///
/// # Panics
/// Panics when the golden is missing, has a different size, or differs by more
/// than `tolerance`. That is the point: call it from a test.
pub fn assert_golden(dir: &Path, name: &str, actual: &Image, tolerance: Tolerance) {
    let golden_path = dir.join(format!("{name}.png"));
    if std::env::var_os("THIRA_UPDATE_GOLDEN").is_some() {
        write_png(&golden_path, actual)
            .unwrap_or_else(|e| panic!("cannot write {}: {e}", golden_path.display()));
        return;
    }
    let golden = match read_png(&golden_path) {
        Ok(image) => image,
        Err(err) => {
            let saved = save_failure(name, "actual", actual);
            panic!(
                "golden {} could not be read ({err}). Actual frame saved to {}. \
                 Run with THIRA_UPDATE_GOLDEN=1 to create it.",
                golden_path.display(),
                saved.display()
            );
        }
    };
    let Some(result) = compare(actual, &golden, tolerance) else {
        let saved = save_failure(name, "actual", actual);
        panic!(
            "{name}: size {}x{} differs from golden {}x{}. Actual saved to {}",
            actual.width,
            actual.height,
            golden.width,
            golden.height,
            saved.display()
        );
    };
    if !result.passes(tolerance) {
        let saved = save_failure(name, "actual", actual);
        save_failure(name, "diff", &result.diff);
        panic!(
            "{name}: {} of {} pixels ({:.4}%) differ by more than {} (max difference {}). \
             Actual and diff images saved next to {}",
            result.bad_pixels,
            result.total_pixels,
            result.bad_fraction() * 100.0,
            tolerance.max_channel_diff,
            result.max_diff,
            saved.display()
        );
    }
}

fn failure_dir() -> PathBuf {
    std::env::var_os("THIRA_GOLDEN_OUT").map_or_else(
        || {
            // This crate lives in <workspace>/crates/thira-golden.
            let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
            let workspace = manifest.ancestors().nth(2).unwrap_or(manifest);
            workspace.join("target").join("golden-failures")
        },
        PathBuf::from,
    )
}

fn save_failure(name: &str, kind: &str, image: &Image) -> PathBuf {
    let path = failure_dir().join(format!("{name}.{kind}.png"));
    if let Err(err) = write_png(&path, image) {
        eprintln!("could not save {}: {err}", path.display());
    }
    path
}

/// Reads a PNG as RGBA8. Accepts RGB and RGBA 8-bit files.
pub fn read_png(path: &Path) -> Result<Image, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("image too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    let rgba = match (info.color_type, info.bit_depth) {
        (png::ColorType::Rgba, png::BitDepth::Eight) => buf,
        (png::ColorType::Rgb, png::BitDepth::Eight) => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        other => return Err(format!("unsupported PNG format {other:?}")),
    };
    Ok(Image {
        width: info.width,
        height: info.height,
        rgba,
    })
}

/// Writes an RGBA8 PNG, creating parent directories.
pub fn write_png(path: &Path, image: &Image) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&image.rgba)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(width: u32, height: u32, rgba: [u8; 4]) -> Image {
        Image {
            width,
            height,
            rgba: rgba.repeat((width * height) as usize),
        }
    }

    #[test]
    fn identical_images_pass_exactly() {
        let a = solid(4, 4, [10, 20, 30, 255]);
        let r = compare(&a, &a, Tolerance::EXACT).expect("same size");
        assert_eq!((r.bad_pixels, r.max_diff), (0, 0));
        assert!(r.passes(Tolerance::EXACT));
    }

    #[test]
    fn small_channel_differences_are_within_rounding() {
        let golden = solid(4, 4, [100, 100, 100, 255]);
        let actual = solid(4, 4, [102, 99, 100, 255]);
        let r = compare(&actual, &golden, Tolerance::ROUNDING).expect("same size");
        assert!(r.passes(Tolerance::ROUNDING));
        assert_eq!(r.max_diff, 2);
        let strict = compare(&actual, &golden, Tolerance::EXACT).expect("same size");
        assert!(!strict.passes(Tolerance::EXACT));
    }

    #[test]
    fn bad_fraction_limits_outliers() {
        let golden = solid(10, 10, [0, 0, 0, 255]);
        let mut actual = golden.clone();
        actual.rgba[0] = 255; // one pixel of 100
        let loose = Tolerance {
            max_channel_diff: 2,
            max_bad_fraction: 0.01,
        };
        let r = compare(&actual, &golden, loose).expect("same size");
        assert_eq!(r.bad_pixels, 1);
        assert!(r.passes(loose));
        assert!(!r.passes(Tolerance::ROUNDING));
        assert_eq!(&r.diff.rgba[..4], &[255, 0, 0, 255], "bad pixel marked red");
    }

    #[test]
    fn different_sizes_do_not_compare() {
        assert!(compare(&solid(2, 2, [0; 4]), &solid(2, 3, [0; 4]), Tolerance::EXACT).is_none());
    }

    #[test]
    fn png_round_trip() {
        let dir = std::env::temp_dir().join(format!("thira-golden-{}", std::process::id()));
        let path = dir.join("x.png");
        let image = Image {
            width: 3,
            height: 2,
            rgba: (0..24).collect(),
        };
        write_png(&path, &image).expect("write");
        assert_eq!(read_png(&path).expect("read"), image);
        let _ = fs::remove_dir_all(dir);
    }
}
