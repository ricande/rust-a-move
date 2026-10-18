# Character and animation bible

Original simple geometry with expressive faces. Humour must read at the smallest supported creature size and in 1080×1920 recordings. No required image generation or sprite atlas. Names and silhouettes below are proposed and may be tuned during the animation lab.

## Family roster

| Family | Shape / colour direction | Default personality |
| --- | --- | --- |
| Round | Round, blue | Unreasonably cheerful until threatened. |
| Block | Square, green | Irritated, heavy eyebrows. |
| Diamond | Diamond, violet | Suspicious side-eye. |
| Hex | Hexagon, amber | Tries to appear dignified. |
| Triangle | Rounded-looking triangle, coral | Overconfident grin. |
| Oval | Oval, cyan | Sleepy and easily startled. |

Distinct silhouette + colour + face aid recognition. Avoid recreating Snood's exact roster/palette/expressions. Ordinary grumpiness is expression only; special Angry must have a unique unmistakable silhouette/detail, such as a jagged dark-red body with steam marks, not merely the regular red family.

Runner is an ordinary family with an additional persistent visual cue (small sweatband/looking toward exits). Its family remains obvious for matching. Angry and Runner are mutually exclusive in v1. Angry neighbours know whom they are scared of: their pupils look toward the closest angry centre, with stable-id tie-break.

## Procedural rig

Work in local coordinates about the collider centre, radius=1. Define body polygon/ellipse, two eyes with separate pupils, two brows, and mouth as a short polyline or curve approximation. A small shared drawing function consumes FamilyStyle + Pose. Do not make one bespoke animation code path for every family/state combination.

Layer order: subtle body shadow → body fill/outline → eyes → pupils → brows → mouth → sweat/steam accents. Use high contrast; essential expressions are not dependent on particles. Scale contours and minimum face gaps for small bodies. View handles final target scaling exactly once.

Separate expression channels (eyes, mouth, brows, gaze) from body motion (shake, tilt, stretch). An expression blending function combines base personality with nervousness and aimed fear, then applies motion preferences. Angry ignores aimed fear entirely. A runner uses panic body motion while retaining a readable face. Eliminated actors use removal/fall pose and no longer accept live targeting.

## State cues and timing targets

| State/event | Body | Face / gaze | Proposed duration |
| --- | --- | --- | --- |
| Idle | Very slight breathing, separate deterministic phase | Blink, grumpy look, rare small glance | Blink every 2–6 s; 0.12 s closure. |
| Nervous neighbour | Tiny irregular tremor, lean away <=0.04r | Side-eye at Angry, tight mouth, occasional sweat | Blend in 0.12 s, out 0.18 s. |
| Aimed ordinary | Clear tremor up to 0.06r | Wide eyes at launcher, clenched mouth | Ramp in 0.08 s. |
| Target released | Brief relaxed return | Exhale, eyelids settle | 0.18–0.25 s. |
| Angry idle | Constant compact vibration, tense body | Deep brows, compressed mouth, steam | Same with and without aim. |
| Runner struggle | Lean/stretch toward exit, recoil then pull | Exit glance, teeth clenched | Repeating pulls over 0.75 s. |
| Runner panic | Faster pulling if not yet eligible to depart | Looks at Angry then exit | Immediate departure on eligible tick. |
| Runner departure | Squash, release, lateral dash | Relief as it exits | 0.20–0.30 s. |
| Ordinary match | Brief squash then pop fragments | Surprised eyes | 0.15–0.25 s. |
| Angry blast | Tiny smile, flash, ring, geometric debris | Smile just before visible burst | Smile ~0.06 s, effects ~0.35 s. |
| Unsupported fall | Tumble and stretch | Wide eyes | Offscreen or <=0.8 s lifetime. |

The angry smile is presentation lead-in only: logical blast closure is already committed and firing is locked. Never give the player a chance to shoot during the smile after victims were logically removed.

## Readability constraints

- Nervous shaking and aimed terror must be visibly different. Use eye/mouth changes as well as amplitude; do not rely on frequency alone.
- Angry must be exactly as animated under aim as without it. No aim-dependent steam, sound, glow or target icon may leak the target.
- A special-target prediction must never highlight a creature behind it.
- All local body movement stays cosmetic. The collision circle does not drift.
- Prefer 6–9 Hz aimed tremor with deterministic phase over random draw-time jitter; verify comfort, especially in dense boards.
- Reduced motion sets shake/rotation close to zero, retaining expressive eyes, brows, mouth and a stable frightened pose. No trajectory line is added automatically.
- No screen shake by default. Flash is a brief local effect with a reduced-flash option, not full-screen white strobing.
- Visual quality may cap debris, but never suppress the aimed reaction, runner cue, family silhouette or pressure warning.

## Sound direction

Use short original WAV SFX through `Game::SOUNDS`/`Sfx::Game(index)`: launch plop, wall tick, attach thud, group pop, geometric rumble, runner squeak and angry puff. Template handles live volume and video mixing. Avoid continuous chatter and one sound per nervous neighbour. One blast cue per chain plus a bounded accent per wave; one fall cue for the whole detached cluster. Emit sounds from update-time events once, not per draw, and cap concurrent event sounds. No looping music integration in v1; it is absent from the audited template.

## Animation lab: first visual deliverable

Build a selectable game-local lab using the same renderer/emotion code as real gameplay. It must expose:

1. All ordinary shapes side by side, with slow/normal time and size preview.
2. One ordinary target with aim on/off and visible logical collider overlay.
3. Angry surrounded by six nervous neighbours; targeted neighbour becomes extra terrified.
4. Aim directly at Angry: behaviour unchanged; nothing behind reacts.
5. Runner blocked; left exit open; right exit open; both open.
6. Runner adjacent to Angry: blocked panic, then immediate escape on opening.
7. Direct blast, chained blast, falling cluster and the short ending hold.
8. Reduced motion and minimum/maximum board width previews.

Lab can use existing keys (Z/X select scenario, arrows adjust parameters, Space triggers the event), with controls localized and Esc/P reserved. It need not be a general editor. Capture stills and a short video on a real graphical host. The review gate is: funny and readable before polishing gameplay; screenshots alone cannot verify timing.
