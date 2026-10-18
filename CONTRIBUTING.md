# Contributing

Thanks for your interest in *Rust a move*! The game is built in milestones
([plan](docs/game/06_IMPLEMENTATION_PLAN.md)); issues and pull requests are
welcome.

## Before you start

- Read [AGENTS.md](AGENTS.md) (rules for every change, by humans or AI
  agents), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) and the rules in
  [docs/GAME_RULES.md](docs/GAME_RULES.md).
- Rule changes are product decisions: open an issue first. Confirmed
  mechanics (fear instead of an aiming line, the uninformative Angry,
  nervous neighbours, runners) are not changed to simplify code.

## Making a change

1. Keep the change small and focused; code, comments, docs and commit
   messages are in English.
2. Add or update tests next to the code (see
   [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md#tests)).
3. Run the required checks:

   ```bash
   cargo fmt --check
   cargo clippy --all-targets --locked -- -D warnings
   REQUIRE_FFMPEG=1 cargo test --locked
   ./scripts/check-vendored-miniquad.sh
   ```

4. If the change affects what is drawn or recorded, run a scripted tour
   (`scripts/range_tour.txt` or `scripts/lab_tour.txt`) and look at the
   screenshots; say in the pull request what you checked by running the game
   and what only by tests.
5. Update the docs with the code: rules, architecture, README, changelog
   (*Unreleased*), the decision log and, for framework files, the patch
   ledger.

Every player-visible text needs English and Swedish. All art and sound must
be original or under a GPL-3.0-compatible license recorded in
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md); no Snood assets.

By contributing you agree that your contribution is licensed under
GPL-3.0-or-later, like the rest of the project.
