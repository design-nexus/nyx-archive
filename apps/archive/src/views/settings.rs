//! The settings dialog: the window's theme and where files are extracted.

use crate::{paths, prefs, widgets, window};
use gtk::prelude::*;

pub const SHORTCUTS: &[(&[&str], &str)] = &[
    (&["Ctrl", "O"], "Open an archive"),
    (&["Ctrl", "N"], "New archive"),
    (&["Ctrl", "E"], "Extract the selection, or everything"),
    (&["Ctrl", "F"], "Search the archive"),
    (&["Ctrl", "A"], "Select all"),
    (&["Enter"], "Open the folder or file"),
    (&["Backspace"], "Up one folder"),
    (&["Delete"], "Delete from the archive"),
    (&["Menu"], "More actions for the selection (or right-click)"),
    (&["Esc"], "Clear the search, unselect, or go back"),
    (&["Ctrl", ","], "Settings"),
    (&["F1"], "This list (or ?)"),
    (&["Ctrl", "Q"], "Close"),
];

/// The keyboard shortcuts on their own, for F1 and `?`.
pub fn show_help() {
    let (dialog, card) = widgets::dialog("Keyboard shortcuts", 520);
    let list = widgets::vbox(0);
    list.add_css_class("group-list");
    for (keys, what) in SHORTCUTS {
        list.append(&widgets::row(what, "", Some(widgets::key_caps(keys).upcast_ref())));
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        // Scrolls without a scrollbar, which would cover the key caps.
        .vscrollbar_policy(gtk::PolicyType::External)
        .propagate_natural_height(true)
        .max_content_height(560)
        .child(&list)
        .build();
    card.append(&scroll);
    let close = gtk::Button::with_label("Close");
    close.set_halign(gtk::Align::End);
    let d = dialog.clone();
    close.connect_clicked(move |_| d.close());
    card.append(&close);
    dialog.present();
}

/// Settings, as a card over the window (see `settings_dialog`).
pub fn show() {
    let p = prefs::get();

    let body = widgets::vbox(0);
    body.add_css_class("settings-page");
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::External)
        .child(&body)
        .vexpand(true)
        .build();

    // A group: its heading, then one card holding its rows. The dialog lists
    // groups by their widget name.
    let group = |title: &str| {
        let wrapper = widgets::vbox(0);
        wrapper.add_css_class("settings-group");
        wrapper.set_widget_name(title);
        wrapper.append(&widgets::label(&title.to_uppercase(), "group-title"));
        let g = widgets::vbox(0);
        g.add_css_class("group-list");
        wrapper.append(&g);
        body.append(&wrapper);
        g
    };

    // ----- Extracting -----
    let g = group("Extracting");
    let (r, _) = widgets::switch_row("Extract into a new folder", "Files go into a folder named after the archive, so nothing gets mixed in.", p.extract_into_folder, |on| {
        prefs::update(|p| p.extract_into_folder = on);
    });
    g.append(&r);
    let dest_label = if p.extract_dir.is_empty() { "Next to the archive".to_string() } else { paths::pretty(std::path::Path::new(&p.extract_dir)) };
    let dest = gtk::Button::with_label(&dest_label);
    dest.add_css_class("path-button");
    dest.connect_clicked(|b| {
        let b = b.clone();
        window::choose_folder("Extract into", move |dir| {
            b.set_label(&paths::pretty(&dir));
            prefs::update(|p| p.extract_dir = dir.to_string_lossy().to_string());
        });
    });
    g.append(&widgets::row("Extract to", "Where extracted files go.", Some(dest.upcast_ref())));
    let reset = gtk::Button::with_label("Use the archive's folder");
    {
        let dest = dest.clone();
        reset.connect_clicked(move |_| {
            dest.set_label("Next to the archive");
            prefs::update(|p| p.extract_dir.clear());
        });
    }
    g.append(&reset);
    reset.set_halign(gtk::Align::End);
    let (r, _) = widgets::choice_row(
        "If a file already exists",
        "What happens to files with the same name.",
        widgets::opts(&[("ask", "Ask me"), ("replace", "Replace"), ("skip", "Keep existing"), ("rename", "Keep both")]),
        &p.overwrite,
        |id| prefs::update(|p| p.overwrite = id),
    );
    g.append(&r);

    window::open_settings(&scroll, &body);
}
