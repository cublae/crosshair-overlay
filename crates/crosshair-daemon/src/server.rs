use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use crosshair_core::ipc::{self, Request, Response};

/// A request plus the channel its response goes back on.
pub struct Event {
    pub request: Request,
    pub reply: async_channel::Sender<Response>,
}

pub type Events = async_channel::Sender<Event>;

/// Binds the control socket, failing if another daemon already answers on it.
pub fn bind(path: &Path) -> Result<UnixListener> {
    if ipc::daemon_running() {
        anyhow::bail!("another crosshair daemon is already running");
    }
    if path.exists() {
        // Nobody answered, so the socket is left over from a crash.
        std::fs::remove_file(path)
            .with_context(|| format!("removing stale socket {}", path.display()))?;
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    UnixListener::bind(path).with_context(|| format!("binding {}", path.display()))
}

/// Serves connections on a background thread, forwarding each request to the
/// GTK main loop and writing back whatever it answers.
pub fn spawn(listener: UnixListener, events: Events) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(err) = handle(stream, &events) {
                        eprintln!("crosshair: client error: {err:#}");
                    }
                }
                Err(err) => eprintln!("crosshair: accept failed: {err:#}"),
            }
        }
    });
}

fn handle(stream: UnixStream, events: &Events) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(());
    }

    let response = match serde_json::from_str::<Request>(line.trim()) {
        Ok(request) => dispatch(request, events)?,
        Err(err) => Response::Error {
            message: format!("bad request: {err}"),
        },
    };

    let mut writer = &stream;
    writeln!(writer, "{}", serde_json::to_string(&response)?)?;
    writer.flush()?;
    Ok(())
}

fn dispatch(request: Request, events: &Events) -> Result<Response> {
    let (tx, rx) = async_channel::bounded(1);
    events
        .send_blocking(Event { request, reply: tx })
        .context("main loop is gone")?;
    rx.recv_blocking().context("main loop dropped the reply")
}

/// Removes the socket on shutdown so the next start doesn't see a stale file.
pub struct SocketGuard(pub PathBuf);

impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
