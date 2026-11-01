//! Project-relative media paths, so a project folder can move with its media.

use std::path::{Component, Path, PathBuf};

/// `target` relative to the directory `base`, written with `/` separators so it
/// works on every OS. `None` if there is no relative path (different drives on
/// Windows, or either path is relative).
pub(crate) fn relative(base: &Path, target: &Path) -> Option<String> {
    if !base.is_absolute() || !target.is_absolute() {
        return None;
    }
    let base: Vec<Component> = base.components().collect();
    let target: Vec<Component> = target.components().collect();
    // Both start with the same prefix and root, or there is no relative path.
    if base.first() != target.first() {
        return None;
    }
    let common = base.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = vec!["..".into(); base.len() - common];
    for component in &target[common..] {
        parts.push(component.as_os_str().to_str()?.to_owned());
    }
    Some(parts.join("/"))
}

/// Joins a `/`-separated relative path onto `base`.
pub(crate) fn resolve(base: &Path, relative: &str) -> PathBuf {
    let mut path = base.to_path_buf();
    for part in relative.split('/').filter(|p| !p.is_empty() && *p != ".") {
        if part == ".." {
            path.pop();
        } else {
            path.push(part);
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        // An absolute root on every OS.
        std::env::temp_dir()
            .ancestors()
            .last()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    #[test]
    fn relative_paths_round_trip() {
        let base = root().join("projects").join("film");
        for target in [
            base.join("media").join("a.mp4"),
            root().join("projects").join("shared").join("b.wav"),
            base.join("c.png"),
        ] {
            let rel = relative(&base, &target).expect("same root");
            assert!(!rel.contains('\\'), "{rel}");
            assert_eq!(resolve(&base, &rel), target, "{rel}");
        }
        assert_eq!(
            relative(&base, &root().join("projects").join("x.mov")).as_deref(),
            Some("../x.mov")
        );
    }

    #[test]
    fn relative_input_has_no_relative_path() {
        assert_eq!(relative(Path::new("a"), Path::new("b")), None);
    }
}
