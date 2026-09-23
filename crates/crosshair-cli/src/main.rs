use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use anyhow::{bail, Context as _, Result};
use clap::{Parser, Subcommand};
use crosshair_core::ipc::{self, Request, Response};
use crosshair_core::{paths, presets};

/// Crosshair overlay for Wayland — control the daemon.
#[derive(Parser)]
#[command(name = "crosshair", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start the overlay daemon if it isn't running yet.
    Start {
        /// Run in the foreground instead of detaching.
        #[arg(long)]
        foreground: bool,
    },
    /// Stop the daemon and remove the overlay.
    Stop,
    /// Show what the daemon is currently doing.
    Status,
    /// Show the crosshair.
    Show,
    /// Hide the crosshair.
    Hide,
    /// Flip between shown and hidden — the one to bind to a key.
    Toggle,
    /// Switch to a profile by name, or to the next one.
    Profile {
        /// Profile name; omit together with --next to just list them.
        name: Option<String>,
        /// Cycle to the next profile instead.
        #[arg(long)]
        next: bool,
    },
    /// List the profiles in the config.
    List,
    /// Copy a built-in preset into the config and activate it.
    Preset {
        /// Preset name; omit to list what's available.
        name: Option<String>,
        /// Store it under this profile name instead of the preset's own.
        #[arg(long)]
        save_as: Option<String>,
    },
    /// Choose the output to draw on.
    Monitor {
        /// Connector name such as DP-5; omit to print the current setting.
        name: Option<String>,
        /// Go back to letting the compositor pick.
        #[arg(long, conflicts_with = "name")]
        auto: bool,
    },
    /// Open the settings window (starts the daemon if needed).
    Gui,
    /// Open the config file in $EDITOR.
    Edit,
    /// Re-read the config from disk.
    Reload,
    /// Print the paths this build uses.
    Paths,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Cmd::Start { foreground } => start(foreground),
        Cmd::Stop => ok(Request::Quit),
        Cmd::Status => status(),
        Cmd::Show => ok(Request::Show),
        Cmd::Hide => ok(Request::Hide),
        Cmd::Toggle => ok(Request::Toggle),
        Cmd::Profile { name, next } => profile(name, next),
        Cmd::List => list(),
        Cmd::Preset { name, save_as } => preset(name, save_as),
        Cmd::Monitor { name, auto } => monitor(name, auto),
        Cmd::Gui => gui(),
        Cmd::Edit => edit(),
        Cmd::Reload => ok(Request::Reload),
        Cmd::Paths => {
            println!("config  {}", paths::config_file().display());
            println!("socket  {}", paths::socket_path().display());
            Ok(())
        }
    }
}

fn ok(request: Request) -> Result<()> {
    ipc::call(&request)?;
    Ok(())
}

fn status() -> Result<()> {
    let Response::Status(status) = ipc::call(&Request::Status)? else {
        bail!("unexpected reply from the daemon");
    };
    println!(
        "state    {}",
        if status.visible { "visible" } else { "hidden" }
    );
    println!("profile  {}", status.active);
    println!("monitor  {}", status.monitor.as_deref().unwrap_or("(auto)"));
    if !status.monitors.is_empty() {
        println!("outputs  {}", status.monitors.join(", "));
    }
    if status.previewing {
        println!("preview  unsaved changes are on screen");
    }
    Ok(())
}

fn profile(name: Option<String>, next: bool) -> Result<()> {
    match (name, next) {
        (Some(name), _) => ok(Request::SetProfile { name }),
        (None, true) => ok(Request::NextProfile),
        (None, false) => list(),
    }
}

fn list() -> Result<()> {
    let Response::Status(status) = ipc::call(&Request::Status)? else {
        bail!("unexpected reply from the daemon");
    };
    for name in status.profiles {
        let marker = if name == status.active { "*" } else { " " };
        println!("{marker} {name}");
    }
    Ok(())
}

fn preset(name: Option<String>, save_as: Option<String>) -> Result<()> {
    let Some(name) = name else {
        for preset in presets::names() {
            println!("{preset}");
        }
        return Ok(());
    };
    let profile = presets::get(&name)
        .with_context(|| format!("no preset {name:?}; run `crosshair preset` to list them"))?;
    ok(Request::SaveProfile {
        name: save_as.unwrap_or(name),
        profile,
    })
}

fn monitor(name: Option<String>, auto: bool) -> Result<()> {
    if auto {
        return ok(Request::SetMonitor { name: None });
    }
    match name {
        Some(name) => ok(Request::SetMonitor { name: Some(name) }),
        None => {
            let Response::Status(status) = ipc::call(&Request::Status)? else {
                bail!("unexpected reply from the daemon");
            };
            println!("current  {}", status.monitor.as_deref().unwrap_or("(auto)"));
            println!("outputs  {}", status.monitors.join(", "));
            Ok(())
        }
    }
}

fn gui() -> Result<()> {
    start(false)?;
    let exe = sibling("crosshair-gui")?;
    let status = Command::new(&exe)
        .status()
        .with_context(|| format!("running {}", exe.display()))?;
    if !status.success() {
        bail!("{} exited with {status}", exe.display());
    }
    Ok(())
}

fn edit() -> Result<()> {
    let path = paths::config_file();
    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_owned());
    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("running {editor}"))?;
    if !status.success() {
        bail!("{editor} exited with {status}");
    }
    // The daemon's file watcher normally catches this; nudge it in case the
    // editor wrote through a path the watch didn't see.
    let _ = ipc::send(&Request::Reload);
    Ok(())
}

fn start(foreground: bool) -> Result<()> {
    if ipc::daemon_running() {
        if foreground {
            println!("crosshair: daemon already running");
        }
        return Ok(());
    }
    let exe = sibling("crosshair-daemon")?;
    if foreground {
        let status = Command::new(&exe).status()?;
        if !status.success() {
            bail!("{} exited with {status}", exe.display());
        }
        return Ok(());
    }

    Command::new(&exe)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| format!("starting {}", exe.display()))?;

    // Wait for it to answer before the caller sends the next command.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if ipc::daemon_running() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    bail!("daemon did not come up within 5s");
}

/// Finds a companion binary next to this one, falling back to $PATH.
fn sibling(name: &str) -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Ok(PathBuf::from(name))
}
