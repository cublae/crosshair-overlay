use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use crosshair_core::ipc::Request;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::server::Events;

/// Debounce window — editors write configs in bursts (write, rename, chmod).
const QUIET: Duration = Duration::from_millis(120);

/// Watches the config file's directory and asks the daemon to reload on change.
///
/// The directory rather than the file itself, because saving through a
/// temp-file rename replaces the inode a file watch would be holding.
pub fn spawn(config: &Path, events: Events) -> Result<RecommendedWatcher> {
    let dir = config
        .parent()
        .context("config path has no parent directory")?
        .to_path_buf();
    std::fs::create_dir_all(&dir).ok();

    let target = config.to_path_buf();
    let mut last = Instant::now() - QUIET;

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        let Ok(event) = res else { return };
        if !matches!(
            event.kind,
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
        ) {
            return;
        }
        if !event.paths.contains(&target) {
            return;
        }
        if last.elapsed() < QUIET {
            return;
        }
        last = Instant::now();

        let (tx, _rx) = async_channel::bounded(1);
        let _ = events.send_blocking(crate::server::Event {
            request: Request::Reload,
            reply: tx,
        });
    })?;

    watcher
        .watch(&dir, RecursiveMode::NonRecursive)
        .with_context(|| format!("watching {}", dir.display()))?;
    Ok(watcher)
}
