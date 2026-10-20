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

## M3 – The ordinary playable game

| Check | Result |
| --- | --- |
| fmt, clippy, tests (151), vendored check, build with Rust 1.87 | PASS |
| Rule tests R01–R07, R11–R13, F01, F04–F05, F07–F10, F14 (see docs/DEVELOPMENT.md) | PASS |
| Framework patch P3: lost level plays the Fail sound (no fanfare) and offers Retry of the same level | PASS (`app` tests) |
| Determinism: same level, options and inputs give the same round, with or without pose evaluation | PASS |
| Random play on five board configurations always ends; result stable and only after the ending hold | PASS |
| `scripts/play_tour.txt` on the real display (Prioritize gameplay): level 1 won (260 points), results "LEVEL 1 COMPLETE" with Next round, video saved; level 1 lost at the danger line, "LEVEL 1 FAILED" in red with Retry, video saved; Retry gave the identical board; pause and abort; options file written with the defaults | PASS |
| End frames of both saved videos show the in-game outcome ("Board cleared!", "Too low: the danger line!") before the results | PASS |
| Saved videos: 1080×1920, 60/1 fps, H.264 + AAC of equal length (25.6 s and 41.2 s); no temporary files left | PASS |
| Found while verifying: in Prioritize recording a script can outrun the game (documented in DEVELOPMENT.md); flat shots fly for seconds (the tour waits by angle); the outcome banner was too transparent (darkened) | fixed / documented |
| Sound effects heard by a person, and their balance | **NOT RUN** |
| Hands-on play with a real mouse; whether a 4 s flat bank shot feels too slow | **NOT RUN** (worth a playtest) |
| Pressure every five shots: fun or frustrating? | **NOT RUN** (OPEN product choice) |

## M4 – Angry and chain reactions

| Check | Result |
| --- | --- |
| fmt, clippy, tests (164), vendored check, build with Rust 1.87 | PASS |
| Blast tests X01–X08, R07–R08, radius 1/2/3, one sound per kind, input locked until the last burst | PASS |
| `scripts/blast_tour.txt` on the real display (Prioritize gameplay): levels 1–4 won in a row with the predicted scores (260, 280, 250, 270); level-4 video saved | PASS |
| Level 3: aiming at the Angry shows no reaction while its hex neighbours look nervous; blast: white flash, ring and fragments, the six around it gone | PASS (screenshots) |
| Level 4: chain of three in waves 0.1 s apart; the third Angry's evil grin (furious brows kept) one frame pair before its burst; victims frozen in fear until their wave; the loose oval falls; score 110 = 9 × 10 + 1 × 20 | PASS (screenshots every two frames) |
| A test assumption that was wrong: in a checkerboard each Angry touches only one other Angry up and one down, so the chain is a column. The test now checks that, and a separate cyclic test checks overlap | test corrected (logic was right) |
| Reduced flashes judged by a person; blast and chain sounds heard | **NOT RUN** |
| Whether chains feel rewarding and the smile is noticeable at normal speed | **NOT RUN** (playtest) |

## Not yet verifiable

Runner escapes in the game (M5); level progression,
tutorial and quality settings (M6); full recording-mode and platform review
(M7).
