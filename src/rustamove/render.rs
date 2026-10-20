//! Procedural drawing of the creatures and their effects through `View`.
//! Everything here only reads: a `Pose` from `emotion` in, shapes out.
//!
//! Shapes are defined in the creature's own coordinates (collider centre at
//! the origin, collider radius 1, y down) and always fit inside the
//! collider, so a square's corners or a jagged Angry never suggest a larger
//! collision surface. Eyebrows and accents may stick out a little.

use macroquad::color::Color;
use macroquad::math::{vec2, Vec2};

use super::emotion::{hash01, Family, Kind, Pose};
use crate::view::{with_alpha, View};

const INK: Color = Color::new(0.07, 0.05, 0.10, 1.0);
const EYE_WHITE: Color = Color::new(0.98, 0.98, 0.96, 1.0);
const SWEAT: Color = Color::new(0.55, 0.85, 1.0, 1.0);
const STEAM: Color = Color::new(0.92, 0.92, 0.95, 1.0);
const BAND: Color = Color::new(0.97, 0.97, 0.97, 1.0);
const BAND_STRIPE: Color = Color::new(0.9, 0.15, 0.2, 1.0);

/// Body colour of a kind.
pub fn body_color(kind: Kind) -> Color {
    match kind {
        Kind::Ordinary(Family::Round) => Color::new(0.27, 0.55, 1.0, 1.0),
        Kind::Ordinary(Family::Block) => Color::new(0.30, 0.78, 0.33, 1.0),
        Kind::Ordinary(Family::Diamond) => Color::new(0.66, 0.42, 0.96, 1.0),
        Kind::Ordinary(Family::Hex) => Color::new(1.0, 0.72, 0.18, 1.0),
        Kind::Ordinary(Family::Triangle) => Color::new(1.0, 0.47, 0.40, 1.0),
        Kind::Ordinary(Family::Oval) => Color::new(0.30, 0.88, 0.88, 1.0),
        Kind::Angry => Color::new(0.58, 0.04, 0.07, 1.0),
    }
}

fn darker(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

fn lighter(c: Color, k: f32) -> Color {
    Color::new(
        c.r + (1.0 - c.r) * k,
        c.g + (1.0 - c.g) * k,
        c.b + (1.0 - c.b) * k,
        c.a,
    )
}

fn arc(centre: Vec2, r: f32, from: f32, to: f32, n: usize) -> impl Iterator<Item = Vec2> {
    (0..=n).map(move |i| {
        let a = from + (to - from) * i as f32 / n as f32;
        centre + vec2(a.cos(), a.sin()) * r
    })
}

/// A polygon with rounded corners: `corners` in order, each rounded with
/// radius `round`.
fn rounded(corners: &[Vec2], round: f32) -> Vec<Vec2> {
    let mut out = Vec::new();
    let n = corners.len();
    for i in 0..n {
        let (prev, c, next) = (corners[(i + n - 1) % n], corners[i], corners[(i + 1) % n]);
        let a = (prev - c).normalize();
        let b = (next - c).normalize();
        let half = a.angle_between(b).abs() / 2.0;
        let along = round / half.tan();
        let centre = c + (a + b).normalize() * (round / half.sin());
        let start = c + a * along - centre;
        let end = c + b * along - centre;
        let (sa, mut ea) = (start.y.atan2(start.x), end.y.atan2(end.x));
        // Turn the short way round.
        while ea - sa > std::f32::consts::PI {
            ea -= std::f32::consts::TAU;
        }
        while sa - ea > std::f32::consts::PI {
            ea += std::f32::consts::TAU;
        }
        out.extend(arc(centre, round, sa, ea, 4));
    }
    out
}

fn regular(n: usize, r: f32, start: f32) -> Vec<Vec2> {
    (0..n)
        .map(|i| {
            let a = start + i as f32 / n as f32 * std::f32::consts::TAU;
            vec2(a.cos(), a.sin()) * r
        })
        .collect()
}

/// The body outline of a kind, star-shaped around the origin and inside
/// the unit collider.
pub fn body_shape(kind: Kind) -> Vec<Vec2> {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, FRAC_PI_6};
    match kind {
        Kind::Ordinary(Family::Round) => regular(32, 0.9, 0.0),
        Kind::Ordinary(Family::Block) => rounded(&regular(4, 0.95, FRAC_PI_4), 0.16),
        Kind::Ordinary(Family::Diamond) => rounded(&regular(4, 0.96, -FRAC_PI_2), 0.1),
        Kind::Ordinary(Family::Hex) => rounded(&regular(6, 0.94, FRAC_PI_6), 0.08),
        Kind::Ordinary(Family::Triangle) => {
            // Centred on the collider: corners at radius 0.98 around (0, 0.1).
            let tri: Vec<Vec2> = regular(3, 0.98, -FRAC_PI_2)
                .into_iter()
                .map(|p| p * 0.9 + vec2(0.0, 0.08))
                .collect();
            rounded(&tri, 0.2)
        }
        Kind::Ordinary(Family::Oval) => (0..32)
            .map(|i| {
                let a = i as f32 / 32.0 * std::f32::consts::TAU;
                vec2(0.94 * a.cos(), 0.72 * a.sin())
            })
            .collect(),
        Kind::Angry => {
            // Jagged, slightly irregular spikes.
            const SPIKES: usize = 11;
            (0..SPIKES * 2)
                .map(|i| {
                    let a = i as f32 / (SPIKES * 2) as f32 * std::f32::consts::TAU - FRAC_PI_2;
                    let r = if i % 2 == 0 {
                        0.93 + 0.05 * hash01(5, i as u64)
                    } else {
                        0.72
                    };
                    vec2(a.cos(), a.sin()) * r
                })
                .collect()
        }
    }
}

/// Where the face sits in the body and how big it is.
fn face_frame(kind: Kind) -> (Vec2, f32) {
    match kind {
        Kind::Ordinary(Family::Triangle) => (vec2(0.0, 0.22), 0.7),
        Kind::Ordinary(Family::Diamond) => (vec2(0.0, 0.02), 0.8),
        Kind::Ordinary(Family::Oval) => (vec2(0.0, 0.02), 0.85),
        Kind::Angry => (vec2(0.0, 0.0), 0.88),
        _ => (vec2(0.0, 0.0), 1.0),
    }
}

/// Maps creature coordinates to the field for one drawn creature.
#[derive(Clone, Copy)]
struct Frame {
    centre: Vec2,
    r: f32,
    sin: f32,
    cos: f32,
    scale: Vec2,
}

impl Frame {
    fn new(centre: Vec2, r: f32, pose: &Pose) -> Frame {
        let (sin, cos) = pose.angle.sin_cos();
        Frame {
            centre: centre + pose.offset * r,
            r,
            sin,
            cos,
            scale: pose.scale,
        }
    }

    fn map(&self, p: Vec2) -> Vec2 {
        let p = p * self.scale;
        self.centre
            + vec2(
                p.x * self.cos - p.y * self.sin,
                p.x * self.sin + p.y * self.cos,
            ) * self.r
    }

    fn map_all(&self, ps: &[Vec2]) -> Vec<Vec2> {
        ps.iter().map(|&p| self.map(p)).collect()
    }
}

/// Fills a shape given in creature coordinates.
fn fill(v: &View, f: &Frame, origin: Vec2, shape: &[Vec2], color: Color) {
    v.fan(f.map(origin), &f.map_all(shape), color);
}

/// A thick line in creature coordinates.
fn stroke(v: &View, f: &Frame, pts: &[Vec2], thick: f32, color: Color) {
    v.polyline(&f.map_all(pts), thick * f.r, color);
}

/// The leftmost and rightmost x of `shape` at height `y`.
fn span_at(shape: &[Vec2], y: f32) -> Option<(f32, f32)> {
    let mut span: Option<(f32, f32)> = None;
    for (i, &a) in shape.iter().enumerate() {
        let b = shape[(i + 1) % shape.len()];
        if (a.y - y) * (b.y - y) <= 0.0 && a.y != b.y {
            let x = a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y);
            span = Some(span.map_or((x, x), |(l, r)| (l.min(x), r.max(x))));
        }
    }
    span
}

/// Draws one creature at `centre` (field coordinates) with collider radius
/// `r`. `alpha` fades the whole creature.
pub fn draw_creature(
    v: &View,
    centre: Vec2,
    r: f32,
    kind: Kind,
    runner: bool,
    pose: &Pose,
    alpha: f32,
) {
    let f = Frame::new(centre, r, pose);
    let shape = body_shape(kind);
    let color = with_alpha(body_color(kind), alpha);
    let outline = with_alpha(
        if kind == Kind::Angry {
            INK
        } else {
            darker(body_color(kind), 0.35)
        },
        alpha,
    );
    let ink = with_alpha(INK, alpha);

    // Shadow, not transformed by the pose's offset so the body seems to
    // move over it.
    let shadow: Vec<Vec2> = shape
        .iter()
        .map(|&p| centre + (p * 0.98 + vec2(0.06, 0.1)) * r)
        .collect();
    v.fan(
        centre + vec2(0.06, 0.1) * r,
        &shadow,
        with_alpha(INK, 0.28 * alpha),
    );

    if runner {
        // Little feet, ready to go.
        for x in [-0.32, 0.32] {
            let foot = f.map(vec2(x, 0.8));
            v.ellipse(foot, 0.2 * r, 0.11 * r, f.sin.atan2(f.cos), ink);
        }
    }

    // Outline as a slightly bigger body behind the fill.
    let thick = if kind == Kind::Angry { 1.1 } else { 1.07 };
    let big: Vec<Vec2> = shape.iter().map(|&p| p * thick).collect();
    fill(v, &f, Vec2::ZERO, &big, outline);
    fill(v, &f, Vec2::ZERO, &shape, color);
    // A soft highlight, upper left, kept well inside the body.
    let hl: Vec<Vec2> = shape
        .iter()
        .map(|&p| p * 0.6 + vec2(-0.12, -0.14))
        .collect();
    fill(
        v,
        &f,
        vec2(-0.12, -0.14),
        &hl,
        with_alpha(lighter(body_color(kind), 0.3), 0.22 * alpha),
    );

    let (face_at, s) = face_frame(kind);
    let fp = |p: Vec2| face_at + p * s;

    if runner {
        // Sweatband across the forehead, clipped to the body.
        let (y0, y1) = (fp(vec2(0.0, -0.62)).y, fp(vec2(0.0, -0.44)).y);
        if let (Some((l0, r0)), Some((l1, r1))) = (span_at(&shape, y0), span_at(&shape, y1)) {
            let band = [vec2(l0, y0), vec2(r0, y0), vec2(r1, y1), vec2(l1, y1)];
            fill(
                v,
                &f,
                vec2(0.0, (y0 + y1) / 2.0),
                &band,
                with_alpha(BAND, alpha),
            );
            let ym = (y0 + y1) / 2.0;
            if let Some((lm, rm)) = span_at(&shape, ym) {
                stroke(
                    v,
                    &f,
                    &[vec2(lm, ym), vec2(rm, ym)],
                    0.05,
                    with_alpha(BAND_STRIPE, alpha),
                );
            }
        }
    }

    // Eyes and pupils. The gaze is a field direction, so undo the tilt.
    let gaze = vec2(
        pose.pupil.x * f.cos + pose.pupil.y * f.sin,
        -pose.pupil.x * f.sin + pose.pupil.y * f.cos,
    );
    let open = pose.eye_open.clamp(0.0, 1.45);
    for side in [-1.0f32, 1.0] {
        let c = fp(vec2(0.3 * side, -0.12));
        let (rx, ry) = (0.19 * s, 0.19 * s * open);
        if open < 0.14 {
            stroke(
                v,
                &f,
                &[c - vec2(rx, 0.0), c + vec2(rx, 0.0)],
                0.06 * s,
                ink,
            );
            continue;
        }
        let ec = f.map(c);
        let angle = f.sin.atan2(f.cos);
        v.ellipse(
            ec,
            (rx + 0.045 * s) * r * f.scale.x,
            (ry + 0.045 * s) * r * f.scale.y,
            angle,
            ink,
        );
        v.ellipse(
            ec,
            rx * r * f.scale.x,
            ry * r * f.scale.y,
            angle,
            with_alpha(EYE_WHITE, alpha),
        );
        let pr = 0.085 * s * pose.pupil_size.clamp(0.4, 1.3);
        let room = vec2((rx - pr).max(0.0), (ry - pr).max(0.0));
        let pc = c + vec2(
            gaze.x.clamp(-1.0, 1.0) * room.x,
            gaze.y.clamp(-1.0, 1.0) * room.y,
        );
        v.circle(f.map(pc), pr * r, ink);
        // A catch light keeps the eyes alive at small sizes.
        v.circle(
            f.map(pc + vec2(-0.3, -0.3) * pr),
            pr * 0.35 * r,
            with_alpha(EYE_WHITE, 0.8 * alpha),
        );
    }

    // Brows: positive angle drops the inner ends (a frown).
    let brow_thick = match kind {
        Kind::Angry => 0.14,
        Kind::Ordinary(Family::Block) => 0.13,
        _ => 0.08,
    } * s;
    for side in [-1.0f32, 1.0] {
        // A positive skew raises the left brow (side −1).
        let lift = pose.brow_lift - pose.brow_skew * 0.5 * side;
        let y = -0.42 - lift - (0.19 * (open - 1.0)).max(0.0);
        let inner = fp(vec2(0.12 * side, y + pose.brow_angle * 0.13));
        let outer = fp(vec2(0.46 * side, y - pose.brow_angle * 0.13));
        stroke(v, &f, &[outer, inner], brow_thick, ink);
    }

    draw_mouth(v, &f, pose, &fp, s, alpha);

    // Sweat: a drop that slides down the temple.
    if pose.sweat > 0.05 {
        let slide = pose.accent * 0.25;
        let top = fp(vec2(0.66, -0.55 + slide));
        let a = with_alpha(SWEAT, alpha * pose.sweat.min(1.0));
        let base = f.map(top + vec2(0.0, 0.1 * s));
        v.circle(base, 0.07 * s * r, a);
        v.fan(
            base,
            &[
                f.map(top),
                f.map(top + vec2(0.07 * s, 0.1 * s)),
                f.map(top + vec2(-0.07 * s, 0.1 * s)),
            ],
            a,
        );
    }
    // Steam puffs rising from the head.
    if pose.steam > 0.05 {
        for (i, side) in [-1.0f32, 1.0].into_iter().enumerate() {
            let k = (pose.accent + i as f32 * 0.5).fract();
            let p = centre + vec2(side * (0.55 + 0.25 * k), -0.85 - 0.6 * k) * r;
            let a = with_alpha(STEAM, alpha * pose.steam * (1.0 - k) * 0.85);
            v.circle(p, (0.12 + 0.12 * k) * r, a);
            v.circle(p + vec2(side * 0.1, -0.08) * r, (0.08 + 0.1 * k) * r, a);
        }
    }
}

fn draw_mouth(v: &View, f: &Frame, pose: &Pose, fp: &dyn Fn(Vec2) -> Vec2, s: f32, alpha: f32) {
    const N: usize = 10;
    let ink = with_alpha(INK, alpha);
    let w = pose.mouth_width.max(0.05);
    let y0 = 0.33;
    let upper: Vec<Vec2> = (0..=N)
        .map(|i| {
            let u = i as f32 / N as f32 * 2.0 - 1.0;
            let y = y0 - pose.mouth_curve * 0.35 * (u * u) - pose.mouth_skew * 0.3 * u;
            fp(vec2(u * w, y))
        })
        .collect();
    let open = pose.mouth_open.clamp(0.0, 0.6);
    if open > 0.03 {
        let lower: Vec<Vec2> = upper
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                let u = i as f32 / N as f32 * 2.0 - 1.0;
                p + vec2(0.0, open * 0.5 * (1.0 - u * u) * s)
            })
            .collect();
        for i in 0..N {
            let quad = [upper[i], upper[i + 1], lower[i + 1], lower[i]];
            let c = (quad[0] + quad[1] + quad[2] + quad[3]) / 4.0;
            fill(v, f, c, &quad, ink);
        }
        if pose.teeth {
            // A row of teeth under the upper lip.
            let tooth_h = (open * 0.25).min(0.09) * s;
            for i in 1..N - 1 {
                let quad = [
                    upper[i],
                    upper[i + 1],
                    upper[i + 1] + vec2(0.0, tooth_h),
                    upper[i] + vec2(0.0, tooth_h),
                ];
                let c = (quad[0] + quad[2]) / 2.0;
                fill(v, f, c, &quad, with_alpha(EYE_WHITE, alpha));
            }
        }
        stroke(v, f, &lower, 0.05 * s, ink);
    } else if pose.teeth {
        // Clenched: a white strip with tooth gaps.
        let h = 0.1 * s;
        for i in 1..N - 1 {
            let quad = [
                upper[i] - vec2(0.0, h / 2.0),
                upper[i + 1] - vec2(0.0, h / 2.0),
                upper[i + 1] + vec2(0.0, h / 2.0),
                upper[i] + vec2(0.0, h / 2.0),
            ];
            fill(
                v,
                f,
                upper[i].lerp(upper[i + 1], 0.5),
                &quad,
                with_alpha(EYE_WHITE, alpha),
            );
        }
        for i in (2..N - 1).step_by(2) {
            stroke(
                v,
                f,
                &[upper[i] - vec2(0.0, h / 2.0), upper[i] + vec2(0.0, h / 2.0)],
                0.03 * s,
                ink,
            );
        }
    }
    stroke(v, f, &upper, 0.065 * s, ink);
}

/// The logical collider of a creature, for the lab's debug overlay.
pub fn draw_collider(v: &View, centre: Vec2, r: f32) {
    let c = Color::new(0.2, 1.0, 0.6, 0.9);
    v.circle_lines(centre, r, 1.5, c);
    v.line(centre - vec2(4.0, 0.0), centre + vec2(4.0, 0.0), 1.0, c);
    v.line(centre - vec2(0.0, 4.0), centre + vec2(0.0, 4.0), 1.0, c);
}

/// How much decoration the effects draw. Never changes the rules or an
/// essential cue (fear, runner, silhouette, pressure), only fragments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    #[default]
    Normal,
    Low,
}

impl Quality {
    /// Fragments per removed creature.
    fn pieces(self) -> u64 {
        match self {
            Quality::Normal => 7,
            Quality::Low => 3,
        }
    }
}

/// Fragments flying out of a popped or blasted creature, `t` seconds after
/// it went. Deterministic from `seed`.
/// `time` is (seconds since it went, lifetime).
pub fn draw_debris(
    v: &View,
    centre: Vec2,
    r: f32,
    color: Color,
    time: (f32, f32),
    seed: u64,
    quality: Quality,
) {
    let (t, life) = time;
    if t < 0.0 || t > life {
        return;
    }
    let fade = 1.0 - t / life;
    for i in 0..quality.pieces() {
        let a = hash01(seed, i * 3) * std::f32::consts::TAU;
        let speed = (2.5 + 3.5 * hash01(seed, i * 3 + 1)) * r;
        let size = (0.12 + 0.14 * hash01(seed, i * 3 + 2)) * r * (0.4 + 0.6 * fade);
        let p = centre + vec2(a.cos(), a.sin()) * speed * t + vec2(0.0, 4.0 * r * t * t);
        let spin = a + t * 9.0;
        let tri = regular(3, size, spin)
            .into_iter()
            .map(|q| p + q)
            .collect::<Vec<_>>();
        v.fan(p, &tri, with_alpha(color, fade));
    }
}

/// An explosion `t` seconds in: a short local flash (unless `reduced_flash`)
/// and an expanding ring.
pub fn draw_blast(v: &View, centre: Vec2, r: f32, t: f32, life: f32, reduced_flash: bool) {
    if t < 0.0 || t > life {
        return;
    }
    let x = t / life;
    if !reduced_flash && x < 0.25 {
        let k = 1.0 - x / 0.25;
        v.circle(
            centre,
            r * (1.4 + 1.2 * x),
            Color::new(1.0, 0.95, 0.75, 0.8 * k),
        );
    }
    let ring = Color::new(1.0, 0.55, 0.15, 0.9 * (1.0 - x));
    v.circle_lines(
        centre,
        r * (0.8 + 2.6 * x),
        (0.35 * (1.0 - x) + 0.05) * r,
        ring,
    );
}

/// The launcher pointing along `dir` (unit, field coordinates) with the
/// loaded creature on it.
pub fn draw_launcher(v: &View, at: Vec2, dir: Vec2, r: f32, loaded: Option<(Kind, Pose)>) {
    let base = Color::new(0.22, 0.24, 0.38, 1.0);
    let rim = Color::new(0.45, 0.85, 1.0, 1.0);
    let side = vec2(-dir.y, dir.x);
    let tip = at + dir * 2.4 * r;
    let barrel = [
        at + side * 0.55 * r,
        tip + side * 0.4 * r,
        tip - side * 0.4 * r,
        at - side * 0.55 * r,
    ];
    v.fan((at + tip) / 2.0, &barrel, base);
    v.polyline(&[barrel[0], barrel[1], barrel[2], barrel[3]], 2.0, rim);
    v.circle(at, 1.05 * r, base);
    v.circle_lines(at, 1.05 * r, 2.0, rim);
    if let Some((kind, pose)) = loaded {
        draw_creature(v, at, r, kind, false, &pose, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustamove::emotion::FAMILIES;

    fn kinds() -> Vec<Kind> {
        FAMILIES
            .iter()
            .map(|&f| Kind::Ordinary(f))
            .chain([Kind::Angry])
            .collect()
    }

    /// Visual shapes never reach outside the common collider.
    #[test]
    fn bodies_fit_inside_the_collider() {
        for kind in kinds() {
            let shape = body_shape(kind);
            assert!(shape.len() >= 8);
            for p in &shape {
                assert!(p.length() <= 0.99, "{kind:?} {p}");
            }
            // Wide enough to read: something reaches 0.7 out.
            assert!(shape.iter().any(|p| p.length() > 0.7), "{kind:?}");
        }
    }

    /// The fan fill needs a star-shaped outline around the origin: going
    /// round, the angle keeps turning the same way.
    #[test]
    fn bodies_are_star_shaped_around_the_centre() {
        for kind in kinds() {
            let shape = body_shape(kind);
            let mut sign = 0.0f32;
            for (i, &a) in shape.iter().enumerate() {
                let b = shape[(i + 1) % shape.len()];
                let cross = a.perp_dot(b);
                if cross.abs() < 1e-6 {
                    continue;
                }
                if sign == 0.0 {
                    sign = cross.signum();
                }
                assert_eq!(cross.signum(), sign, "{kind:?} at {i}");
            }
        }
    }

    #[test]
    fn silhouettes_differ() {
        // The six families and Angry each have their own outline.
        let shapes: Vec<Vec<Vec2>> = kinds().into_iter().map(body_shape).collect();
        for (i, a) in shapes.iter().enumerate() {
            for b in &shapes[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    #[test]
    fn span_finds_the_body_width() {
        let square = vec![
            vec2(-1.0, -1.0),
            vec2(1.0, -1.0),
            vec2(1.0, 1.0),
            vec2(-1.0, 1.0),
        ];
        assert_eq!(span_at(&square, 0.0), Some((-1.0, 1.0)));
        assert_eq!(span_at(&square, 2.0), None);
    }
}
