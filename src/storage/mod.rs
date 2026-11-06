//! Filesystem layout helpers for Saturn Launcher.

use std::env;
use std::io;
use std::path::{Path, PathBuf};

const APP_DIRECTORY_NAME: &str = "Saturn";

/// Returns the platform's roaming application-data directory on Windows.
pub fn suggested_data_directory() -> Option<PathBuf> {
    env::var_os("APPDATA")
        .or_else(|| env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .map(|base| base.join(APP_DIRECTORY_NAME))
}

/// Creates the baseline directory tree used by launcher services.
pub fn ensure_data_layout() -> io::Result<PathBuf> {
    let root = suggested_data_directory().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "APPDATA and LOCALAPPDATA environment variables are not set",
        )
    })?;

    for relative in [
        "config",
        "accounts",
        "metadata",
        "cache/assets",
        "cache/libraries",
        "cache/runtimes",
        "instances/vanilla",
        "instances/snapshot",
        "instances/april-fools",
        "instances/pre-release",
        "instances/old-beta",
        "instances/old-alpha",
        "instances/experimental",
        "logs",
    ] {
        create_directory(&root, relative)?;
    }

    Ok(root)
}

fn create_directory(root: &Path, relative: &str) -> io::Result<()> {
    std::fs::create_dir_all(root.join(relative))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::create_directory;

    #[test]
    fn directory_layout_helper_uses_relative_paths() {
        let root = Path::new("test-root");
        assert_eq!(root.join("config"), Path::new("test-root/config"));
        assert!(create_directory(root, "config").is_ok());
        let _ = std::fs::remove_dir_all(root);
    }
}
