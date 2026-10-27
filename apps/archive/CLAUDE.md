# Archive

GTK4 (gtk4-rs, no libadwaita) archive manager wrapping the `7zz`/`7z` command line.

- `src/sevenzip/parse.rs`: `7z l -slt` parser and folder tree. `job.rs`: command builders (`plan`), the runner (`run`, progress and cancel) and the tests, including real round trips that skip without 7-Zip.
- `src/views/{home,browse,create,settings}.rs`: the three views and the settings page. `src/window.rs`: window, progress card, toasts, dialogs, file pickers.
- The window is one flat, monospace surface split by hairlines: the view, and a status bar (Back away from the start, `F1 Shortcuts`; while browsing, the read-only tag, the archive line and the counts; settings). The window title names the view (the archive's name while browsing). Browse's `title`, `subtitle`, `readonly` and `status` labels live in those bars (`window::refresh_bars`). Settings is a card over the window (`settings_dialog.rs`), rebuilt each time `views::settings::show` runs, listing its groups by widget name.
- `src/theme.rs`, `style.css`: semantic colour tokens, 15 themes, Follow Omarchy (polled each second).
- 7-Zip redraws progress with backspaces, not carriage returns; the runner splits on both.
- Dev: `NARC_APP_ID=io.github.design_nexus.nyx.Dev ./target/debug/nyx-archive FILE` runs a separate instance.
- Single-stream formats (gz, xz, bz2) list one nameless entry; the parser names it after the archive. A `.tar.*` opens as the inner tar, extracted to the cache (`Browse::open_inner`).
- The browse list is a `gtk::ListView` over a `StringList` of row numbers; rows look their `Entry` up in `State.shown` when bound (no GObject subclass).
- Toolbar labels and table columns hide by window width via the `SHED` table in `window.rs`; rows made later ask `window::fits(class)`.
- Dev: `NARC_MEASURE=1` (or `=460` to adapt to that width first) prints each view's minimum width and what holds it. Every view must fit a narrow tile: no fixed margins (use `widgets::clamp`), wide rows get `adaptive-row`.
- Control (`src/control.rs`): `nyx_core::control::serve_async` on the app's bus name, for Nyx Assistant: `archive.list`, `open`, `extract`, `create` (paths inside home; jobs through `window::run_job`).
