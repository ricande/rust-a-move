# Decisions, scope and authority

Version 1.0, 2026-10-18. Project name **Rust a move** (the design package originally used the working title GRIN). This is a specification; the implementation status is in the README and `docs/VERIFICATION.md`.

## Status vocabulary

- **CONFIRMED:** explicitly requested by the user or accepted in the visible conversation.
- **PROPOSED:** a concrete implementation default chosen for this plan. Build it for the prototype, record changes, and do not describe it as an earlier user decision.
- **DEFERRED:** excluded from the first release unless requested later.
- **OPEN:** needs a product choice at the relevant milestone; does not block unrelated work.

User instructions override this package. For mechanics, `docs/GAME_RULES.md` is authoritative; geometry/API implementation belongs in `03_TECHNICAL_DESIGN.md`; numerical defaults belong in `05_CONFIGURATION_CONTENT.md`. Update all affected references when changing a rule.

## Confirmed requirements

| ID | Requirement |
| --- | --- |
| C01 | Use RustyHusky as the starter; implement with Claude using this package. |
| C02 | A Snood-like aimed matching game with original grumpy geometric creatures. Matching at least three removes a group; unsupported creatures fall. |
| C03 | No visible trajectory line. The predicted first creature hit expresses fear instead. |
| C04 | The angry creature is constantly furious; aiming at it does not increase its usual shaking. |
| C05 | Hitting an angry creature causes an explosion removing nearby creatures. |
| C06 | Every creature adjacent to an angry creature is visibly nervous; an ordinary aimed-at neighbour must still show extra fear. |
| C07 | An escape creature struggles to leave when a route to the left or right opens. |
| C08 | An escape creature touching an angry one escapes as soon as it can. |
| C09 | Number of creatures across the board, and their resulting scale/size, are configurable. |
| C10 | Retain the portrait-game, menus, recording and audio foundation rather than rebuilding the framework. |

## Proposed implementation choices

| ID | Decision and reason |
| --- | --- |
| P01 | ~~Working title GRIN.~~ **Decided 2026-10-18 by the user:** the project is named *Rust a move* (package id `rust-a-move`, environment prefix `RUST_A_MOVE`), in the public repository `ricande/rust-a-move`. |
| P02 | Alternating N / N−1 rows, six-neighbour topology. Different visible shapes share a circular collider. |
| P03 | Six ordinary match families, each identified by both colour and silhouette. |
| P04 | Angry is a colourless special. Runner is a trait on an ordinary family. A creature cannot be both. |
| P05 | Explosions cover graph radius 1 by default; angry neighbours chain. |
| P06 | Runners need a straight horizontal corridor wide enough for their collider. No searching around corners. |
| P07 | Normal runners struggle for 0.75 simulation seconds. Angry contact reduces remaining struggle to zero while the route is open. |
| P08 | Runners change board occupancy only while aiming, never during projectile flight or resolution. A visible pending panic resolves on the next eligible tick. |
| P09 | Explicit fire wins over runner departure on the same aiming tick; the launched projectile sees that board. |
| P10 | The ceiling descends one row pitch after five shots, including successful shots. It adds no new creatures. |
| P11 | A single finite-board classic mode first; victory is an empty board, defeat is the danger line or blocked attachment. |
| P12 | No ammo swapping, extra powers, score multipliers or punishment for waiting in v1. Autonomous runners can help the player. |
| P13 | Game-specific setup is displayed inside the game module before the active board starts, using an existing round. Recording may include this setup; no recorder refactor just to hide it. |
| P14 | Baseline geometry settings apply between rounds only. No board repacking during play. |
| P15 | Small generic framework extensions only for outcome/retry and logical drawing primitives. Keep a patch ledger. |

## Release scope

**First playable:** procedural creatures and animation lab; ordinary shots/matches/falls; fear aiming; configurable board; angry chain explosions; runners; win/loss and replay; template recording with SFX. A small curated set of levels demonstrates these mechanics without relying on a lucky generator.

**First release:** readable onboarding, persisted game configuration, EN/SV strings, reduced motion, valid generated boards or curated progression, repeatable fixtures, Linux and Apple Silicon smoke checks, inherited CI retained.

**Deferred:** daily challenges, online scoreboards, network play, background music system, level editor UI, extra special creatures, mobile release, achievements, arbitrary polygon collision physics, fully portable bit-identical floating-point replays. JSON authoring and seeded tests come before an editor.

## Open choices and their timing

| Choice | Latest useful time | Safe interim choice |
| --- | --- | --- |
| ~~Final name / new repo / visibility~~ | Decided 2026-10-18 | *Rust a move*, public repository `ricande/rust-a-move`. |
| Humour and visual palette | Animation lab review | Simple colourful shapes, expressive faces, no copied character artwork. |
| Explosion size and runner delay | First special-mechanics playtest | Defaults in configuration document. |
| Pressure: every shot or only misses | Classic-mode playtest | Every five shots; clearly shown counter. |
| Normal runner waits or moves immediately | Runner playtest | 0.75 s normal; immediate when touching angry. |
| Separate width and zoom UX | Setup screen review | Column count sets relative creature size; zoom scales whole board within safe area. |

## Change discipline

Record decisions in a dated log: requirement, old behaviour, new behaviour, reason, affected tests. A rule change is not a collision bug fix. Do not silently make the angry creature reveal aim, remove autonomous escape, add a trajectory line, or replace procedural expressions with static faces to simplify implementation.

## Decision log

| Date | Item | Before | Now | Reason | Tests |
| --- | --- | --- | --- | --- | --- |
| 2026-10-18 | Snap rule after a creature hit (M2) | "Short snap segment may touch the target but not pass its interior" (open wording) | Candidate cells are at most 60° round the struck creature from the contact direction (one cell step), with the snap path ≥ 2r from every other creature. | Any occupied neighbour within 60° would already overlap the projectile, so the rule always finds the natural cell and never snaps round a creature; no attachment on the far side. Implementation detail, not a rule change. | `shot::tests::attachments_are_local_and_empty`, `blocked_when_no_local_cell_exists`, `simultaneous_contacts_go_to_the_smaller_id` |
| 2026-10-18 | Simultaneous contacts (M2) | "Creature over wall/ceiling, then stable id; corner order documented" | Creature, then ceiling, then side wall (a wall/ceiling corner stops at the ceiling); creatures by smaller `CreatureId`. Tolerance 1e−7 · diameter. | Fixes the policy the design left to implementation. | `shot::tests::simultaneous_contacts_go_to_the_smaller_id`, `tangent_touch_counts_as_contact` |
| 2026-10-18 | Pre-game setup screen (P13) | Proposed | A round opens a game-local setup screen: Shooting range (M2) or Animation lab (M1). Clicks and keys there never fire. Width/zoom move here as real options in M6. | Interim until the ordinary game (M3). | `rustamove::tests::setup_chooses_by_keys_and_never_fires` |
| 2026-10-18 | M2 shot sounds | WAV SFX proposed | The range uses the framework's synthesized Blip (launch) and Click (bounce, attach) until original WAV SFX arrive in M3/M4. | Keeps M2 to geometry. | – |
| 2026-10-18 | Name and repository | Working title GRIN, local folder only | *Rust a move*, id `rust-a-move`, prefix `RUST_A_MOVE`, public GitHub repository `ricande/rust-a-move`. | User decision. | `config::tests::names_are_consistent` |
| 2026-10-20 | Setup before every round (M3) | P13 proposed | Every round, including Next round and Retry, opens the setup screen with the saved options and focus on Play: one press continues. Retry uses the same level, options and seed, so the same board. | `Game::new(level)` cannot tell a retry from a fresh start; persisted options keep them identical without a hidden global. | `rustamove::tests::setup_options_change_the_board_and_are_saved_on_start` |
| 2026-10-20 | Game options file early (M3) | Planned for M6 | `game-settings.json` (columns, zoom, reduced motion) is read at the first setup input and written only when a round starts; broken or newer files give defaults and a warning and are never rewritten by reading. | Retry needs the same options (above). Level progression, tutorial and quality settings stay M6. | `persistence::tests::*` |
| 2026-10-20 | Direct Angry hit before M4 | – | Uses up the shot; no blast, no attachment, no loss. M3 levels contain no Angry. | Keeps M3 to ordinary play without inventing a provisional rule. | `shot::tests::angry_first_hides_what_is_behind` |
| 2026-10-20 | Original sound effects (M3) | Template sounds in M2 | Six original WAV effects (launch, bounce, attach, pop, fall, ceiling) synthesized by `scripts/make_sfx.py`; at most one of each per resolution. | Spec: original SFX through `Game::SOUNDS`. | `sfx::tests::every_sound_decodes`, `round::tests::a_resolution_makes_one_sound_of_each_kind` |
| 2026-10-20 | Levels (M3) | Proposed progression | Three curated 8-column ordinary levels (first-grin, off-the-wall, full-house); every other level and width is generated (seeded, connected, ordinary, 3–6 families, at least 45 % free height). Level 1 is shown winnable by a test; no other solvability is claimed. | Curated fixtures before generation; specials arrive with M4/M5. | `content::tests::*`, `round::tests::level_one_can_be_won` |
| 2026-10-20 | Blast presentation (M4) | Smile ~0.06 s, effects ~0.35 s | The chain is decided at once; it is shown in waves 0.1 s apart (`blast_wave_ticks` 12). Each exploding Angry smiles 7 ticks before its own burst; its victims freeze in fear until the burst of the first blast that covers them; falls start halfway through the last burst. Aiming is locked until the last burst has played. One blast sound plus at most three accents for later waves, at their bursts. The score is added when the shot resolves. | Readable chains without letting the player shoot during a smile. | `round::tests::blasts_lock_input_until_shown_and_sound_per_wave` |
| 2026-10-20 | Angry levels (M4) | Progression proposed | Curated 8-column levels: 3 angry-neighbours (one Angry, reachable through a gap), 4 chain-reaction (three Angry), full-house moves to 5 and gets one Angry. Generated levels from 3 on: Angry with 4 % per cell, hard cap 5 % of the cells (at least one). | Spec progression 3–4; specials stay rare. | `content::tests::*` |
| 2026-10-20 | Reduced flashes on the setup screen (M4) | Planned for M6 | Saved with the other options; blasts then show a ring without the white flash. | R14 accessibility, needed as soon as blasts exist. | `persistence::tests::*` |
| 2026-10-20 | Runner timing (M5) | 90 / 0 struggle ticks | A runner leaves on open aiming tick max(1, N): normal N = 90 (there on tick 89, gone on 90), panic N = 0 (its first open tick). The tick that first sees the route open counts. The held route is kept while open; a blocked route resets the count and the other open side (if any) starts at 1. | Pins down E03/E04/E06 exactly. | `round::tests::an_open_runner_struggles_then_leaves`, `a_runner_next_to_angry_leaves_at_once`, `a_closed_route_restarts_and_the_last_escape_wins` |
| 2026-10-20 | Escape presentation (M5) | 0.25 s, input locked | The runner is removed at the start of its 0.25 s squash-and-dash; what it held falls a third of the way in; aiming waits for the dash and, if something fell, for the fall. Escape sound once. No shot is counted and the ceiling counter does not move. | R10. | `round::tests::*runner*`, `runners_leave_one_at_a_time_and_fallen_ones_do_not_escape` |
| 2026-10-20 | Runner levels (M5) | Progression proposed | Curated 8-column levels: 5 runner-intro (boxed in until a match frees it), 6 runner-panic (next to an Angry, boxed in until a match opens the way, then it bolts), full-house moves to 7 and gets a runner. Generated levels get runners from level 5 (6 % per ordinary cell, cap 8 %). | Spec progression 5–7; the first taught runner board never has an unavoidable immediate escape. | `content::tests::*`, `round::tests::runner_levels_start_blocked` |
| 2026-10-20 | Setup preview (M6) | "Show a preview" | The next round's own board (same level, width and zoom, so the same seed) is drawn dimmed behind the setup menu and rebuilt when width or zoom change. | Shows exactly what the settings do; no separate preview geometry. | `rustamove::tests::the_preview_is_the_next_board` |
| 2026-10-20 | Effects quality (M6) | Visual quality Low/Normal | "Effects: normal / fewer": fragments per removed creature 7 / 3. Nothing else changes; fear, runner cues, silhouettes and the pressure counter are never reduced. Saved as `visual_quality` (`normal`/`low`). | R14. | `persistence::tests::*`, `rustamove::tests::effects_quality_is_saved_and_used` |
| 2026-10-20 | Onboarding (M6) | Short tutorial progression | The seven curated levels are the progression; levels 1–6 show a one-line hint (observable actions, no engine terms) until the first shot. A How to play page on the setup screen covers fear aiming, matching, Angry, runners, the ceiling and how a level ends. | Brief guides, clean recordings. | `round::tests::taught_levels_have_hints`, `rustamove::tests::help_opens_and_closes_without_starting` |
| 2026-10-20 | Generator fallback (M6) | "Known valid fallbacks" | A generated board that fails the start checks (enough creatures, all anchored, clear of danger) is replaced by a fixed three-row board; tests show the generator itself passes for every offered width and zoom. | Never a broken start. | `content::tests::the_fallback_board_is_valid_everywhere`, `every_setting_gets_a_valid_board` |
| 2026-10-20 | Effects: maximum | Normal / fewer | A third level, "maximum": 14 fragments per removed creature (normal 7, fewer 3) and a thin, faster shock-wave ring in blasts. Left/Right step fewer ↔ normal ↔ maximum, Space goes round; saved as `"maximum"`. Still cosmetic only. | User request. | `render::tests::effects_quality_steps_and_scales_fragments`, `rustamove::tests::effects_step_and_cycle` |
| 2026-10-20 | Level editor | DEFERRED ("level editor UI") | **User decision:** an in-game level editor on the setup screen: paint families, Angry and runners on a board of the chosen width; live validation (loose creatures ringed, danger line); test play with the real rules that returns to the editor; nine save slots in `levels/custom-N.json` in the level file format; ceiling interval and blast radius per level within the format's limits. | Requested by the user. | `editor::tests::*`, `content::tests::levels_round_trip_through_json` |

