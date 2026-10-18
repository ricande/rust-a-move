//! What a creature feels and how that shows: pure expression logic.
//!
//! The board decides the *cues* (nervous, aimed at, runner route …). Each
//! creature keeps a small `Mood` that blends towards those cues over fixed
//! simulation time (`Mood::step`, called from `update`). `pose` then turns
//! kind + mood + animation time into a `Pose` – body motion and face –
//! without changing anything, so `draw` can call it as often as it likes.
//! See docs/game/04_CHARACTERS_ANIMATION.md.
//!
//! Angry is the important exception: its mood never follows the aim, so its
//! pose under aim is exactly its pose without aim (rule R03).

use macroquad::math::{vec2, Vec2};

/// The six ordinary match families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Family {
    Round,
    Block,
    Diamond,
    Hex,
    Triangle,
    Oval,
}

pub const FAMILIES: [Family; 6] = [
    Family::Round,
    Family::Block,
    Family::Diamond,
    Family::Hex,
    Family::Triangle,
    Family::Oval,
];

/// What kind of creature it is. A runner is an ordinary family member with
/// a trait, so it is not a kind of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ordinary(Family),
    Angry,
}

/// What a runner is doing, as the board sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RunnerCue {
    /// Not a runner.
    #[default]
    None,
    /// A runner with no open route.
    Blocked,
    /// Pulling towards an open exit (`dir` −1 left, +1 right).
    Struggle { dir: f32 },
    /// Touching an angry creature: frantic. `dir` is the open exit, if any.
    Panic { dir: Option<f32> },
}

/// The cues for one creature, derived from the logical board every tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cues {
    /// Directly next to at least one angry creature.
    pub nervous: bool,
    /// The predicted first creature the shot would hit.
    pub aimed: bool,
    /// Unit direction (field coordinates, y down) to the nearest angry
    /// neighbour, if any.
    pub angry_dir: Option<Vec2>,
    /// Unit direction to the launcher.
    pub launcher_dir: Vec2,
    pub runner: RunnerCue,
}

/// Blend times in seconds (docs/game/04_CHARACTERS_ANIMATION.md).
pub const NERVOUS_IN: f32 = 0.12;
pub const NERVOUS_OUT: f32 = 0.18;
pub const FEAR_IN: f32 = 0.08;
pub const FEAR_OUT: f32 = 0.22;
/// The exhale after the aim moves away.
pub const RELIEF: f32 = 0.22;
/// Fear above this counts as "was really afraid" for the relief.
const RELIEF_FROM: f32 = 0.5;

/// A creature's blended emotional state, advanced only in fixed steps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Mood {
    /// 0..1, blends towards 1 while nervous.
    pub nervous: f32,
    /// 0..1, blends towards 1 while aimed at (never for Angry).
    pub fear: f32,
    /// Seconds of relief left after the aim moved away.
    pub relief: f32,
}

fn approach(value: f32, target: f32, dt: f32, time_in: f32, time_out: f32) -> f32 {
    if target > value {
        (value + dt / time_in).min(target)
    } else {
        (value - dt / time_out).max(target)
    }
}

impl Mood {
    /// Advances the blend by `dt` seconds of animation time towards `cues`.
    pub fn step(&mut self, kind: Kind, cues: &Cues, dt: f32) {
        // Angry's own fury takes priority over nervousness, and it never
        // reacts to the aim.
        let angry = kind == Kind::Angry;
        let nervous = if cues.nervous && !angry { 1.0 } else { 0.0 };
        let afraid = if cues.aimed && !angry { 1.0 } else { 0.0 };
        self.nervous = approach(self.nervous, nervous, dt, NERVOUS_IN, NERVOUS_OUT);
        let was = self.fear;
        self.fear = approach(self.fear, afraid, dt, FEAR_IN, FEAR_OUT);
        if afraid > 0.0 {
            self.relief = 0.0;
        } else if was >= RELIEF_FROM && self.relief == 0.0 {
            self.relief = RELIEF;
        } else {
            self.relief = (self.relief - dt).max(0.0);
        }
    }
}

/// Body motion and face of one creature, in its own coordinates: the
/// collider centre is the origin and its radius is 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    /// Body displacement (radius units, y down). Cosmetic only.
    pub offset: Vec2,
    /// Body tilt in radians.
    pub angle: f32,
    /// Body stretch.
    pub scale: Vec2,
    /// 0 closed, 1 normal, above 1 wide open.
    pub eye_open: f32,
    /// Pupil position inside the eye, −1..1 per axis.
    pub pupil: Vec2,
    /// Pupil size relative to normal.
    pub pupil_size: f32,
    /// Brow slant: positive frowns (inner ends down), negative worries.
    pub brow_angle: f32,
    /// Brow height, positive up.
    pub brow_lift: f32,
    /// Extra height of the left brow over the right (raised-brow look).
    pub brow_skew: f32,
    /// Half width of the mouth (radius units).
    pub mouth_width: f32,
    /// Mouth corners up (smile) for positive values.
    pub mouth_curve: f32,
    /// Right corner raised over the left (smirk).
    pub mouth_skew: f32,
    /// How far the mouth opens.
    pub mouth_open: f32,
    /// Clenched teeth showing.
    pub teeth: bool,
    /// Sweat drop strength 0..1.
    pub sweat: f32,
    /// Steam strength 0..1 (Angry).
    pub steam: f32,
    /// A slowly repeating 0..1 phase for sweat and steam accents.
    pub accent: f32,
}

impl Pose {
    pub const NEUTRAL: Pose = Pose {
        offset: Vec2::ZERO,
        angle: 0.0,
        scale: Vec2::ONE,
        eye_open: 1.0,
        pupil: Vec2::ZERO,
        pupil_size: 1.0,
        brow_angle: 0.0,
        brow_lift: 0.0,
        brow_skew: 0.0,
        mouth_width: 0.35,
        mouth_curve: 0.0,
        mouth_skew: 0.0,
        mouth_open: 0.0,
        teeth: false,
        sweat: 0.0,
        steam: 0.0,
        accent: 0.0,
    };

    /// Blends the face (not the body) towards `to` by `w`.
    fn blend_face(mut self, to: &Pose, w: f32) -> Pose {
        let l = |a: f32, b: f32| a + (b - a) * w;
        self.eye_open = l(self.eye_open, to.eye_open);
        self.pupil = self.pupil.lerp(to.pupil, w);
        self.pupil_size = l(self.pupil_size, to.pupil_size);
        self.brow_angle = l(self.brow_angle, to.brow_angle);
        self.brow_lift = l(self.brow_lift, to.brow_lift);
        self.brow_skew = l(self.brow_skew, to.brow_skew);
        self.mouth_width = l(self.mouth_width, to.mouth_width);
        self.mouth_curve = l(self.mouth_curve, to.mouth_curve);
        self.mouth_skew = l(self.mouth_skew, to.mouth_skew);
        self.mouth_open = l(self.mouth_open, to.mouth_open);
        self.teeth = if w >= 0.5 { to.teeth } else { self.teeth };
        self.sweat = l(self.sweat, to.sweat);
        self
    }
}

/// The resting face that gives each kind its personality.
pub fn base_face(kind: Kind) -> Pose {
    let n = Pose::NEUTRAL;
    match kind {
        // Unreasonably cheerful.
        Kind::Ordinary(Family::Round) => Pose {
            brow_angle: -0.1,
            brow_lift: 0.1,
            mouth_width: 0.42,
            mouth_curve: 0.32,
            ..n
        },
        // Irritated, heavy brows.
        Kind::Ordinary(Family::Block) => Pose {
            eye_open: 0.8,
            brow_angle: 0.45,
            brow_lift: -0.05,
            mouth_width: 0.38,
            mouth_curve: -0.1,
            ..n
        },
        // Suspicious side-eye.
        Kind::Ordinary(Family::Diamond) => Pose {
            eye_open: 0.55,
            pupil: vec2(0.7, 0.0),
            brow_angle: 0.15,
            brow_skew: 0.18,
            mouth_width: 0.28,
            mouth_curve: 0.05,
            mouth_skew: 0.12,
            ..n
        },
        // Tries to look dignified.
        Kind::Ordinary(Family::Hex) => Pose {
            eye_open: 0.72,
            pupil: vec2(0.0, -0.2),
            brow_lift: 0.08,
            mouth_width: 0.22,
            ..n
        },
        // Overconfident grin.
        Kind::Ordinary(Family::Triangle) => Pose {
            eye_open: 0.9,
            brow_skew: 0.22,
            brow_angle: 0.1,
            mouth_width: 0.5,
            mouth_curve: 0.42,
            mouth_open: 0.12,
            teeth: true,
            ..n
        },
        // Sleepy.
        Kind::Ordinary(Family::Oval) => Pose {
            eye_open: 0.32,
            pupil: vec2(0.0, 0.3),
            brow_lift: -0.08,
            mouth_width: 0.18,
            mouth_curve: 0.05,
            ..n
        },
        // Constantly furious.
        Kind::Angry => Pose {
            eye_open: 0.8,
            pupil_size: 0.8,
            brow_angle: 0.85,
            brow_lift: -0.15,
            mouth_width: 0.4,
            mouth_curve: -0.22,
            mouth_open: 0.1,
            teeth: true,
            steam: 1.0,
            ..n
        },
    }
}

/// Nervous next to an angry creature: side-eye at it, tight mouth.
fn nervous_face(angry_dir: Option<Vec2>) -> Pose {
    Pose {
        eye_open: 0.95,
        pupil: angry_dir.unwrap_or(Vec2::ZERO) * 0.75,
        pupil_size: 0.9,
        brow_angle: -0.35,
        brow_lift: 0.08,
        brow_skew: 0.0,
        mouth_width: 0.24,
        mouth_curve: -0.14,
        mouth_skew: 0.06,
        mouth_open: 0.0,
        teeth: false,
        sweat: 0.6,
        ..Pose::NEUTRAL
    }
}

/// Aimed at: wide eyes on the launcher, clenched teeth.
fn fear_face(launcher_dir: Vec2) -> Pose {
    Pose {
        eye_open: 1.4,
        pupil: launcher_dir * 0.55,
        pupil_size: 0.55,
        brow_angle: -0.55,
        brow_lift: 0.32,
        brow_skew: 0.0,
        mouth_width: 0.4,
        mouth_curve: -0.28,
        mouth_skew: 0.0,
        mouth_open: 0.38,
        teeth: true,
        sweat: 1.0,
        ..Pose::NEUTRAL
    }
}

/// The exhale when the aim moves on.
fn relief_face() -> Pose {
    Pose {
        eye_open: 0.45,
        pupil: Vec2::ZERO,
        brow_angle: -0.1,
        brow_lift: 0.0,
        mouth_width: 0.14,
        mouth_curve: 0.0,
        mouth_open: 0.22,
        teeth: false,
        sweat: 0.3,
        ..Pose::NEUTRAL
    }
}

/// A deterministic number in [0, 1) from a seed and an index (splitmix64).
pub fn hash01(seed: u64, k: u64) -> f32 {
    // Multiplying the seed first keeps neighbouring seeds apart.
    let z =
        splitmix(seed.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ k.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    (z >> 40) as f32 / (1u64 << 24) as f32
}

/// The splitmix64 step: a well-mixed 64-bit value from any input.
pub fn splitmix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `sin(2π·(t·freq + phase))`, keeping the phase precise for long times.
fn wave(t: f64, freq: f64, phase: f32) -> f32 {
    let cycles = (t * freq).fract() as f32 + phase;
    (cycles * std::f32::consts::TAU).sin()
}

/// A shake made of two incommensurate sines, roughly at `freq` Hz.
fn tremor(t: f64, freq: f64, seed: u64) -> Vec2 {
    let (p1, p2) = (hash01(seed, 1), hash01(seed, 2));
    vec2(
        0.7 * wave(t, freq, p1) + 0.3 * wave(t, freq * 1.73, p2),
        0.7 * wave(t, freq * 1.11, p2) + 0.3 * wave(t, freq * 1.9, p1),
    )
}

/// Blink every 2–6 s, closing for 0.12 s: blinks sit at a random point in
/// the middle half of each 4 s slot. Returns the eyelid factor 0..1.
pub fn blink(t: f64, seed: u64) -> f32 {
    const SLOT: f64 = 4.0;
    const CLOSE: f64 = 0.12;
    let slot = (t / SLOT).floor();
    let at = slot * SLOT + 1.0 + 2.0 * hash01(seed, 100 + slot as u64) as f64;
    let d = (t - at).abs();
    if d < CLOSE / 2.0 {
        (d / (CLOSE / 2.0)) as f32
    } else {
        1.0
    }
}

/// How strongly a runner pulls right now (0..1): a recoil followed by a
/// pull, repeating every `period` seconds.
fn pull(t: f64, period: f64) -> f32 {
    let x = ((t / period).fract()) as f32;
    if x < 0.3 {
        -0.4 * (x / 0.3)
    } else {
        -0.4 + 1.4 * ((x - 0.3) / 0.7).powf(0.6)
    }
}

/// Runner struggle: three pulls within the normal 0.75 s struggle.
pub const STRUGGLE_PERIOD: f64 = 0.25;
const PANIC_PERIOD: f64 = 0.12;

/// The pose of a living creature on the board at animation time `t`
/// seconds. `seed` decorrelates creatures; `reduced_motion` keeps the face
/// but almost stills the body.
pub fn pose(kind: Kind, cues: &Cues, mood: &Mood, t: f64, seed: u64, reduced_motion: bool) -> Pose {
    let motion = if reduced_motion { 0.05 } else { 1.0 };
    let mut p = base_face(kind);
    let phase = hash01(seed, 0);
    p.accent = ((t * 0.8).fract() as f32 + phase).fract();
    // Breathing.
    let breath = 0.018 * wave(t, 1.0 / 3.2, phase);
    p.scale = vec2(1.0 - breath * 0.5, 1.0 + breath);

    if kind == Kind::Angry {
        // Constant compact vibration and steam, independent of the aim.
        p.offset = tremor(t, 11.0, seed) * 0.03 * motion;
        p.angle = 0.04 * wave(t, 5.0, phase) * motion;
        p.steam = 0.6 + 0.4 * (0.5 + 0.5 * wave(t, 0.7, phase));
        p.eye_open *= blink(t, seed);
        return p;
    }

    // Idle glances for runners: they keep an eye on the exits.
    if cues.runner != RunnerCue::None {
        let glance = wave(t, 0.4, phase);
        p.pupil = vec2(glance.signum() * 0.6, 0.0).lerp(p.pupil, 0.3);
    }

    // Nervous: side-eye at the angry neighbour, lean away, a tiny tremor.
    let n = mood.nervous;
    if n > 0.0 {
        p = p.blend_face(&nervous_face(cues.angry_dir), n);
        // Sweat comes and goes.
        p.sweat *= 0.5 + 0.5 * wave(t, 0.5, phase).max(0.0);
        let away = -cues.angry_dir.unwrap_or(Vec2::ZERO) * 0.04;
        p.offset += (away + tremor(t, 4.5, seed) * 0.015) * n * motion;
    }

    // Runner body: struggle or panic towards the exit.
    match cues.runner {
        RunnerCue::Struggle { dir } => {
            let k = pull(t, STRUGGLE_PERIOD);
            p.offset.x += dir * (0.05 + 0.07 * k) * motion;
            p.angle += dir * 0.12 * k * motion;
            p.scale.x += 0.08 * k.max(0.0) * motion;
            p.pupil = vec2(dir * 0.8, 0.0);
            p.teeth = true;
            p.mouth_curve = -0.12;
            p.brow_angle = 0.3;
        }
        RunnerCue::Panic { dir } => {
            let k = pull(t, PANIC_PERIOD);
            let side = dir.unwrap_or(wave(t, 1.5, phase).signum());
            p.offset.x += side * (0.04 + 0.06 * k) * motion;
            p.offset += tremor(t, 9.0, seed) * 0.03 * motion;
            p.angle += side * 0.1 * k * motion;
            // Looks at the angry one, then at the exit.
            let at_angry = (t * 2.5).fract() < 0.5;
            p.pupil = match (at_angry, cues.angry_dir) {
                (true, Some(a)) => a * 0.8,
                _ => vec2(side * 0.8, 0.0),
            };
            p.eye_open = p.eye_open.max(1.2);
            p.mouth_open = 0.3;
            p.teeth = true;
            p.brow_angle = -0.5;
            p.sweat = 1.0;
        }
        RunnerCue::None | RunnerCue::Blocked => {}
    }

    // Aimed at: the strongest reaction, clearly beyond nervousness.
    let f = mood.fear;
    if f > 0.0 {
        let keep_runner_pupil = matches!(cues.runner, RunnerCue::Panic { .. });
        let pupil = p.pupil;
        p = p.blend_face(&fear_face(cues.launcher_dir), f);
        if keep_runner_pupil {
            p.pupil = pupil;
        }
        p.offset += tremor(t, 7.5, seed ^ 0x55) * 0.06 * f * motion;
        // Shrinks back a little.
        p.scale *= vec2(1.0 - 0.05 * f, 1.0 - 0.07 * f);
    }

    // The exhale after the aim moved on.
    if mood.relief > 0.0 && f < RELIEF_FROM {
        let w = (mood.relief / RELIEF) * (1.0 - f);
        p = p.blend_face(&relief_face(), w);
        p.scale.y -= 0.04 * w;
    }

    p.eye_open *= blink(t, seed);
    p
}

/// Angry's smile just before its blast: the brows stay furious.
pub fn blast_smile(mut p: Pose) -> Pose {
    p.mouth_curve = 0.55;
    p.mouth_width = 0.48;
    p.mouth_open = 0.08;
    p.teeth = true;
    p.eye_open = 0.6;
    p.brow_angle = 0.95;
    p
}

/// A creature that pops in a match, `t` seconds into the pop: squash, then
/// stretch, eyes wide.
pub fn pop_pose(kind: Kind, t: f32) -> Pose {
    let mut p = base_face(kind).blend_face(&fear_face(Vec2::ZERO), 0.8);
    let x = (t / 0.2).clamp(0.0, 1.0);
    let squash = (x * std::f32::consts::PI).sin();
    p.scale = vec2(1.0 + 0.3 * squash, 1.0 - 0.25 * squash);
    p.mouth_open = 0.45;
    p.teeth = false;
    p.steam = 0.0;
    p
}

/// A creature falling off the board: wide eyes, tumbling.
pub fn fall_pose(kind: Kind, t: f32, seed: u64, reduced_motion: bool) -> Pose {
    let mut p = base_face(kind).blend_face(&fear_face(vec2(0.0, -1.0)), 1.0);
    let spin = if hash01(seed, 7) < 0.5 { -1.0 } else { 1.0 };
    let motion = if reduced_motion { 0.1 } else { 1.0 };
    p.angle = spin * 4.0 * t * motion;
    p.scale = vec2(0.92, 1.1);
    p.mouth_open = 0.5;
    p.teeth = false;
    p.steam = 0.0;
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 120.0;

    fn aimed() -> Cues {
        Cues {
            aimed: true,
            launcher_dir: vec2(0.0, 1.0),
            ..Default::default()
        }
    }

    fn steps(mood: &mut Mood, kind: Kind, cues: &Cues, n: u32) {
        for _ in 0..n {
            mood.step(kind, cues, DT);
        }
    }

    #[test]
    fn fear_ramps_in_over_its_time_and_out_more_slowly() {
        let kind = Kind::Ordinary(Family::Round);
        let mut m = Mood::default();
        // 0.08 s at 120 Hz is 9.6 steps.
        steps(&mut m, kind, &aimed(), 5);
        assert!(m.fear > 0.4 && m.fear < 0.7, "{}", m.fear);
        steps(&mut m, kind, &aimed(), 5);
        assert_eq!(m.fear, 1.0);
        // Released: relief starts, fear fades over 0.22 s.
        steps(&mut m, kind, &Cues::default(), 1);
        assert!(m.relief > 0.0);
        assert!(m.fear < 1.0 && m.fear > 0.9);
        steps(&mut m, kind, &Cues::default(), 26);
        assert_eq!(m.fear, 0.0);
        steps(&mut m, kind, &Cues::default(), 30);
        assert_eq!(m.relief, 0.0, "relief is brief");
    }

    #[test]
    fn nervousness_blends_in_and_out() {
        let kind = Kind::Ordinary(Family::Hex);
        let cues = Cues {
            nervous: true,
            angry_dir: Some(vec2(1.0, 0.0)),
            ..Default::default()
        };
        let mut m = Mood::default();
        steps(&mut m, kind, &cues, 15); // 0.125 s > 0.12 s
        assert_eq!(m.nervous, 1.0);
        steps(&mut m, kind, &Cues::default(), 11);
        assert!(m.nervous > 0.0, "fades out more slowly than in");
        steps(&mut m, kind, &Cues::default(), 11);
        assert_eq!(m.nervous, 0.0);
    }

    /// R03/C04: aiming at Angry changes nothing about it.
    #[test]
    fn angry_ignores_the_aim_completely() {
        let mut with_aim = Mood::default();
        let mut without = Mood::default();
        let near_angry = Cues {
            nervous: true,
            angry_dir: Some(vec2(-1.0, 0.0)),
            ..Default::default()
        };
        for tick in 0..600u32 {
            // The aim flickers on and off; the angry one never notices.
            let mut c = near_angry;
            c.aimed = tick % 50 < 25;
            c.launcher_dir = vec2(0.3, 0.9);
            with_aim.step(Kind::Angry, &c, DT);
            without.step(Kind::Angry, &near_angry, DT);
            assert_eq!(with_aim, without);
            let t = tick as f64 / 120.0;
            for reduced in [false, true] {
                assert_eq!(
                    pose(Kind::Angry, &c, &with_aim, t, 9, reduced),
                    pose(Kind::Angry, &Cues::default(), &without, t, 9, reduced),
                );
            }
        }
        assert_eq!(with_aim, Mood::default());
    }

    /// C06: a nervous neighbour that is also aimed at looks different from
    /// a merely nervous one.
    #[test]
    fn aimed_nervous_neighbour_shows_extra_fear() {
        let kind = Kind::Ordinary(Family::Block);
        let nervous = Cues {
            nervous: true,
            angry_dir: Some(vec2(1.0, 0.0)),
            launcher_dir: vec2(0.0, 1.0),
            ..Default::default()
        };
        let both = Cues {
            aimed: true,
            ..nervous
        };
        let (mut a, mut b) = (Mood::default(), Mood::default());
        steps(&mut a, kind, &nervous, 30);
        steps(&mut b, kind, &both, 30);
        // Before the first blink, which is never earlier than 1 s.
        let t = 0.5;
        let pa = pose(kind, &nervous, &a, t, 3, false);
        let pb = pose(kind, &both, &b, t, 3, false);
        assert!(pb.eye_open > pa.eye_open + 0.3, "eyes wide");
        assert!(pb.mouth_open > pa.mouth_open + 0.2, "mouth open");
        assert!(pb.pupil.y > pa.pupil.y, "looks at the launcher below");
        // The difference holds with reduced motion too.
        let ra = pose(kind, &nervous, &a, t, 3, true);
        let rb = pose(kind, &both, &b, t, 3, true);
        assert!(rb.eye_open > ra.eye_open + 0.3 && rb.mouth_open > ra.mouth_open + 0.2);
    }

    #[test]
    fn reduced_motion_stills_the_body_but_keeps_the_face() {
        let kind = Kind::Ordinary(Family::Triangle);
        let cues = Cues {
            nervous: true,
            aimed: true,
            angry_dir: Some(vec2(0.0, -1.0)),
            launcher_dir: vec2(0.0, 1.0),
            runner: RunnerCue::Panic { dir: Some(1.0) },
        };
        let mut m = Mood::default();
        steps(&mut m, kind, &cues, 60);
        let mut max_full = 0.0f32;
        for i in 0..240 {
            let t = i as f64 / 120.0;
            let full = pose(kind, &cues, &m, t, 5, false);
            let reduced = pose(kind, &cues, &m, t, 5, true);
            assert!(reduced.offset.length() < 0.02, "{}", reduced.offset);
            assert!(reduced.angle.abs() < 0.02);
            max_full = max_full.max(full.offset.length());
            assert_eq!(full.mouth_open, reduced.mouth_open);
            assert_eq!(full.brow_angle, reduced.brow_angle);
        }
        assert!(max_full > 0.08, "the full motion does move");
    }

    #[test]
    fn poses_are_pure_and_deterministic() {
        let cues = aimed();
        let mut m = Mood::default();
        steps(&mut m, Kind::Ordinary(Family::Oval), &cues, 4);
        let a = pose(Kind::Ordinary(Family::Oval), &cues, &m, 12.345, 77, false);
        let b = pose(Kind::Ordinary(Family::Oval), &cues, &m, 12.345, 77, false);
        assert_eq!(a, b);
        // Different seeds do not move in lockstep.
        let c = pose(Kind::Ordinary(Family::Oval), &cues, &m, 12.345, 78, false);
        assert_ne!(a.offset, c.offset);
    }

    #[test]
    fn blinks_come_every_two_to_six_seconds() {
        let mut closed_at = Vec::new();
        let mut was_closed = false;
        for i in 0..(120 * 120) {
            let t = i as f64 / 120.0;
            let closed = blink(t, 42) < 0.5;
            if closed && !was_closed {
                closed_at.push(t);
            }
            was_closed = closed;
        }
        assert!(closed_at.len() >= 20);
        for w in closed_at.windows(2) {
            let gap = w[1] - w[0];
            assert!((1.9..=6.1).contains(&gap), "gap {gap}");
        }
    }

    #[test]
    fn faces_stay_distinct_and_angry_is_fiercest() {
        let faces: Vec<Pose> = FAMILIES
            .iter()
            .map(|&f| base_face(Kind::Ordinary(f)))
            .collect();
        for (i, a) in faces.iter().enumerate() {
            for b in &faces[i + 1..] {
                assert_ne!(a, b);
            }
            assert!(base_face(Kind::Angry).brow_angle > a.brow_angle);
            assert_eq!(a.steam, 0.0, "only Angry steams");
        }
    }
}
