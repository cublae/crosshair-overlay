use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

use anyhow::{bail, Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::{paths, Config, Profile};

/// Newline-delimited JSON: one `Request` in, one `Response` out.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    /// Liveness probe, also used for the single-instance check.
    Ping,
    Status,
    GetConfig,
    Show,
    Hide,
    Toggle,
    SetProfile {
        name: String,
    },
    NextProfile,
    /// Draw `profile` without touching the config — this is the GUI's live preview.
    Preview {
        profile: Profile,
    },
    /// Drop the preview and go back to the config's active profile.
    ClearPreview,
    /// Store `profile` under `name`, make it active and persist.
    SaveProfile {
        name: String,
        profile: Profile,
    },
    DeleteProfile {
        name: String,
    },
    SetMonitor {
        name: Option<String>,
    },
    /// Re-read the config file from disk.
    Reload,
    Quit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok,
    Status(Status),
    Config(Config),
    Error { message: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    pub visible: bool,
    pub active: String,
    pub profiles: Vec<String>,
    pub monitor: Option<String>,
    pub monitors: Vec<String>,
    pub previewing: bool,
}

/// Sends one request to a running daemon and returns its response.
pub fn send(request: &Request) -> Result<Response> {
    let path = paths::socket_path();
    let stream = UnixStream::connect(&path).with_context(|| {
        format!(
            "no daemon on {} — start it with `crosshair start`",
            path.display()
        )
    })?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;

    let mut writer = &stream;
    let line = serde_json::to_string(request)?;
    writeln!(writer, "{line}")?;
    writer.flush()?;

    let mut reply = String::new();
    BufReader::new(&stream).read_line(&mut reply)?;
    if reply.trim().is_empty() {
        bail!("daemon closed the connection without replying");
    }
    Ok(serde_json::from_str(reply.trim())?)
}

/// Like [`send`], but turns `Response::Error` into an `Err`.
pub fn call(request: &Request) -> Result<Response> {
    match send(request)? {
        Response::Error { message } => bail!("daemon: {message}"),
        other => Ok(other),
    }
}

/// True when a daemon answers on the socket.
pub fn daemon_running() -> bool {
    matches!(send(&Request::Ping), Ok(Response::Ok))
}
