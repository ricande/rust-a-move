# Design package

The design of *Rust a move*, written before implementation started
(2026-10-18, as a package for the working title GRIN) and kept up to date
since. It describes an original game inspired by the Snood principle: shoot
geometric creatures, match three or more, and make loose clusters fall. The
game's identity is its characters: no aiming line is shown – the creature
that will be hit reacts with fear; the Angry one is permanently furious and
gives nothing away; its neighbours are nervous; the Runner tries to escape
when it has a free path, and flees at once if it also touches an Angry one.

## What is decided

The core idea, the characters, fear as the aiming aid, the exploding Angry,
nervous neighbours, the Runner and the configurable board width/size are the
user's decisions (**CONFIRMED** in [01_DECISIONS.md](01_DECISIONS.md)).
Exact timings, scores, grid rules and blast radius are worked-out
**PROPOSED** starting rules: implemented as clearly documented prototype
values and evaluated in play. Decisions taken during implementation are in
the decision log at the end of [01_DECISIONS.md](01_DECISIONS.md).

## Documents

| Document | Purpose |
| --- | --- |
| [../GAME_RULES.md](../GAME_RULES.md) | The authoritative rulebook (R01–R14), with an unambiguous order and the special cases. |
| [01_DECISIONS.md](01_DECISIONS.md) | Decision status, scope, open questions and the decision log. |
| [03_TECHNICAL_DESIGN.md](03_TECHNICAL_DESIGN.md) | Grid, shots, data, time step and integration with the template. |
| [04_CHARACTERS_ANIMATION.md](04_CHARACTERS_ANIMATION.md) | Characters, animations, sound direction and the animation lab. |
| [05_CONFIGURATION_CONTENT.md](05_CONFIGURATION_CONTENT.md) | Width, zoom, settings, level format and balancing defaults. |
| [06_IMPLEMENTATION_PLAN.md](06_IMPLEMENTATION_PLAN.md) | Milestones M0–M7 with deliverables and gates. |
| [07_TEST_ACCEPTANCE.md](07_TEST_ACCEPTANCE.md) | Rule tests, visual checks and platform acceptance. |
| [08_TEMPLATE_AUDIT.md](08_TEMPLATE_AUDIT.md) | The audited RustyHusky version, its limits and sources. |

The rulebook was originally `02_GAME_RULES.md` in this folder; it lives in
`docs/GAME_RULES.md` so the template's documentation layout keeps working.
The package's agent rules are the appendix of [../../AGENTS.md](../../AGENTS.md).
What has actually been built and verified is recorded in
[../VERIFICATION.md](../VERIFICATION.md).

## Key technical choices

- Macroquad and the template's existing `Game` interface stay the
  foundation.
- Logic runs without a window, at the template's 120 updates per simulated
  second.
- The same shot calculation is used for fear and for the actual hit.
- Creatures are drawn procedurally: body, face and motion. No sprite sheets
  are required.
- Six neighbours in a staggered grid; circular colliders even for squares
  and diamonds.
- One game transaction decides explosions and falls; animations show the
  result without changing it.
- Recording, pause, languages and sound are reused from the template; the
  win/loss outcome needs a small generic extension (planned for M3).

No Snood images, sounds, character names or levels are copied; all
expressions and content are original.
