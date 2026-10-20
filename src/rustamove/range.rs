//! The shooting range (milestone M2): a real board, aiming without a line,
//! fear on the predicted first creature, and shots that fly along exactly
//! that plan and attach locally. No matching, falls, blasts or runners yet
//! (M3–M5): a direct Angry hit and a blocked board are only reported.
//!
//! `handle` stores intent; `update` does everything, one fixed step at a
//! time, so a press arrives exactly once however frames and steps line up.

use macroquad::math::vec2;

use super::aim::{Aim, Intent};
use super::board::{Board, CreatureId};
use super::emotion::{self, Cues, Family, Kind, Mood, RunnerCue, FAMILIES};
use super::geometry::{Cell, Layout, DANGER_Y, LAUNCHER};
use super::render;
use super::rng::Rng;
use super::sfx;
use super::shot::{self, Flight, Hit, ShotPlan, SPEED_DIAMETERS};
use crate::audio::Sfx;
use crate::game::FIXED_DT;
use crate::t;
use crate::ui::{Input, Key};
use crate::view::{with_alpha, View, ACCENT, FIELD_W, NEON, TEXT};

/// Board sizes to try: (columns, zoom). Up/Down cycles them.
pub const PRESETS: [(u16, f64); 6] = [
    (8, 1.0),
    (6, 1.0),
    (12, 1.0),
    (18, 1.0),
    (18, 0.75),
    (6, 0.75),
];
/// Test boards. Z/X cycles them.
pub const FIXTURES: usize = 4;
/// Pause after an attachment before aiming again (seconds).
const SETTLE: f64 = 0.15;
/// How long a message stays (seconds).
const MESSAGE: f64 = 1.6;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Aiming,
    Flight,
    /// Until this tick.
    Settling(u64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Message {
    AngryHit,
    Blocked,
    /// The board reached the danger line and was set up again.
    Danger,
}

pub struct Range {
    pub board: Board,
    preset: usize,
    fixture: usize,
    level: u32,
    dev: bool,
    show_colliders: bool,
    show_path: bool,
    pub reduced_motion: bool,
    pub aim: Aim,
    intent: Intent,
    phase: Phase,
    /// The current prediction while aiming.
    pub plan: Option<ShotPlan>,
    pub flight: Option<Flight>,
    loaded: Family,
    next: Family,
    rng: Rng,
    pub ticks: u64,
    pub shots: u32,
    message: Option<(Message, u64)>,
    sounds: Vec<Sfx>,
}

/// The board for `fixture` on `layout`, deterministic from `seed`.
pub fn fixture_board(layout: Layout, fixture: usize, seed: u64) -> Board {
    let mut b = Board::new(layout);
    let mut rng = Rng::new(seed);
    let n = layout.columns;
    let kind = |rng: &mut Rng| -> (Kind, bool) {
        match rng.below(100) {
            0..=4 => (Kind::Angry, false),
            k => (Kind::Ordinary(FAMILIES[rng.below(6) as usize]), k < 13),
        }
    };
    // Never place anything that reaches the danger line.
    let put = |b: &mut Board, rng: &mut Rng, cell: Cell| {
        let (k, runner) = kind(rng);
        if b.layout.centre(cell).y + b.layout.r < DANGER_Y {
            b.place(cell, k, runner);
        }
    };
    match fixture {
        // Mixed: two full rows and a ragged fringe.
        0 => {
            for row in 0..6u16 {
                for col in 0..layout.row_len(row) {
                    if row < 2 || rng.below(100) < 70 - 10 * row as u32 {
                        put(&mut b, &mut rng, Cell::new(row, col));
                    }
                }
            }
        }
        // Gaps in the top row, clusters hanging from the anchors.
        1 => {
            for row in 0..5u16 {
                for col in 0..layout.row_len(row) {
                    let gap = row == 0 && col % 3 == 1;
                    if !gap && (row == 0 || rng.below(100) < 60) {
                        put(&mut b, &mut rng, Cell::new(row, col));
                    }
                }
            }
        }
        // A curtain down the middle: the sides need bank shots.
        2 => {
            for row in 0..3u16 {
                for col in 0..layout.row_len(row) {
                    put(&mut b, &mut rng, Cell::new(row, col));
                }
            }
            let mid = n / 2;
            for row in 3..12u16 {
                let cols: &[u16] = if row % 2 == 0 {
                    &[mid - 1, mid]
                } else {
                    &[mid - 1]
                };
                for &col in cols {
                    put(&mut b, &mut rng, Cell::new(row, col));
                }
            }
        }
        // A pocket: a full block with a cavity opening downwards.
        _ => {
            let mid = n / 2;
            for row in 0..7u16 {
                for col in 0..layout.row_len(row) {
                    let hole = row >= 4 && (col == mid - 1 || (row % 2 == 0 && col == mid));
                    if !hole {
                        put(&mut b, &mut rng, Cell::new(row, col));
                    }
                }
            }
        }
    }
    // Keep only what hangs from the ceiling.
    let anchored = b.anchored();
    let loose: Vec<Cell> = b
        .occupied()
        .map(|(c, _)| c)
        .filter(|c| !anchored.contains(c))
        .collect();
    for c in loose {
        b.remove(c);
    }
    b
}

impl Range {
    pub fn new(level: u32, dev: bool) -> Range {
        let mut r = Range {
            board: Board::new(Layout::new(8, 1.0).unwrap()),
            preset: 0,
            fixture: 0,
            level,
            dev,
            show_colliders: false,
            show_path: false,
            reduced_motion: false,
            aim: Aim::default(),
            intent: Intent::default(),
            phase: Phase::Aiming,
            plan: None,
            flight: None,
            loaded: Family::Round,
            next: Family::Round,
            rng: Rng::new(0),
            ticks: 0,
            shots: 0,
            message: None,
            sounds: Vec::new(),
        };
        r.rebuild();
        r
    }

    /// A fresh board for the current preset and fixture.
    fn rebuild(&mut self) {
        let (columns, zoom) = PRESETS[self.preset];
        let layout = Layout::new(columns, zoom).expect("presets are valid");
        let seed = self.level as u64 * 1_000_003 + self.fixture as u64 * 101 + self.preset as u64;
        self.board = fixture_board(layout, self.fixture, seed);
        self.rng = Rng::new(seed ^ 0xA77E);
        self.loaded = self.draw_family();
        self.next = self.draw_family();
        self.plan = None;
        self.flight = None;
        self.phase = Phase::Aiming;
        self.shots = 0;
        self.message = None;
    }

    /// An ammunition family from those still on the board (R13), in stable
    /// order; any family when none is left.
    fn draw_family(&mut self) -> Family {
        let mut present: Vec<Family> = self
            .board
            .occupied()
            .filter_map(|(_, c)| match c.kind {
                Kind::Ordinary(f) => Some(f),
                Kind::Angry => None,
            })
            .collect();
        present.sort();
        present.dedup();
        if present.is_empty() {
            present = FAMILIES.to_vec();
        }
        present[self.rng.below(present.len() as u32) as usize]
    }

    pub fn handle(&mut self, input: &Input) {
        self.intent.add(input);
    }

    fn seconds(&self, s: f64) -> u64 {
        (s / FIXED_DT as f64).round() as u64
    }

    pub fn update(&mut self) {
        self.ticks += 1;
        let intent = self.intent.take();

        for &key in &intent.keys {
            self.key(key);
        }
        self.aim.update(&intent);

        match self.phase {
            Phase::Aiming => {
                if intent.fire {
                    self.fire();
                } else {
                    self.refresh_plan();
                }
            }
            Phase::Flight => self.fly(),
            Phase::Settling(until) => {
                if self.ticks >= until {
                    self.phase = Phase::Aiming;
                    self.refresh_plan();
                }
            }
        }
        if self.message.is_some_and(|(_, until)| self.ticks >= until) {
            self.message = None;
        }
        self.update_moods();
    }

    fn key(&mut self, key: Key) {
        // Board changes only while nothing flies.
        let idle = self.phase == Phase::Aiming;
        match key {
            Key::Z | Key::X if idle => {
                self.fixture = if key == Key::X {
                    (self.fixture + 1) % FIXTURES
                } else {
                    (self.fixture + FIXTURES - 1) % FIXTURES
                };
                self.rebuild();
            }
            Key::Up | Key::Down if idle => {
                let n = PRESETS.len();
                self.preset = if key == Key::Up {
                    (self.preset + 1) % n
                } else {
                    (self.preset + n - 1) % n
                };
                self.rebuild();
            }
            Key::S if self.dev => self.show_colliders = !self.show_colliders,
            Key::W if self.dev => self.show_path = !self.show_path,
            _ => {}
        }
    }

    fn refresh_plan(&mut self) {
        let current = self
            .plan
            .as_ref()
            .is_some_and(|p| p.angle == self.aim.angle && p.revision == self.board.revision());
        if !current {
            self.plan = shot::plan(&self.board, self.aim.angle).ok();
        }
    }

    fn fire(&mut self) {
        // Always against the latest board (G12).
        self.plan = None;
        self.refresh_plan();
        if let Some(plan) = self.plan.clone() {
            self.flight = Some(Flight::new(plan));
            self.phase = Phase::Flight;
            self.shots += 1;
            self.sounds.push(sfx::LAUNCH);
        }
    }

    fn fly(&mut self) {
        let Some(flight) = self.flight.as_mut() else {
            self.phase = Phase::Aiming;
            return;
        };
        let speed = SPEED_DIAMETERS * 2.0 * self.board.layout.r;
        let (bounces, arrived) = flight.advance(speed * FIXED_DT as f64);
        if bounces > 0 {
            self.sounds.push(sfx::BOUNCE);
        }
        if !arrived {
            return;
        }
        let plan = flight.plan.clone();
        debug_assert_eq!(
            plan.revision,
            self.board.revision(),
            "occupancy is frozen in flight"
        );
        self.flight = None;
        match (plan.attachment, plan.hit) {
            (Some(cell), _) => {
                self.board.place(cell, Kind::Ordinary(self.loaded), false);
                self.sounds.push(sfx::ATTACH);
                if self.board.reaches_danger() {
                    // A loss from M3; here the range simply starts over.
                    self.rebuild();
                    self.message = Some((Message::Danger, self.ticks + self.seconds(MESSAGE)));
                }
            }
            (
                None,
                Hit::Creature {
                    kind: Kind::Angry, ..
                },
            ) => {
                self.message = Some((Message::AngryHit, self.ticks + self.seconds(MESSAGE)));
            }
            (None, _) => {
                self.message = Some((Message::Blocked, self.ticks + self.seconds(MESSAGE)));
            }
        }
        self.loaded = self.next;
        self.next = self.draw_family();
        self.phase = Phase::Settling(self.ticks + self.seconds(SETTLE));
        self.plan = None;
    }

    /// The creature that shows fear: the plan's first hit, while aiming
    /// and in flight.
    pub fn target(&self) -> Option<CreatureId> {
        match self.phase {
            Phase::Aiming => self.plan.as_ref().and_then(ShotPlan::first_hit),
            Phase::Flight => self.flight.as_ref().and_then(|f| f.plan.first_hit()),
            Phase::Settling(_) => None,
        }
    }

    #[allow(dead_code)] // Used by tests now, by the rules from M3.
    pub fn is_aiming(&self) -> bool {
        self.phase == Phase::Aiming
    }

    pub fn message(&self) -> Option<Message> {
        self.message.map(|(m, _)| m)
    }

    fn cues(&self) -> Vec<(CreatureId, Cues)> {
        let l = self.board.layout;
        let target = self.target();
        self.board
            .occupied()
            .map(|(cell, c)| {
                let here = l.centre(cell);
                let angry_dir = l
                    .neighbours(cell)
                    .into_iter()
                    .filter_map(|n| {
                        self.board
                            .get(n)
                            .filter(|a| a.kind == Kind::Angry)
                            .map(|a| (a.id, n))
                    })
                    .min()
                    .map(|(_, n)| {
                        let d = (l.centre(n) - here).normalize();
                        vec2(d.x as f32, d.y as f32)
                    });
                let to_launcher = (LAUNCHER - here).normalize();
                (
                    c.id,
                    Cues {
                        nervous: angry_dir.is_some(),
                        aimed: Some(c.id) == target,
                        angry_dir,
                        launcher_dir: vec2(to_launcher.x as f32, to_launcher.y as f32),
                        runner: if c.runner {
                            RunnerCue::Blocked
                        } else {
                            RunnerCue::None
                        },
                    },
                )
            })
            .collect()
    }

    fn update_moods(&mut self) {
        let cues = self.cues();
        for ((_, c), (_, cue)) in self.board.occupied_mut().zip(cues) {
            c.mood.step(c.kind, &cue, FIXED_DT);
        }
    }

    pub fn take_sounds(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sounds)
    }

    fn time(&self) -> f64 {
        self.ticks as f64 * FIXED_DT as f64
    }

    pub fn draw(&self, v: &View) {
        let l = self.board.layout;
        let t = self.time();
        let r = l.r as f32;
        draw_frame(v, &l);
        let cues = self.cues();
        for ((cell, c), (_, cue)) in self.board.occupied().zip(cues) {
            let pose = emotion::pose(c.kind, &cue, &c.mood, t, c.seed(), self.reduced_motion);
            let p = l.centre(cell);
            render::draw_creature(
                v,
                vec2(p.x as f32, p.y as f32),
                r,
                c.kind,
                c.runner,
                &pose,
                1.0,
            );
        }
        // Development overlays: never in normal play.
        if self.dev && self.show_colliders {
            for (cell, _) in self.board.occupied() {
                let p = l.centre(cell);
                render::draw_collider(v, vec2(p.x as f32, p.y as f32), r);
            }
        }
        if self.dev && self.show_path {
            let plan = self.flight.as_ref().map(|f| &f.plan).or(self.plan.as_ref());
            if let Some(plan) = plan {
                let c = macroquad::color::Color::new(1.0, 0.3, 0.9, 0.8);
                for s in &plan.segments {
                    v.line(
                        vec2(s.from.x as f32, s.from.y as f32),
                        vec2(s.to.x as f32, s.to.y as f32),
                        1.5,
                        c,
                    );
                }
                if let Some(cell) = plan.attachment {
                    let p = l.centre(cell);
                    v.circle_lines(vec2(p.x as f32, p.y as f32), r, 1.5, c);
                }
            }
        }

        let at = vec2(LAUNCHER.x as f32, LAUNCHER.y as f32);
        let d = shot::direction(self.aim.angle);
        let calm = Cues::default();
        let loaded = Kind::Ordinary(self.loaded);
        let face = emotion::pose(loaded, &calm, &Mood::default(), t, 1, self.reduced_motion);
        let in_flight = self.flight.is_some();
        render::draw_launcher(
            v,
            at,
            vec2(d.x as f32, d.y as f32),
            r,
            (!in_flight).then_some((loaded, face)),
        );
        if let Some(f) = &self.flight {
            let p = f.position();
            let mut pose = face;
            pose.eye_open = 1.3;
            pose.mouth_open = 0.3;
            render::draw_creature(
                v,
                vec2(p.x as f32, p.y as f32),
                r,
                loaded,
                false,
                &pose,
                1.0,
            );
        }
        // The next creature, small, beside the launcher.
        let next = Kind::Ordinary(self.next);
        let next_face = emotion::pose(next, &calm, &Mood::default(), t, 2, self.reduced_motion);
        let nr = (r * 0.7).min(18.0);
        let next_at = at + vec2(-80.0, 6.0);
        render::draw_creature(v, next_at, nr, next, false, &next_face, 1.0);
        let label = with_alpha(TEXT, 0.6);
        v.text_centered(
            t!("range.next"),
            next_at.x,
            next_at.y - nr - 8.0,
            13.0,
            label,
        );

        self.draw_hud(v);
    }

    fn draw_hud(&self, v: &View) {
        let name = match self.fixture {
            0 => t!("range.f.mixed"),
            1 => t!("range.f.gaps"),
            2 => t!("range.f.curtain"),
            _ => t!("range.f.pocket"),
        };
        let (columns, zoom) = PRESETS[self.preset];
        v.text_centered(
            &t!("range.title", name = name),
            FIELD_W / 2.0,
            34.0,
            22.0,
            NEON,
        );
        let detail = t!(
            "range.detail",
            columns = columns,
            zoom = format!("{:.0}", zoom * 100.0),
            shots = self.shots
        );
        v.text_centered(&detail, FIELD_W / 2.0, 60.0, 17.0, ACCENT);
        if let Some(m) = self.message() {
            let text = match m {
                Message::AngryHit => t!("range.m.angry"),
                Message::Blocked => t!("range.m.blocked"),
                Message::Danger => t!("range.m.danger"),
            };
            v.text_centered(text, FIELD_W / 2.0, 700.0, 22.0, ACCENT);
        }
        let help = with_alpha(TEXT, 0.65);
        let y = v.text_wrapped_centered(t!("range.help"), 912.0, 14.0, help);
        if self.dev {
            v.text_centered(
                t!("range.dev"),
                FIELD_W / 2.0,
                y,
                13.0,
                with_alpha(ACCENT, 0.7),
            );
        }
    }
}

/// Board edges, ceiling and danger line.
pub fn draw_frame(v: &View, l: &Layout) {
    let edge = with_alpha(NEON, 0.35);
    let (left, right, top) = (l.left as f32, l.right as f32, l.ceiling_y as f32);
    let danger = DANGER_Y as f32;
    v.rect(
        left,
        top,
        right - left,
        danger - top,
        macroquad::color::Color::new(0.0, 0.0, 0.0, 0.18),
    );
    v.line(vec2(left, top), vec2(right, top), 3.0, edge);
    v.line(vec2(left, top), vec2(left, danger), 2.0, edge);
    v.line(vec2(right, top), vec2(right, danger), 2.0, edge);
    let warn = macroquad::color::Color::new(1.0, 0.35, 0.35, 0.45);
    let mut x = left;
    while x < right {
        v.line(
            vec2(x, danger),
            vec2((x + 10.0).min(right), danger),
            2.0,
            warn,
        );
        x += 18.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustamove::shot::AIM_MIN;

    fn step(r: &mut Range, input: Input) {
        r.handle(&input);
        r.update();
    }

    fn fire() -> Input {
        Input {
            keys: vec![Key::Space],
            ..Default::default()
        }
    }

    /// Runs until the shot has landed and aiming is possible again.
    fn settle(r: &mut Range) {
        for _ in 0..2000 {
            if r.is_aiming() {
                return;
            }
            r.update();
        }
        panic!("never settled");
    }

    /// The M2 gate: on an unchanged board, the creature that is afraid
    /// while aiming is the one the fired shot strikes, at every preset and
    /// fixture, straight and after bounces.
    #[test]
    fn the_afraid_creature_is_the_one_hit() {
        let mut checked = 0;
        let mut bounced = 0;
        for preset in 0..PRESETS.len() {
            for fixture in 0..FIXTURES {
                let mut r = Range::new(1, false);
                r.preset = preset;
                r.fixture = fixture;
                r.rebuild();
                let mut rng = Rng::new((preset * 10 + fixture) as u64);
                for _ in 0..8 {
                    let angle = AIM_MIN + 160.0 * (rng.below(10_000) as f64 / 1e4);
                    r.aim.set(angle);
                    step(&mut r, Input::default());
                    let afraid = r.target();
                    let plan = r.plan.clone().unwrap();
                    let board_before = r.board.revision();
                    step(&mut r, fire());
                    let flown = r.flight.as_ref().unwrap().plan.clone();
                    assert_eq!(flown, plan, "the shot flies the plan that made it afraid");
                    assert_eq!(flown.first_hit(), afraid);
                    assert_eq!(r.target(), afraid, "the target stays afraid in flight");
                    // Every step of the flight stays clear of every creature
                    // (no tunnelling), and the board does not change.
                    let l = r.board.layout;
                    while let Some(f) = &r.flight {
                        let p = f.position();
                        for (cell, _) in r.board.occupied() {
                            let d = l.centre(cell).distance(p);
                            assert!(
                                d >= 2.0 * l.r - 1e-6,
                                "preset {preset} fixture {fixture} angle {angle} cell {cell:?} d {d} r {} travelled {} of {} hit {:?} segs {:?}",
                                l.r, f.travelled, f.plan.travel, f.plan.hit, f.plan.segments
                            );
                        }
                        assert_eq!(r.board.revision(), board_before, "frozen in flight");
                        r.update();
                    }
                    // It stopped touching the creature that was afraid.
                    if let Hit::Creature { cell, .. } = flown.hit {
                        let end = flown.segments.last().unwrap().to;
                        assert!((l.centre(cell).distance(end) - 2.0 * l.r).abs() < 1e-6);
                    }
                    checked += 1;
                    bounced += (plan.bounces() > 0) as u32;
                    settle(&mut r);
                }
            }
        }
        assert!(checked >= 150 && bounced > 20, "{checked} {bounced}");
    }

    /// F01: a press during flight is dropped, not kept for later.
    #[test]
    fn presses_during_flight_are_discarded() {
        let mut r = Range::new(1, false);
        step(&mut r, fire());
        assert_eq!(r.shots, 1);
        step(&mut r, fire());
        settle(&mut r);
        for _ in 0..30 {
            r.update();
        }
        assert_eq!(r.shots, 1, "no delayed autofire");
    }

    /// F02/F03: one press, several steps: one shot.
    #[test]
    fn one_press_is_one_shot_whatever_the_step_count() {
        let mut r = Range::new(1, false);
        r.handle(&fire());
        r.handle(&Input::default());
        for _ in 0..5 {
            r.update();
        }
        assert_eq!(r.shots, 1);
        settle(&mut r);
        assert_eq!(r.shots, 1);
    }

    #[test]
    fn clicks_outside_the_field_do_not_fire() {
        let mut r = Range::new(1, false);
        let click = Input {
            pointer: None,
            left_pressed: true,
            left_down: true,
            ..Default::default()
        };
        step(&mut r, click);
        assert_eq!(r.shots, 0);
        let inside = Input {
            pointer: Some(vec2(270.0, 300.0)),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        };
        step(&mut r, inside);
        assert_eq!(r.shots, 1);
    }

    /// G12: a board change between aiming and firing is planned anew.
    #[test]
    fn fire_replans_against_the_latest_board() {
        let mut r = Range::new(1, false);
        r.aim.set(90.0);
        step(&mut r, Input::default());
        let old = r.plan.clone().unwrap();
        // Something appears in the way (as an escape or fall would do).
        let cell = r
            .board
            .layout
            .nearest_cell(shot::direction(90.0) * 300.0 + LAUNCHER)
            .unwrap();
        r.board.place(cell, Kind::Ordinary(Family::Oval), false);
        step(&mut r, fire());
        let f = r.flight.as_ref().unwrap();
        assert_eq!(f.plan.revision, r.board.revision());
        assert_ne!(f.plan.hit, old.hit);
    }

    #[test]
    fn fixtures_hang_from_the_ceiling_and_fit() {
        for (columns, zoom) in PRESETS {
            for fixture in 0..FIXTURES {
                let b = fixture_board(Layout::new(columns, zoom).unwrap(), fixture, 3);
                assert!(b.count() > 5);
                assert_eq!(b.anchored().len(), b.count());
                for (cell, _) in b.occupied() {
                    let y = b.layout.centre(cell).y + b.layout.r;
                    assert!(y < DANGER_Y, "{columns} {fixture} {cell:?}");
                }
            }
        }
    }

    #[test]
    fn same_seed_same_board_and_ammo() {
        let a = Range::new(3, false);
        let b = Range::new(3, false);
        let cells = |r: &Range| {
            r.board
                .occupied()
                .map(|(c, x)| (c, x.kind))
                .collect::<Vec<_>>()
        };
        assert_eq!(cells(&a), cells(&b));
        assert_eq!((a.loaded, a.next), (b.loaded, b.next));
    }

    #[test]
    fn overlays_need_development_mode() {
        let mut r = Range::new(1, false);
        step(
            &mut r,
            Input {
                keys: vec![Key::W, Key::S],
                ..Default::default()
            },
        );
        assert!(!r.show_path && !r.show_colliders);
        let mut r = Range::new(1, true);
        step(
            &mut r,
            Input {
                keys: vec![Key::W, Key::S],
                ..Default::default()
            },
        );
        assert!(r.show_path && r.show_colliders);
    }
}
