//! Nyx-style building blocks: pages, groups and option rows.

pub use nyx_ui::widgets::*;

use gtk::prelude::*;

// ---------- Page / group ----------

// ---------- Open config ----------

// ---------- Rows ----------

/// `child` at most `max` pixels wide, centred.
pub fn clamp(child: &impl IsA<gtk::Widget>, max: i32) -> crate::clamp::Clamp {
    crate::clamp::Clamp::new(child, max)
}

/// A flat icon-only button with a tooltip.
pub fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    let b = gtk::Button::from_icon_name(icon);
    b.add_css_class("flat");
    b.set_tooltip_text(Some(tooltip));
    b.set_valign(gtk::Align::Center);
    b
}

/// A toolbar button: icon plus a label that hides when the window gets narrower
/// than its `priority` class allows (`label-low`, `label-mid`, `label-high`).
pub fn tool_button(icon: &str, text: &str, tooltip: &str, priority: &str) -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("tool-button");
    let c = hbox(8);
    c.append(&gtk::Image::from_icon_name(icon));
    let l = gtk::Label::new(Some(text));
    l.add_css_class("tool-label");
    l.add_css_class(priority);
    c.append(&l);
    b.set_child(Some(&c));
    b.set_tooltip_text(Some(tooltip));
    b
}

