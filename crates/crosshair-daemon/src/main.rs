mod overlay;
mod server;
mod watcher;

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;
use crosshair_core::ipc::{Request, Response, Status};
use crosshair_core::{paths, Config, Profile};
use gtk4::gdk;
use gtk4::glib;

use crate::overlay::Overlay;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "crosshair-daemon — draws the crosshair overlay.\n\n\
             It takes no options; drive it with `crosshair <command>`.\n\
             Config:  {}\n\
             Socket:  {}",
            paths::config_file().display(),
            paths::socket_path().display()
        );
        return Ok(());
    }

    let config = Config::load_or_create()?;
    let socket = paths::socket_path();
    let listener = server::bind(&socket)?;
    let _socket_guard = server::SocketGuard(socket);

    gtk4::init()?;

    let (tx, rx) = async_channel::unbounded::<server::Event>();
    server::spawn(listener, tx.clone());
    // Held for the lifetime of the daemon — dropping it stops the watch.
    let _watcher = watcher::spawn(&paths::config_file(), tx.clone())?;

    let app = Rc::new(RefCell::new(App::new(config)?));
    let main_loop = glib::MainLoop::new(None, false);

    glib::MainContext::default().spawn_local({
        let app = Rc::clone(&app);
        let main_loop = main_loop.clone();
        async move {
            while let Ok(event) = rx.recv().await {
                let quitting = matches!(event.request, Request::Quit);
                let response = app.borrow_mut().handle(event.request);
                let _ = event.reply.send(response).await;
                if quitting {
                    app.borrow().overlay.close();
                    main_loop.quit();
                }
            }
        }
    });

    main_loop.run();
    Ok(())
}

struct App {
    config: Config,
    /// Set while the GUI is previewing unsaved changes.
    preview: Option<Profile>,
    overlay: Overlay,
    /// Set once we have complained about not being able to write the config,
    /// so a declarative setup does not spam the log on every toggle.
    warned_readonly: bool,
}

impl App {
    fn new(config: Config) -> Result<Self> {
        let overlay = Overlay::new(
            config.active_profile(),
            config.monitor.as_deref(),
            config.visible,
        )?;
        Ok(Self {
            config,
            preview: None,
            overlay,
            warned_readonly: false,
        })
    }

    /// Writes the config unless it is managed declaratively.
    ///
    /// A failed write must not break the command that triggered it — hiding the
    /// crosshair still has to hide the crosshair on a read-only config.
    fn persist(&mut self) {
        if crosshair_core::config::is_managed() {
            if !self.warned_readonly {
                eprintln!(
                    "crosshair: {} is managed by Nix; changes apply now but are not saved",
                    paths::config_file().display()
                );
                self.warned_readonly = true;
            }
            return;
        }
        if let Err(err) = self.config.save() {
            if !self.warned_readonly {
                eprintln!("crosshair: cannot save the config: {err:#}");
                self.warned_readonly = true;
            }
        }
    }

    /// Whatever should currently be on screen: the preview wins over the config.
    fn effective(&self) -> Profile {
        self.preview.unwrap_or_else(|| self.config.active_profile())
    }

    fn redraw(&self) {
        self.overlay.set_profile(self.effective());
    }

    fn handle(&mut self, request: Request) -> Response {
        match self.handle_inner(request) {
            Ok(response) => response,
            Err(err) => Response::Error {
                message: format!("{err:#}"),
            },
        }
    }

    fn handle_inner(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::Ping | Request::Quit => {}

            Request::Status => return Ok(Response::Status(self.status())),
            Request::GetConfig => return Ok(Response::Config(self.config.clone())),

            Request::Show => self.set_visible(true)?,
            Request::Hide => self.set_visible(false)?,
            Request::Toggle => self.set_visible(!self.config.visible)?,

            Request::SetProfile { name } => {
                if !self.config.profiles.contains_key(&name) {
                    anyhow::bail!("no profile named {name:?}");
                }
                self.config.active = name;
                self.preview = None;
                self.persist();
                self.redraw();
            }

            Request::NextProfile => {
                if let Some(next) = self.config.next_profile() {
                    self.config.active = next;
                    self.preview = None;
                    self.persist();
                    self.redraw();
                }
            }

            Request::Preview { profile } => {
                self.preview = Some(profile);
                self.redraw();
            }

            Request::ClearPreview => {
                self.preview = None;
                self.redraw();
            }

            Request::SaveProfile { name, profile } => {
                self.config.profiles.insert(name.clone(), profile);
                self.config.active = name;
                self.preview = None;
                self.persist();
                self.redraw();
            }

            Request::DeleteProfile { name } => {
                if self.config.profiles.len() <= 1 {
                    anyhow::bail!("refusing to delete the last profile");
                }
                if self.config.profiles.remove(&name).is_none() {
                    anyhow::bail!("no profile named {name:?}");
                }
                if self.config.active == name {
                    self.config.active = self
                        .config
                        .profiles
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or_default();
                }
                self.persist();
                self.redraw();
            }

            Request::SetMonitor { name } => {
                self.config.monitor = name;
                self.persist();
                self.rebuild_overlay()?;
            }

            Request::Reload => {
                let fresh = Config::load_or_create()?;
                // The daemon is a writer too; ignore the echo of our own save
                // so a running preview survives a Show/Hide or a profile switch.
                if fresh == self.config {
                    return Ok(Response::Ok);
                }
                let moved = fresh.monitor != self.config.monitor;
                self.config = fresh;
                self.preview = None;
                if moved {
                    self.rebuild_overlay()?;
                } else {
                    self.overlay.set_visible(self.config.visible);
                    self.redraw();
                }
            }
        }
        Ok(Response::Ok)
    }

    fn set_visible(&mut self, visible: bool) -> Result<()> {
        self.config.visible = visible;
        self.overlay.set_visible(visible);
        self.persist();
        Ok(())
    }

    /// layer-shell picks its output at map time, so moving screens means a new
    /// surface.
    fn rebuild_overlay(&mut self) -> Result<()> {
        let fresh = Overlay::new(
            self.effective(),
            self.config.monitor.as_deref(),
            self.config.visible,
        )?;
        let old = std::mem::replace(&mut self.overlay, fresh);
        old.close();
        Ok(())
    }

    fn status(&self) -> Status {
        let monitors = gdk::Display::default()
            .map(|d| overlay::monitor_names(&d))
            .unwrap_or_default();
        Status {
            visible: self.config.visible,
            active: self.config.active.clone(),
            profiles: self.config.profile_names(),
            monitor: self.config.monitor.clone(),
            monitors,
            previewing: self.preview.is_some(),
        }
    }
}
