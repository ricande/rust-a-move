# Test and acceptance specification

This is a test plan. No listed gameplay test has been executed by the author of this document. Claude must fill the implementation verification record with actual results. Keep headless rule tests distinct from graphical/manual and hardware checks.

## Required automated checks

Retain template CI and minimum Rust support. Run `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`, and `scripts/check-vendored-miniquad.sh`. Use the template's FFmpeg-required test configuration in CI (REQUIRE_FFMPEG=1). Follow actual inherited AGENTS commands when they differ. Report dependency/tool absence explicitly. Do not repeatedly rerun a long unchanged baseline merely to rediscover it is green; run the complete required checks for meaningful implementation milestones.

Pure core tests construct boards/options directly and advance integer ticks without window/GPU/audio or reading saved user files. Typed event assertions verify semantics, not internal implementation details. A fixture holds board, rules, seed, tick commands and expected contacts/removals/score/outcome. Keep a small set of regression fixtures with meaningful names rather than snapshotting every float in the renderer.

## Geometry and targeting cases

| ID | Scenario | Expected |
| --- | --- | --- |
| G01 | Corners/edges/interior, both row parities | Valid reciprocal neighbours, max six, no wraparound. |
| G02 | Columns 6/8/12/18 at zoom .75/1.0 | All centres and colliders fit horizontal bounds; launcher clear. |
| G03 | World→nearest legal cell around half-way points | Documented row/col tie-break. |
| G04 | Direct ray to one creature | Same id afraid and physically hit. |
| G05 | One and multiple side bounces | Same contact through prediction and execution. |
| G06 | Circle tangency / near miss / overlapping simultaneous candidates | Tolerance and stable-id policy applied consistently. |
| G07 | Flight crosses more than one segment in a tick | No missed bounce/contact or duplicate impact. |
| G08 | Lowest-speed and highest-speed test presets | Same contact and attachment; no tunnelling. |
| G09 | Aim at Angry with ordinary behind it | No extra Angry reaction and no hidden ordinary target. |
| G10 | Ceiling hit at gap / occupied edge / corner | Local valid attachment or BoardBlocked; never remote placement. |
| G11 | Creature with occupied neighbouring slots | Does not overwrite, teleport or snap through blocker. |
| G12 | Radius/board revision changes between frames | Prediction refreshed before launch; revision stable during flight. |
| G13 | Below-launcher and zero-length pointer | Clamp or retain valid aim; no NaN/infinite shot. |
| G14 | Pointer unchanged while keyboard aiming | Keyboard aim retained; actual pointer movement regains control. |
| G15 | Minimum-angle shot at narrowest board | Finite path within validated reflection bound. |

## Matching, support and explosion cases

| ID | Scenario | Expected |
| --- | --- | --- |
| R01 | Group sizes 2, 3 and 5 including projectile | Two stays; three/five remove entire component. |
| R02 | Matching family across odd/even row boundary | Graph connectivity correct. |
| R03 | Unrelated pre-existing triple | Persists until a shot joins that component. |
| R04 | Ordinary match disconnects a hanging branch | Whole unsupported branch removed once. |
| R05 | Component touches side wall but not ceiling | Falls. |
| R06 | Top row has gaps with separate anchored components | Both anchored components remain. |
| R07 | Angry adjacent to ordinary match but still supported | Remains inert. |
| R08 | Angry detached by match or escape | Falls inert, no blast. |
| X01 | Direct Angry hit with all attachment slots occupied | Explodes; no attachment needed. |
| X02 | Radius 1 interior | Centre plus occupied immediate neighbours removed. |
| X03 | Radius 2 blast with an empty intervening cell | Radius crosses emptiness. |
| X04 | Chain of three Angry creatures | Full closure before support removal. |
| X05 | Overlapping/cyclic blast areas | Each id removed/scored once, chain terminates. |
| X06 | One Angry would fall after first blast but is in another blast radius | Snapshot closure still triggers it. |
| X07 | Runner in blast area | Removed by blast, never later escapes. |
| X08 | All ceiling anchors removed | Every surviving unsupported creature falls. |

## Runner cases

| ID | Scenario | Expected |
| --- | --- | --- |
| E01 | No corridor, including staggered-row near blocker | No escape progress/departure. |
| E02 | Only left / only right / both open | Correct route, nearest exit, left on tie. |
| E03 | Normal open corridor | Remains occupied for 89 ticks, departs on tick 90 of eligibility. |
| E04 | Open corridor plus adjacent Angry | Departs on first eligible tick. |
| E05 | Angry adjacent but no route | Nervous/panic presentation, no wall tunnelling. |
| E06 | Route closes before expiry | Progress resets; can choose another free side. |
| E07 | Runner is ordinary aimed target and fire occurs at expiry | Fire priority; runner stays through flight. |
| E08 | Corridor opens by impact resolution | No departure until resolution/settlement ends. |
| E09 | Two ready runners | Stable-id first, second reevaluated after first+support. |
| E10 | First departure detaches second runner | Second falls, no extra escape event/score. |
| E11 | Runner family makes a triple | Matched normally, timer irrelevant. |
| E12 | Angry removed before blocked route later opens | Normal delay; no stored panic bonus. |
| E13 | Pause in middle of struggle | No timer advance until resumed. |
| E14 | Last creature escapes | Victory with escape score and visible departure before completion. |

## Phases, outcome and scoring cases

| ID | Scenario | Expected |
| --- | --- | --- |
| F01 | Fire press during flight/settlement | Discarded, no delayed autofire. |
| F02 | Display frames with zero simulation updates | Press delivered once on next update. |
| F03 | One handle followed by multiple updates | One shot; held keyboard state retained. |
| F04 | Fifth shot clears entire board | Victory before pressure. |
| F05 | Fifth nonclearing shot | Ceiling moves one pitch; parity unchanged. |
| F06 | Escape without a shot | Pressure counter unchanged. |
| F07 | Collider bottom equals danger line after shot/pressure | Lost. |
| F08 | Blocked local attachment | Lost, no panic/hang or remote snap. |
| F09 | Overlapping blast plus detached actors | Score has one removal cause per entity. |
| F10 | Final result queried repeatedly | Identical outcome/score, no extra events. |
| F11 | Loss results then Retry | Same level/seed/options, no Success fanfare. |
| F12 | Win then Next | Next level, fresh score, template round number advances. |
| F13 | Abort from pause | Distinct from loss; recorder cleaned up as template specifies. |
| F14 | Ending effects/hold | Completion delayed until required presentation finishes. |

## Configuration and determinism

Validate invalid columns/zoom, bad tokens, malformed row lengths, unsupported schemas, oversized level data, floating nonfinite values, disconnected starts and initial danger overlap. Missing config fields default; bad file gives a visible warning; no automatic destructive rewrite. Restart round options persisted; structural changes cannot occur mid-flight.

Same seed+config+commands should produce same events/score/board in repeated headless runs; cosmetic variation and Low/Normal quality must not change these results. Test a draw-free run against a run that evaluates poses twice per step. The pose evaluator is pure. Rendering APIs require real visual verification; a pose test alone is not proof that a video looks right.

## Visual acceptance on a real host

- At both largest and smallest creature sizes: ordinary family, Runner and Angry are distinguishable without reading text.
- Aimed fear is obvious among already nervous neighbours; no trajectory line/ghost is visible in normal play.
- Angry has no aim-dependent change; its neighbours glance toward it.
- Runner's blocked/open/panic states read correctly and dash matches logical direction.
- Shape deformation never visually drags the collider away from the hit position.
- Local explosion smile/flash/fall appears before Ending; capped effects do not hide essential cues.
- Reduced motion/flash options retain gameplay information.
- EN/SV text fits 540×960, wide and tall letterboxed windows, with correct mouse hit regions.
- Video view and window show the same creature poses at the same simulation tick.

## Video and platform acceptance

On an available graphical host, play one ordinary win, one loss/retry and one mixed-special round in each recording mode as relevant. Save a video with custom WAV SFX; inspect with ffprobe for 1080×1920, 60/1 fps, H.264/AAC; listen for synchronization. Pause creates no extra gameplay time or animation. Save/Discard/abort and max-duration cutoff behave as inherited. Test recorder failure using existing template fixtures instead of a second encoder subsystem. Isolate test data/video directories.

Record system/architecture, graphics backend actually observed, GPU/driver, build SHA and mode. A Wayland desktop may run an XWayland game; do not label it native Wayland without evidence. Linux Intel UHD 630 and macOS M2 are target manual hosts, not assumed test access. Windows CI green is not a manual Windows graphics test.

Performance target is 60 displayed fps on the user's Intel machine for maximum supported board/effect presets with recording Off; measure before claiming. Recording mode may intentionally slow simulated wall-time under encoder backpressure while preserving video frames. Compare simulation-time correctness separately from wall-clock fps. Record baseline and worst scene; avoid arbitrary 800,000-object tests unrelated to this game.

## Verification record template

For each milestone: SHA/build, host, commands + pass/fail/skip, fixtures changed, real gameplay checked, video checked, known limitations. Keep unchecked items as NOT RUN. Never prefill PASS from this plan.
