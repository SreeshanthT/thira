//! Checks that the FFmpeg we link is an LGPL build, and makes its shared
//! libraries findable when this crate's tests run.
//!
//! rusty_ffmpeg does the linking itself, from the FFMPEG_* variables set in
//! .cargo/config.toml. The library directory belongs to a package made by the
//! "FFmpeg (LGPL)" workflow, which records its configure line in BUILDCONF.txt.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Configure options that would make FFmpeg GPL, (L)GPLv3 or non-redistributable.
const FORBIDDEN: [&str; 3] = ["--enable-gpl", "--enable-version3", "--enable-nonfree"];

fn main() {
    println!("cargo:rerun-if-env-changed=FFMPEG_LIBS_DIR");
    let Some(libs) = env::var_os("FFMPEG_LIBS_DIR").map(PathBuf::from) else {
        fail("FFMPEG_LIBS_DIR is not set; .cargo/config.toml normally sets it.");
    };
    if !libs.is_dir() {
        fail(&format!(
            "FFmpeg is missing at {}. Run `pnpm ffmpeg:fetch` first.",
            libs.display()
        ));
    }
    let package = libs.parent().unwrap_or(&libs);
    check_license(&package.join("BUILDCONF.txt"));

    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        copy_dlls(&package.join("bin"));
    } else {
        // Search the package's lib directory at run time. On Linux, use
        // DT_RPATH rather than DT_RUNPATH so FFmpeg's libraries also find each other.
        let flag = if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
            "-Wl,--disable-new-dtags,-rpath,"
        } else {
            "-Wl,-rpath,"
        };
        println!("cargo:rustc-link-arg={flag}{}", libs.display());
    }
}

fn check_license(buildconf: &Path) {
    println!("cargo:rerun-if-changed={}", buildconf.display());
    let config = fs::read_to_string(buildconf).unwrap_or_else(|e| {
        fail(&format!(
            "cannot read {}: {e}. Thira links only FFmpeg packages from `pnpm ffmpeg:fetch`, \
             which record their configure options there.",
            buildconf.display()
        ))
    });
    let options: Vec<&str> = config.split([' ', '"']).collect();
    for option in FORBIDDEN {
        if options.contains(&option) {
            fail(&format!(
                "FFmpeg was configured with {option}; Thira may only link LGPL 2.1+ FFmpeg."
            ));
        }
    }
}

/// Windows looks for DLLs next to the executable, so copy them to where
/// Cargo puts binaries (`target/<profile>`) and tests (`target/<profile>/deps`).
fn copy_dlls(bin: &Path) {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap_or_default());
    // OUT_DIR is target/<profile>/build/<package>-<hash>/out.
    let Some(profile) = out_dir.ancestors().nth(3) else {
        fail(&format!("unexpected OUT_DIR {}", out_dir.display()));
    };
    let dlls = fs::read_dir(bin).unwrap_or_else(|e| fail(&format!("{}: {e}", bin.display())));
    for entry in dlls.flatten() {
        let path = entry.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
        {
            println!("cargo:rerun-if-changed={}", path.display());
            for dir in [profile.to_path_buf(), profile.join("deps")] {
                let target = dir.join(entry.file_name());
                if let Err(e) = fs::create_dir_all(&dir).and_then(|()| fs::copy(&path, &target)) {
                    fail(&format!(
                        "cannot copy {} to {}: {e}",
                        path.display(),
                        target.display()
                    ));
                }
            }
        }
    }
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(1);
}
