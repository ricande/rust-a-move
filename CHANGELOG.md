# Changelog

All notable changes to GRIN are documented here.

## [Unreleased]

### Added

- Started from the RustyHusky template 0.3.0.
- GRIN design package in `docs/game/`; authoritative rules in
  `docs/GAME_RULES.md`; game-specific agent rules appended to `AGENTS.md`.
- Character animation lab (M1), opened by a round: eight scenarios covering
  the cast, aimed fear, Angry with nervous neighbours, aiming at Angry (no
  reaction), runner routes and panic, blasts/chains/falls with the ending
  hold, and board widths 6–18 at zoom 75–100 %. Slow motion, collider
  overlay and reduced motion. `scripts/lab_tour.txt` visits all of it.
- Procedural creatures: six families (shape + colour + personality), Runner
  cue (sweatband, feet) and the jagged, steaming Angry.
- Board layout with alternating N / N−1 rows, six-neighbour graph and the
  straight-corridor runner route check, with tests.
- `View::fan`, `View::ellipse`, `View::polyline`, `View::circle_lines`
  (framework patch, see `docs/FRAMEWORK_PATCHES.md`).

### Removed

- The template's example game and its assets (`paw.png`, `chime.wav`).
