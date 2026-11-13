//! Making a new archive: pick files, then format, level, name and options.

use crate::sevenzip::job::{CreateOpts, Format, Op, Outcome};
use crate::window::JobInfo;
use crate::{cmd, fmt, paths, prefs, widgets, window};
use gtk::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct State {
    inputs: Vec<PathBuf>,
    format: Format,
    level: u8,
    name: String,
    name_touched: bool,
    dir: PathBuf,
    /// The user picked the folder, so adding files no longer moves it.
    dir_touched: bool,
    password: String,
    encrypt_names: bool,
    solid: bool,
    split_mb: u32,
    threads: u32,
    /// Move the inputs to the trash once the archive is made and tested.
    trash_originals: bool,
    /// Total size of the inputs, once measured.
    measured: u64,
}

pub struct Create {
    pub root: gtk::ScrolledWindow,
    title: gtk::Label,
    files_summary: gtk::Label,
    files_link: gtk::Button,
    files_rev: gtk::Revealer,
    files: gtk::Box,
    empty_hint: gtk::Label,
    cards: Vec<(Format, gtk::Button)>,
    name: gtk::Entry,
    ext: gtk::Label,
    dir_label: gtk::Label,
    more_label: gtk::Label,
    password_row: gtk::Box,
    names_row: gtk::Box,
    solid_row: gtk::Box,
    split_row: gtk::Box,
    tar_note: gtk::Label,
    summary: gtk::Label,
    create: gtk::Button,
    state: RefCell<State>,
}

/// The compression levels the slider steps through, with their names.
const LEVELS: [(u8, &str); 5] = [(0, "Store"), (1, "Fast"), (5, "Normal"), (7, "Maximum"), (9, "Ultra")];

/// What each format is for, on its card.
fn about(format: Format) -> (&'static str, &'static str) {
    match format {
        Format::SevenZ => ("Smallest", "Best packing. Opens with 7-Zip and most archive apps."),
        Format::Zip => ("Works everywhere", "Opens on any computer or phone without extra apps."),
        Format::TarGz => ("For Linux", "Keeps permissions. Quick to make and open."),
        Format::TarXz => ("Smaller tar", "Like tar.gz, packed tighter, a little slower."),
    }
}

/// A row of the options card: what it is (and a line about it), then its control.
fn option_row(key: &str, desc: &str, control: &impl IsA<gtk::Widget>) -> gtk::Box {
    let row = widgets::hbox(12);
    row.add_css_class("cc-row");
    let text = widgets::vbox(1);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let k = widgets::label(key, "cc-key");
    k.set_wrap(true);
    text.append(&k);
    if !desc.is_empty() {
        let d = widgets::label(desc, "cc-desc");
        d.set_wrap(true);
        text.append(&d);
    }
    row.append(&text);
    control.set_valign(gtk::Align::Center);
    row.append(control);
    row
}

fn dir_size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else { return 0 };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    std::fs::read_dir(path).map(|d| d.flatten().map(|e| dir_size(&e.path())).sum()).unwrap_or(0)
}

impl Create {
    pub fn new() -> Rc<Self> {
        let p = prefs::get();
        let format = Format::from_id(&p.last_format);
        let level = if LEVELS.iter().any(|(l, _)| *l == p.last_level) { p.last_level } else { 5 };

        let body = widgets::vbox(0);
        body.add_css_class("task-page");
        body.set_valign(gtk::Align::Center);

        // ----- What's being packed -----
        let title = widgets::label("", "hero-title");
        body.append(&title);
        let files_summary = widgets::label("", "dim");
        files_summary.add_css_class("tnum");
        files_summary.set_wrap(true);
        body.append(&files_summary);
        let files_link = widgets::link_button("Add or remove files", true);
        files_link.add_css_class("files-link");
        files_link.set_halign(gtk::Align::Start);
        body.append(&files_link);

        let card = widgets::vbox(0);
        card.add_css_class("files-card");
        let files = widgets::vbox(0);
        card.append(&files);
        let empty_hint = widgets::label("Drop files or folders here, or add them with the buttons below.", "dim");
        empty_hint.add_css_class("files-empty");
        empty_hint.set_wrap(true);
        empty_hint.set_justify(gtk::Justification::Center);
        empty_hint.set_halign(gtk::Align::Center);
        card.append(&empty_hint);
        let buttons = widgets::hbox(8);
        buttons.add_css_class("files-actions");
        let add_files = widgets::labeled_button("list-add-symbolic", "Add files");
        let add_folder = widgets::labeled_button("folder-new-symbolic", "Add folder");
        buttons.append(&add_files);
        buttons.append(&add_folder);
        card.append(&buttons);
        let files_rev = gtk::Revealer::new();
        files_rev.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        files_rev.set_child(Some(&card));
        body.append(&files_rev);

        // ----- Format, as cards -----
        let grid = gtk::Grid::new();
        grid.add_css_class("format-cards");
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(true);
        grid.set_column_spacing(10);
        grid.set_row_spacing(10);
        let mut cards = Vec::new();
        for (i, f) in Format::ALL.into_iter().enumerate() {
            let (name, desc) = about(f);
            let b = gtk::Button::new();
            b.add_css_class("format-card");
            let c = widgets::vbox(4);
            c.append(&widgets::format_badge(f.id()));
            let t = widgets::label(name, "format-title");
            t.set_wrap(true);
            t.set_margin_top(4);
            c.append(&t);
            let d = widgets::label(desc, "format-desc");
            d.set_wrap(true);
            d.set_valign(gtk::Align::Start);
            c.append(&d);
            b.set_child(Some(&c));
            grid.attach(&b, (i % 2) as i32, (i / 2) as i32, 1, 1);
            cards.push((f, b));
        }
        body.append(&grid);

        // ----- How hard to pack -----
        body.append(&widgets::label("How hard to pack", "level-title"));
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, (LEVELS.len() - 1) as f64, 1.0);
        scale.add_css_class("level-scale");
        scale.set_draw_value(false);
        scale.set_round_digits(0);
        scale.set_value(LEVELS.iter().position(|(l, _)| *l == level).unwrap_or(2) as f64);
        scale.update_property(&[gtk::accessible::Property::Label("How hard to pack")]);
        body.append(&scale);
        let ticks = widgets::hbox(0);
        ticks.add_css_class("level-ticks");
        ticks.set_homogeneous(true);
        for (i, (_, name)) in LEVELS.iter().enumerate() {
            let l = widgets::label(name, "");
            l.set_xalign(if i == 0 { 0.0 } else if i == LEVELS.len() - 1 { 1.0 } else { 0.5 });
            ticks.append(&l);
        }
        body.append(&ticks);

        // ----- Name, place and the rest -----
        let opts = widgets::vbox(0);
        opts.add_css_class("card");
        opts.add_css_class("cc");
        let name = gtk::Entry::new();
        name.set_width_chars(10);
        let ext = widgets::label("", "tnum");
        ext.add_css_class("dim");
        let name_box = widgets::hbox(6);
        name_box.add_css_class("name-field");
        name_box.append(&name);
        name_box.append(&ext);
        opts.append(&option_row("Name", "", &name_box));
        let dir_button = gtk::Button::new();
        dir_button.add_css_class("path-button");
        let dc = widgets::hbox(8);
        dc.append(&gtk::Image::from_icon_name("folder-symbolic"));
        let dir_label = gtk::Label::new(None);
        dir_label.set_ellipsize(gtk::pango::EllipsizeMode::Start);
        dir_label.set_max_width_chars(28);
        dc.append(&dir_label);
        dc.append(&gtk::Image::from_icon_name("pan-down-symbolic"));
        dir_button.set_child(Some(&dc));
        dir_button.set_tooltip_text(Some("Choose where the archive is saved"));
        opts.append(&option_row("Save in", "", &dir_button));

        let more = gtk::Button::new();
        more.add_css_class("flat");
        more.add_css_class("more-toggle");
        let mc = widgets::hbox(8);
        let chevron = gtk::Image::from_icon_name("pan-end-symbolic");
        chevron.add_css_class("chevron");
        mc.append(&chevron);
        let more_label = widgets::label("", "");
        more_label.set_wrap(true);
        more_label.set_hexpand(true);
        mc.append(&more_label);
        more.set_child(Some(&mc));
        opts.append(&more);

        let (password_row, password_entry) = {
            let e = gtk::PasswordEntry::new();
            e.set_show_peek_icon(true);
            e.set_width_chars(10);
            (option_row("Password", "Anyone opening the archive will need it.", &e), e)
        };
        let names_switch = gtk::Switch::new();
        let names_row = option_row("Hide file names", "The names are encrypted too, so nobody sees what's inside without the password.", &names_switch);
        let solid_switch = gtk::Switch::new();
        solid_switch.set_active(true);
        let solid_row = option_row("Solid archive", "Smaller for many similar files, but slower to open one file.", &solid_switch);
        let split_opts = widgets::opts(&[("0", "Off"), ("100", "100 MB"), ("650", "650 MB (CD)"), ("700", "700 MB"), ("1024", "1 GB"), ("4000", "4 GB (FAT32)")]);
        let split_dd = widgets::dropdown(&split_opts, "0");
        let split_row = option_row("Split into parts", "Make several smaller files.", &split_dd);
        let thread_opts = widgets::opts(&[("0", "Automatic"), ("1", "1"), ("2", "2"), ("4", "4"), ("8", "8")]);
        let threads_dd = widgets::dropdown(&thread_opts, "0");
        let threads_row = option_row("Processor threads", "Fewer threads leave the computer responsive.", &threads_dd);
        let trash_switch = gtk::Switch::new();
        let trash_row = option_row("Move the originals to the trash", "Only after the new archive has been made and tested. They can be restored from the trash.", &trash_switch);
        let tar_note = widgets::label("tar.gz and tar.xz can't have a password, be split or be solid.", "cc-desc");
        tar_note.add_css_class("cc-note");
        tar_note.set_wrap(true);
        let inner = widgets::vbox(0);
        for w in [password_row.upcast_ref::<gtk::Widget>(), names_row.upcast_ref(), solid_row.upcast_ref(), split_row.upcast_ref(), threads_row.upcast_ref(), trash_row.upcast_ref(), tar_note.upcast_ref()] {
            inner.append(w);
        }
        let more_rev = gtk::Revealer::new();
        more_rev.set_transition_type(gtk::RevealerTransitionType::SlideDown);
        more_rev.set_child(Some(&inner));
        opts.append(&more_rev);
        body.append(&opts);

        // ----- Go -----
        let go = widgets::hbox(8);
        go.add_css_class("go-row");
        let summary = widgets::label("", "tnum");
        summary.add_css_class("dim");
        summary.set_hexpand(true);
        summary.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        let cancel = gtk::Button::with_label("Cancel");
        cancel.add_css_class("flat");
        cancel.add_css_class("ghost");
        cancel.set_tooltip_text(Some("Back (Esc)"));
        let create = widgets::labeled_button("package-x-generic-symbolic", "Create archive");
        create.add_css_class("suggested-action");
        create.add_css_class("go-button");
        go.append(&summary);
        go.append(&cancel);
        go.append(&create);
        body.append(&go);

        let root = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .child(&widgets::clamp(&body, 676))
            .vexpand(true)
            .build();

        let state = State {
            inputs: Vec::new(),
            format,
            level,
            name: String::new(),
            name_touched: false,
            dir: paths::home(),
            dir_touched: false,
            password: String::new(),
            encrypt_names: false,
            solid: true,
            split_mb: 0,
            threads: 0,
            trash_originals: false,
            measured: 0,
        };

        let this = Rc::new(Create {
            root: root.clone(),
            title,
            files_summary,
            files_link: files_link.clone(),
            files_rev: files_rev.clone(),
            files,
            empty_hint,
            cards,
            name: name.clone(),
            ext,
            dir_label,
            more_label,
            password_row,
            names_row,
            solid_row,
            split_row,
            tar_note,
            summary,
            create: create.clone(),
            state: RefCell::new(state),
        });

        // ----- Wiring -----
        for (f, b) in &this.cards {
            let (t, f) = (this.clone(), *f);
            b.connect_clicked(move |_| {
                t.state.borrow_mut().format = f;
                prefs::update(|p| p.last_format = f.id().to_string());
                t.sync();
            });
        }
        let t = this.clone();
        scale.connect_value_changed(move |s| {
            let i = (s.value().round() as usize).min(LEVELS.len() - 1);
            let level = LEVELS[i].0;
            if t.state.borrow().level != level {
                t.state.borrow_mut().level = level;
                prefs::update(|p| p.last_level = level);
            }
        });
        let t = this.clone();
        files_link.connect_clicked(move |_| {
            let open = !t.files_rev.reveals_child();
            t.files_rev.set_reveal_child(open);
            t.update_title();
        });
        let (rv, ch) = (more_rev.clone(), chevron.clone());
        more.connect_clicked(move |_| {
            let open = !rv.reveals_child();
            rv.set_reveal_child(open);
            if open {
                ch.add_css_class("open");
            } else {
                ch.remove_css_class("open");
            }
        });
        let t = this.clone();
        name.connect_changed(move |e| {
            let mut s = t.state.borrow_mut();
            // Text that already matches the state was set by the program, not typed.
            if s.name != e.text() {
                s.name = e.text().to_string();
                s.name_touched = true;
            }
            drop(s);
            t.update_summary();
        });
        let t = this.clone();
        dir_button.connect_clicked(move |_| {
            let t = t.clone();
            window::choose_folder("Save the archive in", move |dir| {
                {
                    let mut s = t.state.borrow_mut();
                    s.dir = dir;
                    s.dir_touched = true;
                }
                t.sync();
            });
        });
        let t = this.clone();
        password_entry.connect_changed(move |e| {
            t.state.borrow_mut().password = e.text().to_string();
            t.update_summary();
        });
        let t = this.clone();
        names_switch.connect_active_notify(move |s| t.state.borrow_mut().encrypt_names = s.is_active());
        let t = this.clone();
        solid_switch.connect_active_notify(move |s| t.state.borrow_mut().solid = s.is_active());
        let t = this.clone();
        split_dd.connect_selected_notify(move |d| {
            if let Some((id, _)) = split_opts.get(d.selected() as usize) {
                t.state.borrow_mut().split_mb = id.parse().unwrap_or(0);
            }
            t.update_summary();
        });
        let t = this.clone();
        trash_switch.connect_active_notify(move |s| {
            t.state.borrow_mut().trash_originals = s.is_active();
            t.update_summary();
        });
        let t = this.clone();
        threads_dd.connect_selected_notify(move |d| {
            if let Some((id, _)) = thread_opts.get(d.selected() as usize) {
                t.state.borrow_mut().threads = id.parse().unwrap_or(0);
            }
        });
        let t = this.clone();
        add_files.connect_clicked(move |_| {
            let t = t.clone();
            window::choose_files("Add files", false, true, move |f| t.add_inputs(f));
        });
        let t = this.clone();
        add_folder.connect_clicked(move |_| {
            let t = t.clone();
            window::choose_folder("Add a folder", move |f| t.add_inputs(vec![f]));
        });
        cancel.connect_clicked(|_| window::back());
        let t = this.clone();
        create.connect_clicked(move |_| t.run());

        let target = gtk::DropTarget::new(gtk::gdk::FileList::static_type(), gtk::gdk::DragAction::COPY);
        let t = this.clone();
        target.connect_drop(move |_, value, _, _| {
            t.add_inputs(crate::views::home::dropped(value));
            true
        });
        root.add_controller(target);

        this.rebuild_files();
        this.sync();
        this
    }

    /// Open the view, optionally with files already chosen.
    pub fn start(self: &Rc<Self>, files: Vec<PathBuf>) {
        if !files.is_empty() {
            self.add_inputs(files);
            self.files_rev.set_reveal_child(false);
        }
        self.update_title();
        window::show("create");
    }

    /// The page's heading: how many things are being packed, their names and size.
    fn update_title(&self) {
        let s = self.state.borrow();
        let n = s.inputs.len();
        self.title.set_text(&match n {
            0 => "Compress files".to_string(),
            _ => format!("Compress {}", fmt::count(n, "item", "items")),
        });
        let names: Vec<String> = s.inputs.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
        let listed = match names.as_slice() {
            [] => String::new(),
            [a] => a.clone(),
            [a, b] => format!("{a} and {b}"),
            [a, b, c] => format!("{a}, {b} and {c}"),
            [a, b, rest @ ..] => format!("{a}, {b} and {} more", rest.len()),
        };
        self.files_summary.set_text(&if n == 0 {
            "Add the files and folders to pack, or drop them on this window.".to_string()
        } else if s.measured > 0 {
            format!("{listed} · {}", fmt::size(s.measured))
        } else {
            listed
        });
        self.files_link.set_visible(n > 0);
        if n == 0 {
            self.files_rev.set_reveal_child(true);
        }
        if let Some(l) = self.files_link.child().and_then(|c| c.first_child()).and_downcast::<gtk::Label>() {
            l.set_text(if self.files_rev.reveals_child() { "Hide the file list" } else { "Add or remove files" });
        }
    }

    pub fn add_inputs(self: &Rc<Self>, files: Vec<PathBuf>) {
        {
            let mut s = self.state.borrow_mut();
            for f in files {
                if f.exists() && !s.inputs.contains(&f) {
                    s.inputs.push(f);
                }
            }
            if !s.dir_touched
                && let Some(parent) = s.inputs.first().and_then(|p| p.parent())
            {
                s.dir = parent.to_path_buf();
            }
            if !s.name_touched || s.name.is_empty() {
                s.name = default_name(&s.inputs);
            }
        }
        self.rebuild_files();
        self.sync();
        self.measure();
    }

    fn remove_input(self: &Rc<Self>, path: &Path) {
        {
            let mut s = self.state.borrow_mut();
            s.inputs.retain(|p| p != path);
            if !s.name_touched {
                s.name = default_name(&s.inputs);
            }
        }
        self.rebuild_files();
        self.sync();
        self.measure();
    }

    /// Add up the sizes off the UI thread: folders can be large.
    fn measure(self: &Rc<Self>) {
        let inputs = self.state.borrow().inputs.clone();
        let n = inputs.len();
        if n == 0 {
            self.state.borrow_mut().measured = 0;
            self.update_title();
            return;
        }
        let this = self.clone();
        cmd::background(
            move || inputs.iter().map(|p| dir_size(p)).sum::<u64>(),
            move |total| {
                if this.state.borrow().inputs.len() == n {
                    this.state.borrow_mut().measured = total;
                    this.update_title();
                }
            },
        );
    }

    fn rebuild_files(self: &Rc<Self>) {
        while let Some(c) = self.files.first_child() {
            self.files.remove(&c);
        }
        let inputs = self.state.borrow().inputs.clone();
        self.empty_hint.set_visible(inputs.is_empty());
        for path in inputs {
            let row = widgets::hbox(10);
            row.add_css_class("input-row");
            let icon = gtk::Image::from_icon_name(if path.is_dir() { "folder-symbolic" } else { "text-x-generic-symbolic" });
            icon.add_css_class(if path.is_dir() { "accent-text" } else { "dim" });
            row.append(&icon);
            let name = widgets::label(&path.file_name().unwrap_or_default().to_string_lossy(), "file-name");
            name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            name.set_hexpand(true);
            row.append(&name);
            let dir = widgets::label(&path.parent().map(paths::pretty).unwrap_or_default(), "dim");
            dir.add_css_class("dim");
            dir.add_css_class("col-hide-narrow");
            dir.set_xalign(1.0);
            dir.set_max_width_chars(40);
            dir.set_ellipsize(gtk::pango::EllipsizeMode::Start);
            row.append(&dir);
            let remove = widgets::icon_button("window-close-symbolic", "Remove");
            let (t, p) = (self.clone(), path.clone());
            remove.connect_clicked(move |_| t.remove_input(&p));
            row.append(&remove);
            self.files.append(&row);
        }
        self.update_title();
        window::apply_narrow();
    }

    /// Bring every dependent control in line with the state.
    fn sync(self: &Rc<Self>) {
        let (format, dir, name) = {
            let s = self.state.borrow();
            (s.format, s.dir.clone(), s.name.clone())
        };
        self.ext.set_text(&format!(".{}", format.extension()));
        if self.name.text() != name {
            self.name.set_text(&name);
        }
        self.dir_label.set_text(&paths::pretty(&dir));
        for (f, b) in &self.cards {
            if *f == format {
                b.add_css_class("on");
            } else {
                b.remove_css_class("on");
            }
        }
        let splits = matches!(format, Format::SevenZ | Format::Zip);
        self.password_row.set_visible(format.can_encrypt());
        self.names_row.set_visible(format.can_encrypt_names());
        self.solid_row.set_visible(format.can_solid());
        self.split_row.set_visible(splits);
        self.tar_note.set_visible(!format.can_encrypt());
        let mut more = Vec::new();
        if format.can_encrypt() {
            more.push("password");
        }
        if format.can_encrypt_names() {
            more.push("hidden names");
        }
        if splits {
            more.push("split into parts");
        }
        if format.can_solid() {
            more.push("solid");
        }
        more.extend(["threads", "trash the originals"]);
        self.more_label.set_text(&format!("More: {}", more.join(", ")));
        self.update_summary();
    }

    fn update_summary(&self) {
        let s = self.state.borrow();
        let output = output_path(&s);
        let problem = name_problem(&s.name).map(str::to_string).or_else(|| {
            (s.split_mb > 0 && matches!(s.format, Format::SevenZ | Format::Zip) && split_parts_exist(&output)).then(|| "Parts with this name already exist. Pick another name.".to_string())
        });
        if problem.is_some() && !s.name.is_empty() {
            self.name.add_css_class("error");
        } else {
            self.name.remove_css_class("error");
        }
        let ready = !s.inputs.is_empty() && problem.is_none();
        self.create.set_sensitive(ready);
        let (text, bad) = if s.inputs.is_empty() {
            ("Add at least one file or folder.".to_string(), false)
        } else if let Some(p) = problem {
            (p, true)
        } else {
            let mut extra = String::new();
            if s.format.can_encrypt() && !s.password.is_empty() {
                extra.push_str(" · password");
            }
            if s.trash_originals {
                extra.push_str(" · originals to trash");
            }
            (format!("{}{extra}", short_path(&output)), false)
        };
        self.summary.set_text(&text);
        self.summary.set_tooltip_text(Some(&paths::pretty(&output)));
        if bad {
            self.summary.add_css_class("danger-text");
        } else {
            self.summary.remove_css_class("danger-text");
        }
    }

    fn run(self: &Rc<Self>) {
        let (opts, output) = {
            let s = self.state.borrow();
            let output = output_path(&s);
            let opts = CreateOpts {
                format: s.format,
                level: s.level,
                output: output.clone(),
                inputs: s.inputs.clone(),
                password: Some(s.password.clone()).filter(|p| !p.is_empty()),
                encrypt_names: s.encrypt_names,
                solid: s.solid,
                split_mb: Some(s.split_mb).filter(|m| *m > 0 && matches!(s.format, Format::SevenZ | Format::Zip)),
                threads: Some(s.threads).filter(|t| *t > 0),
            };
            (opts, output)
        };
        let go = {
            let this = self.clone();
            move || {
                let (opts, output) = (opts.clone(), output.clone());
                let this = this.clone();
                let split = opts.split_mb.is_some();
                let (original, trash) = {
                    let s = this.state.borrow();
                    (s.measured, s.trash_originals)
                };
                let inputs = opts.inputs.clone();
                let password = opts.password.clone();
                let info = JobInfo { total: original, folder: output.parent().map(Path::to_path_buf) };
                window::run_job(Op::Create(opts), info, move |out| match out {
                    Outcome::Ok => {
                        let size = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                        let ratio = if original > 0 && !split { format!(" ({:.0}% of the original)", size as f64 / original as f64 * 100.0) } else { String::new() };
                        let o = output.clone();
                        let o2 = output.clone();
                        window::toast_action(
                            &format!("Created {}{}", output.file_name().unwrap_or_default().to_string_lossy(), if split { String::new() } else { format!(" · {}{ratio}", fmt::size(size)) }),
                            "Show in folder",
                            move || window::show_in_folder(&o),
                        );
                        {
                            let mut s = this.state.borrow_mut();
                            s.inputs.clear();
                            s.name.clear();
                            s.name_touched = false;
                        }
                        this.state.borrow_mut().measured = 0;
                        this.rebuild_files();
                        this.sync();
                        let next = move || {
                            if split {
                                window::show("home");
                            } else {
                                window::open_archive(output);
                            }
                        };
                        if trash {
                            let first = if split { first_volume(&o2) } else { o2.clone() };
                            trash_after_test(first, inputs.clone(), password.clone(), next);
                        } else {
                            next();
                        }
                    }
                    other => window::report_failure(&other),
                });
            }
        };
        let existing = {
            let s = self.state.borrow();
            (s.split_mb == 0 || !matches!(s.format, Format::SevenZ | Format::Zip)) && output_path(&s).exists()
        };
        if existing {
            let p = output_path(&self.state.borrow());
            window::confirm("Replace the existing file?", &format!("{} already exists. Creating the archive replaces it.", paths::pretty(&p)), "Replace", true, go);
        } else {
            go();
        }
    }
}

fn output_path(s: &State) -> PathBuf {
    s.dir.join(format!("{}.{}", s.name.trim(), s.format.extension()))
}

fn default_name(inputs: &[PathBuf]) -> String {
    match inputs {
        [] => String::new(),
        [first, ..] if first.is_dir() => first.file_name().unwrap_or_default().to_string_lossy().to_string(),
        [first, ..] => first.file_stem().or_else(|| first.file_name()).unwrap_or_default().to_string_lossy().to_string(),
    }
}

/// Test the new archive, and only if it's sound move the originals to the trash.
fn trash_after_test(archive: PathBuf, inputs: Vec<PathBuf>, password: Option<String>, then: impl FnOnce() + 'static) {
    window::run_job(Op::Test { archive, password }, JobInfo::default(), move |out| {
        if out != Outcome::Ok {
            window::report_failure(&out);
            window::toast("The new archive didn't pass its test, so the originals were kept.");
            return;
        }
        let mut failed = 0;
        for p in &inputs {
            if gtk::gio::File::for_path(p).trash(gtk::gio::Cancellable::NONE).is_err() {
                failed += 1;
            }
        }
        if failed == 0 {
            window::toast(&format!("Moved {} to the trash.", fmt::count(inputs.len(), "original", "originals")));
        } else {
            window::toast(&format!("{} couldn't be moved to the trash and were kept.", fmt::count(failed, "original", "originals")));
        }
        then();
    });
}

/// `x.7z` split into parts starts at `x.7z.001`.
fn first_volume(output: &Path) -> PathBuf {
    let name = output.file_name().unwrap_or_default().to_string_lossy();
    output.with_file_name(format!("{name}.001"))
}

fn split_parts_exist(output: &Path) -> bool {
    first_volume(output).exists()
}

/// Why this archive name can't be used, if it can't.
fn name_problem(name: &str) -> Option<&'static str> {
    let n = name.trim();
    if n.is_empty() {
        Some("Give the archive a name.")
    } else if n.contains('/') {
        Some("Names can't contain “/”.")
    } else if n == "." || n == ".." {
        Some("Pick a real name.")
    } else if n.len() > 240 {
        Some("That name is too long.")
    } else {
        None
    }
}

/// `~/Documents/work/2026/report.7z` → `~/…/2026/report.7z`.
fn short_path(path: &Path) -> String {
    let full = paths::pretty(path);
    let parts: Vec<&str> = full.split('/').collect();
    if parts.len() <= 4 {
        return full;
    }
    format!("{}/…/{}", parts[0], parts[parts.len() - 2..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_names() {
        assert!(name_problem("").is_some());
        assert!(name_problem("   ").is_some());
        assert!(name_problem("a/b").is_some());
        assert!(name_problem("..").is_some());
        assert!(name_problem("photos 2026").is_none());
    }

    #[test]
    fn shortens_long_paths() {
        assert_eq!(short_path(Path::new("/a/b/c/d/e/f.7z")), "/…/e/f.7z");
        assert_eq!(short_path(Path::new("/a/b.7z")), "/a/b.7z");
        assert_eq!(first_volume(Path::new("/x/y.7z")), PathBuf::from("/x/y.7z.001"));
    }
}
