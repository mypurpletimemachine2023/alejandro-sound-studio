# ACA Sound Studio — ALEJANDRO Creative Academy derivative

Local source fork of [SoundCraft](https://github.com/storytold/soundcraft) at `85ffa1f53ee6d04c4e99b66afd142bd4345d2009`.
This is an independent ACA derivative, not an official ArtCraft distribution or endorsement.

Display branding and the actual default egui palette use purple surfaces and gold controls.
Native document formats, engine/command IDs, extensions, semantic meters/errors, and saved documents remain upstream-compatible.
MIT/Apache license texts, NOTICE, contributor attribution and dependency/asset notices remain.
The upstream ArtCraft trademark artwork was excluded; ACA branding is original code-drawn type/geometry.

Browser build: trunk build --release in apps/soundcraft-web.
Use Rust's wasm32-unknown-unknown target and wasm-bindgen CLI exactly 0.2.129.
Source edits are implemented; successful browser compilation and interactive behavior must be recorded separately.
Do not publish the upstream installers as rebranded Academy binaries or claim upstream features are tested by this palette change.

## Academy build pipeline

The fork-only `Academy WASM build` workflow is manual (`workflow_dispatch`) and uses two sequential jobs on standard public Ubuntu runners. It builds this modified source with Rust1.95 and wasm-bindgen0.2.129; it performs no signing, deployment, or desktop release matrix and reads no user secrets. Browser verification and upstream full-suite CI remain separate gates.

Source: https://github.com/mypurpletimemachine2023/alejandro-sound-studio
# Browser save compatibility follow-up — 2026-11-01

Save Session Copy In now delivers the same self-contained `.scraftz` bundle as normal browser saving, while leaving the working session, path, dirty flag and undo history unchanged. Revert to Saved restores the Engine's existing immutable last-opened/last-successfully-saved session checkpoint, including shared decoded audio, instead of trying to read a downloaded file through the unavailable native filesystem. The existing dirty confirmation and Cancel behavior remain. Native disk-based save/revert behavior is unchanged.

Focused regression tests cover embedded-audio copy reopening, failed delivery, dirty/path/history isolation, loaded and saved checkpoint restoration, audio sharing and rejecting revert after a new unsaved session. A separate manual Academy workflow job runs these tests on a standard hosted runner; WASM compilation and later browser interaction are separate evidence. Full upstream CI is not claimed by these scoped checks.
