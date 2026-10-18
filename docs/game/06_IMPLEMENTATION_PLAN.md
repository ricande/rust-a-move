# Implementation plan and milestones

Deliver small compilable increments, each with an updated rules/decision entry if behaviour changed. Do not build every special before ordinary contact geometry is trustworthy. Time estimates are intentionally omitted until Claude inspects the actual host and toolchain.

## M0 — Establish the project

Read the handoff start instructions (`CLAUDE_START_HERE.md`, kept in the original package) and the actual template's AGENTS/architecture/new-game documents. Report hostname, pwd, architecture, git root/status and current source SHA. Compare current template to audited baseline. Create a separate local game working copy, preserve notices/vendor and use the naming script only after checking its effects. If no final name exists, use the declared local working name GRIN, and keep it easy to rename. No publishing, pushing or remote creation is included in this task.

Place these docs under `docs/game/` and link them from the game README; place the authoritative rulebook in `docs/GAME_RULES.md` with links to technical detail. Merge the agent appendix into inherited AGENTS, never replace the original wholesale. Record the template commit in `docs/FRAMEWORK_PATCHES.md` and README. Run the inherited required checks once to establish the baseline; record known environment limitations rather than rerunning unchanged baseline checks after every read.

**Gate:** separate game copy, correct identity, clean reviewable diff, retained framework functions, no accidental edit to RustyHusky itself. If the environment is cloud Linux, do not claim access to Richard's Kubuntu desktop or M2 GPU.

## M1 — Character lab and rendering seam

Implement pure Pose/expression logic and a minimal creature renderer, adding only generic View primitives required for geometric faces. Select the lab through an explicit development flag or game-local setup option; do not make a second unrelated renderer. Demonstrate all scenarios in the animation bible, including no aim reaction for Angry and extra fear for an already nervous target. UI need only be functional at this stage.

**Tests:** time-driven expression blending, reduced motion, aim suppression on Angry, draw purity by inspection/state snapshots, geometry mapping helpers. **Manual gate:** real-host video of nervous neighbours, panic runner and anger smile; smallest-size still is readable. If no graphical host is available, mark this gate pending and continue headless work, never report it passed.

## M2 — Board and shared shot prediction

Implement validated layout and six-neighbour graph for 6/8/12/18 columns, fixed cell colliders and a bounded flight planner. Implement local attachment and ceiling hits. Use first_hit for fear. Freeze occupancy while the projectile flies. Add debug fixtures and collider/path overlays only in development mode.

**Tests:** parity, symmetry, borders, tangency, simultaneous-hit policy, bounce prediction vs actual first contact, no tunnelling, blocked attachment, both extreme widths and zoom values. **Gate:** repeated aiming and firing at unchanged board always hits the fearful creature, including after multiple bounces. No hidden trajectory line in normal mode.

## M3 — Ordinary playable loop

Replace placeholder gameplay with loaded/next ammunition, ordinary matching, support flood-fill, deterministic falling events and score. Add phase input locks, pressure movement, danger check, terminal hold and game-local setup. Curated ordinary fixtures precede generation. Implement generic result Won/Lost + Retry integration now so defeat is honest from the first playable.

**Tests:** all R01–R05/R07/R11–R14 relevant cases, score once, clearing shot wins before pressure, loss/retry at same level, no success cue on loss. **Manual gate:** play, pause, win, lose, retry, save/discard a video. Aiming frozen during pause; output remains portrait. This is the first ordinary playable.

## M4 — Angry and chain reactions

Introduce Angry kind, derived nervous neighbours, snapshot blast radius and iterative chain closure. Add short smile/blast/fall presentation and bounded WAV SFX. Test direct anger impact even when no attachment cell exists. Keep falling Angry inert.

**Gate:** an aimed Angry gives no additional cue; every adjacent ordinary creature looks nervous; blast union and falling are correct; no double score or infinite chain. Video shows the smile and blast before results. Radius 1 is default; radius 2/3 are developer configurations with tests.

## M5 — Runners

Add Runner trait, horizontal swept-corridor tests, stable route selection, fixed-tick struggle, immediate panic and autonomous removal while aiming. Freeze departures during flight/resolution, lock input during exit presentation, and reevaluate after each departure/fall. Support matching runners as ordinary family members.

**Gate:** blocked runner cannot pass through staggered neighbours; opening triggers struggle; angry adjacency triggers departure at first eligible tick; a same-tick fire takes priority; departure can cause a fall; other runners do not run twice or escape after being removed.

## M6 — Content and complete setup

Persist width/zoom/accessibility/quality options in game-owned config, validate versioned level JSON and add short tutorial progression. Build a bounded seeded generator for unsupported widths with known valid fallbacks. Add in-game help for fear aiming and pressure. Family silhouettes remain distinct at all settings. Use Game::TEXTS for every text.

**Gate:** width and zoom really change preview and next round layout; restarting preserves settings; corrupt settings recover without silently overwriting the file; seeded start is reproducible; no copied assets. Documentation distinguishes tested solvability from merely valid generation.

## M7 — Recording, platform review and release candidate

Exercise all inherited recording modes with full gameplay SFX and terminal holds. Check that a failed encoder does not break the game and cosmetic work is bounded. Reuse inherited tests rather than duplicating the entire recorder suite. Verify real-host gameplay on Kubuntu/Wayland Intel UHD 630 and macOS Apple Silicon when available; Windows CI is compilation/headless coverage until a real playtest exists.

**Gate:** required formatting/clippy/tests, vendored integrity check and FFmpeg-enabled checks pass; real videos are 1080×1920 at 60 fps with synchronized sound; pause/abort/limit behaviour is retained. README, rules, patch ledger and changelog match the actual game. List unavailable checks explicitly.

## Review packet at each milestone

Report what works, files changed, exact tests run/results, manual evidence, unresolved risks and next milestone. Cite actual commit when one exists. Do not write a large diary or claim tests run on machines not accessible. Ask for decisions only when they block the next concrete milestone; do not ask the user to reapprove already confirmed mechanics.

## Done means

A standalone RustyHusky-derived game, own identity, procedural expressive cast, fear aiming without a line, special Angry and Runner mechanics, selectable geometry, readable UI, correct win/loss/retry, working sound/video, tests for interaction edges, preserved licensing and an honest platform verification record. A pretty mockup or headless core alone is not completion.
