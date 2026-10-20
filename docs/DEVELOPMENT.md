# Development

How to build, check and work on *Rust a move*. The rules for changes are in
[AGENTS.md](../AGENTS.md); the structure is in
[ARCHITECTURE.md](ARCHITECTURE.md); the game's behaviour is in
[GAME_RULES.md](GAME_RULES.md).

## Prerequisites

- Rust stable, 1.87 or later (`rust-version` in `Cargo.toml`), via
  [rustup](https://rustup.rs). CI also builds with exactly 1.87.
- Linux: `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`.
- FFmpeg with libx264 for recording and for the video tests
  (`sudo apt install ffmpeg`, `brew install ffmpeg`).

## Required checks

Run all of these before a change is done; CI runs the same on Linux,
Windows and macOS with FFmpeg installed:

```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
REQUIRE_FFMPEG=1 cargo test --locked
./scripts/check-vendored-miniquad.sh
cargo +1.87 build --locked
```

`REQUIRE_FFMPEG=1` makes the video tests fail instead of skipping when
FFmpeg is missing. The two compiler warnings from `vendor/miniquad-0.4.11`
are upstream's and expected ([UPSTREAM_FIXES.md](UPSTREAM_FIXES.md)).

## Tests

All tests run without a window, GPU or sound card. The game's tests live
next to the code in `src/rustamove/`:

| Module | What the tests pin down |
| --- | --- |
| `geometry` | Layouts for 6–18 columns and 75–100 % zoom fit the board; neighbours are reciprocal and touching; nearest-cell tie-breaks; runner corridors check staggered rows; nearest exit, left on a tie. |
| `board` | Row-major storage, ids, revisions, no overwriting, ceiling anchoring, ceiling drop by one pitch, danger line. |
| `round` | R01–R13, X01, X04, X06–X08, E01–E12, E14: boxed-in runners stay (frantic next to an Angry); open ones struggle 90 ticks and leave for 10 points without counting as a shot; panic leaves at once; a closed route restarts on the other side; a shot beats a leaving runner and none leaves in flight; routes opened by a shot wait for aiming; runners leave one at a time and fallen ones do not escape; runners match and get blasted; panic is not remembered; the last escape wins; runner levels start blocked; a boxed-in Angry still explodes; chains resolve before falls and score once; a blast through the anchors drops the rest; Angry is inert unless hit; the rules set the radius; blasts lock input until shown, with one blast sound and bounded accents; three match and two do not; groups across rows; unrelated triples stay; cut branches fall once for fall points; walls hold nothing; a clearing shot wins before the ceiling drops; every fifth shot drops the ceiling; the danger line and blocked shots lose; ammunition follows the board; one sound per kind; presses outside aiming are dropped; rounds are deterministic; random play always ends and the result is stable after the ending hold; level 1 can be won. |
| `blast` | Areas count graph steps through empty cells and never wrap; radius 1, 2 and 3; chains in waves on the snapshot; overlapping and cyclic blasts take each creature and each Angry once; a checkerboard chain climbs one column. |
| `content` | Curated levels parse and fit at every zoom; bad files are rejected with a reason; width, support and danger are checked; generated boards are valid, connected, reproducible and grow with the level. |
| `persistence` | Options round-trip; broken, out-of-range and newer files give defaults and a warning and are never rewritten by reading. |
| `aim` | Pointer clamping, keyboard/mouse ownership, range limits, presses taken once. |
| `sfx` | Every sound decodes. |
| `shot` | The planner against an independent fine-step simulation (1 200+ random shots, 300+ with bounces); mirrored boards give mirrored shots; tangent contact; simultaneous contacts; ceiling gaps; blocked boards; Angry hides what is behind it; flights stop exactly once at any speed. |
| `range` | The afraid creature is the one hit, at every board size and test board; no tunnelling; occupancy frozen in flight; presses during flight are dropped; one press is one shot; replanning on fire. |
| `emotion` | Blend timings; Angry ignores the aim completely; an aimed nervous neighbour shows more fear; reduced motion keeps the face; poses are pure; blink intervals. |
| `render` | Shapes stay inside the collider, are star-shaped for the fan fill, and differ per family. |
| `lab` | Every scenario runs; nervous ring; Angry twins move identically under aim; runner timing (90 ticks, panic on the first); chain blast removes each creature once. |
| `mod` (game) | Presses apply once in `update`; the setup screen never fires, changes width and zoom, saves on start, and the next round reads the same options and board; the preview is the next board; How to play opens and closes without starting; effects quality is saved and used. |

The framework's own tests (flow including win/loss/retry, settings, translations, recording) use
their own test game in `src/app.rs`. Tests never touch the user's folders:
use `crate::tests::temp_dir` for files, and never set environment variables
in tests.

## Running

```bash
cargo run --release
```

### Development mode

`RUST_A_MOVE_DEV=1` enables overlays in the shooting range: **S** shows the
colliders, **W** the predicted shot path and attachment cell. Use it to
check that the path ends at the afraid creature. Normal play must never show
a trajectory (rule R03).

### Scripted runs

Script mode replays a text script instead of the real mouse and keyboard, at
a fixed 1/60 s per frame, and saves screenshots. Always use temporary
folders so your own settings and videos are untouched:

```bash
RUST_A_MOVE_SCRIPT=scripts/range_tour.txt RUST_A_MOVE_SHOTS=/tmp/ram/shots \
RUST_A_MOVE_DATA_DIR=/tmp/ram/data RUST_A_MOVE_VIDEO_DIR=/tmp/ram/videos \
cargo run --release
```

| Script | What it does |
| --- | --- |
| `scripts/setup_tour.txt` | The setup screen: the dimmed preview at 18 and 6 columns and zoom 75 %, Effects, How to play, the level-1 hint; run it also with `RUST_A_MOVE_LANG=sv` and `RUST_A_MOVE_WINDOW=1000x640`. |
| `scripts/runner_tour.txt` | Plays levels 1–6 in a row with pictures every 6 frames in levels 5–6: a runner escaping once a match frees it, and a panicking runner next to an Angry that bolts the moment a match opens its way. Run it in Prioritize gameplay. |
| `scripts/blast_tour.txt` | Plays levels 1–4 in a row: matches, the first Angry blast (level 3) and a three-wave chain (level 4) with pictures every two frames around the blasts, then saves the level-4 video. Run it in Prioritize gameplay. |
| `scripts/play_tour.txt` | A real game: wins level 1 and saves the video, loses level 1 at the danger line and saves that video, Retry (same board), pause and abort. Its angles come from `cargo test tour_angles -- --ignored --nocapture`. Run it in Prioritize gameplay (below). |
| `scripts/range_tour.txt` | Shooting range: mouse aim left/right, a shot in flight and landed, a bank shot on the curtain board, keyboard aim, the pocket board, 18 columns, zoom 75 %, pause, results and Save video. Add `RUST_A_MOVE_DEV=1` to see the path overlay in the screenshots. |
| `scripts/lab_tour.txt` | Animation lab: all eight scenarios with their events, pause, slow motion, reduced motion, results and Save video. |
| `scripts/record_tour.txt` | Wins level 1 with generous waits so it plays the same in every recording mode; saves the video. Used with each `recording_mode`, a hanging fake encoder and a missing FFmpeg. |
| `scripts/ui_tour.txt`, `scripts/window_check.txt` | The template's tours of menus, settings, pause, results, Save/Discard, abort and letterboxing (window-pixel clicks), adapted to the game: they open the animation lab from the setup screen, where Enter ends the round. |

The command language (`key`, `hold`, `release`, `click`, `move`, `shot`,
`quit`) is documented at the top of `src/script.rs`. Frames count from 1;
two simulation steps run per frame. Board changes in the range are only
accepted while aiming, so give a shot time to land before switching boards.

Runners move while the player aims, so a tour's waiting times change the
game. `cargo test tour_angles -- --ignored --nocapture` plays every level
with exactly the tick schedule of `runner_tour.txt` (91 ticks before the
first shot, then 2 × (wait + 22) ticks between shots, with the tour's
`wait(angle)`) and prints the angles the tour uses; keep both in step when
changing either.

In the default recording mode (Prioritize recording) the game waits for
the video encoder, so a script can run ahead of the game and a click can
arrive while a shot still flies (and is rightly ignored). For tours that
depend on timing, put `{"recording_mode": "gameplay"}` in the data folder's
`settings.json`; then every script frame is exactly two game steps. Flat
shots bounce many times and can fly for several seconds.

**Frame time.** Script mode does not wait between frames except for the
display's vsync, so `(wall time of a long run − a short run) / extra frames`
is the time per frame. With `vblank_mode=0` (Mesa) it measures the game
itself rather than the monitor; see docs/VERIFICATION.md (M7).

Check a saved video:

```bash
ffprobe -v error -show_entries stream=codec_name,width,height,r_frame_rate,duration -of csv=p=0 <file>
```

Expect `h264,1080,1920,60/1,…` and an AAC stream of the same length.

## Code tour

`src/rustamove/` is the game; everything else is the template's framework.

| File | Role |
| --- | --- |
| `mod.rs` | `RustAMove`, the `Game` implementation: setup screen, mode switching, all game texts (English and Swedish). |
| `round.rs` | The classic round: phases, firing, `resolve` (attach or blast, match, falls, score, ceiling, danger, win/loss), timed presentation and sounds, HUD. |
| `blast.rs` | The Angry blast area and the chain closure, pure and on a snapshot. |
| `rules.rs` | The tunable rule values (match size, ceiling interval, scores, timings). |
| `content.rs` | The level JSON format and its validation, curated levels (`assets/levels/`), the seeded generator. |
| `persistence.rs` | The game's options file `game-settings.json`. |
| `aim.rs` | Aiming input shared by the round and the range. |
| `sfx.rs` | The game's sounds (`assets/sfx/`, made by `scripts/make_sfx.py`). |
| `geometry.rs` | `Layout` (columns, zoom, radius, rows), `Cell`, centres, neighbours, runner corridors. |
| `board.rs` | `Board`: creatures by cell, `CreatureId`, revision counter, ceiling anchoring. |
| `shot.rs` | `plan` (shot prediction with bounces, contact and attachment) and `Flight`. |
| `range.rs` | The M2 shooting range: aim input, fire, flight, attachment, test boards. |
| `emotion.rs` | Families and kinds, `Cues` (what the board says), `Mood` (blended in fixed steps) and `pose` (pure body motion and face). |
| `render.rs` | Procedural drawing of bodies, faces, accents, debris, blasts and the launcher through `View`. |
| `lab.rs` | The M1 animation lab and its eight scenarios. |
| `rng.rs` | The owned, seeded gameplay random generator (xorshift64\*). |

### Conventions that keep the game deterministic

- `handle(&Input)` only records intent (keys, clicks, pointer). Everything
  changes in `update`, one `FIXED_DT` step at a time; the framework may run
  several updates after one `handle`, or none.
- `draw(&self)` only reads. It runs twice per frame while recording (window
  and video), so it must not change or sample anything.
- Gameplay randomness comes from the round's `Rng`; cosmetic variation
  (blinks, tremor phases) from `emotion::hash01` of a creature's id. Never
  `macroquad::rand` or the clock.
- Shot prediction and flight use the same `ShotPlan`. Never draw a
  trajectory in normal play to work around a targeting problem.
- Every player-visible text is a `t!("key")` with English and Swedish in
  `TEXTS`; `cargo test` fails on a missing key.

### Adding things

- **A lab scenario:** add a case to `Lab::setup`, `trigger`, `param_count`,
  `detail` and the title match in `draw_hud`, with texts in `TEXTS`, and
  bump `SCENARIOS`. Add a test that runs it.
- **A range test board:** add a case to `fixture_board` and bump
  `FIXTURES`; `fixtures_hang_from_the_ceiling_and_fit` checks it at every
  size.
- **A level:** add a JSON file to `assets/levels/` (format in
  [game/05_CONFIGURATION_CONTENT.md](game/05_CONFIGURATION_CONTENT.md)) and
  list it in `CURATED` in `content.rs`; the tests check that it parses and
  fits.
- **A sound:** add it to `scripts/make_sfx.py`, run the script, and list the
  file in `sfx.rs`.
- **A rule:** change [GAME_RULES.md](GAME_RULES.md) first, record the
  decision in [game/01_DECISIONS.md](game/01_DECISIONS.md), then the code and
  its tests. A rule change is never hidden inside a bug fix.

## Template updates

Rust a move changes some framework files (`view.rs`, `game.rs`, `app.rs`,
`screens.rs`, `audio.rs`, `i18n.rs`, `pacing.rs`, `AGENTS.md`, `main.rs`,
`config.rs`). `scripts/update-framework.sh` overwrites framework files, so
never run it blindly: follow [FRAMEWORK_PATCHES.md](FRAMEWORK_PATCHES.md).
