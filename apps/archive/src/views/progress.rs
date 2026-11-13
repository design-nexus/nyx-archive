//! While a job runs the window is its progress: a ring, what's being worked on, how
//! long is left and Cancel, with this session's earlier jobs under it.

use crate::{prefs, widgets, window};
use gtk::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// How many finished jobs the page remembers.
const KEEP: usize = 5;

/// A finished job, as the Earlier list shows it.
#[derive(Clone)]
pub struct Done {
    pub ok: bool,
    pub title: String,
    pub detail: String,
    /// Where the result is, for "Show in folder".
    pub folder: Option<PathBuf>,
    /// 7-Zip's full output when it failed, for "Details".
    pub log: Option<String>,
}

thread_local! {
    static HISTORY: RefCell<Vec<Done>> = const { RefCell::new(Vec::new()) };
}

pub fn record(done: Done) {
    HISTORY.with(|h| {
        let mut h = h.borrow_mut();
        h.insert(0, done);
        h.truncate(KEEP);
    });
}

pub fn has_history() -> bool {
    HISTORY.with(|h| !h.borrow().is_empty())
}

/// The finished jobs as a card, newest first.
pub fn history_card() -> gtk::Box {
    let card = widgets::vbox(0);
    card.add_css_class("card");
    card.add_css_class("history");
    for d in HISTORY.with(|h| h.borrow().clone()) {
        let row = widgets::hbox(12);
        row.add_css_class("history-row");
        let icon = gtk::Image::from_icon_name(if d.ok { "emblem-ok-symbolic" } else { "dialog-warning-symbolic" });
        icon.add_css_class(if d.ok { "ok-icon" } else { "bad-icon" });
        row.append(&icon);
        let text = widgets::vbox(1);
        text.set_hexpand(true);
        text.set_valign(gtk::Align::Center);
        let t = widgets::label(&d.title, "history-title");
        t.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let s = widgets::label(&d.detail, "dim");
        s.add_css_class("history-detail");
        s.set_ellipsize(gtk::pango::EllipsizeMode::End);
        text.append(&t);
        text.append(&s);
        row.append(&text);
        if let Some(log) = d.log.clone() {
            let b = gtk::Button::with_label("Details");
            b.add_css_class("small-button");
            b.set_valign(gtk::Align::Center);
            let title = d.title.clone();
            b.connect_clicked(move |_| window::show_details(&title, &log));
            row.append(&b);
        } else if let Some(folder) = d.folder.clone() {
            let b = gtk::Button::with_label("Show in folder");
            b.add_css_class("small-button");
            b.set_valign(gtk::Align::Center);
            b.connect_clicked(move |_| window::show_in_folder(&folder));
            row.append(&b);
        }
        card.append(&row);
    }
    card
}

pub struct Progress {
    pub root: gtk::ScrolledWindow,
    ring: widgets::Ring,
    pub title: gtk::Label,
    pub current: gtk::Label,
    pub readout: gtk::Label,
    pub open_when_done: gtk::CheckButton,
    earlier: gtk::Box,
    history: gtk::Box,
}

impl Progress {
    pub fn new(cancel: impl Fn() + 'static) -> Rc<Self> {
        let body = widgets::vbox(0);
        body.add_css_class("task-page");
        body.set_valign(gtk::Align::Center);

        let run = widgets::vbox(6);
        run.add_css_class("card");
        run.add_css_class("run-card");
        let ring = widgets::Ring::new(120);
        ring.root.set_margin_bottom(10);
        run.append(&ring.root);
        let title = widgets::label("", "run-title");
        title.set_halign(gtk::Align::Center);
        title.set_justify(gtk::Justification::Center);
        title.set_wrap(true);
        let current = widgets::label("", "run-file");
        current.add_css_class("mono");
        current.set_halign(gtk::Align::Center);
        current.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let readout = widgets::label("", "dim");
        readout.add_css_class("tnum");
        readout.set_halign(gtk::Align::Center);
        let open_when_done = gtk::CheckButton::with_label("Open the folder when done");
        open_when_done.add_css_class("run-check");
        open_when_done.set_halign(gtk::Align::Center);
        open_when_done.set_active(prefs::get().open_when_done);
        open_when_done.connect_toggled(|c| {
            let on = c.is_active();
            prefs::update(|p| p.open_when_done = on);
        });
        let stop = gtk::Button::with_label("Cancel");
        stop.set_halign(gtk::Align::Center);
        stop.connect_clicked(move |_| cancel());
        run.append(&title);
        run.append(&current);
        run.append(&readout);
        run.append(&open_when_done);
        run.append(&stop);
        body.append(&run);

        let earlier = widgets::vbox(0);
        earlier.append(&widgets::label("Earlier", "earlier-title"));
        let history = widgets::vbox(0);
        earlier.append(&history);
        body.append(&earlier);

        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .child(&widgets::clamp(&body, 616))
            .vexpand(true)
            .build();
        Rc::new(Progress { root, ring, title, current, readout, open_when_done, earlier, history })
    }

    /// Get ready for a new job.
    pub fn start(&self, title: &str, has_folder: bool) {
        self.title.set_text(title);
        self.current.set_text("");
        self.readout.set_text("Starting…");
        self.ring.set(0.0);
        self.open_when_done.set_active(prefs::get().open_when_done);
        self.open_when_done.set_visible(has_folder);
        while let Some(c) = self.history.first_child() {
            self.history.remove(&c);
        }
        self.earlier.set_visible(has_history());
        if has_history() {
            self.history.append(&history_card());
        }
    }

    pub fn set_fraction(&self, fraction: f64) {
        self.ring.set(fraction);
    }
}
