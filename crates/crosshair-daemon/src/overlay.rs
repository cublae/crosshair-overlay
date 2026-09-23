use std::cell::Cell;
use std::rc::Rc;

use anyhow::{anyhow, Result};
use crosshair_core::{draw, Profile};
use gtk4::gdk;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const NAMESPACE: &str = "crosshair-overlay";
const CSS: &str = "window.crosshair-overlay { background-color: transparent; }";

/// A full-output, click-through layer-shell surface with the crosshair on it.
pub struct Overlay {
    window: gtk4::Window,
    profile: Rc<Cell<Profile>>,
    area: gtk4::DrawingArea,
}

impl Overlay {
    pub fn new(profile: Profile, monitor: Option<&str>, visible: bool) -> Result<Self> {
        let display = gdk::Display::default().ok_or_else(|| anyhow!("no display"))?;
        install_css(&display);

        let shared = Rc::new(Cell::new(profile));
        let area = gtk4::DrawingArea::new();
        area.set_draw_func({
            let shared = Rc::clone(&shared);
            move |_, cr, width, height| {
                let profile = shared.get();
                draw::draw(
                    cr,
                    &profile,
                    f64::from(width) / 2.0,
                    f64::from(height) / 2.0,
                );
            }
        });

        let window = gtk4::Window::new();
        window.set_decorated(false);
        window.set_child(Some(&area));
        window.add_css_class(NAMESPACE);

        window.init_layer_shell();
        window.set_namespace(NAMESPACE);
        // The overlay layer sits above fullscreen clients, which is the point.
        window.set_layer(Layer::Overlay);
        window.set_keyboard_mode(KeyboardMode::None);
        // Anchoring all four edges makes the surface span the whole output,
        // so "centre of the surface" is "centre of the screen".
        for edge in [Edge::Left, Edge::Right, Edge::Top, Edge::Bottom] {
            window.set_anchor(edge, true);
        }
        // -1: don't reserve space and don't let bars push us around.
        window.set_exclusive_zone(-1);

        if let Some(name) = monitor {
            match find_monitor(&display, name) {
                Some(m) => window.set_monitor(&m),
                None => eprintln!("crosshair: no monitor named {name:?}, using the default one"),
            }
        }

        // Clicks, scrolls and pointer motion must reach the game underneath.
        window.connect_map(make_click_through);

        let overlay = Self {
            window,
            profile: shared,
            area,
        };
        overlay.set_visible(visible);
        Ok(overlay)
    }

    pub fn set_profile(&self, profile: Profile) {
        self.profile.set(profile);
        self.area.queue_draw();
    }

    pub fn set_visible(&self, visible: bool) {
        if visible {
            self.window.present();
        } else {
            self.window.set_visible(false);
        }
    }

    pub fn close(&self) {
        self.window.close();
    }
}

fn make_click_through(window: &gtk4::Window) {
    let Some(surface) = window.surface() else {
        return;
    };
    let empty = gtk4::cairo::Region::create();
    surface.set_input_region(&empty);
    // GTK recomputes regions on resize, so re-assert on every layout change.
    surface.connect_layout(|surface, _, _| {
        surface.set_input_region(&gtk4::cairo::Region::create());
    });
}

fn install_css(display: &gdk::Display) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_data(CSS);
    gtk4::style_context_add_provider_for_display(
        display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

pub fn find_monitor(display: &gdk::Display, name: &str) -> Option<gdk::Monitor> {
    monitors(display)
        .into_iter()
        .find(|m| m.connector().is_some_and(|c| c == name))
}

pub fn monitors(display: &gdk::Display) -> Vec<gdk::Monitor> {
    let model = display.monitors();
    (0..model.n_items())
        .filter_map(|i| model.item(i))
        .filter_map(|obj| obj.downcast::<gdk::Monitor>().ok())
        .collect()
}

pub fn monitor_names(display: &gdk::Display) -> Vec<String> {
    monitors(display)
        .into_iter()
        .filter_map(|m| m.connector().map(|c| c.to_string()))
        .collect()
}
