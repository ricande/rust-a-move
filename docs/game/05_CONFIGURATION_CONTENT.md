# Configuration, content and balancing

All values here are PROPOSED prototype defaults. Store constants once in validated Rust rules; generate documentation/examples from those values where practical. Do not scatter duplicate magic numbers across renderer, collision and tests.

## Player-facing setup

| Setting | Default / range | Meaning |
| --- | --- | --- |
| Long-row columns | 8 / integer 6–18 | Short rows have N−1 cells. Changing this rebuilds the next board. |
| Board zoom | 100% / 75–100% | Uniform scale of cell geometry within the board area; lower zoom creates smaller creatures and a narrower centred board. |
| Reduced motion | Off | Reduces shaking, tumbling and recoil; retains expression-based fear. |
| Reduced flashes | Off | Replaces explosion flash with a ring and colour pulse. |
| Visual quality | Normal / Low, Normal | Debris density only; never changes rules or essential cues. |

The width control changes topology/difficulty; zoom changes presentation geometry and world-unit distances uniformly, not speed in diameters/second. Explain both in EN/SV tooltips and show a preview. Do not offer a separate raw radius slider that contradicts the selected N. Reject invalid values with a visible explanation and restore the last valid configuration. Structural edits apply on Start/new round; do not resize occupied boards in place.

## Layout defaults

Logical view is 540×960. Safe board horizontal area is x=30..510 (480 units). Board width W=480·zoom, left=(540−W)/2, right=left+W. Common radius r=W/(2N). Initial ceiling_y=100; danger_y=760; launcher=(270,850). Danger line and launcher stay fixed while board geometry scales. The zoom scales creature geometry, spacing, projectile radius and rule speeds together; it is not a camera crop. UI remains fixed-size and readable. Initial board has 8 rows unless the selected level specifies otherwise and passes fit validation.

Maximum addressable rows derive from layout: `ceil((danger_y−initial_ceiling_y)/h)+2` with h=sqrt(3)r. Extra rows allow an attachment to cross the danger line and lose normally. Never allocate from an untrusted unbounded row count. Validate a hard total-cell limit (e.g. 1024) and initial lowest collider strictly above danger. Descending offset does not allocate more rows. Reject malformed boards rather than clipping or dropping creatures silently.

## Rule/timing defaults

| Field | Default | Validation / units |
| --- | --- | --- |
| match_min | 3 | Fixed v1 mechanic; not a player setting. |
| family_count | 6 | Exactly six authored ordinary families in v1. |
| projectile_speed | 16 | Collider diameters / simulation second. |
| aim_min / aim_max | 10° / 170° | Strict positive upward direction. |
| keyboard_aim_speed | 100°/s | Applied in fixed update. |
| explosion_radius | 1 | Integer 1–3 hex graph rings, developer rule. |
| runner_struggle_ticks | 90 | 0.75 s at 120 Hz. |
| panic_struggle_ticks | 0 | Departure at next eligible tick. |
| escape_present_ticks | 30 | 0.25 s; blocks firing. |
| pop_present_ticks | 24 | 0.20 s. |
| blast_present_ticks | 42 | 0.35 s including short smile lead-in. |
| fall_max_ticks | 96 | 0.8 s maximum wait; delete offscreen actors earlier. |
| ending_hold_ticks | 96 | 0.8 s after important effects. |
| ceiling_shots | 5 | Every shot counts, no new rows. |
| ceiling_step | 1 | Row pitch per pressure event. |
| match/explosion score | 10 | Per removed board creature. |
| detached score | 20 | Per unsupported board creature. |
| escape score | 10 | Per voluntary departure. |
| max_particles | 256 / 64 low | Cosmetic cap, independent of logical actors. |

Do not use arbitrary float epsilon everywhere. Define a geometry tolerance relative to diameter (initial proposal 1e−7·diameter in f64) for intersections and tie comparisons. Freeze and test the policy. Use squared-distance comparisons where appropriate; do not let a large rendering-scale epsilon become a collision advantage.

## Saved game configuration

Use `<game data dir>/game-settings.json`; framework settings.json remains owned by the template. Suggested document:

```json
{
  "schema_version": 1,
  "columns": 8,
  "board_zoom": 1.0,
  "reduced_motion": false,
  "reduced_flashes": false,
  "visual_quality": "normal"
}
```

Deserialize with defaults for missing fields; unknown fields may be ignored for forward-compatible reads. Unsupported schema versions and bad types yield defaults plus a nonfatal localized warning. Reject NaN/infinite values and out-of-range geometry after decoding. Write atomically through temporary file + rename using the existing template pattern. Do not overwrite a corrupt/newer file automatically just because it was read; write only on explicit setup Save/Start. No simulation save-game promise in v1.

Developer rule presets and individual levels are embedded into the binary for normal use. Optional external developer overrides must be explicit, size-bounded and validated. Record effective config and seed in debug logs to reproduce a bug; never rely on an invisible global RNG.

## Level format

Keep initial authoring as JSON. These are proposed fields; implement a versioned serde type and validate before constructing a round. Example of a connected 6-column board (row lengths 6 and 5):

```json
{
  "schema_version": 1,
  "id": "intro-anger",
  "columns": 6,
  "seed": 101,
  "rows": [
    ["round", "block", "angry", "hex", "diamond", "oval"],
    ["round", "runner:block", "triangle", "hex", "diamond"]
  ],
  "rules_override": { "ceiling_shots": 5, "explosion_radius": 1 }
}
```

`null` represents an empty cell. Tokens map to known families/specials only; no arbitrary strings reach game logic. IDs assigned by row-major parse order. Verify exact row lengths, valid columns, no duplicate occupancy, bounds, ceiling connectivity, family palette, valid runner-kind combinations, allowed override fields and initial danger clearance. Blasts and corridors use the resulting layout, not JSON token shapes.

Do not stretch a six-column curated level into eight columns. Curated levels are indexed by width. Supply a short intro set for default width; for other widths use a separately validated seeded generator until those widths have authored levels. Menu text must not claim identical difficulty across widths.

## Progression plan

1. Ordinary matching with an obvious three-group and one hanging cluster.
2. Side-wall bounce with fear feedback revealing the eventual target.
3. Angry in a reachable location, nervous neighbours and a useful blast.
4. Two angry creatures demonstrating chain reactions.
5. Runner with an opening that teaches autonomous departure.
6. Runner beside angry, initially blocked, opening via a match.
7. Mixed board with choices between blast, match and support removal.

Tutorial text describes observable action, never engine terms. Keep guides brief and recorded gameplay visually clean. The launcher shows current+next creature and the pressure counter. Outcome uses localized Won/Lost text and final score in the ending hold.

Seeded generator starts with a supported mask anchored to the ceiling, fills a balanced ordinary palette, then optionally replaces cells with specials according to a level preset. Constrain initial special frequency (proposal <=5% Angry, <=8% Runner) and avoid unavoidable immediate escape in the first taught runner board. Deterministic bounded generation retries; on failure use a known valid fallback for that width. Connected and valid is not the same as solvable: log the distinction and verify tutorial winning sequences explicitly.

## Prototype tuning measurements

During local playtests record shots, duration in simulation time, biggest fall, blast chain length, escape count and loss reason. No telemetry service. Evaluate whether fear remains readable at 18 columns, whether waiting for runners dominates play, and whether every-five-shot pressure is satisfying. Adjust presets with a documented reason; do not change mechanics while diagnosing a geometry defect.
