mod widgets;

use std::cell::RefCell;
use std::rc::Rc;

use crosshair_core::ipc::{self, Request, Response};
use crosshair_core::{draw, presets, Config, Profile, Rgba, Shape};
use gtk4::gdk;
use gtk4::glib;
use gtk4::prelude::*;

use crate::widgets::{color_row, scale_row, section, spin_row, switch_row};

const APP_ID: &str = "org.crosshair_overlay.Settings";

/// Shared editing state. `loading` suppresses the widget callbacks while we
/// repopulate the form, so refreshing the UI never fires a preview.
struct State {
    config: Config,
    name: String,
    profile: Profile,
    connected: bool,
    loading: bool,
}

impl State {
    fn load() -> Self {
        match ipc::call(&Request::GetConfig) {
            Ok(Response::Config(config)) => {
                let name = config.active.clone();
                let profile = config.active_profile();
                Self {
                    config,
                    name,
                    profile,
                    connected: true,
                    loading: false,
                }
            }
            Ok(_) | Err(_) => {
                eprintln!(
                    "crosshair-gui: no daemon, editing {} without live preview",
                    crosshair_core::paths::config_file().display()
                );
                let config = Config::load_or_create().unwrap_or_default();
                let name = config.active.clone();
                let profile = config.active_profile();
                Self {
                    config,
                    name,
                    profile,
                    connected: false,
                    loading: false,
                }
            }
        }
    }

    fn push_preview(&self) {
        if !self.connected || self.loading {
            return;
        }
        if let Err(err) = ipc::call(&Request::Preview {
            profile: self.profile,
        }) {
            eprintln!("crosshair-gui: preview failed: {err:#}");
        }
    }
}

type Shared = Rc<RefCell<State>>;

fn main() -> glib::ExitCode {
    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(build_ui);
    app.run_with_args::<&str>(&[])
}

fn build_ui(app: &gtk4::Application) {
    let state: Shared = Rc::new(RefCell::new(State::load()));

    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("Crosshair")
        .default_width(720)
        .default_height(560)
        .build();

    let preview = gtk4::DrawingArea::builder()
        .content_width(220)
        .content_height(220)
        .build();
    preview.set_draw_func({
        let state = Rc::clone(&state);
        move |_, cr, width, height| {
            let profile = state.borrow().profile;
            paint_preview_background(cr, width, height);
            let (cx, cy) = (f64::from(width) / 2.0, f64::from(height) / 2.0);
            // Ignore the screen offset here — the preview is about the shape.
            let centred = Profile {
                offset_x: 0.0,
                offset_y: 0.0,
                ..profile
            };
            draw::draw(cr, &centred, cx, cy);
        }
    });

    // Redraws the preview and pushes the change to the overlay.
    let apply: Rc<dyn Fn()> = Rc::new({
        let state = Rc::clone(&state);
        let preview = preview.clone();
        move || {
            preview.queue_draw();
            state.borrow().push_preview();
        }
    });

    let grid = gtk4::Grid::builder()
        .row_spacing(8)
        .column_spacing(12)
        .halign(gtk4::Align::Start)
        .build();
    let mut row = 0;

    // ---- shape ----------------------------------------------------------
    section(&grid, &mut row, "Shape");

    let shape_labels: Vec<&str> = Shape::ALL.iter().map(|s| s.label()).collect();
    let shape_dd = gtk4::DropDown::from_strings(&shape_labels);
    shape_dd.set_halign(gtk4::Align::Start);
    shape_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let apply = Rc::clone(&apply);
        move |dd| {
            let mut st = state.borrow_mut();
            if st.loading {
                return;
            }
            if let Some(shape) = Shape::ALL.get(dd.selected() as usize) {
                st.profile.shape = *shape;
            }
            drop(st);
            apply();
        }
    });
    widgets::labelled(&grid, &mut row, "Shape", &shape_dd);

    let color_btn = color_row(&grid, &mut row, "Colour", &state, &apply, |p| &mut p.color);
    let thickness = scale_row(
        &grid,
        &mut row,
        "Thickness",
        0.5,
        20.0,
        0.5,
        &state,
        &apply,
        |p| &mut p.thickness,
    );
    let length = scale_row(
        &grid,
        &mut row,
        "Length / radius",
        0.0,
        200.0,
        1.0,
        &state,
        &apply,
        |p| &mut p.length,
    );
    let gap = scale_row(
        &grid,
        &mut row,
        "Gap",
        0.0,
        60.0,
        1.0,
        &state,
        &apply,
        |p| &mut p.gap,
    );
    let rotation = scale_row(
        &grid,
        &mut row,
        "Rotation",
        0.0,
        360.0,
        1.0,
        &state,
        &apply,
        |p| &mut p.rotation,
    );
    let opacity = scale_row(
        &grid,
        &mut row,
        "Opacity",
        0.0,
        1.0,
        0.01,
        &state,
        &apply,
        |p| &mut p.opacity,
    );

    // ---- outline --------------------------------------------------------
    section(&grid, &mut row, "Outline");
    let outline_on = switch_row(&grid, &mut row, "Enabled", &state, &apply, |p| {
        &mut p.outline.enabled
    });
    let outline_width = scale_row(
        &grid,
        &mut row,
        "Width",
        0.0,
        6.0,
        0.25,
        &state,
        &apply,
        |p| &mut p.outline.width,
    );
    let outline_color = color_row(&grid, &mut row, "Colour", &state, &apply, |p| {
        &mut p.outline.color
    });

    // ---- centre dot -----------------------------------------------------
    section(&grid, &mut row, "Centre dot");
    let dot_on = switch_row(&grid, &mut row, "Enabled", &state, &apply, |p| {
        &mut p.dot.enabled
    });
    let dot_size = scale_row(
        &grid,
        &mut row,
        "Size",
        0.5,
        20.0,
        0.5,
        &state,
        &apply,
        |p| &mut p.dot.size,
    );

    // ---- placement ------------------------------------------------------
    section(&grid, &mut row, "Placement");
    let offset_x = spin_row(
        &grid,
        &mut row,
        "Offset X",
        -1000.0,
        1000.0,
        &state,
        &apply,
        |p| &mut p.offset_x,
    );
    let offset_y = spin_row(
        &grid,
        &mut row,
        "Offset Y",
        -1000.0,
        1000.0,
        &state,
        &apply,
        |p| &mut p.offset_y,
    );

    let monitor_dd = gtk4::DropDown::from_strings(&["(auto)"]);
    monitor_dd.set_halign(gtk4::Align::Start);
    widgets::labelled(&grid, &mut row, "Monitor", &monitor_dd);

    // ---- profile bar ----------------------------------------------------
    let profile_dd = gtk4::DropDown::from_strings(&[]);
    let name_entry = gtk4::Entry::builder()
        .width_request(180)
        .hexpand(false)
        .placeholder_text("profile name")
        .build();
    let preset_dd = gtk4::DropDown::from_strings(
        &std::iter::once("presets…")
            .chain(presets::names())
            .collect::<Vec<_>>(),
    );
    let save_btn = gtk4::Button::with_label("Save");
    save_btn.add_css_class("suggested-action");
    let delete_btn = gtk4::Button::with_label("Delete");
    let toggle_btn = gtk4::ToggleButton::with_label("Visible");

    // Repopulates every widget from the current state.
    let refresh: Rc<dyn Fn()> = Rc::new({
        let state = Rc::clone(&state);
        let widgets = (
            shape_dd.clone(),
            color_btn.clone(),
            thickness.clone(),
            length.clone(),
            gap.clone(),
            rotation.clone(),
            opacity.clone(),
            outline_on.clone(),
            outline_width.clone(),
            outline_color.clone(),
            dot_on.clone(),
            dot_size.clone(),
            offset_x.clone(),
            offset_y.clone(),
        );
        let profile_dd = profile_dd.clone();
        let name_entry = name_entry.clone();
        let toggle_btn = toggle_btn.clone();
        let preview = preview.clone();
        move || {
            // Setting a widget emits its change signal synchronously, and the
            // handler wants the same RefCell — so snapshot, release the borrow,
            // then touch widgets. `loading` makes those handlers no-ops.
            let (p, name, visible, names) = {
                let mut st = state.borrow_mut();
                st.loading = true;
                (
                    st.profile,
                    st.name.clone(),
                    st.config.visible,
                    st.config.profile_names(),
                )
            };
            let (
                shape_dd,
                color_btn,
                thickness,
                length,
                gap,
                rotation,
                opacity,
                outline_on,
                outline_width,
                outline_color,
                dot_on,
                dot_size,
                offset_x,
                offset_y,
            ) = &widgets;

            let shape_idx = Shape::ALL.iter().position(|s| *s == p.shape).unwrap_or(0);
            shape_dd.set_selected(shape_idx as u32);
            color_btn.set_rgba(&to_gdk(p.color));
            thickness.set_value(p.thickness);
            length.set_value(p.length);
            gap.set_value(p.gap);
            rotation.set_value(p.rotation);
            opacity.set_value(p.opacity);
            outline_on.set_active(p.outline.enabled);
            outline_width.set_value(p.outline.width);
            outline_color.set_rgba(&to_gdk(p.outline.color));
            dot_on.set_active(p.dot.enabled);
            dot_size.set_value(p.dot.size);
            offset_x.set_value(p.offset_x);
            offset_y.set_value(p.offset_y);
            name_entry.set_text(&name);
            toggle_btn.set_active(visible);

            let model =
                gtk4::StringList::new(&names.iter().map(String::as_str).collect::<Vec<_>>());
            profile_dd.set_model(Some(&model));
            if let Some(idx) = names.iter().position(|n| *n == name) {
                profile_dd.set_selected(idx as u32);
            }

            state.borrow_mut().loading = false;
            preview.queue_draw();
        }
    });

    profile_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let refresh = Rc::clone(&refresh);
        let apply = Rc::clone(&apply);
        move |dd| {
            {
                let mut st = state.borrow_mut();
                if st.loading {
                    return;
                }
                let names = st.config.profile_names();
                let Some(name) = names.get(dd.selected() as usize).cloned() else {
                    return;
                };
                if name == st.name {
                    return;
                }
                st.profile = st.config.profiles.get(&name).copied().unwrap_or_default();
                st.name = name;
            }
            refresh();
            apply();
        }
    });

    preset_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        let refresh = Rc::clone(&refresh);
        let apply = Rc::clone(&apply);
        move |dd| {
            let index = dd.selected() as usize;
            if index == 0 {
                return;
            }
            {
                let mut st = state.borrow_mut();
                if st.loading {
                    return;
                }
                let Some(name) = presets::names().get(index - 1).copied() else {
                    return;
                };
                if let Some(profile) = presets::get(name) {
                    st.profile = profile;
                }
            }
            dd.set_selected(0);
            refresh();
            apply();
        }
    });

    monitor_dd.connect_selected_notify({
        let state = Rc::clone(&state);
        move |dd| {
            let st = state.borrow();
            if st.loading || !st.connected {
                return;
            }
            let index = dd.selected() as usize;
            let name = monitor_names(&st).get(index).cloned().flatten();
            drop(st);
            if let Err(err) = ipc::call(&Request::SetMonitor { name }) {
                eprintln!("crosshair-gui: {err:#}");
            }
        }
    });

    save_btn.connect_clicked({
        let state = Rc::clone(&state);
        let refresh = Rc::clone(&refresh);
        let name_entry = name_entry.clone();
        move |_| {
            let name = name_entry.text().trim().to_owned();
            if name.is_empty() {
                return;
            }
            let profile = {
                let mut st = state.borrow_mut();
                let profile = st.profile;
                st.name = name.clone();
                st.config.profiles.insert(name.clone(), profile);
                st.config.active = name.clone();
                profile
            };
            let connected = state.borrow().connected;
            let result = if connected {
                ipc::call(&Request::SaveProfile { name, profile }).map(|_| ())
            } else {
                state.borrow().config.save()
            };
            if let Err(err) = result {
                eprintln!("crosshair-gui: save failed: {err:#}");
            }
            refresh();
        }
    });

    delete_btn.connect_clicked({
        let state = Rc::clone(&state);
        let refresh = Rc::clone(&refresh);
        move |_| {
            let name = state.borrow().name.clone();
            if state.borrow().config.profiles.len() <= 1 {
                eprintln!("crosshair-gui: refusing to delete the last profile");
                return;
            }
            if state.borrow().connected {
                if let Err(err) = ipc::call(&Request::DeleteProfile { name: name.clone() }) {
                    eprintln!("crosshair-gui: {err:#}");
                    return;
                }
            }
            let mut st = state.borrow_mut();
            st.config.profiles.remove(&name);
            let fallback = st
                .config
                .profile_names()
                .first()
                .cloned()
                .unwrap_or_default();
            st.name = fallback.clone();
            st.config.active = fallback;
            st.profile = st.config.active_profile();
            drop(st);
            refresh();
        }
    });

    toggle_btn.connect_toggled({
        let state = Rc::clone(&state);
        move |btn| {
            let mut st = state.borrow_mut();
            if st.loading || !st.connected {
                return;
            }
            st.config.visible = btn.is_active();
            let request = if btn.is_active() {
                Request::Show
            } else {
                Request::Hide
            };
            drop(st);
            if let Err(err) = ipc::call(&request) {
                eprintln!("crosshair-gui: {err:#}");
            }
        }
    });

    // Leaving the editor should not leave an unsaved preview on screen.
    window.connect_close_request({
        let state = Rc::clone(&state);
        move |_| {
            if state.borrow().connected {
                let _ = ipc::send(&Request::ClearPreview);
            }
            glib::Propagation::Proceed
        }
    });

    // ---- layout ---------------------------------------------------------
    let profile_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(8)
        .build();
    profile_bar.append(&gtk4::Label::new(Some("Profile")));
    profile_bar.append(&profile_dd);
    profile_bar.append(&name_entry);
    profile_bar.append(&preset_dd);
    profile_bar.append(&save_btn);
    profile_bar.append(&delete_btn);
    profile_bar.append(&toggle_btn);

    let left = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .build();
    left.append(&preview);

    let scroller = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .child(&grid)
        .build();

    let columns = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(16)
        .build();
    columns.append(&left);
    columns.append(&scroller);

    let root = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .build();
    root.append(&profile_bar);
    root.append(&columns);
    window.set_child(Some(&root));

    populate_monitors(&monitor_dd, &state);
    refresh();
    window.present();
}

/// `None` is the auto entry, the rest are connector names.
fn monitor_names(state: &State) -> Vec<Option<String>> {
    let mut names = vec![None];
    if let Ok(Response::Status(status)) = ipc::call(&Request::Status) {
        names.extend(status.monitors.into_iter().map(Some));
    } else if let Some(current) = state.config.monitor.clone() {
        names.push(Some(current));
    }
    names
}

fn populate_monitors(dd: &gtk4::DropDown, state: &Shared) {
    // Same dance as `refresh`: selecting a row fires the handler, which would
    // otherwise push a redundant SetMonitor while we're just filling the list.
    let (names, current) = {
        let mut st = state.borrow_mut();
        st.loading = true;
        let names = monitor_names(&st);
        (names, st.config.monitor.clone())
    };
    let labels: Vec<String> = names
        .iter()
        .map(|n| n.clone().unwrap_or_else(|| "(auto)".to_owned()))
        .collect();
    let model = gtk4::StringList::new(&labels.iter().map(String::as_str).collect::<Vec<_>>());
    dd.set_model(Some(&model));
    let selected = names.iter().position(|n| *n == current).unwrap_or(0);
    dd.set_selected(selected as u32);
    state.borrow_mut().loading = false;
}

fn to_gdk(color: Rgba) -> gdk::RGBA {
    gdk::RGBA::new(
        color.r as f32,
        color.g as f32,
        color.b as f32,
        color.a as f32,
    )
}

pub fn from_gdk(color: gdk::RGBA) -> Rgba {
    Rgba::new(
        f64::from(color.red()),
        f64::from(color.green()),
        f64::from(color.blue()),
        f64::from(color.alpha()),
    )
}

/// Checkerboard so translucent crosshairs and dark colours stay readable.
fn paint_preview_background(cr: &gtk4::cairo::Context, width: i32, height: i32) {
    const CELL: f64 = 10.0;
    cr.set_source_rgb(0.18, 0.18, 0.20);
    let _ = cr.paint();
    cr.set_source_rgb(0.24, 0.24, 0.26);
    let (w, h) = (f64::from(width), f64::from(height));
    let mut y = 0.0;
    let mut r = 0;
    while y < h {
        let mut x = if r % 2 == 0 { 0.0 } else { CELL };
        while x < w {
            cr.rectangle(x, y, CELL, CELL);
            x += CELL * 2.0;
        }
        y += CELL;
        r += 1;
    }
    let _ = cr.fill();
}
