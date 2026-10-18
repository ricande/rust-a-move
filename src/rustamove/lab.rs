//! The character animation lab (milestone M1): selectable scenarios that
//! show every expression from docs/game/04_CHARACTERS_ANIMATION.md with the
//! same emotion and drawing code the game will use.
//!
//! The lab is a presentation test bench, not the game: aim targets are set
//! by the scenario ("forced aim") rather than by shot prediction, and
//! blasts/falls are worked out from the hex graph only to stage them. Runner
//! routes do use the real corridor check in `geometry`.
//!
//! Time: lab animation time advances only in `update`, in quarter ticks so
//! slow motion (a quarter of normal speed) stays exact.

use macroquad::color::Color;
use macroquad::math::{vec2, Vec2};

use super::emotion::{
    self, blast_smile, fall_pose, hash01, pop_pose, Cues, Family, Kind, Mood, Pose, RunnerCue,
};
use super::geometry::{Cell, Layout, LAUNCHER};
use super::render;
use crate::t;
use crate::ui::Key;
use crate::view::{with_alpha, View, ACCENT, FIELD_W, NEON, TEXT};

pub const SCENARIOS: usize = 8;

/// Quarter ticks per second (4 per 1/120 s fixed step).
const QUARTERS_PER_SECOND: f64 = 480.0;
/// Normal runner struggle and the panic delay, in seconds of lab time
/// (docs/game/05_CONFIGURATION_CONTENT.md: 90 and 0 ticks).
const STRUGGLE: f64 = 0.75;
const ESCAPE_PRESENT: f64 = 0.25;
/// Blast staging: Angry smiles this long before its burst; chained waves
/// follow each other by `WAVE_GAP`.
const SMILE: f64 = 0.06;
const WAVE_GAP: f64 = 0.1;
const BLAST_LIFE: f64 = 0.35;
const POP_LIFE: f64 = 0.2;
const FALL_MAX: f64 = 0.8;
const ENDING_HOLD: f64 = 0.8;

/// Cast sizes: collider radius for 6, 8, 12 and 18 columns at zoom 1, then
/// 18 columns at zoom 0.75 (the smallest), plus a big close-up.
const CAST_RADII: [f32; 6] = [60.0, 40.0, 30.0, 20.0, 13.333, 10.0];
/// Widths scenario: (columns, zoom).
const WIDTHS: [(u16, f64); 3] = [(6, 1.0), (18, 1.0), (18, 0.75)];

#[derive(Clone, Debug)]
struct Critter {
    id: u64,
    kind: Kind,
    runner: bool,
    cell: Option<Cell>,
    pos: Vec2,
    r: f32,
    mood: Mood,
    cues: Cues,
    /// Seed for idle phases; the "Aim at Angry" scenario gives two angry
    /// creatures the same seed so their motion can be compared.
    seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Act {
    /// Removed in a match: squash, then fragments.
    Pop,
    /// Removed by a blast at `start`; Angry smiles from `smile`.
    Blasted { smile: f64, angry_source: bool },
    /// Unsupported: waits in place until `start`, then falls.
    Fall { vx: f32 },
    /// A runner leaving sideways.
    Escape { dir: f32 },
}

#[derive(Clone, Debug)]
struct Actor {
    kind: Kind,
    runner: bool,
    from: Vec2,
    r: f32,
    start: f64,
    act: Act,
    seed: u64,
}

impl Actor {
    fn end(&self) -> f64 {
        self.start
            + match self.act {
                Act::Pop => POP_LIFE + 0.3,
                Act::Blasted { .. } => BLAST_LIFE,
                Act::Fall { .. } => FALL_MAX,
                Act::Escape { .. } => ESCAPE_PRESENT + 0.35,
            }
    }
}

pub struct Lab {
    pub scenario: usize,
    param: usize,
    slow: bool,
    pub collider: bool,
    pub reduced_motion: bool,
    quarters: u64,
    critters: Vec<Critter>,
    actors: Vec<Actor>,
    layout: Option<Layout>,
    /// Aimed creature id (forced by the scenario).
    aim: Option<u64>,
    next_id: u64,
    /// When the current runner's route opened (lab time).
    struggle_since: Option<(f64, i8)>,
    /// The scenario restarts at this time (after an escape or a blast).
    reset_at: Option<f64>,
    /// A blast sequence is running; its end of presentation.
    ending: Option<(f64, f64)>,
}

impl Default for Lab {
    fn default() -> Self {
        Lab::new()
    }
}

impl Lab {
    pub fn new() -> Lab {
        let mut lab = Lab {
            scenario: 0,
            param: 1,
            slow: false,
            collider: false,
            reduced_motion: false,
            quarters: 0,
            critters: Vec::new(),
            actors: Vec::new(),
            layout: None,
            aim: None,
            next_id: 1,
            struggle_since: None,
            reset_at: None,
            ending: None,
        };
        lab.setup();
        lab
    }

    /// Lab animation time in seconds.
    pub fn time(&self) -> f64 {
        self.quarters as f64 / QUARTERS_PER_SECOND
    }

    fn param_count(&self) -> usize {
        match self.scenario {
            0 => CAST_RADII.len(),
            1 => emotion::FAMILIES.len(),
            2 => 7,
            3 => 1,
            4 => 4,
            5 => 1,
            6 => 3,
            _ => WIDTHS.len(),
        }
    }

    /// Applies one key press. Returns false for keys the lab does not use.
    pub fn press(&mut self, key: Key) -> bool {
        match key {
            Key::Z | Key::X => {
                let n = SCENARIOS;
                self.scenario = if key == Key::X {
                    (self.scenario + 1) % n
                } else {
                    (self.scenario + n - 1) % n
                };
                self.param = if self.scenario == 0 { 1 } else { 0 };
                self.collider = self.scenario == 1;
                self.setup();
            }
            Key::Up | Key::Down => {
                let n = self.param_count();
                self.param = if key == Key::Up {
                    (self.param + 1) % n
                } else {
                    (self.param + n - 1) % n
                };
                self.setup();
            }
            Key::Left => self.slow = true,
            Key::Right => self.slow = false,
            Key::D => self.collider = !self.collider,
            Key::A => self.reduced_motion = !self.reduced_motion,
            Key::Space => self.trigger(),
            _ => return false,
        }
        true
    }

    fn add(&mut self, kind: Kind, runner: bool, cell: Option<Cell>, pos: Vec2, r: f32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.critters.push(Critter {
            id,
            kind,
            runner,
            cell,
            pos,
            r,
            mood: Mood::default(),
            cues: Cues::default(),
            seed: id.wrapping_mul(7919),
        });
        id
    }

    fn add_cell(&mut self, kind: Kind, runner: bool, cell: Cell) -> u64 {
        let l = self.layout.expect("cell scenarios have a layout");
        let c = l.centre(cell);
        self.add(
            kind,
            runner,
            Some(cell),
            vec2(c.x as f32, c.y as f32),
            l.r as f32,
        )
    }

    fn ordinary(i: usize) -> Kind {
        Kind::Ordinary(emotion::FAMILIES[i % emotion::FAMILIES.len()])
    }

    /// A deterministic family for a filler cell.
    fn filler(cell: Cell) -> Kind {
        Lab::ordinary((hash01(cell.row as u64 * 31 + cell.col as u64, 3) * 6.0) as usize)
    }

    fn fill_rows(&mut self, rows: std::ops::Range<u16>) {
        let l = self.layout.unwrap();
        for row in rows {
            for col in 0..l.row_len(row) {
                let cell = Cell::new(row, col);
                self.add_cell(Lab::filler(cell), false, cell);
            }
        }
    }

    fn id_at(&self, cell: Cell) -> Option<u64> {
        self.critters
            .iter()
            .find(|c| c.cell == Some(cell))
            .map(|c| c.id)
    }

    /// Builds the current scenario from scratch. Animation time keeps
    /// running, so restarting never jumps the clock backwards.
    fn setup(&mut self) {
        self.critters.clear();
        self.actors.clear();
        self.aim = None;
        self.struggle_since = None;
        self.reset_at = None;
        self.ending = None;
        self.layout = None;
        match self.scenario {
            0 => {
                let r = CAST_RADII[self.param];
                let xs = [90.0, 210.0, 330.0, 450.0];
                for i in 0..8 {
                    let pos = vec2(xs[i % 4], if i < 4 { 330.0 } else { 560.0 });
                    let (kind, runner) = match i {
                        6 => (Kind::Ordinary(Family::Block), true),
                        7 => (Kind::Angry, false),
                        _ => (Lab::ordinary(i), false),
                    };
                    self.add(kind, runner, None, pos, r);
                }
            }
            1 => {
                self.add(
                    Lab::ordinary(self.param),
                    false,
                    None,
                    vec2(270.0, 420.0),
                    80.0,
                );
            }
            2 => {
                self.layout = Layout::new(8, 1.0).ok();
                self.fill_rows(0..3);
                self.add_cell(Kind::Angry, false, Cell::new(3, 3));
                for (i, cell) in [(3, 2), (3, 4), (4, 3), (4, 4)].into_iter().enumerate() {
                    self.add_cell(Lab::ordinary(i + 1), false, Cell::new(cell.0, cell.1));
                }
                // Neighbours in the order shown by the parameter.
                let ring = [(2, 3), (2, 4), (3, 4), (4, 4), (4, 3), (3, 2)];
                if self.param > 0 {
                    let (r, c) = ring[self.param - 1];
                    self.aim = self.id_at(Cell::new(r, c));
                }
            }
            3 => {
                self.layout = Layout::new(8, 1.0).ok();
                self.fill_rows(0..4);
                let a = self.add_cell(Kind::Angry, false, Cell::new(4, 2));
                let b = self.add_cell(Kind::Angry, false, Cell::new(4, 5));
                for col in [1, 3, 4, 6] {
                    let cell = Cell::new(4, col);
                    self.add_cell(Lab::filler(cell), false, cell);
                }
                // Same seed: the two angry creatures move identically, so
                // any aim-dependent difference would show.
                let seed = self.critters.iter().find(|c| c.id == a).unwrap().seed;
                if let Some(c) = self.critters.iter_mut().find(|c| c.id == b) {
                    c.seed = seed;
                }
                self.aim = Some(a);
            }
            4 => {
                self.layout = Layout::new(8, 1.0).ok();
                let runner = Cell::new(0, 3);
                let cells: &[(u16, u16)] = match self.param {
                    // Boxed in.
                    0 => &[
                        (0, 0),
                        (0, 1),
                        (0, 2),
                        (0, 4),
                        (0, 5),
                        (1, 2),
                        (1, 3),
                        (1, 4),
                        (2, 3),
                    ],
                    // Only left open.
                    1 => &[
                        (0, 4),
                        (0, 5),
                        (0, 6),
                        (0, 7),
                        (1, 3),
                        (1, 4),
                        (1, 5),
                        (2, 4),
                        (2, 5),
                    ],
                    // Only right open.
                    2 => &[
                        (0, 0),
                        (0, 1),
                        (0, 2),
                        (1, 0),
                        (1, 1),
                        (1, 2),
                        (2, 1),
                        (2, 2),
                    ],
                    // Both open.
                    _ => &[],
                };
                self.add_cell(Kind::Ordinary(Family::Block), true, runner);
                for &(r, c) in cells {
                    let cell = Cell::new(r, c);
                    self.add_cell(Lab::filler(cell), false, cell);
                }
            }
            5 => {
                self.layout = Layout::new(8, 1.0).ok();
                self.add_cell(Kind::Ordinary(Family::Oval), true, Cell::new(0, 3));
                self.add_cell(Kind::Angry, false, Cell::new(1, 3));
                for (r, c) in [(0, 2), (0, 4), (0, 5), (1, 4), (2, 4)] {
                    let cell = Cell::new(r, c);
                    self.add_cell(Lab::filler(cell), false, cell);
                }
            }
            6 => {
                self.layout = Layout::new(8, 1.0).ok();
                self.fill_rows(0..3);
                match self.param {
                    // Direct blast with a hanging cluster below it.
                    0 => {
                        self.add_cell(Kind::Angry, false, Cell::new(3, 3));
                        for (r, c) in [(3, 2), (3, 4), (4, 3), (4, 4), (5, 3), (6, 3)] {
                            self.add_cell(Lab::filler(Cell::new(r, c)), false, Cell::new(r, c));
                        }
                    }
                    // A chain of three.
                    1 => {
                        for (r, c) in [(3, 2), (3, 3), (4, 4)] {
                            self.add_cell(Kind::Angry, false, Cell::new(r, c));
                        }
                        for (r, c) in [
                            (3, 0),
                            (3, 1),
                            (3, 4),
                            (4, 1),
                            (4, 2),
                            (4, 5),
                            (5, 4),
                            (5, 5),
                            (6, 5),
                        ] {
                            self.add_cell(Lab::filler(Cell::new(r, c)), false, Cell::new(r, c));
                        }
                    }
                    // A match of three that drops the cluster below it.
                    _ => {
                        for (r, c) in [(3, 3), (3, 4), (4, 4)] {
                            self.add_cell(Kind::Ordinary(Family::Triangle), false, Cell::new(r, c));
                        }
                        for (r, c) in [(5, 4), (5, 5), (6, 4), (6, 5), (7, 5)] {
                            self.add_cell(Lab::filler(Cell::new(r, c)), false, Cell::new(r, c));
                        }
                    }
                }
                // The launcher aims at the trigger; Angry does not care.
                self.aim = self.id_at(Cell::new(
                    if self.param == 1 { 4 } else { 3 },
                    if self.param == 1 { 4 } else { 3 },
                ));
            }
            _ => {
                let (columns, zoom) = WIDTHS[self.param];
                self.layout = Layout::new(columns, zoom).ok();
                self.fill_rows(0..5);
                let l = self.layout.unwrap();
                let mid = l.columns / 2;
                for (cell, kind) in [
                    (Cell::new(3, mid - 1), Kind::Angry),
                    (Cell::new(2, 1), Kind::Angry),
                ] {
                    if let Some(c) = self.critters.iter_mut().find(|c| c.cell == Some(cell)) {
                        c.kind = kind;
                    }
                }
                if let Some(c) = self
                    .critters
                    .iter_mut()
                    .find(|c| c.cell == Some(Cell::new(0, l.columns - 1)))
                {
                    c.runner = true;
                }
                self.aim = self.id_at(Cell::new(4, mid));
            }
        }
    }

    /// Space: the scenario's event.
    fn trigger(&mut self) {
        let now = self.time();
        match self.scenario {
            0 => {
                // Cycle the aim through the cast and off.
                let ids: Vec<u64> = self.critters.iter().map(|c| c.id).collect();
                self.aim = match self.aim.and_then(|a| ids.iter().position(|&i| i == a)) {
                    None => ids.first().copied(),
                    Some(i) if i + 1 < ids.len() => Some(ids[i + 1]),
                    Some(_) => None,
                };
            }
            1 => {
                let id = self.critters[0].id;
                self.aim = if self.aim.is_some() { None } else { Some(id) };
            }
            2 => {
                self.param = (self.param + 1) % self.param_count();
                self.setup();
            }
            3 => {
                let a = self
                    .critters
                    .iter()
                    .find(|c| c.kind == Kind::Angry)
                    .map(|c| c.id);
                self.aim = if self.aim.is_some() { None } else { a };
            }
            4 => self.setup(),
            5 => {
                // Pop the left blocker: the route opens.
                if let Some(id) = self.id_at(Cell::new(0, 2)) {
                    self.remove(id, Act::Pop, now);
                }
            }
            6 => {
                if self.ending.is_none() && self.reset_at.is_none() {
                    self.stage_resolution(now);
                }
            }
            _ => {
                // Cycle the aim: an ordinary, a nervous neighbour, an Angry,
                // nobody.
                let l = self.layout.unwrap();
                let mid = l.columns / 2;
                let order = [Cell::new(4, mid), Cell::new(3, mid), Cell::new(3, mid - 1)];
                let cur = self
                    .aim
                    .and_then(|a| order.iter().position(|&c| self.id_at(c) == Some(a)));
                self.aim = match cur {
                    None => self.id_at(order[0]),
                    Some(i) if i + 1 < order.len() => self.id_at(order[i + 1]),
                    Some(_) => None,
                };
            }
        }
    }

    fn remove(&mut self, id: u64, act: Act, start: f64) {
        if let Some(i) = self.critters.iter().position(|c| c.id == id) {
            let c = self.critters.remove(i);
            self.actors.push(Actor {
                kind: c.kind,
                runner: c.runner,
                from: c.pos,
                r: c.r,
                start,
                act,
                seed: c.seed,
            });
            if self.aim == Some(id) {
                self.aim = None;
            }
        }
    }

    /// Scenario 6: works out the whole resolution at once (blast closure on
    /// one snapshot, or the match), then the unsupported creatures, and
    /// stages them in time. Logically everything is already decided here.
    fn stage_resolution(&mut self, now: f64) {
        let l = self.layout.unwrap();
        let Some(target) = self
            .aim
            .and_then(|a| self.critters.iter().find(|c| c.id == a))
            .and_then(|c| c.cell)
        else {
            return;
        };
        let at =
            |cells: &[Critter], cell: Cell| cells.iter().find(|c| c.cell == Some(cell)).cloned();
        // (cell, burst time, smile start for Angry)
        let mut removed: Vec<(Cell, f64, Option<f64>)> = Vec::new();
        let mut last = now;
        let snapshot = self.critters.clone();
        if at(&snapshot, target).is_some_and(|c| c.kind == Kind::Angry) {
            // Waves of the closure: radius 1 around each exploding Angry.
            let mut wave = vec![target];
            let mut seen = vec![target];
            let mut k = 0.0;
            while !wave.is_empty() {
                let mut next = Vec::new();
                let burst = now + k * WAVE_GAP + SMILE;
                for &centre in &wave {
                    removed.push((centre, burst, Some(burst - SMILE)));
                    for n in l.neighbours(centre) {
                        if seen.contains(&n) {
                            continue;
                        }
                        let Some(c) = at(&snapshot, n) else { continue };
                        seen.push(n);
                        if c.kind == Kind::Angry {
                            next.push(n);
                        } else {
                            removed.push((n, burst, None));
                        }
                    }
                }
                last = burst;
                wave = next;
                k += 1.0;
            }
        } else {
            // An ordinary match of the target's family.
            let family = at(&snapshot, target).unwrap().kind;
            let mut group = vec![target];
            let mut i = 0;
            while i < group.len() {
                for n in l.neighbours(group[i]) {
                    if !group.contains(&n) && at(&snapshot, n).is_some_and(|c| c.kind == family) {
                        group.push(n);
                    }
                }
                i += 1;
            }
            if group.len() < 3 {
                return;
            }
            for cell in group {
                removed.push((cell, now, None));
            }
        }
        for (cell, burst, smile) in removed.clone() {
            let id = self.id_at(cell).unwrap();
            let act = match smile {
                Some(s) => Act::Blasted {
                    smile: s,
                    angry_source: true,
                },
                None if self.ending.is_none() && removed.iter().any(|r| r.2.is_some()) => {
                    Act::Blasted {
                        smile: burst,
                        angry_source: false,
                    }
                }
                None => Act::Pop,
            };
            self.remove(id, act, burst);
        }
        // Unsupported creatures: not reachable from the top row.
        let occupied: Vec<Cell> = self.critters.iter().filter_map(|c| c.cell).collect();
        let mut held: Vec<Cell> = occupied.iter().copied().filter(|c| c.row == 0).collect();
        let mut i = 0;
        while i < held.len() {
            for n in l.neighbours(held[i]) {
                if occupied.contains(&n) && !held.contains(&n) {
                    held.push(n);
                }
            }
            i += 1;
        }
        let fall_at = last
            + if removed.iter().any(|r| r.2.is_some()) {
                BLAST_LIFE * 0.5
            } else {
                POP_LIFE
            };
        let falling: Vec<u64> = self
            .critters
            .iter()
            .filter(|c| c.cell.is_some_and(|cell| !held.contains(&cell)))
            .map(|c| c.id)
            .collect();
        for id in falling {
            let vx = (hash01(id, 11) - 0.5) * 120.0;
            self.remove(id, Act::Fall { vx }, fall_at);
        }
        let end = self.actors.iter().map(Actor::end).fold(now, f64::max);
        self.ending = Some((end, end + ENDING_HOLD));
    }

    /// One fixed step of lab time.
    pub fn update(&mut self) {
        let step = if self.slow { 1 } else { 4 };
        self.quarters += step;
        let dt = step as f32 / QUARTERS_PER_SECOND as f32;
        let now = self.time();

        if self.reset_at.is_some_and(|t| now >= t) || self.ending.is_some_and(|(_, t)| now >= t) {
            self.setup();
        }
        self.update_runners(now, step as f64 / QUARTERS_PER_SECOND);
        self.actors.retain(|a| a.end() + 0.05 > now);
        self.update_cues();
        for c in &mut self.critters {
            c.mood.step(c.kind, &c.cues, dt);
        }
    }

    /// `dt` is this step's lab time: the step in which the route is first
    /// seen open counts as the first step of the struggle, so a normal
    /// runner leaves on its 90th open step (0.75 s) and a panicking one on
    /// its first.
    fn update_runners(&mut self, now: f64, dt: f64) {
        let Some(l) = self.layout else { return };
        if !matches!(self.scenario, 4 | 5) || self.reset_at.is_some() {
            return;
        }
        let occupied: Vec<Cell> = self.critters.iter().filter_map(|c| c.cell).collect();
        let Some(runner) = self.critters.iter().find(|c| c.runner) else {
            return;
        };
        let (id, cell) = (runner.id, runner.cell.unwrap());
        let panic = self.angry_next_to(cell).is_some();
        let exit = l.chosen_exit(cell, &occupied);
        match exit {
            None => self.struggle_since = None,
            Some(dir) => {
                if self.struggle_since.is_none_or(|(_, d)| d != dir) {
                    self.struggle_since = Some((now - dt, dir));
                }
                let since = self.struggle_since.unwrap().0;
                if panic || now - since >= STRUGGLE - 1e-9 {
                    self.remove(id, Act::Escape { dir: dir as f32 }, now);
                    self.reset_at = Some(now + ESCAPE_PRESENT + 1.2);
                }
            }
        }
    }

    /// The nearest angry neighbour of `cell` (stable: lowest id on a tie).
    fn angry_next_to(&self, cell: Cell) -> Option<Vec2> {
        let l = self.layout?;
        let here = l.centre(cell);
        l.neighbours(cell)
            .into_iter()
            .filter_map(|n| {
                self.critters
                    .iter()
                    .find(|c| c.cell == Some(n) && c.kind == Kind::Angry)
            })
            .min_by_key(|c| c.id)
            .map(|c| {
                let d = l.centre(c.cell.unwrap()) - here;
                vec2(d.x as f32, d.y as f32).normalize()
            })
    }

    fn update_cues(&mut self) {
        let occupied: Vec<Cell> = self.critters.iter().filter_map(|c| c.cell).collect();
        let exits: Option<Option<i8>> = self.layout.and_then(|l| {
            self.critters
                .iter()
                .find(|c| c.runner && c.cell.is_some())
                .map(|c| l.chosen_exit(c.cell.unwrap(), &occupied))
        });
        let mut cues = Vec::with_capacity(self.critters.len());
        for c in &self.critters {
            let angry_dir = c.cell.and_then(|cell| self.angry_next_to(cell));
            let to_launcher = vec2(LAUNCHER.x as f32, LAUNCHER.y as f32) - c.pos;
            let runner = if !c.runner {
                RunnerCue::None
            } else if !matches!(self.scenario, 4 | 5) {
                RunnerCue::Blocked
            } else {
                let exit = exits.flatten().map(|d| d as f32);
                match (angry_dir.is_some(), exit) {
                    (true, dir) => RunnerCue::Panic { dir },
                    (false, Some(dir)) => RunnerCue::Struggle { dir },
                    (false, None) => RunnerCue::Blocked,
                }
            };
            cues.push(Cues {
                nervous: angry_dir.is_some(),
                aimed: self.aim == Some(c.id),
                angry_dir,
                launcher_dir: to_launcher.normalize_or_zero(),
                runner,
            });
        }
        for (c, cue) in self.critters.iter_mut().zip(cues) {
            c.cues = cue;
        }
    }

    /// Poses of the living creatures at the current time (pure).
    fn poses(&self) -> Vec<Pose> {
        let t = self.time();
        self.critters
            .iter()
            .map(|c| emotion::pose(c.kind, &c.cues, &c.mood, t, c.seed, self.reduced_motion))
            .collect()
    }

    pub fn draw(&self, v: &View) {
        let t = self.time();
        if let Some(l) = self.layout {
            draw_board_frame(v, &l);
        }
        let launcher_on = !matches!(self.scenario, 0 | 4 | 5);
        if launcher_on {
            let at = vec2(LAUNCHER.x as f32, LAUNCHER.y as f32);
            let target = self
                .aim
                .and_then(|a| self.critters.iter().find(|c| c.id == a))
                .map(|c| c.pos);
            let dir = target.map_or(vec2(0.0, -1.0), |p| {
                let d = (p - at).normalize_or_zero();
                if d == Vec2::ZERO {
                    vec2(0.0, -1.0)
                } else {
                    d
                }
            });
            let r = self.layout.map_or(30.0, |l| l.r as f32);
            let loaded = Kind::Ordinary(Family::Round);
            let pose = emotion::pose(
                loaded,
                &Cues::default(),
                &Mood::default(),
                t,
                1,
                self.reduced_motion,
            );
            render::draw_launcher(v, at, dir, r, Some((loaded, pose)));
        }
        for (c, pose) in self.critters.iter().zip(self.poses()) {
            render::draw_creature(v, c.pos, c.r, c.kind, c.runner, &pose, 1.0);
        }
        for a in &self.actors {
            self.draw_actor(v, a, t);
        }
        if self.collider {
            for c in &self.critters {
                render::draw_collider(v, c.pos, c.r);
            }
        }
        if self.scenario == 0 {
            for c in &self.critters {
                let name = kind_name(c.kind, c.runner);
                v.text_centered(
                    name,
                    c.pos.x,
                    c.pos.y + c.r + 28.0,
                    18.0,
                    with_alpha(TEXT, 0.8),
                );
            }
        }
        self.draw_hud(v);
    }

    fn draw_actor(&self, v: &View, a: &Actor, t: f64) {
        let since = (t - a.start) as f32;
        let calm = Mood::default();
        let live = |pose: Pose| render::draw_creature(v, a.from, a.r, a.kind, a.runner, &pose, 1.0);
        match a.act {
            Act::Pop => {
                if since < POP_LIFE as f32 {
                    live(pop_pose(a.kind, since.max(0.0)));
                } else {
                    let c = render::body_color(a.kind);
                    render::draw_debris(v, a.from, a.r, c, since - POP_LIFE as f32, 0.3, a.seed);
                }
            }
            Act::Blasted {
                smile,
                angry_source,
            } => {
                if since < 0.0 {
                    // Logically gone already; still shown until its burst.
                    let pose = emotion::pose(
                        a.kind,
                        &Cues::default(),
                        &calm,
                        t,
                        a.seed,
                        self.reduced_motion,
                    );
                    let pose = if angry_source && t >= smile {
                        blast_smile(pose)
                    } else if angry_source {
                        pose
                    } else {
                        pop_pose(a.kind, 0.0)
                    };
                    live(pose);
                } else {
                    let c = render::body_color(a.kind);
                    render::draw_debris(v, a.from, a.r, c, since, BLAST_LIFE as f32, a.seed);
                    if angry_source {
                        render::draw_blast(v, a.from, a.r, since, BLAST_LIFE as f32, false);
                    }
                }
            }
            Act::Fall { vx } => {
                let s = since.max(0.0);
                let pos = a.from + vec2(vx * s, 1600.0 * s * s);
                if pos.y < 1000.0 + a.r {
                    let pose = fall_pose(a.kind, s, a.seed, self.reduced_motion);
                    render::draw_creature(v, pos, a.r, a.kind, a.runner, &pose, 1.0);
                }
            }
            Act::Escape { dir } => {
                let s = since.max(0.0);
                // A squash, then a dash out of the side.
                let squash = (s / 0.06).min(1.0);
                let dash = (s - 0.06).max(0.0);
                let pos = a.from + vec2(dir * 2600.0 * dash * dash, -6.0 * (1.0 - squash) * squash);
                let mut pose = emotion::base_face(a.kind);
                pose.scale = if s < 0.06 {
                    vec2(1.0 - 0.2 * squash, 1.0 + 0.15 * squash)
                } else {
                    vec2(1.25, 0.85)
                };
                pose.angle = -dir * 0.25;
                pose.pupil = vec2(dir, 0.0);
                pose.mouth_curve = 0.45;
                pose.mouth_open = 0.15;
                pose.eye_open = 0.6;
                if pos.x > -2.0 * a.r && pos.x < FIELD_W + 2.0 * a.r {
                    render::draw_creature(v, pos, a.r, a.kind, true, &pose, 1.0);
                    // Speed lines.
                    let trail = with_alpha(TEXT, 0.5);
                    for k in [-0.4, 0.0, 0.4] {
                        let y = pos.y + k * a.r;
                        let x = pos.x - dir * 1.1 * a.r;
                        v.line(vec2(x, y), vec2(x - dir * 0.9 * a.r, y), 2.0, trail);
                    }
                }
            }
        }
    }

    fn draw_hud(&self, v: &View) {
        let title = match self.scenario {
            0 => t!("lab.s.cast"),
            1 => t!("lab.s.target"),
            2 => t!("lab.s.ring"),
            3 => t!("lab.s.angry_aim"),
            4 => t!("lab.s.routes"),
            5 => t!("lab.s.panic"),
            6 => t!("lab.s.blast"),
            _ => t!("lab.s.widths"),
        };
        let head = t!(
            "lab.title",
            n = self.scenario + 1,
            total = SCENARIOS,
            name = title
        );
        v.text_centered(&head, FIELD_W / 2.0, 34.0, 22.0, NEON);
        let detail = self.detail();
        v.text_centered(&detail, FIELD_W / 2.0, 60.0, 17.0, ACCENT);
        let speed = if self.slow {
            t!("lab.slow")
        } else {
            t!("lab.normal")
        };
        let motion = if self.reduced_motion {
            t!("lab.reduced_on")
        } else {
            t!("lab.reduced_off")
        };
        let status = format!("{speed}   {motion}");
        v.text_centered(&status, FIELD_W / 2.0, 907.0, 14.0, with_alpha(TEXT, 0.75));
        if let Some((presented, _)) = self.ending {
            if self.time() >= presented {
                v.text_centered(t!("lab.ending_hold"), FIELD_W / 2.0, 700.0, 26.0, ACCENT);
            }
        }
        let help = with_alpha(TEXT, 0.65);
        v.text_wrapped_centered(t!("lab.help"), 926.0, 14.0, help);
    }

    fn detail(&self) -> String {
        match self.scenario {
            0 => t!("lab.d.size", r = format!("{:.0}", CAST_RADII[self.param])),
            1 => {
                let aim = if self.aim.is_some() {
                    t!("lab.on")
                } else {
                    t!("lab.off")
                };
                t!(
                    "lab.d.target",
                    family = kind_name(Lab::ordinary(self.param), false),
                    aim = aim
                )
            }
            2 => {
                if self.param == 0 {
                    t!("lab.d.ring_none").to_owned()
                } else {
                    t!("lab.d.ring", n = self.param)
                }
            }
            3 => {
                if self.aim.is_some() {
                    t!("lab.d.angry_aimed").to_owned()
                } else {
                    t!("lab.d.angry_free").to_owned()
                }
            }
            4 | 5 => {
                let occupied: Vec<Cell> = self.critters.iter().filter_map(|c| c.cell).collect();
                let route = self
                    .layout
                    .zip(self.critters.iter().find(|c| c.runner))
                    .map(|(l, c)| l.chosen_exit(c.cell.unwrap(), &occupied));
                let route = match route {
                    None => t!("lab.d.escaped"),
                    Some(None) => t!("lab.d.blocked"),
                    Some(Some(d)) if d < 0 => t!("lab.d.left"),
                    Some(Some(_)) => t!("lab.d.right"),
                };
                if self.scenario == 4 {
                    let setup = match self.param {
                        0 => t!("lab.p.boxed"),
                        1 => t!("lab.p.left"),
                        2 => t!("lab.p.right"),
                        _ => t!("lab.p.both"),
                    };
                    t!("lab.d.route", setup = setup, route = route)
                } else {
                    t!("lab.d.panic", route = route)
                }
            }
            6 => match self.param {
                0 => t!("lab.p.direct").to_owned(),
                1 => t!("lab.p.chain").to_owned(),
                _ => t!("lab.p.fall").to_owned(),
            },
            _ => {
                let (columns, zoom) = WIDTHS[self.param];
                t!(
                    "lab.d.width",
                    columns = columns,
                    zoom = format!("{:.0}", zoom * 100.0)
                )
            }
        }
    }
}

fn kind_name(kind: Kind, runner: bool) -> &'static str {
    if runner {
        return t!("lab.k.runner");
    }
    match kind {
        Kind::Ordinary(Family::Round) => t!("lab.k.round"),
        Kind::Ordinary(Family::Block) => t!("lab.k.block"),
        Kind::Ordinary(Family::Diamond) => t!("lab.k.diamond"),
        Kind::Ordinary(Family::Hex) => t!("lab.k.hex"),
        Kind::Ordinary(Family::Triangle) => t!("lab.k.triangle"),
        Kind::Ordinary(Family::Oval) => t!("lab.k.oval"),
        Kind::Angry => t!("lab.k.angry"),
    }
}

/// Board edges, ceiling and danger line.
fn draw_board_frame(v: &View, l: &Layout) {
    let edge = with_alpha(NEON, 0.35);
    let (left, right, top) = (l.left as f32, l.right as f32, l.ceiling_y as f32);
    let danger = super::geometry::DANGER_Y as f32;
    v.rect(
        left,
        top,
        right - left,
        danger - top,
        Color::new(0.0, 0.0, 0.0, 0.18),
    );
    v.line(vec2(left, top), vec2(right, top), 3.0, edge);
    v.line(vec2(left, top), vec2(left, danger), 2.0, edge);
    v.line(vec2(right, top), vec2(right, danger), 2.0, edge);
    let warn = Color::new(1.0, 0.35, 0.35, 0.45);
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

    fn run(lab: &mut Lab, ticks: u32) {
        for _ in 0..ticks {
            lab.update();
        }
    }

    fn goto(lab: &mut Lab, scenario: usize) {
        while lab.scenario != scenario {
            lab.press(Key::X);
        }
    }

    #[test]
    fn every_scenario_and_parameter_builds_and_runs() {
        let mut lab = Lab::new();
        for s in 0..SCENARIOS {
            goto(&mut lab, s);
            for _ in 0..lab.param_count() {
                lab.press(Key::Up);
                run(&mut lab, 30);
                lab.press(Key::Space);
                run(&mut lab, 400);
                assert!(!lab.detail().is_empty());
            }
        }
    }

    #[test]
    fn ring_neighbours_are_all_nervous_and_the_aimed_one_afraid() {
        let mut lab = Lab::new();
        goto(&mut lab, 2);
        lab.press(Key::Up); // first neighbour aimed
        run(&mut lab, 60);
        let nervous: Vec<&Critter> = lab.critters.iter().filter(|c| c.cues.nervous).collect();
        assert_eq!(nervous.len(), 6);
        assert!(nervous.iter().all(|c| c.mood.nervous == 1.0));
        let afraid: Vec<&Critter> = lab.critters.iter().filter(|c| c.mood.fear > 0.0).collect();
        assert_eq!(afraid.len(), 1);
        assert!(afraid[0].cues.nervous, "the aimed one is also nervous");
        let angry = lab.critters.iter().find(|c| c.kind == Kind::Angry).unwrap();
        assert!(!angry.cues.nervous || angry.mood.nervous == 0.0);
    }

    /// The two angry creatures share a seed: aiming at one leaves their
    /// poses identical, and nothing behind it becomes afraid.
    #[test]
    fn aimed_angry_matches_its_unaimed_twin() {
        let mut lab = Lab::new();
        goto(&mut lab, 3);
        for i in 0..240 {
            if i == 100 {
                lab.press(Key::Space); // aim off
            }
            if i == 150 {
                lab.press(Key::Space); // aim on again
            }
            lab.update();
            let poses = lab.poses();
            let angry: Vec<usize> = (0..lab.critters.len())
                .filter(|&i| lab.critters[i].kind == Kind::Angry)
                .collect();
            assert_eq!(angry.len(), 2);
            assert_eq!(poses[angry[0]], poses[angry[1]]);
            assert!(
                lab.critters.iter().all(|c| c.mood.fear == 0.0),
                "nobody is afraid"
            );
        }
    }

    #[test]
    fn runner_struggles_then_leaves_by_the_open_side() {
        let mut lab = Lab::new();
        goto(&mut lab, 4);
        lab.press(Key::Up); // only left open
        let runner = |lab: &Lab| {
            lab.critters
                .iter()
                .find(|c| c.runner)
                .map(|c| c.cues.runner)
        };
        run(&mut lab, 1);
        assert_eq!(runner(&lab), Some(RunnerCue::Struggle { dir: -1.0 }));
        // 0.75 s = 90 ticks: still there on open tick 89, gone on tick 90.
        run(&mut lab, 88);
        assert!(runner(&lab).is_some());
        run(&mut lab, 1);
        assert!(runner(&lab).is_none());
        assert!(lab
            .actors
            .iter()
            .any(|a| a.act == Act::Escape { dir: -1.0 }));
        // Boxed in: never leaves.
        lab.press(Key::Down);
        run(&mut lab, 400);
        assert_eq!(runner(&lab), Some(RunnerCue::Blocked));
    }

    #[test]
    fn panicking_runner_leaves_as_soon_as_the_route_opens() {
        let mut lab = Lab::new();
        goto(&mut lab, 5);
        run(&mut lab, 200);
        let cue = lab.critters.iter().find(|c| c.runner).unwrap().cues.runner;
        assert_eq!(
            cue,
            RunnerCue::Panic { dir: None },
            "blocked panic, no tunnelling"
        );
        lab.press(Key::Space);
        run(&mut lab, 1);
        assert!(
            lab.critters.iter().all(|c| !c.runner),
            "gone on the first tick"
        );
    }

    #[test]
    fn chain_blast_removes_the_closure_once_and_drops_the_rest() {
        let mut lab = Lab::new();
        goto(&mut lab, 6);
        lab.press(Key::Up); // chain
        let before = lab.critters.len();
        lab.press(Key::Space);
        let blasted = lab
            .actors
            .iter()
            .filter(|a| matches!(a.act, Act::Blasted { .. }))
            .count();
        let angry = lab
            .actors
            .iter()
            .filter(|a| {
                matches!(
                    a.act,
                    Act::Blasted {
                        angry_source: true,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(angry, 3, "all three Angry in the chain");
        assert_eq!(
            lab.critters.len() + lab.actors.len(),
            before,
            "each creature removed once"
        );
        assert!(blasted > 3);
        // Everything left hangs from the top row.
        assert!(lab.actors.iter().any(|a| matches!(a.act, Act::Fall { .. })));
        // It ends with the hold, then restarts.
        let (_, end) = lab.ending.unwrap();
        while lab.time() < end {
            lab.update();
        }
        lab.update();
        assert!(lab.ending.is_none() && lab.critters.len() == before);
    }

    #[test]
    fn slow_motion_runs_at_a_quarter() {
        let mut lab = Lab::new();
        run(&mut lab, 120);
        assert!((lab.time() - 1.0).abs() < 1e-9);
        lab.press(Key::Left);
        run(&mut lab, 120);
        assert!((lab.time() - 1.25).abs() < 1e-9);
    }
}
