//! Nyx-style building blocks: pages, groups and option rows.

pub use nyx_ui::widgets::*;

use gtk::prelude::*;
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;

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

/// A borderless button with an icon and a label, for the quieter actions of a page.
pub fn ghost_button(icon: &str, text: &str) -> gtk::Button {
    let b = labeled_button(icon, text);
    b.add_css_class("flat");
    b.add_css_class("ghost");
    b
}

/// Text in the accent colour that acts like a link, with an optional chevron after it.
pub fn link_button(text: &str, chevron: bool) -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("flat");
    b.add_css_class("link-button");
    let c = hbox(4);
    c.append(&gtk::Label::new(Some(text)));
    if chevron {
        c.append(&gtk::Image::from_icon_name("pan-end-symbolic"));
    }
    b.set_child(Some(&c));
    b
}

// ---------- Archive formats ----------

/// The format to show for an archive file, from its name: `TAR.GZ`, `7Z`, `ZIP`.
pub fn kind_of(path: &Path) -> String {
    let name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    for (suffix, kind) in [(".tar.gz", "tar.gz"), (".tar.xz", "tar.xz"), (".tar.bz2", "tar.bz2"), (".tar.zst", "tar.zst"), (".7z.001", "7z"), (".zip.001", "zip")] {
        if name.ends_with(suffix) {
            return kind.to_uppercase();
        }
    }
    path.extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_default()
}

/// The theme swatch a format is drawn in, so each format keeps its colour everywhere.
pub fn swatch_for(kind: &str) -> &'static str {
    match kind.to_lowercase().as_str() {
        "7z" => "violet",
        "zip" | "jar" | "apk" | "epub" => "blue",
        "tar.gz" | "tgz" | "gz" | "gzip" => "teal",
        "tar.xz" | "txz" | "xz" => "green",
        "rar" | "rar5" => "coral",
        "iso" | "img" | "dmg" => "pink",
        _ => "amber",
    }
}

/// A small coloured tag naming a format.
pub fn format_badge(kind: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(&kind.to_uppercase()));
    l.add_css_class("fmt-badge");
    l.add_css_class(&format!("section-swatch-{}", swatch_for(kind)));
    l.set_valign(gtk::Align::Center);
    l.set_halign(gtk::Align::Start);
    l
}

/// The archive's package icon on a tile tinted with its format's colour (`"accent"`
/// for the app's own).
pub fn package_tile(kind: &str, size: i32) -> gtk::Box {
    let tile = hbox(0);
    tile.add_css_class("pkg-tile");
    tile.add_css_class(&if kind == "accent" { "section-accent".to_string() } else { format!("section-swatch-{}", swatch_for(kind)) });
    tile.set_size_request(size, size);
    tile.set_valign(gtk::Align::Center);
    tile.set_halign(gtk::Align::Start);
    tile.set_hexpand(false);
    let icon = gtk::Image::from_icon_name("package-x-generic-symbolic");
    icon.set_pixel_size(size * 55 / 100);
    icon.set_size_request(size, size);
    tile.append(&icon);
    tile
}

/// The one big action of a page: an icon, a title and a line under it.
pub struct BigButton {
    pub button: gtk::Button,
    pub title: gtk::Label,
    pub detail: gtk::Label,
}

pub fn big_button(icon: &str) -> BigButton {
    let button = gtk::Button::new();
    button.add_css_class("big-button");
    button.add_css_class("suggested-action");
    let c = hbox(14);
    let image = gtk::Image::from_icon_name(icon);
    image.set_pixel_size(24);
    c.append(&image);
    let text = vbox(1);
    text.set_hexpand(true);
    let title = label("", "big-title");
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    let detail = label("", "big-detail");
    detail.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    text.append(&title);
    text.append(&detail);
    c.append(&text);
    button.set_child(Some(&c));
    BigButton { button, title, detail }
}

// ---------- Progress ring ----------

/// A round progress gauge with the percentage in the middle.
pub struct Ring {
    pub root: gtk::Overlay,
    area: gtk::DrawingArea,
    label: gtk::Label,
    fraction: Rc<Cell<f64>>,
}

impl Ring {
    pub fn new(size: i32) -> Self {
        let fraction = Rc::new(Cell::new(0.0_f64));
        let area = gtk::DrawingArea::new();
        area.add_css_class("job-ring");
        area.set_content_width(size);
        area.set_content_height(size);
        let f = fraction.clone();
        area.set_draw_func(move |a, cr, w, h| {
            let colour = a.color();
            let line = 8.0;
            let r = f64::from(w.min(h)) / 2.0 - line / 2.0;
            let (cx, cy) = (f64::from(w) / 2.0, f64::from(h) / 2.0);
            cr.set_line_width(line);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            #[allow(deprecated)]
            let track = a.style_context().lookup_color("track");
            match track {
                Some(t) => cr.set_source_rgba(f64::from(t.red()), f64::from(t.green()), f64::from(t.blue()), f64::from(t.alpha())),
                None => cr.set_source_rgba(f64::from(colour.red()), f64::from(colour.green()), f64::from(colour.blue()), 0.14),
            }
            cr.arc(cx, cy, r, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
            let done = f.get().clamp(0.0, 1.0);
            if done > 0.0 {
                let start = -std::f64::consts::FRAC_PI_2;
                cr.set_source_rgba(f64::from(colour.red()), f64::from(colour.green()), f64::from(colour.blue()), 1.0);
                cr.arc(cx, cy, r, start, start + done * std::f64::consts::TAU);
                let _ = cr.stroke();
            }
        });
        let root = gtk::Overlay::new();
        root.set_child(Some(&area));
        root.set_halign(gtk::Align::Center);
        let label = gtk::Label::new(Some("0%"));
        label.add_css_class("ring-label");
        label.add_css_class("tnum");
        root.add_overlay(&label);
        Ring { root, area, label, fraction }
    }

    pub fn set(&self, fraction: f64) {
        self.fraction.set(fraction);
        self.label.set_text(&format!("{:.0}%", fraction.clamp(0.0, 1.0) * 100.0));
        self.area.queue_draw();
    }
}
