//! Settings as a card over the window: the groups listed on the left, the
//! chosen group's options on the right, and a search across all of them.

use gtk::prelude::*;
use nyx_ui::settings_card::{self, About, Spec, Standard};

pub use nyx_ui::settings_card::{escape, fit, focus_search, is_open};

/// Each settings group's icon and one-line summary in the dialog's list.
const GROUPS: &[(&str, &str, &str)] = &[("Creating", "package-x-generic-symbolic", "Format, compression"), ("Extracting", "folder-download-symbolic", "Where files go, overwriting")];

const SPEC: Spec = Spec {
    groups: GROUPS,
    search_text: None,
    standard: Some(Standard {
        app: "archive",
        about: About {
            name: "Archive",
            icon: "io.github.design_nexus.nyx.Archive",
            version: env!("CARGO_PKG_VERSION"),
            description: "Browse, extract and create archives.",
            licence: "MIT",
            website: "https://github.com/design-nexus/nyx-archive",
            issues: "https://github.com/design-nexus/nyx-archive/issues",
            credits: &[("7-Zip", "packing and unpacking")],
        },
        shortcuts: crate::views::settings::SHORTCUTS,
        reduce_motion: || crate::prefs::get().reduce_motion,
        set_reduce_motion: |on| crate::prefs::update(|p| p.reduce_motion = on),
    }),
};

/// Show the card over `overlay`, holding the settings page `page` (whose
/// groups are the children of `body`). It is rebuilt each time it opens.
pub fn open(overlay: &gtk::Overlay, window: &gtk::ApplicationWindow, page: &gtk::ScrolledWindow, body: &gtk::Box) {
    settings_card::open_page(overlay, window, SPEC, page.upcast_ref(), body);
}
