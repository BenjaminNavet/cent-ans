//! Repository root detection: `data/` is read by `data-model`, `game/assets/` is Bevy's asset root.

use std::path::{Path, PathBuf};

/// Environment variable that overrides the detection.
pub const ROOT_ENV: &str = "CENT_ANS_ROOT";

/// True when `dir` looks like the repository root.
pub fn is_root(dir: &Path) -> bool {
    dir.join("data").is_dir() && dir.join("game/assets").is_dir()
}

/// First ancestor of `start` (itself included) that is the repository root.
pub fn find_root_from(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| is_root(dir))
        .map(Path::to_path_buf)
}

/// Repository root: `CENT_ANS_ROOT`, else walk up from the working directory, else from the
/// executable (a `.app` bundle starts in `/`, but lives under `client/target/`).
pub fn find_root() -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os(ROOT_ENV) {
        let dir = PathBuf::from(value);
        return if is_root(&dir) {
            Ok(dir)
        } else {
            Err(format!(
                "{ROOT_ENV}={} has no data/ and game/assets/",
                dir.display()
            ))
        };
    }
    let from_cwd = std::env::current_dir()
        .ok()
        .and_then(|dir| find_root_from(&dir));
    let from_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| find_root_from(&exe));
    from_cwd
        .or(from_exe)
        .ok_or_else(|| format!("repository root not found (set {ROOT_ENV})"))
}

/// Absolute asset root handed to Bevy's `AssetPlugin`.
pub fn asset_root(root: &Path) -> PathBuf {
    root.join("game/assets")
}

/// Absolute game data directory handed to `GameData::load`.
pub fn data_dir(root: &Path) -> PathBuf {
    root.join("data")
}
