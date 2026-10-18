# Changelog

All notable changes to Rust a move are documented here.

## [Unreleased]

### Changed

- The project is named *Rust a move* (id `rust-a-move`, environment prefix
  `RUST_A_MOVE`); the design package's working title was GRIN. The game
  module is `src/rustamove/`.
- Documentation completed and all in English: README, `docs/DEVELOPMENT.md`,
  `docs/VERIFICATION.md`, `docs/game/README.md` (replacing the Swedish
  overview), `CONTRIBUTING.md`.

### Added

- Started from the RustyHusky template 0.3.0.
- Design package in `docs/game/`; authoritative rules in
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

- Shooting range (M2), chosen on a new game-local setup screen: aiming
  with the mouse or keys (10°–170°), no aiming line, fear on the predicted
  first creature (never on Angry), shots with any number of side-wall
  bounces, local attachment or a reported blocked board, ammunition from
  the families on the board, four seeded test boards at six sizes.
- Shared shot planner with continuous collision; tests compare it with a
  fine-step simulation over 1 200+ random shots, mirrored boards, tangent
  and simultaneous contacts, and check that the afraid creature is the one
  hit. `RUST_A_MOVE_DEV=1` path and collider overlays. `scripts/range_tour.txt`.

### Removed

- The template's example game and its assets (`paw.png`, `chime.wav`).
