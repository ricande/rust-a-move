# RustyHusky audit and source references

Read-only source audit on 2026-10-18. Repository: https://github.com/ricande/RustyHusky

Verified commit: `91c257d8e60ca381768dde0258e04d745a30d184`, commit message `Release 0.3.0`, commit timestamp 2026-10-18T08:10:19Z. Cargo package version 0.3.0; edition 2021; minimum Rust 1.87. This was verified through GitHub source reads, not a local build or a game execution. Recheck actual latest source before implementation. No changes were made to the remote repository.

## Existing capabilities and exact seams

| Finding | Source at audited commit | Consequence |
| --- | --- | --- |
| Game trait has new(level), handle, update, draw, take_sounds, finished; TEXTS/SOUNDS | `src/game.rs` | Implement a new game module behind this interface. |
| FIXED_DT is 1/120 s | `src/game.rs` | Keep all animation/game timers in simulation time. |
| Actual App::advance hands gathered input to handle, then loops updates | `src/app.rs` | handle is not a reliable per-update clock; consume one-shot intent in update. |
| Trait handle comment still says displayed-frame call | `src/game.rs` vs `src/app.rs` | Follow actual implementation, note comment mismatch. |
| RoundResult contains score only; completion always plays Success | `src/game.rs`, `src/app.rs` | Add generic Won/Lost and Retry; do not pretend the feature exists. |
| View uses 540×960 and supports rect/circle/line/text/image | `src/view.rs` | Add minimal logical polygon/ellipse helpers for faces. |
| English/Swedish game text catalog and embedded PCM WAV sounds | `src/example.rs`, `docs/NEW_GAME.md` | Reuse rather than add custom text/audio pipelines. |
| Record/Prioritize gameplay/Off modes, 1080×1920 60 fps sound recording | `docs/ARCHITECTURE.md`, README | Retain capture/pacing architecture. |
| MAX_VIDEO_SECONDS = Some(180.0) | `src/config.rs` | Gameplay may outlast recording; don't terminate game at cutoff. |
| Settings file + data_dir helper, atomic save pattern | `src/settings.rs` | Separate game-owned settings file, shared data location. |
| Local vendored miniquad 0.4.11 patch | Cargo.toml, `docs/UPSTREAM_FIXES.md` in tree | Preserve patch and integrity checks. |
| GPL-3.0-or-later plus retained RustOut MIT notice | Cargo.toml, AGENTS, LICENSES | Preserve licensing and attribution. |
| No background music loop or high-score system yet | `docs/NEW_GAME.md` | Both deferred, not assumed framework facilities. |

## Scripts need attention

`scripts/new-game.sh` changes project name/id/env prefix, package/version, CHANGELOG and docs/GAME_RULES, and invokes cargo build. Run it before copying the finished game rulebook or it will overwrite that file. Verify correct target working tree; it is intended for a fresh template copy.

`scripts/update-framework.sh` copies numerous framework files and AGENTS from the template. It will overwrite our proposed changes to game.rs/app.rs/screens.rs/view.rs. It only restores selected mod/ActiveGame lines and four config constants, not arbitrary integrations. Maintain a manual patch ledger and merge carefully.

The audited tree contains `scripts/ui_tour.txt` and `scripts/window_check.txt`, matching the guide references. These are placeholder-game tours, not ready-made tests for this game. Replace their game actions in game-owned tour scripts; read the actual script command grammar before generating scripts. Check current filenames with `rg --files scripts` after any template update.

## Source links

All following links pin the audited commit rather than mutable main:

- [AGENTS.md](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/AGENTS.md)
- [Architecture](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/docs/ARCHITECTURE.md)
- [New game guide](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/docs/NEW_GAME.md)
- [Game interface](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/src/game.rs)
- [App and result flow](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/src/app.rs)
- [View](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/src/view.rs)
- [Settings](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/src/settings.rs)
- [Input](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/src/ui.rs)
- [Example game](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/src/example.rs)
- [Naming script](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/scripts/new-game.sh)
- [Framework update script](https://github.com/ricande/RustyHusky/blob/91c257d8e60ca381768dde0258e04d745a30d184/scripts/update-framework.sh)

The plan does not assert native Wayland runtime support or any measured GPU performance; these require checking the actual backend and host at implementation time.
