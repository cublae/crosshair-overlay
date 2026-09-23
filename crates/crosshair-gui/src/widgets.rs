//! Small builders for the settings form — each one wires a widget straight to
//! a field of the profile being edited.

use std::rc::Rc;

use crosshair_core::{Profile, Rgba};
use gtk4::prelude::*;

use crate::{from_gdk, Shared};

type Apply = Rc<dyn Fn()>;

/// Bold heading spanning both columns.
pub fn section(grid: &gtk4::Grid, row: &mut i32, title: &str) {
    let label = gtk4::Label::builder()
        .label(format!("<b>{title}</b>"))
        .use_markup(true)
        .halign(gtk4::Align::Start)
        .margin_top(if *row == 0 { 0 } else { 10 })
        .build();
    grid.attach(&label, 0, *row, 2, 1);
    *row += 1;
}

/// Label in the left column, widget in the right one.
pub fn labelled(grid: &gtk4::Grid, row: &mut i32, name: &str, widget: &impl IsA<gtk4::Widget>) {
    let label = gtk4::Label::builder()
        .label(name)
        .halign(gtk4::Align::Start)
        .build();
    grid.attach(&label, 0, *row, 1, 1);
    grid.attach(widget, 1, *row, 1, 1);
    *row += 1;
}

#[allow(clippy::too_many_arguments)]
pub fn scale_row<F>(
    grid: &gtk4::Grid,
    row: &mut i32,
    name: &str,
    min: f64,
    max: f64,
    step: f64,
    state: &Shared,
    apply: &Apply,
    field: F,
) -> gtk4::Scale
where
    F: Fn(&mut Profile) -> &mut f64 + 'static,
{
    let scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, min, max, step);
    // Fixed width: the window may be tiled very wide, and a 2000px slider is
    // unusable for a value that lives in the 0..20 range.
    scale.set_hexpand(false);
    scale.set_width_request(300);
    scale.set_draw_value(true);
    scale.set_value_pos(gtk4::PositionType::Right);
    scale.set_digits(if step < 0.1 { 2 } else { 1 });

    scale.connect_value_changed({
        let state = Rc::clone(state);
        let apply = Rc::clone(apply);
        move |scale| {
            {
                let mut st = state.borrow_mut();
                if st.loading {
                    return;
                }
                *field(&mut st.profile) = scale.value();
            }
            apply();
        }
    });

    labelled(grid, row, name, &scale);
    scale
}

#[allow(clippy::too_many_arguments)]
pub fn spin_row<F>(
    grid: &gtk4::Grid,
    row: &mut i32,
    name: &str,
    min: f64,
    max: f64,
    state: &Shared,
    apply: &Apply,
    field: F,
) -> gtk4::SpinButton
where
    F: Fn(&mut Profile) -> &mut f64 + 'static,
{
    let spin = gtk4::SpinButton::with_range(min, max, 1.0);
    spin.set_halign(gtk4::Align::Start);
    spin.connect_value_changed({
        let state = Rc::clone(state);
        let apply = Rc::clone(apply);
        move |spin| {
            {
                let mut st = state.borrow_mut();
                if st.loading {
                    return;
                }
                *field(&mut st.profile) = spin.value();
            }
            apply();
        }
    });

    labelled(grid, row, name, &spin);
    spin
}

pub fn switch_row<F>(
    grid: &gtk4::Grid,
    row: &mut i32,
    name: &str,
    state: &Shared,
    apply: &Apply,
    field: F,
) -> gtk4::Switch
where
    F: Fn(&mut Profile) -> &mut bool + 'static,
{
    let switch = gtk4::Switch::builder()
        .halign(gtk4::Align::Start)
        .valign(gtk4::Align::Center)
        .build();
    switch.connect_active_notify({
        let state = Rc::clone(state);
        let apply = Rc::clone(apply);
        move |switch| {
            {
                let mut st = state.borrow_mut();
                if st.loading {
                    return;
                }
                *field(&mut st.profile) = switch.is_active();
            }
            apply();
        }
    });

    labelled(grid, row, name, &switch);
    switch
}

pub fn color_row<F>(
    grid: &gtk4::Grid,
    row: &mut i32,
    name: &str,
    state: &Shared,
    apply: &Apply,
    field: F,
) -> gtk4::ColorDialogButton
where
    F: Fn(&mut Profile) -> &mut Rgba + 'static,
{
    let dialog = gtk4::ColorDialog::builder().with_alpha(true).build();
    let button = gtk4::ColorDialogButton::builder()
        .dialog(&dialog)
        .halign(gtk4::Align::Start)
        .build();

    button.connect_rgba_notify({
        let state = Rc::clone(state);
        let apply = Rc::clone(apply);
        move |button| {
            {
                let mut st = state.borrow_mut();
                if st.loading {
                    return;
                }
                *field(&mut st.profile) = from_gdk(button.rgba());
            }
            apply();
        }
    });

    labelled(grid, row, name, &button);
    button
}
