# Verification record

What has been verified for each milestone, how, and what has not. Automated
tests and real graphical runs are listed separately; anything not checked is
marked **NOT RUN**. No item was pre-filled from the plan
([game/07_TEST_ACCEPTANCE.md](game/07_TEST_ACCEPTANCE.md)).

**Host for all runs below:** Linux x86_64 desktop (`hpelitedesk800g6`),
Rust 1.99.0 (plus the 1.87 toolchain), FFmpeg 8.0.1 with libx264, a Wayland
session with `DISPLAY=:0`. Whether the window ran natively on Wayland or
through XWayland was not determined.

## M0 – Project setup

Commit `7560515` (naming), template `91c257d8e60ca381768dde0258e04d745a30d184`.

| Check | Result |
| --- | --- |
| Local template checkout clean and at the audited commit | PASS |
| `scripts/new-game.sh` effects inspected (config, Cargo, changelog, rules) | PASS |
| `cargo fmt --check`, `clippy -D warnings`, `REQUIRE_FFMPEG=1 cargo test` (63 tests), vendored miniquad check | PASS (baseline) |

## M1 – Characters and animation lab

Commit `4042cb8`.

| Check | Result |
| --- | --- |
| fmt, clippy, tests (91), vendored check, build with Rust 1.87 | PASS |
| `scripts/lab_tour.txt` on the real display: 38 screenshots reviewed (cast at six sizes, target aim/relief, nervous ring, Angry twins, runner routes and panic, blast/chain/fall with ending hold, widths, reduced motion, slow motion, results) | PASS |
| Saved video: 1080×1920, 60/1 fps, H.264 + AAC of equal length (18.7 s); 1 117 distinct frames of 1 123; no temporary recording files left | PASS |
| Issues found and fixed in review: poor family spread from the hash, status line under the REC marker, triangle highlight outside the body, launcher over the status text | fixed |
| Humour and readability judged by a person, especially the smallest size (radius 10) | **NOT RUN** (pending user review) |
| Timing comfort of tremors in motion, judged by a person | **NOT RUN** |

## M2 – Board and shared shot prediction

Commit `1a4184d`.

| Check | Result |
| --- | --- |
| fmt, clippy, tests (119), vendored check, build with Rust 1.87 | PASS |
| Planner vs. independent fine-step simulation: 1 200+ random shots on five board sizes, 300+ with bounces | PASS |
| Gate: the afraid creature is the one hit, all six sizes × four test boards, no tunnelling, occupancy frozen in flight | PASS |
| `scripts/range_tour.txt` on the real display, normal mode: no trajectory visible; afraid creature visible; bank shot; 18 columns; zoom 75 %; pause; results; saved video 1080×1920 60/1 with AAC of equal length | PASS |
| Same tour with `RUST_A_MOVE_DEV=1`: the drawn path ends at the afraid creature and the shot lands in the shown cell, straight and after a wall bounce; a bank shot onto an Angry is reported and Angry shows no fear | PASS |
| Issues found and fixed: a long test filled the board down to the launcher (now the range resets at the danger line and test boards never reach it); the "next" label overlapped the help text | fixed |
| Hands-on play with a real mouse and keyboard | **NOT RUN** |
| macOS (Apple Silicon) and Windows graphical runs | **NOT RUN** (CI covers build and tests only) |
| Performance at the largest board with recording off | **NOT RUN** |

## Not yet verifiable

Matching, falls, scoring, pressure, win/loss and retry (M3); blasts (M4);
runner escapes in the game (M5); levels and saved options (M6); full
recording-mode and platform review (M7).
