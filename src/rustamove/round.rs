//! A classic round (milestone M3): aim by fear, shoot, match three or more,
//! drop what no longer hangs from the ceiling, score, feel the ceiling come
//! down every few shots, and win by clearing the board or lose at the
//! danger line or on a blocked board. See docs/GAME_RULES.md R01–R14.
//!
//! The rules run in `update`, one fixed step at a time, and decide
//! everything at once when a shot lands (`resolve`); the pops and falls that
//! follow are presentation with bounded lifetimes and never change the
//! outcome. Angry blasts (M4) and runner escapes (M5) are not part of this
//! milestone: a direct Angry hit only uses up the shot.

use macroquad::math::{vec2, Vec2};

use super::aim::{Aim, Intent};
use super::blast;
use super::board::{Board, CreatureId};
use super::content::{self, Source};
use super::emotion::{self, Cues, Family, Kind, Mood, RunnerCue, FAMILIES};
use super::geometry::{Cell, Layout, DANGER_Y, LAUNCHER};
use super::render;
use super::rng::Rng;
use super::rules::Rules;
use super::sfx;
use super::shot::{self, Flight, Hit, ShotPlan, SPEED_DIAMETERS};
use crate::audio::Sfx;
use crate::game::{Outcome, FIXED_DT};
use crate::t;
use crate::ui::Input;
use crate::view::{with_alpha, View, ACCENT, ERROR, FIELD_W, NEON, TEXT};

/// Why a round was lost.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LossReason {
    /// A creature reached the danger line.
    Danger,
    /// The shot found no free cell next to what it hit.
    Blocked,
    /// The shot planner hit its safety limit (a geometry fault).
    Fault,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Won,
    Lost(LossReason),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    Aiming,
    Flight,
    /// Input locked until this tick.
    Settling(u64),
    /// The outcome is decided; the round reports it from this tick on.
    Ending(u64),
}

/// What one landed shot did, decided in one go.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resolution {
    pub attached: Option<(Cell, CreatureId)>,
    /// Removed by the match, in row-major order.
    pub matched: Vec<Cell>,
    /// Removed by a direct Angry hit and its chain, in removal order.
    pub blasted: Vec<Cell>,
    /// Number of blast waves (0 without a blast).
    pub blast_waves: u32,
    /// A runner that left, and which way.
    pub escaped: Option<(Cell, i8)>,
    /// Removed because nothing held them any more.
    pub fallen: Vec<Cell>,
    /// Every removed creature with where it was, for presentation.
    pub gone: Vec<Gone>,
    pub ceiling_dropped: bool,
    pub points: u64,
    pub end: Option<End>,
}

/// A creature taken off the board by a resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gone {
    pub cell: Cell,
    pub kind: Kind,
    pub runner: bool,
    /// Its centre before removal (and before any ceiling drop).
    pub at: Vec2,
    pub cause: Cause,
    /// Its cosmetic seed, so its last moments move like it did.
    pub seed: u64,
}

/// Why a creature left the board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Matched,
    /// In blast wave `wave`; `source` for an Angry that exploded itself.
    Blast {
        wave: u32,
        source: bool,
    },
    Fell,
    /// A runner that got away sideways.
    Escaped {
        dir: i8,
    },
}

/// A creature leaving the board, shown for a while. Cosmetic only.
#[derive(Clone, Debug)]
struct Actor {
    kind: Kind,
    runner: bool,
    from: Vec2,
    seed: u64,
    act: Act,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Act {
    /// Squash from `at`, then fragments.
    Pop { at: u64 },
    /// Waits in place until `at`, then tumbles down.
    Fall { at: u64, vx: f32 },
    /// Still in place (an Angry smiles from `smile_from`) until `burst_at`,
    /// then fragments; a `source` also flashes and rings.
    Blast {
        smile_from: u64,
        burst_at: u64,
        source: bool,
    },
    /// Squash, then a dash out of the side.
    Escape { at: u64, dir: f32 },
}

impl Actor {
    /// The tick after which the actor is no longer drawn.
    fn end(&self, rules: &Rules) -> u64 {
        match self.act {
            Act::Pop { at } => at + rules.pop_present_ticks + 36,
            Act::Fall { at, .. } => at + rules.fall_max_ticks,
            Act::Blast { burst_at, .. } => burst_at + rules.blast_present_ticks,
            Act::Escape { at, .. } => at + rules.escape_present_ticks + 30,
        }
    }
}

/// Counters for tuning and debugging (docs: prototype measurements).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub matched: u32,
    pub fallen: u32,
    pub biggest_fall: u32,
    pub ceiling_drops: u32,
    pub blasted: u32,
    pub longest_chain: u32,
    pub escaped: u32,
}

#[derive(Clone)]
pub struct Round {
    pub board: Board,
    pub rules: Rules,
    pub level: u32,
    /// Where the board came from (its hint) and its seed.
    pub source: Source,
    #[allow(dead_code)]
    pub seed: u64,
    rng: Rng,
    pub aim: Aim,
    intent: Intent,
    phase: Phase,
    pub plan: Option<ShotPlan>,
    pub flight: Option<Flight>,
    pub loaded: Family,
    pub next: Family,
    pub ticks: u64,
    pub shots: u32,
    /// Shots since the ceiling last dropped.
    pub since_drop: u32,
    pub score: u64,
    pub stats: Stats,
    pub end: Option<End>,
    pub reduced_motion: bool,
    /// A ring and colour pulse instead of a white flash.
    pub reduced_flashes: bool,
    /// How many fragments effects draw.
    pub quality: render::Quality,
    actors: Vec<Actor>,
    /// Sounds due at a later tick (blast waves, falls).
    scheduled: Vec<(u64, Sfx)>,
    /// When the ceiling last dropped, for its slide.
    dropped_at: Option<u64>,
    sounds: Vec<Sfx>,
}

impl Round {
    /// Level `level` on a board with these settings.
    pub fn new(level: u32, layout: Layout, reduced_motion: bool) -> Round {
        let (board, rules, seed, source) = content::level_board(layout, level);
        Round::with_board(level, board, rules, seed, source, reduced_motion)
    }

    /// A round on a given board (tests and fixtures).
    pub fn with_board(
        level: u32,
        board: Board,
        rules: Rules,
        seed: u64,
        source: Source,
        reduced_motion: bool,
    ) -> Round {
        let mut r = Round {
            board,
            rules,
            level,
            source,
            seed,
            rng: Rng::new(seed ^ 0xA770_5EED),
            aim: Aim::default(),
            intent: Intent::default(),
            phase: Phase::Aiming,
            plan: None,
            flight: None,
            loaded: Family::Round,
            next: Family::Round,
            ticks: 0,
            shots: 0,
            since_drop: 0,
            score: 0,
            stats: Stats::default(),
            end: None,
            reduced_motion,
            reduced_flashes: false,
            quality: render::Quality::Normal,
            actors: Vec::new(),
            scheduled: Vec::new(),
            dropped_at: None,
            sounds: Vec::new(),
        };
        r.loaded = r.draw_family();
        r.next = r.draw_family();
        r
    }

    /// Families still on the board, in stable order (rule R13).
    fn present_families(&self) -> Vec<Family> {
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
        present
    }

    /// An ammunition family from those on the board; any family when only
    /// specials (or nothing) are left.
    fn draw_family(&mut self) -> Family {
        let mut present = self.present_families();
        if present.is_empty() {
            present = FAMILIES.to_vec();
        }
        present[self.rng.below(present.len() as u32) as usize]
    }

    pub fn handle(&mut self, input: &Input) {
        self.intent.add(input);
    }

    pub fn finished(&self) -> Option<(u32, Outcome)> {
        match (self.phase, self.end) {
            (Phase::Ending(at), Some(end)) if self.ticks >= at => {
                let outcome = if end == End::Won {
                    Outcome::Won
                } else {
                    Outcome::Lost
                };
                Some((self.score.min(u32::MAX as u64) as u32, outcome))
            }
            _ => None,
        }
    }

    #[allow(dead_code)] // Used by tests and scripted checks.
    pub fn is_aiming(&self) -> bool {
        self.phase == Phase::Aiming
    }

    /// One fixed step.
    pub fn update(&mut self) {
        if self.finished().is_some() {
            return;
        }
        self.ticks += 1;
        let intent = self.intent.take();
        if !matches!(self.phase, Phase::Ending(_)) {
            self.aim.update(&intent);
        }
        match self.phase {
            Phase::Aiming => {
                // An explicit shot comes first (rule P09); only otherwise do
                // runners struggle and maybe leave.
                if intent.fire {
                    self.fire();
                } else if let Some((cell, dir)) = self.step_runners() {
                    let res = self.escape(cell, dir);
                    self.present(&res);
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
            Phase::Ending(_) => {}
        }
        let now = self.ticks;
        let rules = self.rules;
        self.actors.retain(|a| now <= a.end(&rules));
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.scheduled)
            .into_iter()
            .partition(|&(t, _)| t <= now);
        self.scheduled = later;
        self.sounds.extend(due.into_iter().map(|(_, s)| s));
        self.update_moods();
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
        // Always against the latest board.
        self.plan = None;
        match shot::plan(&self.board, self.aim.angle) {
            Ok(plan) => {
                self.plan = Some(plan.clone());
                self.flight = Some(Flight::new(plan));
                self.phase = Phase::Flight;
                self.sounds.push(sfx::LAUNCH);
            }
            Err(_) => self.finish(End::Lost(LossReason::Fault), self.ticks),
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
        if arrived {
            let plan = flight.plan.clone();
            self.flight = None;
            self.plan = None;
            let res = self.resolve(&plan);
            self.present(&res);
        }
    }

    /// Everything a landed shot does, in the order of rule R11: attach (or
    /// blocked), match, unsupported falls, score, count the shot, win on an
    /// empty board, otherwise drop the ceiling when due, then the danger
    /// check. Next ammunition is chosen from what is left.
    pub fn resolve(&mut self, plan: &ShotPlan) -> Resolution {
        debug_assert_eq!(
            plan.revision,
            self.board.revision(),
            "the board is frozen in flight"
        );
        let mut res = Resolution::default();
        let shot_kind = Kind::Ordinary(self.loaded);
        match (plan.attachment, plan.hit) {
            (Some(cell), _) => {
                let id = self
                    .board
                    .place(cell, shot_kind, false)
                    .expect("the plan's cell is empty");
                res.attached = Some((cell, id));
                res.matched = self.match_group(cell);
                for &c in &res.matched.clone() {
                    self.take(c, Cause::Matched, &mut res);
                }
            }
            // A direct hit on Angry: the projectile is used up and the
            // whole chain goes off (rule R06), before any falls.
            (
                None,
                Hit::Creature {
                    kind: Kind::Angry,
                    cell,
                    ..
                },
            ) => {
                let blast = blast::closure(&self.board, cell, self.rules.explosion_radius);
                res.blast_waves = blast.waves();
                for &(c, wave) in &blast.removed {
                    let source = blast.sources.iter().any(|&(s, _)| s == c);
                    self.take(c, Cause::Blast { wave, source }, &mut res);
                    res.blasted.push(c);
                }
            }
            (None, _) => {
                res.end = Some(End::Lost(LossReason::Blocked));
            }
        }
        if res.end.is_none() {
            let anchored = self.board.anchored();
            res.fallen = self
                .board
                .occupied()
                .map(|(c, _)| c)
                .filter(|c| !anchored.contains(c))
                .collect();
        }
        for &c in &res.fallen.clone() {
            self.take(c, Cause::Fell, &mut res);
        }
        // Each creature scores once, by the way it left.
        res.points = (res.matched.len() + res.blasted.len()) as u64 * self.rules.match_score
            + res.fallen.len() as u64 * self.rules.fall_score;
        self.stats.blasted += res.blasted.len() as u32;
        self.stats.longest_chain = self.stats.longest_chain.max(res.blast_waves);
        self.score = self.score.saturating_add(res.points);
        self.stats.matched += res.matched.len() as u32;
        self.stats.fallen += res.fallen.len() as u32;
        self.stats.biggest_fall = self.stats.biggest_fall.max(res.fallen.len() as u32);
        self.shots += 1;
        if res.end.is_none() {
            if self.board.count() == 0 {
                // A clearing shot wins before any ceiling drop.
                res.end = Some(End::Won);
            } else {
                self.since_drop += 1;
                if self.since_drop >= self.rules.ceiling_shots {
                    self.since_drop = 0;
                    self.board.lower_ceiling();
                    self.stats.ceiling_drops += 1;
                    res.ceiling_dropped = true;
                }
                if self.board.reaches_danger() {
                    res.end = Some(End::Lost(LossReason::Danger));
                }
            }
        }
        // Next ammunition, replacing families that are gone.
        self.loaded = self.next;
        self.next = self.draw_family();
        self.fix_ammo();
        res
    }

    /// Replaces loaded and next ammunition whose family has left the board
    /// (rule R13), deterministically.
    fn fix_ammo(&mut self) {
        let present = self.present_families();
        if !present.is_empty() {
            if !present.contains(&self.loaded) {
                self.loaded = self.draw_family();
            }
            if !present.contains(&self.next) {
                self.next = self.draw_family();
            }
        }
    }

    /// One aiming tick for the runners (rules R09–R10): each runner, in id
    /// order, keeps its route while it stays open, otherwise takes the open
    /// side nearer its exit (left on a tie) and starts over; with no route
    /// it stops. Returns the first runner ready to leave: after its full
    /// struggle, or at once next to an Angry. At most one leaves per tick.
    fn step_runners(&mut self) -> Option<(Cell, i8)> {
        let l = self.board.layout;
        let occupied: Vec<Cell> = self.board.occupied().map(|(c, _)| c).collect();
        let mut runners: Vec<(CreatureId, Cell)> = self
            .board
            .occupied()
            .filter(|(_, c)| c.runner)
            .map(|(cell, c)| (c.id, cell))
            .collect();
        runners.sort();
        let mut ready = None;
        for (_, cell) in runners {
            let held = self.board.get(cell).and_then(|c| c.struggle);
            let dir = match held {
                Some(s) if l.route_clear(cell, s.dir, &occupied) => Some(s.dir),
                _ => l.chosen_exit(cell, &occupied),
            };
            let panic = self.angry_next_to(cell);
            let rules = self.rules;
            let Some(c) = self.board.get_mut(cell) else {
                continue;
            };
            c.struggle = dir.map(|d| {
                let ticks = match held {
                    Some(s) if s.dir == d => s.ticks + 1,
                    _ => 1,
                };
                super::board::Struggle { dir: d, ticks }
            });
            if let Some(s) = c.struggle {
                let limit = if panic {
                    rules.panic_struggle_ticks
                } else {
                    rules.runner_struggle_ticks
                };
                if ready.is_none() && s.ticks >= limit.max(1) {
                    ready = Some((cell, s.dir));
                }
            }
        }
        ready
    }

    fn angry_next_to(&self, cell: Cell) -> bool {
        self.board
            .layout
            .neighbours(cell)
            .into_iter()
            .any(|n| self.board.get(n).is_some_and(|c| c.kind == Kind::Angry))
    }

    /// A runner leaves: it is gone at once (its dash is only shown), what
    /// no longer hangs falls, and the board may be empty (rule R10). It is
    /// not a shot: the ceiling counter does not move.
    pub fn escape(&mut self, cell: Cell, dir: i8) -> Resolution {
        let mut res = Resolution::default();
        self.take(cell, Cause::Escaped { dir }, &mut res);
        res.escaped = Some((cell, dir));
        let anchored = self.board.anchored();
        res.fallen = self
            .board
            .occupied()
            .map(|(c, _)| c)
            .filter(|c| !anchored.contains(c))
            .collect();
        for &c in &res.fallen.clone() {
            self.take(c, Cause::Fell, &mut res);
        }
        res.points = self.rules.escape_score + res.fallen.len() as u64 * self.rules.fall_score;
        self.score = self.score.saturating_add(res.points);
        self.stats.escaped += 1;
        self.stats.fallen += res.fallen.len() as u32;
        self.stats.biggest_fall = self.stats.biggest_fall.max(res.fallen.len() as u32);
        if self.board.count() == 0 {
            res.end = Some(End::Won);
        }
        self.fix_ammo();
        res
    }

    /// Removes the creature in `cell`, noting it in `res`.
    fn take(&mut self, cell: Cell, cause: Cause, res: &mut Resolution) {
        let p = self.board.layout.centre(cell);
        if let Some(c) = self.board.remove(cell) {
            res.gone.push(Gone {
                cell,
                kind: c.kind,
                runner: c.runner,
                at: vec2(p.x as f32, p.y as f32),
                cause,
                seed: c.seed(),
            });
        }
    }

    /// The connected same-family group containing `cell`, if it is at least
    /// `match_min` large (rule R05); otherwise empty.
    fn match_group(&self, cell: Cell) -> Vec<Cell> {
        let Some(Kind::Ordinary(family)) = self.board.get(cell).map(|c| c.kind) else {
            return Vec::new();
        };
        let mut group = vec![cell];
        let mut i = 0;
        while i < group.len() {
            for n in self.board.layout.neighbours(group[i]) {
                let same = self
                    .board
                    .get(n)
                    .is_some_and(|c| c.kind == Kind::Ordinary(family));
                if same && !group.contains(&n) {
                    group.push(n);
                }
            }
            i += 1;
        }
        if group.len() < self.rules.match_min {
            return Vec::new();
        }
        group.sort();
        group
    }

    /// Starts the visible consequences of a resolution and the next phase.
    /// Everything is already decided; this only times the show.
    fn present(&mut self, res: &Resolution) {
        let now = self.ticks;
        let r = self.rules;
        let burst = |wave: u32| now + wave as u64 * r.blast_wave_ticks + r.blast_smile_ticks;
        // When the last visible removal has played, before falls start.
        let removals_done = if res.escaped.is_some() {
            now + r.escape_present_ticks
        } else if res.blast_waves > 0 {
            burst(res.blast_waves - 1) + r.blast_present_ticks
        } else if !res.matched.is_empty() {
            now + r.pop_present_ticks
        } else {
            now
        };
        let fall_at = if res.escaped.is_some() {
            now + r.escape_present_ticks / 3
        } else if res.blast_waves > 0 {
            burst(res.blast_waves - 1) + r.blast_present_ticks / 2
        } else if !res.matched.is_empty() {
            now + r.pop_present_ticks / 2
        } else {
            now
        };
        for g in &res.gone {
            let act = match g.cause {
                Cause::Matched => Act::Pop { at: now },
                Cause::Fell => Act::Fall {
                    at: fall_at,
                    vx: (emotion::hash01(g.seed, 3) - 0.5) * 140.0,
                },
                Cause::Blast { wave, source } => Act::Blast {
                    smile_from: burst(wave) - r.blast_smile_ticks,
                    burst_at: burst(wave),
                    source,
                },
                Cause::Escaped { dir } => Act::Escape {
                    at: now,
                    dir: dir as f32,
                },
            };
            self.actors.push(Actor {
                kind: g.kind,
                runner: g.runner,
                from: g.at,
                seed: g.seed,
                act,
            });
        }
        // One sound per kind: one blast plus a few accents for later waves,
        // one pop, one fall.
        if res.escaped.is_some() {
            self.sounds.push(sfx::ESCAPE);
        } else if res.blast_waves > 0 {
            self.scheduled.push((burst(0), sfx::BLAST));
            for wave in 1..res.blast_waves.min(r.blast_accents + 1) {
                self.scheduled.push((burst(wave), sfx::CHAIN));
            }
        } else if !res.matched.is_empty() {
            self.sounds.push(sfx::POP);
        } else if res.attached.is_some() {
            self.sounds.push(sfx::ATTACH);
        }
        if !res.fallen.is_empty() {
            self.scheduled.push((fall_at, sfx::FALL));
        }
        if res.ceiling_dropped {
            self.dropped_at = Some(now);
            self.sounds.push(sfx::CEILING);
        }
        match res.end {
            Some(end) => {
                let last = if res.fallen.is_empty() {
                    removals_done
                } else {
                    fall_at + r.fall_max_ticks
                };
                self.finish(end, last);
            }
            None => {
                // No aiming while a smile, blast or escape is still showing;
                // after an escape also not while its cluster falls.
                let mut until = removals_done.max(now + r.settle_ticks);
                if res.escaped.is_some() && !res.fallen.is_empty() {
                    until = until.max(fall_at + r.fall_max_ticks);
                }
                if res.ceiling_dropped {
                    until = until.max(now + r.ceiling_slide_ticks);
                }
                self.phase = Phase::Settling(until);
            }
        }
    }

    fn finish(&mut self, end: End, last_event: u64) {
        self.end = Some(end);
        self.phase = Phase::Ending(last_event.max(self.ticks) + self.rules.ending_hold_ticks);
        self.plan = None;
    }

    /// The creature that shows fear: the plan's first hit, while aiming and
    /// in flight.
    pub fn target(&self) -> Option<CreatureId> {
        match self.phase {
            Phase::Aiming => self.plan.as_ref().and_then(ShotPlan::first_hit),
            Phase::Flight => self.flight.as_ref().and_then(|f| f.plan.first_hit()),
            _ => None,
        }
    }

    fn cues(&self) -> Vec<Cues> {
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
                Cues {
                    nervous: angry_dir.is_some(),
                    aimed: Some(c.id) == target,
                    angry_dir,
                    launcher_dir: vec2(to_launcher.x as f32, to_launcher.y as f32),
                    runner: runner_cue(c, angry_dir.is_some()),
                }
            })
            .collect()
    }

    fn update_moods(&mut self) {
        let cues = self.cues();
        for ((_, c), cue) in self.board.occupied_mut().zip(cues) {
            c.mood.step(c.kind, &cue, FIXED_DT);
        }
    }

    pub fn take_sounds(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sounds)
    }

    fn time(&self) -> f64 {
        self.ticks as f64 * FIXED_DT as f64
    }

    /// How far above its logical place the board is drawn while the
    /// ceiling slides down (0 when still). Cosmetic only.
    fn slide(&self) -> f32 {
        let Some(at) = self.dropped_at else {
            return 0.0;
        };
        let n = self.rules.ceiling_slide_ticks.max(1);
        let k = (self.ticks.saturating_sub(at)) as f32 / n as f32;
        if k >= 1.0 {
            return 0.0;
        }
        // Ease out: quick start, gentle landing.
        let eased = 1.0 - (1.0 - k).powi(3);
        -(self.board.layout.pitch() as f32) * (1.0 - eased)
    }

    pub fn draw(&self, v: &View) {
        let l = self.board.layout;
        let t = self.time();
        let r = l.r as f32;
        let shift = vec2(0.0, self.slide());
        self.draw_frame(v, shift.y);

        for ((cell, c), cue) in self.board.occupied().zip(self.cues()) {
            let pose = emotion::pose(c.kind, &cue, &c.mood, t, c.seed(), self.reduced_motion);
            let p = l.centre(cell);
            render::draw_creature(
                v,
                vec2(p.x as f32, p.y as f32) + shift,
                r,
                c.kind,
                c.runner,
                &pose,
                1.0,
            );
        }
        for a in &self.actors {
            self.draw_actor(v, a, r);
        }

        let at = vec2(LAUNCHER.x as f32, LAUNCHER.y as f32);
        let d = shot::direction(self.aim.angle);
        let calm = Cues::default();
        let loaded = Kind::Ordinary(self.loaded);
        let face = emotion::pose(loaded, &calm, &Mood::default(), t, 1, self.reduced_motion);
        let in_flight = self.flight.is_some();
        let ended = self.end.is_some();
        render::draw_launcher(
            v,
            at,
            vec2(d.x as f32, d.y as f32),
            r,
            (!in_flight && !ended).then_some((loaded, face)),
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
        if !ended {
            let next = Kind::Ordinary(self.next);
            let next_face = emotion::pose(next, &calm, &Mood::default(), t, 2, self.reduced_motion);
            let nr = (r * 0.7).min(18.0);
            let next_at = at + vec2(-80.0, 6.0);
            render::draw_creature(v, next_at, nr, next, false, &next_face, 1.0);
            v.text_centered(
                t!("round.next"),
                next_at.x,
                next_at.y - nr - 8.0,
                13.0,
                with_alpha(TEXT, 0.6),
            );
        }
        self.draw_hud(v);
    }

    /// The board area, the ceiling plate (which grows as it comes down) and
    /// the danger line.
    fn draw_frame(&self, v: &View, shift: f32) {
        let l = self.board.layout;
        let edge = with_alpha(NEON, 0.35);
        let (left, right) = (l.left as f32, l.right as f32);
        let top = super::geometry::INITIAL_CEILING_Y as f32;
        let ceiling = l.ceiling_y as f32 + shift;
        let danger = DANGER_Y as f32;
        v.rect(
            left,
            top,
            right - left,
            danger - top,
            macroquad::color::Color::new(0.0, 0.0, 0.0, 0.18),
        );
        // The plate: everything above the ceiling.
        let plate = macroquad::color::Color::new(0.16, 0.18, 0.30, 1.0);
        v.rect(left, top - 6.0, right - left, ceiling - top + 6.0, plate);
        let stripe = macroquad::color::Color::new(1.0, 0.75, 0.2, 0.55);
        let mut x = left;
        while x < right {
            v.line(
                vec2(x, ceiling - 5.0),
                vec2((x + 9.0).min(right), ceiling - 14.0),
                3.0,
                stripe,
            );
            x += 16.0;
        }
        v.line(vec2(left, ceiling), vec2(right, ceiling), 3.0, NEON);
        v.line(vec2(left, top), vec2(left, danger), 2.0, edge);
        v.line(vec2(right, top), vec2(right, danger), 2.0, edge);
        let warn = macroquad::color::Color::new(1.0, 0.35, 0.35, 0.55);
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

    fn draw_actor(&self, v: &View, a: &Actor, r: f32) {
        let secs = |from: u64| (self.ticks as f32 - from as f32) * FIXED_DT;
        let tick = |n: u64| n as f32 * FIXED_DT;
        match a.act {
            Act::Pop { at } => {
                let since = secs(at);
                let pop = tick(self.rules.pop_present_ticks);
                if since < 0.0 {
                    return;
                }
                if since < pop {
                    let pose = emotion::pop_pose(a.kind, since);
                    render::draw_creature(v, a.from, r, a.kind, a.runner, &pose, 1.0);
                } else {
                    let c = render::body_color(a.kind);
                    render::draw_debris(v, a.from, r, c, (since - pop, 0.3), a.seed, self.quality);
                }
            }
            Act::Fall { at, vx } => {
                let since = secs(at);
                let s = since.max(0.0);
                if s > tick(self.rules.fall_max_ticks) {
                    return;
                }
                let pos = a.from + vec2(vx * s, 1800.0 * s * s * 0.5);
                if pos.y > 1000.0 + r {
                    return;
                }
                let pose = if since < 0.0 {
                    emotion::pop_pose(a.kind, 0.0)
                } else {
                    emotion::fall_pose(a.kind, s, a.seed, self.reduced_motion)
                };
                render::draw_creature(v, pos, r, a.kind, a.runner, &pose, 1.0);
            }
            Act::Escape { at, dir } => {
                let s = secs(at).max(0.0);
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
                if pos.x > -2.0 * r && pos.x < FIELD_W + 2.0 * r {
                    render::draw_creature(v, pos, r, a.kind, true, &pose, 1.0);
                    let trail = with_alpha(TEXT, 0.5);
                    for k in [-0.4, 0.0, 0.4] {
                        let y = pos.y + k * r;
                        let x = pos.x - dir * 1.1 * r;
                        v.line(vec2(x, y), vec2(x - dir * 0.9 * r, y), 2.0, trail);
                    }
                }
            }
            Act::Blast {
                smile_from,
                burst_at,
                source,
            } => {
                let since = secs(burst_at);
                if since < 0.0 {
                    // Logically gone already; still shown until its burst.
                    let t = self.time();
                    let calm = Cues::default();
                    let base = emotion::pose(
                        a.kind,
                        &calm,
                        &Mood::default(),
                        t,
                        a.seed,
                        self.reduced_motion,
                    );
                    let pose = if !source {
                        emotion::pop_pose(a.kind, 0.0)
                    } else if self.ticks >= smile_from {
                        emotion::blast_smile(base)
                    } else {
                        base
                    };
                    render::draw_creature(v, a.from, r, a.kind, a.runner, &pose, 1.0);
                } else {
                    let life = tick(self.rules.blast_present_ticks);
                    let c = render::body_color(a.kind);
                    render::draw_debris(v, a.from, r, c, (since, life), a.seed, self.quality);
                    if source {
                        render::draw_blast(
                            v,
                            a.from,
                            r,
                            (since, life),
                            self.reduced_flashes,
                            self.quality,
                        );
                    }
                }
            }
        }
    }

    fn draw_hud(&self, v: &View) {
        v.text(
            &t!("round.level", level = self.level),
            20.0,
            40.0,
            26.0,
            NEON,
        );
        let score = t!("round.score", score = self.score);
        let w = v.text_width(&score, 26.0);
        v.text(&score, FIELD_W - 20.0 - w, 40.0, 26.0, ACCENT);
        // Shots until the ceiling drops, as pips.
        let left = self.rules.ceiling_shots - self.since_drop;
        let label = t!("round.ceiling");
        let lw = v.text_width(label, 15.0);
        let n = self.rules.ceiling_shots as f32;
        let total = lw + 8.0 + n * 16.0;
        let x0 = FIELD_W / 2.0 - total / 2.0;
        v.text(label, x0, 72.0, 15.0, with_alpha(TEXT, 0.75));
        for i in 0..self.rules.ceiling_shots {
            let c = vec2(x0 + lw + 14.0 + i as f32 * 16.0, 67.0);
            let full = i < left;
            let col = if left == 1 && full { ERROR } else { ACCENT };
            if full {
                v.circle(c, 5.5, col);
            } else {
                v.circle_lines(c, 5.0, 1.5, with_alpha(TEXT, 0.4));
            }
        }
        if let Some(end) = self.end {
            let (title, colour) = match end {
                End::Won => (t!("round.won"), ACCENT),
                End::Lost(LossReason::Danger) => (t!("round.lost_danger"), ERROR),
                End::Lost(LossReason::Blocked) => (t!("round.lost_blocked"), ERROR),
                End::Lost(LossReason::Fault) => (t!("round.lost_fault"), ERROR),
            };
            v.rect(
                0.0,
                400.0,
                FIELD_W,
                130.0,
                macroquad::color::Color::new(0.02, 0.02, 0.06, 0.82),
            );
            v.text_centered(title, FIELD_W / 2.0, 455.0, 38.0, colour);
            let s = t!("round.final", score = self.score);
            v.text_centered(&s, FIELD_W / 2.0, 500.0, 24.0, TEXT);
        } else if let Some(hint) = self.hint().filter(|_| self.shots == 0) {
            // The level's lesson, until the first shot.
            v.text_wrapped_centered(hint, 900.0, 16.0, ACCENT);
        } else {
            let help = with_alpha(TEXT, 0.55);
            v.text_wrapped_centered(t!("round.help"), 922.0, 14.0, help);
        }
    }

    /// A curated level's short lesson, in words for what the player sees.
    pub fn hint(&self) -> Option<&'static str> {
        let Source::Curated(id) = &self.source else {
            return None;
        };
        Some(match id.as_str() {
            "first-grin" => t!("hint.first-grin"),
            "off-the-wall" => t!("hint.off-the-wall"),
            "angry-neighbours" => t!("hint.angry-neighbours"),
            "chain-reaction" => t!("hint.chain-reaction"),
            "runner-intro" => t!("hint.runner-intro"),
            "runner-panic" => t!("hint.runner-panic"),
            _ => return None,
        })
    }
}

/// How a creature's runner state shows: pulling towards its route, frantic
/// next to an Angry, or stuck.
fn runner_cue(c: &super::board::Creature, angry_next: bool) -> RunnerCue {
    if !c.runner {
        return RunnerCue::None;
    }
    match (c.struggle, angry_next) {
        (Some(s), true) => RunnerCue::Panic {
            dir: Some(s.dir as f32),
        },
        (Some(s), false) => RunnerCue::Struggle { dir: s.dir as f32 },
        (None, true) => RunnerCue::Panic { dir: None },
        (None, false) => RunnerCue::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustamove::shot::Segment;
    use crate::ui::Key;

    const HEX: Family = Family::Hex;
    const OVAL: Family = Family::Oval;

    /// A round on an 8-column board with `cells`.
    fn round(cells: &[(u16, u16, Family)]) -> Round {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        for &(r, c, f) in cells {
            b.place(Cell::new(r, c), Kind::Ordinary(f), false).unwrap();
        }
        Round::with_board(1, b, Rules::default(), 7, Source::Generated, false)
    }

    /// Lands a `family` shot in `cell` as if a plan had put it there.
    fn land(r: &mut Round, cell: Cell, family: Family) -> Resolution {
        r.loaded = family;
        let plan = ShotPlan {
            angle: 90.0,
            segments: vec![Segment {
                from: LAUNCHER,
                to: LAUNCHER,
            }],
            hit: Hit::Ceiling,
            attachment: Some(cell),
            travel: 0.0,
            revision: r.board.revision(),
        };
        r.resolve(&plan)
    }

    fn c(row: u16, col: u16) -> Cell {
        Cell::new(row, col)
    }

    /// R01: two of a kind stay; three go, including the shot.
    #[test]
    fn three_match_and_two_do_not() {
        let mut r = round(&[(0, 0, HEX), (0, 7, OVAL)]);
        let res = land(&mut r, c(0, 1), HEX);
        assert!(res.matched.is_empty());
        assert_eq!(r.board.count(), 3);
        let res = land(&mut r, c(0, 2), HEX);
        assert_eq!(res.matched, vec![c(0, 0), c(0, 1), c(0, 2)]);
        assert_eq!(res.points, 30);
        assert_eq!(r.board.count(), 1);
    }

    /// R01/R02: a group of five across row boundaries goes as a whole.
    #[test]
    fn whole_group_goes_across_rows() {
        let mut r = round(&[
            (0, 0, HEX),
            (0, 1, HEX),
            (1, 0, HEX),
            (2, 0, HEX),
            (0, 7, OVAL),
        ]);
        let res = land(&mut r, c(1, 1), HEX);
        assert_eq!(res.matched.len(), 5);
        assert_eq!(r.board.count(), 1);
    }

    /// R03: an existing triple elsewhere stays until a shot joins it.
    #[test]
    fn unrelated_triples_stay() {
        let mut r = round(&[(0, 5, OVAL), (0, 6, OVAL), (0, 7, OVAL), (0, 0, HEX)]);
        let res = land(&mut r, c(0, 1), Family::Round);
        assert!(res.matched.is_empty() && res.fallen.is_empty());
        assert_eq!(r.board.count(), 5);
    }

    /// R04/R07: a match that cuts a branch drops the whole branch once,
    /// for fall points.
    #[test]
    fn a_cut_branch_falls_once() {
        let mut r = round(&[
            (0, 1, OVAL),
            (0, 2, OVAL),
            (1, 2, HEX),
            (2, 2, Family::Round),
            (2, 3, Family::Round),
            (0, 7, Family::Block),
        ]);
        let res = land(&mut r, c(0, 0), OVAL);
        assert_eq!(res.matched.len(), 3);
        assert_eq!(res.fallen, vec![c(1, 2), c(2, 2), c(2, 3)]);
        assert_eq!(res.points, 3 * 10 + 3 * 20);
        assert_eq!(r.score, 90);
        assert_eq!(r.board.count(), 1);
        assert_eq!(res.gone.len(), 6, "each creature leaves once");
        assert_eq!(r.stats.biggest_fall, 3);
    }

    /// One pop for the group and one fall for the cluster, not one sound
    /// per creature.
    #[test]
    fn a_resolution_makes_one_sound_of_each_kind() {
        let mut r = round(&[
            (0, 1, OVAL),
            (0, 2, OVAL),
            (1, 2, HEX),
            (2, 2, Family::Round),
            (2, 3, Family::Round),
            (0, 7, Family::Block),
        ]);
        let res = land(&mut r, c(0, 0), OVAL);
        r.present(&res);
        assert_eq!(r.take_sounds(), vec![sfx::POP]);
        // The fall sounds when the cluster starts to fall, once.
        let mut later = Vec::new();
        for _ in 0..200 {
            r.update();
            later.extend(r.take_sounds());
        }
        assert_eq!(later, vec![sfx::FALL]);
        let res = land(&mut r, c(1, 6), Family::Round);
        r.present(&res);
        assert_eq!(r.take_sounds(), vec![sfx::ATTACH]);
    }

    /// A direct hit on the Angry in `cell`, as its plan would land.
    fn hit_angry(r: &mut Round, cell: Cell) -> Resolution {
        let id = r.board.get(cell).unwrap().id;
        let plan = ShotPlan {
            angle: 90.0,
            segments: vec![Segment {
                from: LAUNCHER,
                to: LAUNCHER,
            }],
            hit: Hit::Creature {
                id,
                cell,
                kind: Kind::Angry,
            },
            attachment: None,
            travel: 0.0,
            revision: r.board.revision(),
        };
        r.resolve(&plan)
    }

    fn round_kinds(cells: &[(u16, u16, Kind)]) -> Round {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        for &(row, col, k) in cells {
            b.place(Cell::new(row, col), k, false).unwrap();
        }
        Round::with_board(1, b, Rules::default(), 7, Source::Generated, false)
    }

    const ANGRY: Kind = Kind::Angry;
    const H: Kind = Kind::Ordinary(HEX);

    /// X01: a direct Angry hit explodes even when every cell around it is
    /// taken; the projectile is used up and scores nothing.
    #[test]
    fn a_boxed_in_angry_still_explodes() {
        let mut r = round_kinds(&[
            (0, 2, H),
            (0, 3, H),
            (1, 1, H),
            (1, 2, ANGRY),
            (1, 3, H),
            (2, 2, H),
            (2, 3, H),
            (0, 7, H),
        ]);
        let res = hit_angry(&mut r, c(1, 2));
        assert_eq!(res.blasted.len(), 7);
        assert!(res.attached.is_none());
        assert_eq!(
            res.points, 70,
            "ten per removed creature, nothing for the shot"
        );
        assert_eq!(r.board.count(), 1);
        assert_eq!(res.end, None);
        assert_eq!(r.shots, 1);
    }

    /// X04/X06 and R07: the whole chain goes before falls are worked out;
    /// what then hangs free falls once, for fall points.
    #[test]
    fn chains_resolve_before_falls_and_score_once() {
        let mut r = round_kinds(&[
            (0, 2, H),
            (1, 2, ANGRY),
            (1, 3, H),
            (2, 3, ANGRY),
            (2, 4, ANGRY),
            (2, 5, H),
            (3, 5, Kind::Ordinary(OVAL)),
            (4, 5, Kind::Ordinary(OVAL)),
            (0, 7, H),
            (1, 6, H),
            (2, 6, H),
        ]);
        let res = hit_angry(&mut r, c(2, 4));
        assert_eq!(res.blast_waves, 3);
        let mut blasted = res.blasted.clone();
        blasted.sort();
        assert_eq!(
            blasted,
            vec![c(0, 2), c(1, 2), c(1, 3), c(2, 3), c(2, 4), c(2, 5)]
        );
        // (3,5) hung from (2,5) and (2,6); (2,6) still holds it.
        assert!(res.fallen.is_empty());
        assert_eq!(res.points, 60);
        // Each removed creature appears once.
        let mut gone: Vec<Cell> = res.gone.iter().map(|g| g.cell).collect();
        gone.sort();
        let n = gone.len();
        gone.dedup();
        assert_eq!(gone.len(), n);
        assert_eq!(r.stats.longest_chain, 3);
    }

    /// X08: a blast that takes every ceiling anchor drops everything else.
    #[test]
    fn a_blast_through_the_anchors_drops_the_rest() {
        let mut r = round_kinds(&[(0, 3, H), (0, 4, ANGRY), (1, 3, H), (2, 3, H), (3, 3, H)]);
        let res = hit_angry(&mut r, c(0, 4));
        let mut blasted = res.blasted.clone();
        blasted.sort();
        assert_eq!(blasted, vec![c(0, 3), c(0, 4), c(1, 3)]);
        assert_eq!(res.fallen, vec![c(2, 3), c(3, 3)]);
        assert_eq!(res.points, 3 * 10 + 2 * 20);
        assert_eq!(res.end, Some(End::Won), "the board is empty");
    }

    /// R07/R08: an Angry next to a match stays; one cut loose by a match
    /// falls without exploding.
    #[test]
    fn angry_is_inert_unless_hit() {
        let mut r = round_kinds(&[(0, 0, H), (0, 1, H), (1, 1, ANGRY), (0, 7, H)]);
        let res = land(&mut r, c(1, 0), HEX);
        assert_eq!(res.matched.len(), 3);
        assert!(res.blasted.is_empty());
        assert_eq!(res.fallen, vec![c(1, 1)], "the Angry falls, inert");
        assert_eq!(res.points, 3 * 10 + 20);
        let mut r = round_kinds(&[(0, 0, H), (0, 1, H), (0, 2, ANGRY), (1, 2, H), (0, 7, H)]);
        let res = land(&mut r, c(1, 0), HEX);
        assert_eq!(res.matched.len(), 3);
        assert!(r.board.get(c(0, 2)).is_some(), "a supported Angry stays");
        assert!(res.blasted.is_empty());
    }

    /// X03 in the game: radius 2 from the rules reaches further.
    #[test]
    fn the_rules_set_the_blast_radius() {
        let cells = [
            (0, 0, H),
            (1, 0, H),
            (2, 0, H),
            (3, 0, ANGRY),
            (4, 0, H),
            (5, 0, H),
            (0, 7, H),
        ];
        let mut r = round_kinds(&cells);
        let res = hit_angry(&mut r, c(3, 0));
        assert_eq!(res.blasted.len(), 3);
        let mut r = round_kinds(&cells);
        r.rules.explosion_radius = 2;
        let res = hit_angry(&mut r, c(3, 0));
        assert_eq!(res.blasted.len(), 5);
    }

    /// The chain is shown wave by wave: no aiming until the last burst has
    /// played, one blast sound and bounded accents, all at their bursts.
    #[test]
    fn blasts_lock_input_until_shown_and_sound_per_wave() {
        let mut r = round_kinds(&[
            (0, 2, H),
            (1, 2, ANGRY),
            (1, 3, H),
            (2, 3, ANGRY),
            (2, 4, ANGRY),
            (0, 7, H),
            (1, 6, H),
        ]);
        let res = hit_angry(&mut r, c(2, 4));
        r.present(&res);
        let rules = r.rules;
        let last_burst = 2 * rules.blast_wave_ticks + rules.blast_smile_ticks;
        let mut sounds = Vec::new();
        let mut aiming_at = None;
        for t in 1..400u64 {
            r.update();
            for s in r.take_sounds() {
                sounds.push((t, s));
            }
            if aiming_at.is_none() && r.is_aiming() {
                aiming_at = Some(t);
            }
        }
        assert!(aiming_at.unwrap() >= last_burst + rules.blast_present_ticks);
        let kinds: Vec<Sfx> = sounds.iter().map(|&(_, s)| s).collect();
        assert_eq!(kinds, vec![sfx::BLAST, sfx::CHAIN, sfx::CHAIN]);
        assert_eq!(
            sounds[0].0, rules.blast_smile_ticks,
            "the blast sounds at the burst"
        );
    }

    /// A round with runners (`true`) and others on an 8-column board.
    fn round_runners(cells: &[(u16, u16, Kind, bool)]) -> Round {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        for &(row, col, k, runner) in cells {
            b.place(Cell::new(row, col), k, runner).unwrap();
        }
        Round::with_board(1, b, Rules::default(), 7, Source::Generated, false)
    }

    const RUN: Kind = Kind::Ordinary(Family::Oval);

    /// Runs `n` aiming ticks without firing; returns the tick (1-based) on
    /// which the runner in `cell` left, if it did.
    fn tick_until_gone(r: &mut Round, cell: Cell, n: u32) -> Option<u32> {
        for t in 1..=n {
            r.update();
            if r.board.get(cell).is_none_or(|c| !c.runner) {
                return Some(t);
            }
        }
        None
    }

    /// E01/E05: boxed in, with or without an Angry beside it: it stays,
    /// frantic next to the Angry, and never passes through anything.
    #[test]
    fn a_boxed_in_runner_stays() {
        let mut r = round_runners(&[(0, 2, H, false), (0, 3, RUN, true), (0, 4, H, false)]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 500), None);
        assert_eq!(r.board.get(c(0, 3)).unwrap().struggle, None);
        // A staggered neighbour below the left path blocks it as well.
        let mut r = round_runners(&[
            (1, 2, H, false),
            (0, 2, H, false),
            (0, 3, RUN, true),
            (0, 4, H, false),
            (1, 3, ANGRY, false),
        ]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 500), None);
        let cue = runner_cue(r.board.get(c(0, 3)).unwrap(), true);
        assert_eq!(cue, RunnerCue::Panic { dir: None });
    }

    /// E02/E03: it struggles towards the open side for 90 aiming ticks
    /// (still there on tick 89) and leaves on tick 90, for 10 points; not
    /// a shot.
    #[test]
    fn an_open_runner_struggles_then_leaves() {
        // Left open (the right is blocked by the anchor at column 7).
        let mut r = round_runners(&[(0, 3, RUN, true), (0, 7, H, false)]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 89), None);
        assert_eq!(r.board.get(c(0, 3)).unwrap().struggle.unwrap().dir, -1);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 1), Some(1), "tick 90");
        assert_eq!(r.score, 10);
        assert_eq!((r.shots, r.since_drop), (0, 0), "not a shot");
        assert_eq!(r.stats.escaped, 1);
        // Only the right open: it goes right.
        let mut r = round_runners(&[(0, 0, H, false), (0, 3, RUN, true)]);
        tick_until_gone(&mut r, c(0, 3), 1);
        assert_eq!(r.board.get(c(0, 3)).unwrap().struggle.unwrap().dir, 1);
    }

    /// E04: next to an Angry it leaves on its first open tick.
    #[test]
    fn a_runner_next_to_angry_leaves_at_once() {
        let mut r = round_runners(&[(0, 3, RUN, true), (1, 3, ANGRY, false), (0, 4, H, false)]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 5), Some(1));
    }

    /// E06/E14: a closed route resets the struggle and the other side is
    /// taken; the last creature escaping wins, after its dash and the hold.
    #[test]
    fn a_closed_route_restarts_and_the_last_escape_wins() {
        let mut r = round_runners(&[(0, 3, RUN, true)]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 50), None);
        assert_eq!(
            r.board.get(c(0, 3)).unwrap().struggle.unwrap(),
            super::super::board::Struggle { dir: -1, ticks: 50 }
        );
        // Something now blocks the left (as a fall or attachment could).
        r.board.place(c(0, 0), H, false);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 89), None);
        assert_eq!(r.board.get(c(0, 3)).unwrap().struggle.unwrap().dir, 1);
        // Remove the blocker again so that leaving empties the board.
        r.board.remove(c(0, 0));
        assert_eq!(
            tick_until_gone(&mut r, c(0, 3), 1),
            Some(1),
            "the right route was kept"
        );
        assert_eq!(r.end, Some(End::Won));
        assert_eq!(r.finished(), None, "the dash and the hold come first");
        for _ in 0..(r.rules.escape_present_ticks + r.rules.ending_hold_ticks + 2) {
            r.update();
        }
        assert_eq!(r.finished().map(|f| f.1), Some(Outcome::Won));
    }

    /// E07: firing on the tick the runner would leave: the shot wins and
    /// the runner stays put through the whole flight.
    #[test]
    fn a_shot_beats_a_leaving_runner() {
        // Aim straight at the runner; 89 open ticks pass while aiming.
        let mut r = round_runners(&[(0, 3, RUN, true), (0, 7, H, false)]);
        let d = r.board.layout.centre(c(0, 3)) - LAUNCHER;
        r.aim.set((-d.y).atan2(d.x).to_degrees());
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 89), None);
        // Tick 90 would let it go; a shot on that tick comes first.
        fire(&mut r);
        assert!(r.flight.is_some());
        assert_eq!(
            r.target(),
            Some(r.board.get(c(0, 3)).unwrap().id),
            "afraid and staying"
        );
        while r.flight.is_some() {
            r.update();
            assert!(
                r.board.get(c(0, 3)).is_some_and(|c| c.runner),
                "no escape in flight"
            );
        }
    }

    /// E08: a route opened by a shot is not used until the shot's effects
    /// have played and aiming resumes; then the full struggle follows.
    #[test]
    fn a_route_opened_by_a_shot_waits_for_aiming() {
        let mut r = round_runners(&[
            (0, 1, H, false),
            (0, 2, H, false),
            (0, 3, RUN, true),
            (0, 4, H, false),
            (0, 7, H, false),
        ]);
        let res = land(&mut r, c(0, 0), HEX);
        assert_eq!(res.matched.len(), 3, "the left blockers go");
        r.present(&res);
        let mut aiming_from = None;
        let mut left_at = None;
        for t in 1..400u32 {
            r.update();
            if aiming_from.is_none() && r.is_aiming() {
                aiming_from = Some(t);
            }
            if left_at.is_none() && r.board.get(c(0, 3)).is_none() {
                left_at = Some(t);
            }
        }
        let (a, l) = (aiming_from.unwrap(), left_at.unwrap());
        assert!(
            l >= a + r.rules.runner_struggle_ticks - 1,
            "aiming from {a}, left at {l}"
        );
    }

    /// E09/E10: of two ready runners the older leaves first; if that drops
    /// the other, it falls (20 points), it does not escape.
    #[test]
    fn runners_leave_one_at_a_time_and_fallen_ones_do_not_escape() {
        let mut r = round_runners(&[(0, 1, RUN, true), (0, 6, RUN, true)]);
        assert_eq!(tick_until_gone(&mut r, c(0, 1), 90), Some(90));
        assert!(r.board.get(c(0, 6)).is_some(), "the other waits its turn");
        let more = tick_until_gone(&mut r, c(0, 6), 200).unwrap();
        assert!(
            more > r.rules.escape_present_ticks as u32 - 1,
            "after the first dash: {more}"
        );
        assert_eq!(r.stats.escaped, 2);

        let mut r = round_runners(&[(0, 3, RUN, true), (1, 3, RUN, true)]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 90), Some(90));
        assert!(r.board.get(c(1, 3)).is_none(), "the lower runner fell");
        assert_eq!(r.stats.escaped, 1);
        assert_eq!(r.score, 10 + 20);
        assert_eq!(r.end, Some(End::Won));
    }

    /// E11/X07: a runner is an ordinary family member: matched normally;
    /// and a blast takes it like anything else.
    #[test]
    fn runners_match_and_blast_like_their_family() {
        let mut r = round_runners(&[
            (0, 1, Kind::Ordinary(HEX), true),
            (0, 2, H, false),
            (0, 7, H, false),
        ]);
        let res = land(&mut r, c(0, 3), HEX);
        assert_eq!(res.matched, vec![c(0, 1), c(0, 2), c(0, 3)]);
        assert_eq!(r.stats.escaped, 0);
        let mut r = round_runners(&[(0, 3, RUN, true), (1, 3, ANGRY, false), (0, 7, H, false)]);
        let res = hit_angry(&mut r, c(1, 3));
        assert!(res.blasted.contains(&c(0, 3)));
        r.present(&res);
        for _ in 0..300 {
            r.update();
        }
        assert!(r.board.get(c(0, 3)).is_none());
        assert_eq!(r.stats.escaped, 0, "a blasted runner never escapes");
    }

    /// E12: no panic remembered: once the Angry is gone, the normal delay.
    #[test]
    fn panic_is_not_remembered() {
        let mut r = round_runners(&[
            (0, 2, H, false),
            (0, 3, RUN, true),
            (1, 3, ANGRY, false),
            (0, 7, H, false),
        ]);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 20), None, "blocked");
        r.board.remove(c(1, 3));
        r.board.remove(c(0, 2));
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 89), None);
        assert_eq!(tick_until_gone(&mut r, c(0, 3), 1), Some(1));
    }

    /// Each taught level has its hint until the first shot; generated
    /// boards have none.
    #[test]
    fn taught_levels_have_hints() {
        for level in 1..=6 {
            let r = Round::new(level, Layout::new(8, 1.0).unwrap(), false);
            assert!(r.hint().is_some(), "level {level}");
        }
        assert!(Round::new(9, Layout::new(8, 1.0).unwrap(), false)
            .hint()
            .is_none());
        assert!(Round::new(1, Layout::new(12, 1.0).unwrap(), false)
            .hint()
            .is_none());
    }

    /// The first runner levels start with the runner boxed in: nothing
    /// happens until the player opens a way.
    #[test]
    fn runner_levels_start_blocked() {
        for level in [5, 6] {
            let mut r = Round::new(level, Layout::new(8, 1.0).unwrap(), false);
            let (cell, _) = r.board.occupied().find(|(_, c)| c.runner).unwrap();
            assert_eq!(tick_until_gone(&mut r, cell, 600), None, "level {level}");
            let angry = r.angry_next_to(cell);
            assert_eq!(angry, level == 6, "level 6 puts it next to an Angry");
        }
    }

    /// R05/R06: touching a wall does not hold; separate anchored parts stay.
    #[test]
    fn walls_do_not_hold_and_separate_anchors_do() {
        let mut r = round(&[(0, 0, HEX), (0, 5, OVAL), (1, 5, OVAL)]);
        let res = land(&mut r, c(1, 0), Family::Round);
        assert!(res.fallen.is_empty());
        assert_eq!(r.board.count(), 4);
        // Now remove the left anchor's support by matching it away: the
        // creature hanging at the wall falls.
        let mut r = round(&[
            (0, 1, HEX),
            (0, 2, HEX),
            (1, 0, Family::Round),
            (0, 7, OVAL),
        ]);
        let res = land(&mut r, c(0, 0), HEX);
        assert_eq!(res.matched.len(), 3);
        assert_eq!(res.fallen, vec![c(1, 0)], "the wall holds nothing");
    }

    /// F04: the shot that clears the board wins, even when the ceiling was
    /// due to drop.
    #[test]
    fn a_clearing_shot_wins_before_the_ceiling_drops() {
        let mut r = round(&[(0, 0, HEX), (0, 1, HEX)]);
        r.since_drop = r.rules.ceiling_shots - 1;
        let ceiling = r.board.layout.ceiling_y;
        let res = land(&mut r, c(0, 2), HEX);
        assert_eq!(res.end, Some(End::Won));
        assert!(!res.ceiling_dropped);
        assert_eq!(r.board.layout.ceiling_y, ceiling);
    }

    /// F05/R11: every fifth shot lowers the ceiling one row pitch; rows keep
    /// their parity; nothing new appears.
    #[test]
    fn every_fifth_shot_drops_the_ceiling() {
        let mut r = round(&[(0, 0, HEX), (0, 7, OVAL)]);
        let pitch = r.board.layout.pitch();
        let ceiling = r.board.layout.ceiling_y;
        let x = r.board.layout.centre(c(1, 0)).x;
        // Five shots into separate cells, never three of a kind together.
        let shots = [
            Family::Round,
            Family::Block,
            Family::Diamond,
            Family::Triangle,
            Family::Round,
        ];
        for (i, f) in shots.into_iter().enumerate() {
            let res = land(&mut r, c(1, i as u16), f);
            assert!(res.matched.is_empty());
            assert_eq!(res.ceiling_dropped, i == 4, "shot {}", i + 1);
        }
        assert!((r.board.layout.ceiling_y - ceiling - pitch).abs() < 1e-9);
        assert_eq!(r.board.layout.centre(c(1, 0)).x, x, "parity unchanged");
        assert_eq!(r.since_drop, 0);
        assert_eq!(r.board.count(), 7, "no new creatures");
        assert_eq!(r.shots, 5);
    }

    /// F07: reaching the danger line loses, after an attachment or after the
    /// ceiling drop.
    #[test]
    fn the_danger_line_loses() {
        let l = Layout::new(8, 1.0).unwrap();
        let deepest_safe = (0..l.rows)
            .rev()
            .find(|&row| l.centre(c(row, 0)).y + l.r < DANGER_Y)
            .unwrap();
        // A column down to the deepest safe row, then one more shot below.
        let mut cells: Vec<(u16, u16, Family)> = (0..=deepest_safe)
            .map(|row| (row, 0, if row % 2 == 0 { HEX } else { OVAL }))
            .collect();
        cells.push((0, 7, Family::Block));
        let mut r = round(&cells);
        assert!(!r.board.reaches_danger());
        let res = land(&mut r, c(deepest_safe + 1, 0), Family::Round);
        assert_eq!(res.end, Some(End::Lost(LossReason::Danger)));
        // Or the ceiling pushes the board over the line.
        let mut r = round(&cells);
        r.since_drop = r.rules.ceiling_shots - 1;
        let res = land(&mut r, c(0, 4), Family::Round);
        assert!(res.ceiling_dropped);
        assert_eq!(res.end, Some(End::Lost(LossReason::Danger)));
    }

    /// F08: no free cell: lost, without a far-away placement.
    #[test]
    fn a_blocked_shot_loses() {
        let mut r = round(&[(0, 0, HEX), (0, 7, OVAL)]);
        let id = r.board.get(c(0, 0)).unwrap().id;
        let plan = ShotPlan {
            angle: 90.0,
            segments: vec![Segment {
                from: LAUNCHER,
                to: LAUNCHER,
            }],
            hit: Hit::Creature {
                id,
                cell: c(0, 0),
                kind: Kind::Ordinary(HEX),
            },
            attachment: None,
            travel: 0.0,
            revision: r.board.revision(),
        };
        let res = r.resolve(&plan);
        assert_eq!(res.end, Some(End::Lost(LossReason::Blocked)));
        assert_eq!(r.board.count(), 2, "nothing placed");
    }

    /// R13: ammunition only comes from families still on the board.
    #[test]
    fn ammunition_follows_the_board() {
        let mut r = round(&[(0, 0, HEX), (0, 1, HEX), (0, 6, OVAL), (0, 7, OVAL)]);
        land(&mut r, c(0, 2), HEX);
        for _ in 0..20 {
            assert_eq!((r.loaded, r.next), (OVAL, OVAL));
            r.next = r.draw_family();
        }
    }

    fn fire(r: &mut Round) {
        r.handle(&Input {
            keys: vec![Key::Space],
            ..Default::default()
        });
        r.update();
    }

    fn settle(r: &mut Round) {
        for _ in 0..3000 {
            if r.is_aiming() || r.end.is_some() {
                return;
            }
            r.update();
        }
        panic!("never settled");
    }

    /// Plays a whole round with a fixed list of angles; returns everything
    /// that matters for the outcome.
    fn play(
        columns: u16,
        level: u32,
        angles: &[f64],
        draw_poses: bool,
    ) -> (u64, Option<End>, u32, Vec<(Cell, Kind)>) {
        let mut r = Round::new(level, Layout::new(columns, 1.0).unwrap(), false);
        for &a in angles {
            if r.end.is_some() {
                break;
            }
            r.aim.set(a);
            r.update();
            if draw_poses {
                // Evaluating poses must not change anything.
                let _ = r.cues();
            }
            fire(&mut r);
            settle(&mut r);
        }
        let cells = r.board.occupied().map(|(c, x)| (c, x.kind)).collect();
        (r.score, r.end, r.shots, cells)
    }

    /// Same level, settings and inputs give the same round, whether or not
    /// poses are evaluated in between.
    #[test]
    fn rounds_are_deterministic() {
        let mut rng = Rng::new(99);
        let angles: Vec<f64> = (0..60)
            .map(|_| 15.0 + 150.0 * rng.below(1000) as f64 / 1000.0)
            .collect();
        for (columns, level) in [(8, 1), (8, 3), (12, 5), (6, 2)] {
            let a = play(columns, level, &angles, false);
            let b = play(columns, level, &angles, true);
            assert_eq!(a, b, "{columns} columns, level {level}");
            assert!(a.2 > 0);
        }
    }

    /// A round always ends: random play reaches a win or a loss, and the
    /// result is then stable (F10) and delayed by the ending hold (F14).
    #[test]
    fn random_play_ends_and_the_result_is_stable() {
        let mut rng = Rng::new(3);
        let mut endings = Vec::new();
        for (columns, level) in [(8, 1), (8, 2), (8, 3), (6, 4), (18, 2)] {
            let mut r = Round::new(level, Layout::new(columns, 1.0).unwrap(), false);
            for _ in 0..400 {
                if r.end.is_some() {
                    break;
                }
                r.aim.set(15.0 + 150.0 * rng.below(1000) as f64 / 1000.0);
                r.update();
                fire(&mut r);
                settle(&mut r);
            }
            let end = r.end.expect("the round ended");
            endings.push(end);
            assert_eq!(r.finished(), None, "held before the result");
            let mut ticks = 0;
            while r.finished().is_none() {
                r.update();
                ticks += 1;
                assert!(ticks < 1000);
            }
            assert!(ticks >= r.rules.ending_hold_ticks as u32 - 1);
            let result = r.finished();
            for _ in 0..50 {
                fire(&mut r);
            }
            assert_eq!(r.finished(), result, "stable");
            let outcome = result.unwrap().1;
            assert_eq!(outcome == Outcome::Won, end == End::Won);
        }
        assert!(
            endings.iter().any(|e| matches!(e, End::Lost(_))),
            "{endings:?}"
        );
    }

    /// F01: a press while the shot flies or settles is dropped.
    #[test]
    fn presses_outside_aiming_are_discarded() {
        let mut r = Round::new(1, Layout::new(8, 1.0).unwrap(), false);
        fire(&mut r);
        assert!(r.flight.is_some());
        fire(&mut r); // during flight
        settle(&mut r);
        for _ in 0..60 {
            r.update();
        }
        assert!(r.flight.is_none());
        assert_eq!(r.shots, 1, "no delayed autofire");
    }

    /// The level-1 board can be cleared: aim at the diamonds' free cell.
    #[test]
    fn level_one_can_be_won() {
        // Brute force over angles: shoot, keep the best, until cleared.
        let mut r = Round::new(1, Layout::new(8, 1.0).unwrap(), false);
        for _ in 0..80 {
            if r.end.is_some() {
                break;
            }
            // Pick the angle whose landing cell would make the biggest
            // match with the loaded family.
            let mut best = (0usize, 90.0);
            for i in 0..=320 {
                let a = 10.0 + i as f64 * 0.5;
                let Ok(p) = shot::plan(&r.board, a) else {
                    continue;
                };
                let Some(cell) = p.attachment else { continue };
                let mut b = r.board.clone();
                b.place(cell, Kind::Ordinary(r.loaded), false);
                let probe = Round::with_board(1, b, r.rules, 0, Source::Generated, false);
                let n = probe.match_group(cell).len();
                if n > best.0 {
                    best = (n, a);
                }
            }
            r.aim.set(best.1);
            r.update();
            fire(&mut r);
            settle(&mut r);
        }
        assert_eq!(r.end, Some(End::Won), "score {} shots {}", r.score, r.shots);
    }

    /// The angle with the most points for the loaded creature (ties: the
    /// shorter flight), worked out with the real resolution on a copy; with
    /// `avoid_angry`, never a direct Angry hit.
    fn best_angle_avoiding(r: &Round, avoid_angry: bool) -> f64 {
        let mut best: Option<(u64, f64, f64)> = None;
        for i in 0..=320 {
            let a = 10.0 + i as f64 * 0.5;
            let Ok(p) = shot::plan(&r.board, a) else {
                continue;
            };
            if avoid_angry
                && matches!(
                    p.hit,
                    Hit::Creature {
                        kind: Kind::Angry,
                        ..
                    }
                )
            {
                continue;
            }
            if p.attachment.is_none()
                && p.first_hit().is_some()
                && !matches!(
                    p.hit,
                    Hit::Creature {
                        kind: Kind::Angry,
                        ..
                    }
                )
            {
                continue; // blocked
            }
            let mut probe = r.clone();
            let res = probe.resolve(&p);
            if matches!(res.end, Some(End::Lost(_))) {
                continue;
            }
            let better = best.is_none_or(|(pts, travel, _)| {
                res.points > pts || (res.points == pts && p.travel < travel)
            });
            if better {
                best = Some((res.points, p.travel, a));
            }
        }
        best.map_or(90.0, |b| b.2)
    }

    /// Frames a scripted tour waits after a click at `angle` (see
    /// `scripts/runner_tour.txt`): long enough for a flat shot's bounces.
    fn tour_wait(angle: f64) -> u64 {
        (60.0 * (760.0 / angle.to_radians().sin()) / 960.0) as u64 + 60
    }

    /// Plays `level` like the scripted tours do, tick for tick (91 ticks
    /// before the first shot, then 2 × (wait + 22) ticks between shots),
    /// choosing the best angle at each shot, so runners that leave while the
    /// player aims are part of it. Returns the angles and the round.
    fn scripted_play(level: u32) -> (Vec<f64>, Round) {
        let mut r = Round::new(level, Layout::new(8, 1.0).unwrap(), false);
        for _ in 0..91 {
            r.update();
        }
        let mut angles = Vec::new();
        for _ in 0..60 {
            if r.end.is_some() {
                break;
            }
            // Level 6 shows the runner's panic, so do not blow it up.
            let a = best_angle_avoiding(&r, level == 6);
            angles.push(a);
            r.aim.set(a);
            fire(&mut r);
            for _ in 0..(2 * (tour_wait(a) + 22) - 1) {
                r.update();
            }
        }
        (angles, r)
    }

    /// Prints the angle sequences for the scripted tours:
    /// `cargo test tour_angles -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn tour_angles() {
        for level in 1..=7 {
            let (angles, r) = scripted_play(level);
            println!(
                "LEVEL {level} {:?} score {} chain {} escaped {}: {:?}",
                r.end, r.score, r.stats.longest_chain, r.stats.escaped, angles
            );
        }
    }
}
