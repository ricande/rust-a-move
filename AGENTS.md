# AGENTS.md

Guidance for AI coding agents (and humans) working on Rust a move, a game built on
the RustyHusky template. The template's rules come first; the game-specific
rules are in the appendix at the end. Read this
and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) before changing code; the
current game's rules are in [docs/GAME_RULES.md](docs/GAME_RULES.md).

## Project in one paragraph

Rust a move is a portrait aim-and-match game with grumpy geometric
creatures: no aiming line, the creature about to be hit shows fear instead.
It is built on RustyHusky, a Rust starter framework for portrait games with
automatic gameplay recording (for YouTube Shorts, TikTok, Instagram Reels),
built with Macroquad: a logical 540 × 960 view scaled to the window, main
menu, settings (language, volume, recording mode), pause and results
screens, English/Swedish translations and automatic MP4 recording of each
round via FFmpeg. The game is the module `src/rustamove/` behind the
`game::Game` trait. It is a single binary crate. Development details are in
[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md); what has been verified is in
[docs/VERIFICATION.md](docs/VERIFICATION.md).

## Commands

Run all of these before you consider a change done:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

GitHub Actions (`.github/workflows/ci.yml`) runs the same checks on every
push and pull request on Linux, and clippy and the tests on Windows and
macOS, all with FFmpeg installed and `REQUIRE_FFMPEG=1`, so a test that needs
FFmpeg fails instead of skipping. It also checks the vendored miniquad and
builds with the minimum Rust version, 1.87 (`rust-version` in `Cargo.toml`).

Run with `cargo run --release`. If `cargo` is not on `PATH`, it is in
`~/.cargo/bin`.

## Rules

1. **Keep the framework small.** No plugin systems, generic engines or new
   dependencies without a clear need. Add abstractions only when a second
   real use exists.
2. **Keep the game replaceable.** Game-specific code lives in its own module
   behind `game::Game`. Menus (`app.rs`, `screens.rs`) and recording
   (`capture.rs`, `video.rs`) must not know about a particular game.
3. **Keep logic window-free.** `app.rs`, `game.rs`, game logic, `settings.rs`,
   `config.rs`, `i18n.rs`, `ui.rs`, `video.rs`, `pacing.rs` and `script.rs`
   must not call Macroquad's
   window, graphics, input or audio APIs, so they are tested headlessly. Only
   `main.rs`, `view.rs` (drawing helpers), `screens.rs`, `capture.rs`, the
   playback part of `audio.rs` and a game's `draw` talk to Macroquad.
4. **Use logical coordinates.** Everything is in the 540 × 960 field. Never
   use window pixels outside `view::View`. Define button rectangles once, in
   a function that both input handling and drawing use.
5. **Use the fixed time step.** Game logic advances only in `Game::update`
   with `FIXED_DT`; do not read the clock in game code, or the video will not
   match what was played.
6. **Every player-visible text goes through `t!`.** Framework texts belong
   in `CATALOG` in `src/i18n.rs`, the game's texts in its `Game::TEXTS`; both
   with English and Swedish, using the same named
   placeholders. Code, comments, docs and commit messages are in English.
7. **Names live in `config.rs`.** Never hard-code the project name, folder
   names, file name prefixes or environment variable names; use
   `config::APP_NAME`, `APP_ID` and `config::env("NAME")`.
8. **Recording must never break the game.** FFmpeg may be missing, fail or
   hang; every such case ends in a `VideoStatus` message, never a panic or a
   frozen screen. Never wait for ffmpeg without a time limit: use
   `video::Process` and the timeouts in `video.rs`, and bounded queues only.
   Slow work stays on worker threads. Temporary files are always removed
   (Discard, leaving the results, closing the program).
9. **Test what you add.** Flow changes get tests in `app.rs`, using the
   example game. Tests that use files use `crate::tests::temp_dir`, never the
   user's folders. Tests run in parallel threads; the language is
   thread-local, so change it only on a thread you spawn yourself. Do not set
   environment variables in tests.
10. **Update the docs with the code.** Structure → `docs/ARCHITECTURE.md`;
   game behaviour → `docs/GAME_RULES.md`; user-facing behaviour →
   `README.md`; notable changes → `CHANGELOG.md` under *Unreleased*.

## Verifying changes

Drawing code has no unit tests. When a change affects what is shown or the
recording, run the real program in **script mode** (repeatable, does not use
the real mouse and keyboard), with temp folders so the user's settings and
videos are untouched:

```bash
RUST_A_MOVE_SCRIPT=scripts/range_tour.txt RUST_A_MOVE_SHOTS=/tmp/ram/shots \
RUST_A_MOVE_DATA_DIR=/tmp/ram/data RUST_A_MOVE_VIDEO_DIR=/tmp/ram/videos \
cargo run --release
```

- `scripts/range_tour.txt` and `scripts/lab_tour.txt` tour the game's
  shooting range and animation lab (see docs/DEVELOPMENT.md).
- `scripts/ui_tour.txt` (the template's) goes through menu, settings, a round with pause,
  results with Save video, the next round, abort and Discard video, and saves
  numbered screenshots. The command language is at the top of
  `src/script.rs`.
- Scripts can also hold keys (`hold left` … `release left`), for games
  with movement.
- `scripts/window_check.txt` with `RUST_A_MOVE_WINDOW=1000x640` checks
  letterboxing and the mouse conversion (its clicks are window pixels).
  Run it with `RUST_A_MOVE_LANG=sv` to check Swedish too.
- Check a saved video with
  `ffprobe -v error -show_entries stream=width,height,r_frame_rate,nb_frames -of csv=p=0 <file>`;
  expect `1080,1920,60/1,…` and an AAC stream.
- Check a hanging encoder with a fake ffmpeg that answers `-encoders` with
  `libx264` and otherwise runs `exec sleep 600`, set via
  `RUST_A_MOVE_FFMPEG`: the round must go on at full speed and the results
  must show the error. Check a slow encoder the same way with a fake that
  reads frames at, say, 30 per second.
- Check both recording modes: put `{"recording_mode": "recording"}` (or
  `"gameplay"`) in `settings.json` in `RUST_A_MOVE_DATA_DIR`. To see
  Prioritize recording wait for a slow encoder, point `RUST_A_MOVE_FFMPEG` at
  a wrapper that passes stdin on to the real ffmpeg at, say, 20 frames per
  second. A saved video must be 60/1 fps with every frame unique
  (`ffmpeg -i f.mp4 -map 0:v -f framemd5 -` and count distinct hashes) and
  as long as its sound track.
- Check missing FFmpeg with `RUST_A_MOVE_FFMPEG=/nonexistent`, and that no
  `rust-a-move-rec-*` files are left in the temp folder afterwards.

Say clearly what you verified by running the program and what only by tests.

## Working with the maintainer

- The maintainer usually writes in Swedish; reply in the language they use.
- Ask before anything hard to undo or outward-facing: pushing, publishing,
  deleting files, or changing things outside the repository.
- Do not commit build output (`target/`), screenshots or videos.
- A new framework file (anything a game should get when it updates) must be
  added to the lists in `scripts/update-framework.sh`; keep game-specific
  code out of the framework files, and in `main.rs`/`config.rs` only on the
  lines the script carries over.
- `vendor/miniquad-0.4.11` is miniquad as published plus the patches in
  `vendor/patches`. Never edit it directly; change it only by adding or
  removing a patch, and keep `scripts/check-vendored-miniquad.sh` passing.
  Its build warnings are upstream's. See `docs/UPSTREAM_FIXES.md`.
- The project is GPL-3.0-or-later. Keep `LICENSES/MIT-RustOut.txt` and
  `THIRD_PARTY_NOTICES.md`; when adding a dependency, check that its license
  is compatible with GPLv3 and update the notices if needed.

## Appendix: game-specific rules for agents

These rules add to the template rules above; they do not replace or weaken them. Never copy them into the RustyHusky template itself.

1. Read the game specification and decision statuses before changes. Confirmed behaviour takes priority over implementation convenience. Numerical defaults are proposed and centrally configurable.
2. Game logic, renderer, options, levels and emotion code live in the game module. Framework flow stays generic; Angry and Runner must never appear in recorder/menu framework code.
3. The authoritative mechanical specification is docs/GAME_RULES.md after installation; detailed project docs live in docs/game/. Update links when relocating package files. Preserve the original template's architecture/new-game docs as framework documentation.
4. Simulation advances only in fixed update. Input handling captures intent; drawing is pure. Use separate owned gameplay/cosmetic randomness. No system clock, draw-time mutation or global random state in game behaviour.
5. Prediction and flight share first-contact calculation. Cosmetics do not change colliders. Do not add a visible trajectory line to normal gameplay to mask a targeting defect.
6. Resolve chained explosions on one snapshot before support removal; score each id once. Test runner/Angry/shot order, not just each mechanic alone.
7. Maintain FRAMEWORK_PATCHES.md for outcome/retry and View changes. Do not run template update scripts blindly over game-specific integration patches or this appendix.
8. Preserve upstream licenses, retained MIT attribution and vendored patch process. All visuals/sounds/levels must be original or have recorded compatible licenses. Do not copy Snood assets or character roster.
9. Required fmt/clippy/tests and vendored checks must pass for implementation milestones; real drawing/video changes also need actual graphical evidence. Mark unavailable checks NOT RUN.
10. Keep documentation accurate: rules, architecture, README, changelog, decision log and patch ledger. A game rule change must not be hidden inside a bug fix.
11. Use isolated test data/video directories. Do not alter Richard's normal settings to run automation.
12. Remain in authorized local work. Do not publish/push/create remote repositories or perform irreversible operations without applicable user authorization. Do not ask repeatedly for actions already authorized.

### State at delivery of the design package

Documentation complete; implementation not begun here. RustyHusky audited read-only at 0.3.0 / 91c257d8e60ca381768dde0258e04d745a30d184. No gameplay, animation, performance or platform test is preapproved as passing. First implementation assignment was M0–M1 (see docs/game/06_IMPLEMENTATION_PLAN.md).
