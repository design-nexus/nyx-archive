//! An opened archive at a glance: what it is, one big Extract button with the other
//! ways to extract under it, a peek at what's inside and the archive's other jobs.

use crate::views::browse::Browse;
use crate::{fmt, paths, widgets, window};
use gtk::prelude::*;
use std::path::Path;
use std::rc::Rc;

pub struct Summary {
    pub root: gtk::ScrolledWindow,
    peek_title: gtk::Label,
    tile: gtk::Box,
    name: gtk::Label,
    line: gtk::Label,
    tags: gtk::Box,
    big: widgets::BigButton,
    alt: gtk::Button,
    peek: gtk::Box,
    add: gtk::Button,
    browse: Rc<Browse>,
}

impl Summary {
    pub fn new(browse: Rc<Browse>) -> Rc<Self> {
        let body = widgets::vbox(0);
        body.add_css_class("task-page");
        body.set_valign(gtk::Align::Center);

        let hero = widgets::hbox(16);
        hero.add_css_class("hero");
        let tile = widgets::hbox(0);
        hero.append(&tile);
        let text = widgets::vbox(2);
        text.set_valign(gtk::Align::Center);
        text.set_hexpand(true);
        let name = widgets::label("", "hero-title");
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let line = widgets::label("", "dim");
        line.add_css_class("tnum");
        line.set_wrap(true);
        let tags = widgets::hbox(6);
        tags.set_margin_top(6);
        text.append(&name);
        text.append(&line);
        text.append(&tags);
        hero.append(&text);
        body.append(&hero);

        let big = widgets::big_button("folder-download-symbolic");
        big.button.set_tooltip_text(Some("Extract everything (Ctrl+E)"));
        big.button.set_margin_top(22);
        body.append(&big.button);
        let row2 = widgets::hbox(8);
        row2.add_css_class("row2");
        row2.set_homogeneous(true);
        let choose = widgets::labeled_button("folder-open-symbolic", "Extract to…");
        if let Some(c) = choose.child() {
            c.set_halign(gtk::Align::Center);
        }
        let alt = gtk::Button::with_label("");
        alt.add_css_class("flat");
        alt.add_css_class("ghost");
        row2.append(&choose);
        row2.append(&alt);
        body.append(&row2);

        let peek_card = widgets::vbox(0);
        peek_card.add_css_class("peek");
        let head = widgets::hbox(8);
        head.add_css_class("peek-head");
        let peek_title = widgets::label("Inside", "peek-title");
        peek_title.set_hexpand(true);
        peek_title.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        head.append(&peek_title);
        let all = widgets::link_button("Browse every file", true);
        head.append(&all);
        peek_card.append(&head);
        let peek = widgets::vbox(0);
        peek_card.append(&peek);
        body.append(&peek_card);

        let more = widgets::hbox(6);
        more.add_css_class("more-row");
        more.set_halign(gtk::Align::Center);
        let add = widgets::ghost_button("list-add-symbolic", "Add files");
        add.set_tooltip_text(Some("Add files to this archive"));
        let test = widgets::ghost_button("emblem-ok-symbolic", "Test for errors");
        let show = widgets::ghost_button("folder-symbolic", "Show in folder");
        more.append(&add);
        more.append(&test);
        more.append(&show);
        body.append(&more);

        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .child(&widgets::clamp(&body, 676))
            .vexpand(true)
            .build();

        let this = Rc::new(Summary { root, peek_title, tile, name, line, tags, big, alt, peek, add, browse });
        let b = this.browse.clone();
        this.big.button.connect_clicked(move |_| {
            let (base, new_folder, _) = b.default_target();
            b.extract_all_to(base, new_folder);
        });
        let b = this.browse.clone();
        choose.connect_clicked(move |_| b.choose_and_extract());
        let b = this.browse.clone();
        this.alt.connect_clicked(move |_| {
            let (base, new_folder, _) = b.default_target();
            if new_folder {
                let here = b.source().parent().map(Path::to_path_buf).unwrap_or_default();
                b.extract_all_to(here, false);
            } else {
                b.extract_all_to(base, true);
            }
        });
        let b = this.browse.clone();
        all.connect_clicked(move |_| b.browse_at(&b.facts().inside));
        let b = this.browse.clone();
        this.add.connect_clicked(move |_| b.choose_and_add());
        let b = this.browse.clone();
        test.connect_clicked(move |_| b.test_archive());
        let b = this.browse.clone();
        show.connect_clicked(move |_| window::show_in_folder(&b.source()));

        // Dropping files onto an open archive adds them.
        let target = gtk::DropTarget::new(gtk::gdk::FileList::static_type(), gtk::gdk::DragAction::COPY);
        let b = this.browse.clone();
        target.connect_drop(move |_, value, _, _| {
            let files = crate::views::home::dropped(value);
            if !files.is_empty() {
                b.add_files(files);
            }
            true
        });
        this.root.add_controller(target);
        this
    }

    /// Show the archive that's open now.
    pub fn fill(self: &Rc<Self>) {
        let f = self.browse.facts();
        while let Some(c) = self.tile.first_child() {
            self.tile.remove(&c);
        }
        self.tile.append(&widgets::package_tile(&f.kind, 64));
        self.name.set_text(&f.source.file_name().unwrap_or_default().to_string_lossy());
        let mut line = format!("{} in {}", fmt::count(f.files, "file", "files"), fmt::count(f.folders, "folder", "folders"));
        if f.folders == 0 {
            line = fmt::count(f.files, "file", "files");
        }
        line.push_str(&format!(" · {}, {} unpacked", fmt::size(f.packed), fmt::size(f.unpacked)));
        self.line.set_text(&line);

        while let Some(c) = self.tags.first_child() {
            self.tags.remove(&c);
        }
        self.tags.append(&widgets::format_badge(&f.kind));
        if f.encrypted {
            let (icon, text, tip) = if f.unlocked {
                ("changes-allow-symbolic", "Unlocked", "The password is kept in memory until you close Archive.")
            } else {
                ("changes-prevent-symbolic", "Locked", "Extracting asks for the password.")
            };
            let chip = chip(Some(icon), text);
            chip.set_tooltip_text(Some(tip));
            if f.unlocked {
                chip.add_css_class("ok");
            }
            self.tags.append(&chip);
        }
        let changeable = if f.editable { chip(None, "Can be changed") } else { chip(None, "Read-only") };
        changeable.set_tooltip_text(Some(if f.editable {
            "Files can be added to it and deleted from it."
        } else {
            "7-Zip can read this kind of archive but not change it. Extract it and make a new one to change its contents."
        }));
        self.tags.append(&changeable);
        if f.parts {
            self.tags.append(&chip(None, "In parts"));
        }

        let (base, new_folder, stem) = self.browse.default_target();
        let place = if base == paths::home() {
            "Home".to_string()
        } else {
            base.file_name().unwrap_or_default().to_string_lossy().to_string()
        };
        self.big.title.set_text(&format!("Extract to {place}"));
        self.big.detail.set_text(&if new_folder {
            format!("Into a new folder, {}", paths::pretty(&base.join(&stem)))
        } else {
            format!("Into {}", paths::pretty(&base))
        });
        self.alt.set_label(if new_folder { "Extract here, no new folder" } else { "Extract into a new folder" });
        self.alt.set_tooltip_text(Some(&if new_folder {
            format!("Into {}", f.source.parent().map(paths::pretty).unwrap_or_default())
        } else {
            format!("Into {}", paths::pretty(&base.join(&stem)))
        }));
        self.add.set_visible(f.editable);

        self.peek_title.set_text(&if f.inside.is_empty() { "Inside".to_string() } else { format!("Inside {}", f.inside) });
        while let Some(c) = self.peek.first_child() {
            self.peek.remove(&c);
        }
        if f.top.is_empty() {
            let empty = widgets::label("This archive is empty.", "dim");
            empty.add_css_class("peek-empty");
            self.peek.append(&empty);
        }
        for (e, items) in f.top {
            let row = gtk::Button::new();
            row.add_css_class("flat");
            row.add_css_class("peek-row");
            let c = widgets::hbox(10);
            let name = crate::views::browse::icon_name(&e);
            let icon = gtk::Image::from_icon_name(name);
            icon.add_css_class(match name {
                "folder-symbolic" => "accent-text",
                "image-x-generic-symbolic" => "section-swatch-pink",
                "text-x-generic-symbolic" | "text-x-script-symbolic" => "dim",
                _ => "section-swatch-amber",
            });
            c.append(&icon);
            let n = widgets::label(e.name(), "");
            n.set_hexpand(true);
            n.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            c.append(&n);
            let size = if e.is_dir && e.size == 0 { String::new() } else { fmt::size(e.size) };
            let meta = match items {
                Some(k) if size.is_empty() => fmt::count(k, "item", "items"),
                Some(k) => format!("{} · {size}", fmt::count(k, "item", "items")),
                None => size,
            };
            let m = widgets::label(&meta, "peek-meta");
            m.add_css_class("tnum");
            c.append(&m);
            row.set_child(Some(&c));
            row.set_tooltip_text(Some(if e.is_dir { "Show what's in this folder" } else { "Open it" }));
            let (b, path, dir) = (self.browse.clone(), e.path.clone(), e.is_dir);
            row.connect_clicked(move |_| if dir { b.browse_at(&path) } else { b.open_entry(&path) });
            self.peek.append(&row);
        }
        if f.top_count > 5 {
            let more = widgets::label(&format!("and {} more", fmt::thousands(f.top_count - 5)), "peek-more");
            self.peek.append(&more);
        }
    }
}

fn chip(icon: Option<&str>, text: &str) -> gtk::Box {
    let b = widgets::hbox(5);
    b.add_css_class("chip2");
    b.set_valign(gtk::Align::Center);
    if let Some(icon) = icon {
        let i = gtk::Image::from_icon_name(icon);
        i.set_pixel_size(12);
        b.append(&i);
    }
    b.append(&gtk::Label::new(Some(text)));
    b
}
