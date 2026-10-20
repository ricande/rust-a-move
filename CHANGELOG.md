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

- **Playable game (M3):** the classic round with matching (three or more),
  falls of unsupported creatures, score (10 per match, 20 per fall), a
  ceiling that drops one row every five shots (counter pips), the danger
  line, blocked boards, an in-game outcome banner and ending hold, and
  Retry after a loss. Three hand-made 8-column levels, a seeded generator
  for every other level and width, ammunition from the families on the
  board.
- Setup screen before every round: Play, board width (6–18), zoom
  (75–100 %), reduced motion, shooting range, animation lab; options saved
  in `game-settings.json`.
- Original sound effects (launch, bounce, attach, pop, fall, ceiling) made
  by `scripts/make_sfx.py`.
- Framework: `RoundResult` has an `Outcome` (won/lost); a lost level plays
  a neutral sound and offers Retry (patch P3).
- `scripts/play_tour.txt`: win, loss, Retry, pause and abort with saved
  videos.

- **Angry blasts (M4):** a direct hit on an Angry blows up every creature
  next to it (radius 1, levels may set 1–3) and sets off other Angry ones
  in a chain, decided at once on a snapshot and shown wave by wave with a
  smile, flash, ring and fragments; falls follow the whole chain. Blast and
  chain sounds. Two new hand-made levels (angry neighbours, chain reaction),
  a few Angry in generated levels from level 3, and a Reduced flashes
  option on the setup screen. `scripts/blast_tour.txt`.

- **Runners (M5):** runners struggle towards a straight open path to the
  side for 0.75 s and dash off the board (10 points); next to an Angry
  they bolt at once. They only move while the player aims, one at a time,
  never through anything, and a shot always comes first. What they held
  falls. Escape sound, two new levels (runner intro, runner panic), runners
  in generated levels from level 5. `scripts/runner_tour.txt` and a
  tick-exact tour simulation.

### Removed

- The template's example game and its assets (`paw.png`, `chime.wav`).
