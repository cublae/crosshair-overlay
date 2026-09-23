use std::path::PathBuf;

const APP: &str = "crosshair-overlay";

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(".config"))
        .join(APP)
}

pub fn config_file() -> PathBuf {
    std::env::var_os("CROSSHAIR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|| config_dir().join("config.toml"))
}

/// Control socket. Falls back to /tmp when the session has no runtime dir,
/// keeping the uid in the name so two users never collide.
pub fn socket_path() -> PathBuf {
    if let Some(dir) = std::env::var_os("CROSSHAIR_SOCKET") {
        return PathBuf::from(dir);
    }
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) => PathBuf::from(dir).join(format!("{APP}.sock")),
        None => {
            let uid = unsafe { libc_getuid() };
            PathBuf::from(format!("/tmp/{APP}-{uid}.sock"))
        }
    }
}

// Avoids pulling in the libc crate for a single call.
unsafe fn libc_getuid() -> u32 {
    extern "C" {
        fn getuid() -> u32;
    }
    getuid()
}
