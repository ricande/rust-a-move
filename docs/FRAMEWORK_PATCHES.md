# Framework patches

GRIN is built on the RustyHusky template. This ledger lists every change to
the template's framework files, so they can be reapplied after a template
update. `scripts/update-framework.sh` overwrites framework files (including
`view.rs`, `game.rs`, `app.rs`, `screens.rs` and `AGENTS.md`); never run it
blindly. Update on a clean branch, then reapply the entries below and run
the required checks.

**Template baseline:** RustyHusky 0.3.0,
commit `91c257d8e60ca381768dde0258e04d745a30d184` (the commit audited in
[game/08_TEMPLATE_AUDIT.md](game/08_TEMPLATE_AUDIT.md); the local template
checkout was clean at this commit when GRIN was created on 2026-10-18).

## Expected game-side changes (carried over by the update script)

| File | Change |
| --- | --- |
| `src/config.rs`, `Cargo.toml`, `Cargo.lock` | Name GRIN / id `grin` / prefix `GRIN`, version 0.1.0 (via `scripts/new-game.sh`). |
| `src/main.rs` | `mod grin;` and `type ActiveGame = grin::GrinGame;` instead of the example. |
| `src/example.rs`, `assets/paw.png`, `assets/chime.wav` | Removed, as `docs/NEW_GAME.md` describes. |

## Framework patches

### P1 — Logical drawing primitives for faces (M1)

- **File:** `src/view.rs`.
- **Change:** `View::fan` (fill a star-shaped polygon as a triangle fan),
  `View::ellipse`, `View::polyline` (round joints), `View::circle_lines`; a
  private `View::pixel` mapping shared by them; the pure helper
  `ellipse_points`. All take logical coordinates; no game code knows pixels.
- **Why:** procedural creature bodies and faces need polygons and ellipses;
  the template only had rectangles, circles, lines, text and images
  (docs/game/03_TECHNICAL_DESIGN.md §7).
- **Tests:** `view::tests::drawing_maps_logical_points_like_the_pointer`,
  `view::tests::ellipse_points_lie_on_the_turned_ellipse`; visual check with
  `scripts/lab_tour.txt`.
- **Upstream:** generic; could be offered to RustyHusky later (not done).

### P2 — AGENTS.md appendix

- **File:** `AGENTS.md`.
- **Change:** opening sentence names GRIN, environment variables use the
  `GRIN_` prefix, and the game-specific appendix is added at the end. The
  template's rules are otherwise unchanged.

## Planned (not yet made)

- **Round outcome Won/Lost + Retry (M3):** `RoundResult` only has a score
  and every finished round plays Success. GRIN needs a generic outcome and
  Retry in `game.rs`, `app.rs`, `screens.rs` and `i18n.rs`
  (docs/game/03_TECHNICAL_DESIGN.md §8).

## Known template notes

- The `Game::handle` comment says it is called once per displayed frame;
  `App::advance` actually delivers merged input once before a batch of
  updates. GRIN codes to the actual behaviour (presses are applied in the
  next `update`). Not changed.
