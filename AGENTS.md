# Archive

GTK4 archive manager backed by 7-Zip. The existing application remains
in apps/archive; all shared dependencies use pinned Git sources.

Commands: `cargo build --release --locked -p nyx-archive`; `./install.sh`

Use the public, pinned shared crates; never import another application's internals.
Keep application names, IDs, paths, D-Bus interfaces, and data formats stable.
Use canonical STYLE.md from the pinned nyx-design dependency. Reuse existing
widgets, theme tokens, icons, and shortcuts. Media Player defines video controls.
Run essential targeted checks only. Do not download models or run inference for
routine verification. Install app updates without opening/restarting them.
Common guidance: https://github.com/design-nexus/nyx/blob/main/docs/AGENTS_COMMON.md
This file carries the essential common rules so offline work remains possible.

Shared dependencies use immutable GitHub revisions. Coordinate public API changes
with the suite manifest and affected consumers in design-nexus/nyx.
Historical application tags preserve provenance; standalone builds use main
or subsequent releases. Original release assets remain in the umbrella repo.
