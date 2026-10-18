# Game rules

Normative prototype rules for Rust a move. CONFIRMED mechanics and PROPOSED defaults are classified in [game/01_DECISIONS.md](game/01_DECISIONS.md). All numbers referenced here are supplied by [game/05_CONFIGURATION_CONTENT.md](game/05_CONFIGURATION_CONTENT.md). Geometry and implementation detail: [game/03_TECHNICAL_DESIGN.md](game/03_TECHNICAL_DESIGN.md); characters and animation: [game/04_CHARACTERS_ANIMATION.md](game/04_CHARACTERS_ANIMATION.md).

**Implementation status:** M2. R02 (controls), R03 (fear aiming) and R04 (launch, contact, local attachment) work in the shooting range; matching, falls, blasts, runners, pressure and the ending (R05–R13) do not yet. Framework behaviour (pause with Esc/P, results, recording) is described in the README.

## R01 — Objective and board

Shoot one creature from the launcher into a hanging board. At least three connected ordinary creatures of one family disappear. Creatures no longer connected to the ceiling fall. Clear the board before the remaining board reaches the danger line.

Cells have at most six neighbours in alternating long/short rows. Only occupied top-row cells are ceiling anchors. Side walls do not anchor clusters. Empty top-row gaps remain empty; a ceiling hit can attach there. Visual shapes do not change adjacency or collision radius.

## R02 — Controls

Mouse movement sets aim from the launcher towards the logical pointer. Left click or Space fires once when aiming at a valid angle. Left/Right or A/D rotate aim while held; keyboard movement takes ownership until actual pointer movement occurs (an unchanged pointer must not override keyboard aim). Default aim is vertically up. Valid angles are 10–170 degrees measured counter-clockwise from screen-right with screen-up positive. Below-launcher pointers clamp to the nearest endpoint; a pointer exactly at the launcher keeps the last direction.

No autofire. A press while flying, resolving, escaping, settling or ending is discarded, not queued. Clicks on setup controls never fire. Esc/P remain framework pause. Pause freezes projectiles, emotion time, escape timers and effects. Losing pointer focus does not fire or cancel a shot. Keyboard firing can use the last valid aim outside the field; mouse clicks outside the field do nothing.

## R03 — Aim is expressed by fear

Calculate the first physical creature hit, including wall bounces. Only that creature receives `aimed_at=true`. No trajectory line, dotted route, landing ghost or hidden second target is shown in normal play.

Ordinary creatures show extra fear when targeted. Angry creatures do not change their normal behaviour. If angry is first, nothing behind it becomes afraid. A ceiling-only shot has no afraid target. A nervous angry-neighbour shows a distinguishable additional aimed reaction. A runner can also be targeted and react before departure.

The launcher visibly points in the firing direction; its body may act as orientation feedback without drawing a line into the board. Recompute prediction when aim, layout or occupancy changes. Fear must agree with the actual first impact when fired from the same state.

## R04 — Launch and contact

One projectile at a time; only ordinary match families are fired in v1. Its circle reflects off side walls, never off creatures. At its earliest creature contact it stops and attaches to the nearest legal empty neighbour of that creature, using the tie-break rules in the technical design. At ceiling contact it attaches to the nearest empty top-row cell. No teleporting to a remote free cell.

If no contact-local legal attachment exists, the round is lost with reason `BoardBlocked`. A direct angry hit instead consumes the projectile and explodes the target, with no attachment required. A projectile reaching an implementation safety bound is a reported geometry fault in development; release fallback ends the round safely with an error reason, never an infinite loop.

## R05 — Matching

After normal attachment, flood-fill the attached creature's family. At count >= 3 remove the whole connected group, including runners of that family. Angry creatures have no match family, do not join groups and are not removed just because adjacent matches clear. An ordinary match can detach an angry creature, which then falls without exploding.

An attached projectile that does not match stays in its cell. Only the component containing that projectile is tested; unrelated pre-existing triples do not disappear spontaneously. No automatic matching after ceiling moves or runner departure.

## R06 — Angry explosions

Direct projectile contact is the trigger. Angry adjacency, waiting, matches and falling do not by themselves trigger anger. At trigger time, snapshot board occupancy and neighbour topology. Starting at the struck angry creature, remove occupied cells within the configured hex graph radius, including the centre. Radius crosses empty cells; it is not limited by occupied paths. Any angry creature in the affected area contributes its own blast on the same snapshot. Compute the closure iteratively with a visited set, then remove the union exactly once.

An explosion removes all kinds in its area. No immunity, probabilistic damage or family restriction. Each removed creature scores once. An angry creature touched by two blasts explodes once. Radius is topology distance, not pixel distance or a variable number selected from neighbours. After the full explosion closure, calculate unsupported creatures once. This prevents an intermediate fall from suppressing an otherwise valid chain.

## R07 — Falling and scoring

After a match, explosion or runner departure, flood-fill occupied cells reachable from occupied row-zero cells. Remove every other occupied cell from the board immediately and create falling visual actors. Falling actors do not collide, match, explode, support, block escape or hit the launcher. Their visual exit cannot change score or outcome.

Scoring starts from zero each round. Ordinary match/explosion removal: 10 each. Unsupported falling: 20 each. Voluntary escape: 10 each. The projectile consumed by an angry hit scores nothing. No double scoring for overlapping causes. Use a sufficiently wide internal counter and saturate safely when converting to the template result's u32. Record cause counters for debugging. No chain multiplier in v1.

## R08 — Nervous neighbours

Any creature with at least one directly adjacent angry creature is nervous. Recompute from current logical board after every mutation. Angry behaviour itself takes priority over nervous expressions. Runners use angry contact to enable panic escape. Multiple angry neighbours do not stack movement speed or blast radius. A normal nervous creature remains fully matchable.

## R09 — Escape route and struggle

Only runners can leave voluntarily. A route is a straight horizontal swept-circle path from its centre to beyond the left/right board boundary, clear of all other occupied creature circles. Ordinary projectile walls do not prevent escape: runners exit through the side. Touching at the exact non-overlap boundary is allowed within the documented geometry tolerance. Check the whole corridor, including staggered-row creatures, not only same-row cells.

If both routes are available, choose the shorter distance to an exit; exact ties choose left. The chosen route is held while still valid. If it becomes blocked, reset struggle progress to zero, then choose the remaining route if any. If neither is open, stay in place and stop making escape progress. There is no pathfinding through gaps with turns.

When a route opens during Aiming, struggle for the normal delay. If adjacent to angry while a route is open, depart at the next eligible simulation tick with no extra struggle delay. Angry contact alone does not let the runner pass through occupied cells. If the angry neighbour disappears before a route opens, no permanent panic bonus is retained.

## R10 — Autonomous escape timing

Escape evaluation/timers run only in Aiming. ProjectileFlight, Resolving, Escaping, Settling and Ending freeze autonomous escape timers and logical departures, but may show nervous idle expressions. This keeps the selected target from moving while a shot is travelling.

On an Aiming tick: first validate fire; if a shot is launched, defer escapes. Otherwise evaluate eligible runners in stable entity-id order against the current board. Remove the first ready runner, snapshot its visual exit, apply unsupported fall, and enter Escaping/Settling. Other runners are reevaluated after settlement. A runner removed by that fall never also escapes. No simultaneous batch removal creates intersecting routes.

Logical departure occurs at the start of its short escape animation; the visual body is then non-interactive. Input stays locked until escape/fall presentation finishes. An opened corridor can allow another escape on the next Aiming tick. This is finite because each departure reduces occupancy. Escape itself does not count as a shot and does not advance the ceiling counter.

## R11 — Shot transaction and pressure

After launch: fly; resolve attachment+match or direct blast; remove unsupported creatures; score; present resolution; increment shot counter once; if board is empty, win immediately without pressure. Otherwise if the counter reaches the configured interval, reset it and lower the ceiling and entire board by one row pitch. This changes a board origin offset, not row indices/parity, and spawns no new row.

After pressure, lose if any occupied collider's bottom reaches or crosses the danger line. Apply the same danger test after any completed attachment even when pressure is not due. Topology does not change when lowering. Show the pending pressure counter before the player fires.

Runners do not escape between impact and the pressure danger check. A runner already below the danger threshold cannot rescue a round after it has been declared lost. A final clearing shot wins before ceiling movement, even if movement was due.

## R12 — Phases and terminal state

Game-local phases: Setup → Aiming → ProjectileFlight → Resolving → Settling → Aiming. Autonomous departure uses Aiming → Escaping → Settling → Aiming. Terminal conditions enter Ending, show the outcome in the game view, then report a stable finished result to the framework. Resolving may be one update; its presentation is time-based and bounded.

Use a 0.8 s ending hold after the last important visible event so recording captures the result. Do not return `finished()` while an explosion or exit is still meant to be visible. After completion, score and outcome cannot change; `finished()` is idempotent. Framework pause-abort is distinct from defeat and has no victory fanfare. Results offer retry of the same level on loss and next level on win, retaining save/discard video behaviour.

## R13 — Ammunition and levels

One loaded ordinary creature and one visible next creature. Generate families from the remaining ordinary families (including runners) in stable sorted order using round-local RNG. If none remain but specials do, allow the configured regular family palette so angry targets can still be shot. Revalidate loaded and next after resolution: replace obsolete families before Aiming, deterministically. No player swap in v1.

Start boards are ceiling-connected and valid for the selected width. The first game starts at level 1; Next increments only after a win; Retry preserves the level/seed/configuration for the same challenge. Curated boards take precedence over procedural generation when they exist for that width. No solvability guarantee may be claimed without a solver or a verified winning shot sequence.

## R14 — Configurability and accessibility

Column count changes the maximum long-row capacity and derives creature diameter. Board zoom uniformly scales geometry within a safe rectangle; it never clips the launcher or danger line. Apply structural settings before a round, not while projectiles or occupied cells exist. Colour and shape both identify families. Reduced motion replaces rapid shaking with clear eyes/mouth/pose reactions; angry remains uninformative under aim. Essential fear feedback must never disappear at low visual quality.
