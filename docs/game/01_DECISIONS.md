# Decisions, scope and authority

Version 1.0, 2026-10-18. Working title **GRIN**. This is a specification, not a claim that the software exists.

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
| P01 | Working title GRIN. Final name, package id and repository visibility remain OPEN. |
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
| Final name / new repo / visibility | Before publishing or naming real user data | Local working folder `grin-prototype`; no public creation. |
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
| 2026-10-18 | Pre-game setup screen (P13) | Proposed | A round opens a game-local setup screen: Shooting range (M2) or Animation lab (M1). Clicks and keys there never fire. Width/zoom move here as real options in M6. | Interim until the ordinary game (M3). | `grin::tests::setup_chooses_by_keys_and_never_fires` |
| 2026-10-18 | M2 shot sounds | WAV SFX proposed | The range uses the framework's synthesized Blip (launch) and Click (bounce, attach) until original WAV SFX arrive in M3/M4. | Keeps M2 to geometry. | – |

