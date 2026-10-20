//! Aiming input (rule R02), shared by the classic round and the shooting
//! range: the mouse sets the aim towards the pointer, held Left/Right (A/D)
//! rotate it, and the keyboard keeps the aim until the pointer really moves.
//!
//! `Intent` collects one `handle` call's input; `Aim::update` applies it in
//! a fixed step.

use macroquad::math::Vec2;

use super::geometry::LAUNCHER;
use super::shot::{AIM_MAX, AIM_MIN};
use crate::game::FIXED_DT;
use crate::ui::{Input, Key};

/// Keyboard aim speed in degrees per second (05_CONFIGURATION).
pub const KEY_AIM_SPEED: f64 = 100.0;

/// Input gathered by `handle`, used by the next `update`.
#[derive(Clone, Debug, Default)]
pub struct Intent {
    pub pointer: Option<Vec2>,
    /// A fire press (Space, or a click inside the field).
    pub fire: bool,
    pub keys: Vec<Key>,
    pub held: Vec<Key>,
}

impl Intent {
    /// Adds a `handle` call's input. Presses accumulate until `take`.
    pub fn add(&mut self, input: &Input) {
        self.pointer = input.pointer;
        self.held.clone_from(&input.held);
        self.keys.extend(input.keys.iter().copied());
        // A click in the field or Space fires; outside the field (pointer
        // None) a click does nothing.
        if input.key(Key::Space) || (input.left_pressed && input.pointer.is_some()) {
            self.fire = true;
        }
    }

    /// The gathered input; the lasting state (pointer, held keys) stays for
    /// later steps, so a press is used exactly once.
    pub fn take(&mut self) -> Intent {
        let all = std::mem::take(self);
        self.pointer = all.pointer;
        self.held.clone_from(&all.held);
        all
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Owner {
    Mouse,
    Keyboard,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Aim {
    /// Degrees counter-clockwise from screen-right, up positive.
    pub angle: f64,
    owner: Owner,
    last_pointer: Option<Vec2>,
}

impl Default for Aim {
    fn default() -> Self {
        Aim {
            angle: 90.0,
            owner: Owner::Keyboard,
            last_pointer: None,
        }
    }
}

impl Aim {
    /// One fixed step of aiming.
    pub fn update(&mut self, intent: &Intent) {
        let left = intent.held.iter().any(|k| matches!(k, Key::Left | Key::A));
        let right = intent.held.iter().any(|k| matches!(k, Key::Right | Key::D));
        if left != right {
            self.owner = Owner::Keyboard;
            let step = KEY_AIM_SPEED * FIXED_DT as f64;
            self.angle = (self.angle + if left { step } else { -step }).clamp(AIM_MIN, AIM_MAX);
        } else if intent.pointer.is_some() && intent.pointer != self.last_pointer {
            // Only real pointer movement takes the aim back.
            self.owner = Owner::Mouse;
        }
        if intent.pointer.is_some() {
            self.last_pointer = intent.pointer;
        }
        if self.owner == Owner::Mouse {
            if let Some(p) = intent.pointer {
                self.angle = aim_towards(p, self.angle);
            }
        }
    }

    /// Sets the angle as if by keyboard (tests and scripted checks).
    #[cfg(test)]
    pub fn set(&mut self, angle: f64) {
        self.angle = angle.clamp(AIM_MIN, AIM_MAX);
        self.owner = Owner::Keyboard;
    }
}

/// The aim angle towards a pointer, clamped to the legal range: below the
/// launcher it goes to the nearer end; at the launcher itself it keeps
/// `current`.
pub fn aim_towards(pointer: Vec2, current: f64) -> f64 {
    let dx = pointer.x as f64 - LAUNCHER.x;
    let dy = LAUNCHER.y - pointer.y as f64;
    if dx == 0.0 && dy == 0.0 {
        return current;
    }
    let a = dy.atan2(dx).to_degrees();
    if a < AIM_MIN {
        if a >= -90.0 {
            AIM_MIN
        } else {
            AIM_MAX
        }
    } else {
        a.min(AIM_MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::math::vec2;

    fn step(aim: &mut Aim, pointer: Option<Vec2>, held: &[Key]) {
        let intent = Intent {
            pointer,
            held: held.to_vec(),
            ..Default::default()
        };
        aim.update(&intent);
    }

    /// G13: below the launcher the aim goes to the nearer end; at the
    /// launcher it stays.
    #[test]
    fn pointer_aim_is_clamped() {
        let l = vec2(LAUNCHER.x as f32, LAUNCHER.y as f32);
        assert!((aim_towards(l + vec2(0.0, -100.0), 40.0) - 90.0).abs() < 1e-9);
        assert_eq!(aim_towards(l + vec2(100.0, 50.0), 40.0), AIM_MIN);
        assert_eq!(aim_towards(l + vec2(-100.0, 50.0), 40.0), AIM_MAX);
        assert_eq!(aim_towards(l + vec2(100.0, 0.0), 40.0), AIM_MIN);
        assert_eq!(aim_towards(l, 40.0), 40.0);
        assert_eq!(
            aim_towards(l + vec2(0.0, 80.0), 40.0),
            AIM_MIN,
            "straight down: right end"
        );
    }

    /// G14: holding a key aims; an unchanged pointer does not take the aim
    /// back; moving the pointer does.
    #[test]
    fn keyboard_owns_the_aim_until_the_pointer_moves() {
        let mut aim = Aim::default();
        let p = vec2(400.0, 500.0);
        step(&mut aim, Some(p), &[]);
        let mouse_angle = aim.angle;
        assert!(mouse_angle < 90.0);
        for _ in 0..60 {
            step(&mut aim, Some(p), &[Key::Left]);
        }
        // 100°/s for 0.5 s (FIXED_DT is an f32, hence the tolerance).
        assert!(
            (aim.angle - (mouse_angle + 50.0)).abs() < 1e-4,
            "{}",
            aim.angle
        );
        let kept = aim.angle;
        for _ in 0..10 {
            step(&mut aim, Some(p), &[]);
        }
        assert_eq!(
            aim.angle, kept,
            "an unmoved pointer leaves the keyboard aim"
        );
        step(&mut aim, Some(p + vec2(1.0, 0.0)), &[]);
        assert!(
            (aim.angle - mouse_angle).abs() < 1.0,
            "moving the mouse takes over"
        );
    }

    #[test]
    fn keyboard_aim_stays_in_range() {
        let mut aim = Aim::default();
        for _ in 0..1000 {
            step(&mut aim, None, &[Key::D]);
        }
        assert_eq!(aim.angle, AIM_MIN);
        for _ in 0..1000 {
            step(&mut aim, None, &[Key::A]);
        }
        assert_eq!(aim.angle, AIM_MAX);
    }

    #[test]
    fn presses_are_taken_once_and_state_stays() {
        let mut i = Intent::default();
        i.add(&Input {
            keys: vec![Key::Space],
            held: vec![Key::Left],
            pointer: Some(vec2(1.0, 2.0)),
            ..Default::default()
        });
        let first = i.take();
        assert!(first.fire && first.keys == vec![Key::Space]);
        let second = i.take();
        assert!(!second.fire && second.keys.is_empty());
        assert_eq!(
            (second.pointer, second.held),
            (Some(vec2(1.0, 2.0)), vec![Key::Left])
        );
    }
}
