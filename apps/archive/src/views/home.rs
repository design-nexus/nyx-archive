//! The start: one big Open button, New archive beside the other ways in, the recent
//! archives and this session's finished jobs. Files dropped anywhere on it open or
//! compress.

use crate::sevenzip;
use crate::views::progress;
use crate::{fmt, paths, recent, widgets, window};
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct Home {
    pub root: gtk::ScrolledWindow,
    recents: gtk::Box,
    recents_group: gtk::Box,
    earlier: gtk::Box,
    banner: gtk::Box,
}

/// Files dropped onto a drop target.
pub fn dropped(value: &glib::Value) -> Vec<PathBuf> {
    value
        .get::<gdk::FileList>()
        .map(|l| l.files().iter().filter_map(|f| f.path()).collect())
        .unwrap_or_default()
}

/// What to do with dropped files: a lone archive opens, anything else is compressed.
pub fn handle_drop(files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    if files.len() == 1 && files[0].is_file() && sevenzip::looks_like_archive(&files[0]) {
        window::open_archive(files[0].clone());
    } else {
        window::start_create(files);
    }
}

impl Home {
    pub fn new() -> Rc<Self> {
        let body = widgets::vbox(0);
        body.add_css_class("task-page");
        body.set_valign(gtk::Align::Center);

        let banner = widgets::banner(
            "<b>7-Zip isn't installed.</b> Install the <tt>7zip</tt> package (<tt>sudo pacman -S 7zip</tt>), then reopen this window.",
            true,
        );
        banner.set_visible(false);
        banner.set_margin_bottom(18);
        body.append(&banner);

        let hero = widgets::hbox(16);
        hero.add_css_class("hero");
        hero.append(&widgets::package_tile("accent", 64));
        let text = widgets::vbox(2);
        text.set_valign(gtk::Align::Center);
        text.set_hexpand(true);
        let title = widgets::label("Open an archive, or make one", "hero-title");
        title.set_wrap(true);
        text.append(&title);
        let sub = widgets::label("Drop an archive here to open it, or any other files to compress them.", "dim");
        sub.set_wrap(true);
        text.append(&sub);
        hero.append(&text);
        body.append(&hero);

        let big = widgets::big_button("document-open-symbolic");
        big.title.set_text("Open an archive…");
        big.detail.set_text("7z, zip, rar, tar, iso and anything else 7-Zip reads");
        big.button.set_tooltip_text(Some("Open an archive (Ctrl+O)"));
        big.button.set_margin_top(22);
        big.button.connect_clicked(|_| choose_archive());
        body.append(&big.button);
        let row2 = widgets::hbox(8);
        row2.add_css_class("row2");
        row2.set_homogeneous(true);
        let new = widgets::labeled_button("list-add-symbolic", "New archive");
        new.set_tooltip_text(Some("Compress files into a new archive (Ctrl+N)"));
        new.connect_clicked(|_| window::start_create(Vec::new()));
        let keys = gtk::Button::with_label("Keyboard shortcuts");
        keys.add_css_class("flat");
        keys.add_css_class("ghost");
        keys.set_tooltip_text(Some("F1"));
        keys.connect_clicked(|_| crate::views::settings::show_help());
        row2.append(&new);
        row2.append(&keys);
        body.append(&row2);

        // ----- Recent -----
        let recents_group = widgets::vbox(0);
        recents_group.add_css_class("peek");
        let head = widgets::hbox(8);
        head.add_css_class("peek-head");
        let t = widgets::label("Recent", "peek-title");
        t.set_hexpand(true);
        head.append(&t);
        let clear = widgets::link_button("Clear list", false);
        head.append(&clear);
        recents_group.append(&head);
        let recents = widgets::vbox(0);
        recents_group.append(&recents);
        body.append(&recents_group);

        let earlier = widgets::vbox(0);
        body.append(&earlier);

        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .child(&widgets::clamp(&body, 676))
            .vexpand(true)
            .build();
        let target = gtk::DropTarget::new(gdk::FileList::static_type(), gdk::DragAction::COPY);
        let b = body.clone();
        target.connect_enter(move |_, _, _| {
            b.add_css_class("drop-hover");
            gdk::DragAction::COPY
        });
        let b = body.clone();
        target.connect_leave(move |_| b.remove_css_class("drop-hover"));
        let b = body.clone();
        target.connect_drop(move |_, value, _, _| {
            b.remove_css_class("drop-hover");
            handle_drop(dropped(value));
            true
        });
        root.add_controller(target);

        let home = Rc::new(Home { root, recents, recents_group, earlier, banner });
        let h = home.clone();
        clear.connect_clicked(move |_| {
            for p in recent::list() {
                recent::forget(&p);
            }
            h.refresh();
            window::toast("Cleared the recent list. The archives themselves are untouched.");
        });
        home.refresh();
        home
    }

    pub fn show_missing_tool(&self) {
        self.banner.set_visible(true);
    }

    pub fn choose_archive(&self) {
        choose_archive();
    }

    pub fn refresh(&self) {
        while let Some(c) = self.recents.first_child() {
            self.recents.remove(&c);
        }
        let list = recent::list();
        self.recents_group.set_visible(!list.is_empty());
        for path in list {
            self.recents.append(&recent_row(&path));
        }
        while let Some(c) = self.earlier.first_child() {
            self.earlier.remove(&c);
        }
        if progress::has_history() {
            self.earlier.append(&widgets::label("Earlier", "earlier-title"));
            self.earlier.append(&progress::history_card());
        }
    }
}

pub fn choose_archive() {
    window::choose_files("Open archive", true, false, |files| window::open_archive(files[0].clone()));
}

fn recent_row(path: &Path) -> gtk::Box {
    let card = widgets::hbox(0);
    card.add_css_class("recent-row");
    let open = gtk::Button::new();
    open.add_css_class("flat");
    open.add_css_class("peek-row");
    open.set_hexpand(true);
    let c = widgets::hbox(12);
    let kind = widgets::kind_of(path);
    c.append(&widgets::package_tile(&kind, 30));
    let text = widgets::vbox(0);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let name = widgets::label(&path.file_name().unwrap_or_default().to_string_lossy(), "recent-name");
    name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    let dir = path.parent().map(paths::pretty).unwrap_or_default();
    let sub = widgets::label(&dir, "peek-meta");
    sub.set_ellipsize(gtk::pango::EllipsizeMode::Start);
    text.append(&name);
    text.append(&sub);
    c.append(&text);
    if let Ok(meta) = std::fs::metadata(path) {
        let when = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| glib::DateTime::from_unix_local(d.as_secs() as i64).ok())
            .and_then(|d| d.format("%-d %b %Y").ok())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let facts = widgets::label(&format!("{} · {when}", fmt::size(meta.len())), "peek-meta");
        facts.add_css_class("tnum");
        c.append(&facts);
    }
    open.set_child(Some(&c));
    open.set_tooltip_text(Some(&paths::pretty(path)));
    let p = path.to_path_buf();
    open.connect_clicked(move |_| window::open_archive(p.clone()));
    card.append(&open);
    let remove = widgets::icon_button("window-close-symbolic", "Remove from recent");
    remove.add_css_class("recent-remove");
    let p = path.to_path_buf();
    let weak = card.downgrade();
    remove.connect_clicked(move |_| {
        recent::forget(&p);
        if let Some(card) = weak.upgrade()
            && let Some(parent) = card.parent().and_downcast::<gtk::Box>()
        {
            parent.remove(&card);
        }
    });
    card.append(&remove);
    card
}
