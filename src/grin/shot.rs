//! Shot prediction and flight: one calculation for both.
//!
//! `plan` traces the projectile's centre from the launcher with continuous
//! collision (no frame-by-frame overlap tests): rays against every occupied
//! collider expanded to 2r, the ceiling plane at `ceiling_y + r` and the side
//! planes at `left + r` / `right − r`, reflecting at the sides until the
//! first creature or the ceiling. The same `ShotPlan` decides which creature
//! is afraid while aiming and where the fired projectile stops, so fear and
//! impact cannot disagree (rule R03). See docs/game/03_TECHNICAL_DESIGN.md
//! §3–§4.
//!
//! Policies fixed here (and tested):
//! - A tangent touch counts as contact, within `Layout::tolerance`.
//! - Simultaneous contacts (within the tolerance): a creature beats the
//!   ceiling, the ceiling beats a side wall (a corner hit stops at the
//!   ceiling), and between creatures the smaller `CreatureId` wins.
//! - Attachment after a creature hit: the nearest empty neighbour cell of
//!   the struck creature whose direction is at most 60° from the contact
//!   direction (one step round the struck creature) and whose short snap
//!   path stays clear of every other creature; ties by row, then column.
//!   After a ceiling hit: the nearest empty top-row cell at most one
//!   diameter away along the ceiling with a clear snap; ties by column.
//!   No legal cell means the board is blocked (`attachment == None`).
//! - A direct hit on Angry needs no attachment; the caller decides (M4).

use macroquad::math::{dvec2, DVec2};

use super::board::{Board, CreatureId};
use super::emotion::Kind;
use super::geometry::{Cell, LAUNCHER};

/// Smallest and largest aim angle, degrees counter-clockwise from
/// screen-right with up positive (rule R02).
pub const AIM_MIN: f64 = 10.0;
pub const AIM_MAX: f64 = 170.0;
/// Diagnostic bound on reflections. Every supported layout needs far fewer
/// (tested); reaching it is a geometry fault, never a silent truncation.
pub const MAX_SEGMENTS: usize = 128;
/// Projectile speed in collider diameters per second (05_CONFIGURATION).
pub const SPEED_DIAMETERS: f64 = 16.0;

/// The unit direction of an aim angle in field coordinates (y down).
pub fn direction(angle_deg: f64) -> DVec2 {
    let a = angle_deg.to_radians();
    dvec2(a.cos(), -a.sin())
}

/// What the projectile reaches first.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Creature {
        id: CreatureId,
        cell: Cell,
        kind: Kind,
    },
    Ceiling,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub from: DVec2,
    pub to: DVec2,
}

impl Segment {
    pub fn length(&self) -> f64 {
        self.from.distance(self.to)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShotPlan {
    pub angle: f64,
    /// The centre's path; consecutive segments meet at side-wall bounces.
    pub segments: Vec<Segment>,
    pub hit: Hit,
    /// Where the projectile settles; `None` when no legal cell exists
    /// (blocked board) or the hit is a direct Angry hit.
    pub attachment: Option<Cell>,
    /// Total path length.
    pub travel: f64,
    /// `Board::revision` the plan was made for.
    pub revision: u64,
}

impl ShotPlan {
    /// The physical first creature hit: the one that shows fear.
    pub fn first_hit(&self) -> Option<CreatureId> {
        match self.hit {
            Hit::Creature { id, .. } => Some(id),
            Hit::Ceiling => None,
        }
    }

    #[allow(dead_code)] // Used by tests now, by the rules from M3.
    pub fn bounces(&self) -> usize {
        self.segments.len() - 1
    }

    /// The projectile centre after travelling `s` along the path.
    pub fn position_at(&self, s: f64) -> DVec2 {
        if s >= self.travel {
            return self.segments.last().map_or(LAUNCHER, |s| s.to);
        }
        let mut left = s.max(0.0);
        for seg in &self.segments {
            let len = seg.length();
            if left <= len {
                return seg.from + (seg.to - seg.from) * (left / len.max(f64::MIN_POSITIVE));
            }
            left -= len;
        }
        self.segments.last().map_or(LAUNCHER, |s| s.to)
    }

    /// How many bounces lie within the first `s` of the path.
    pub fn bounces_before(&self, s: f64) -> usize {
        let mut at = 0.0;
        let mut n = 0;
        for seg in &self.segments[..self.segments.len() - 1] {
            at += seg.length();
            if at <= s {
                n += 1;
            }
        }
        n
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlanError {
    /// More than `MAX_SEGMENTS` reflections: a geometry fault.
    TooManyBounces,
}

/// Distance from `p` to the segment `a`–`b`.
fn segment_distance(a: DVec2, b: DVec2, p: DVec2) -> f64 {
    let ab = b - a;
    let len2 = ab.length_squared();
    let t = if len2 == 0.0 {
        0.0
    } else {
        ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
    };
    p.distance(a + ab * t)
}

#[derive(Clone, Copy)]
enum Event {
    Creature(CreatureId, Cell, Kind),
    Ceiling,
    Wall,
}

impl Event {
    /// Tie order: creature, ceiling, wall; creatures by id.
    fn rank(&self) -> (u8, u64) {
        match self {
            Event::Creature(id, ..) => (0, id.0),
            Event::Ceiling => (1, 0),
            Event::Wall => (2, 0),
        }
    }
}

/// Traces a shot at `angle` degrees against the current board.
pub fn plan(board: &Board, angle: f64) -> Result<ShotPlan, PlanError> {
    let l = &board.layout;
    let tol = l.tolerance();
    let reach = 2.0 * l.r;
    let mut p = LAUNCHER;
    let mut d = direction(angle.clamp(AIM_MIN, AIM_MAX));
    let mut segments = Vec::new();
    for _ in 0..MAX_SEGMENTS {
        let mut best: Option<(f64, Event)> = None;
        let mut offer = |t: f64, e: Event| {
            let better = match best {
                None => true,
                Some((bt, be)) => t < bt - tol || (t <= bt + tol && e.rank() < be.rank()),
            };
            if better {
                best = Some((t, e));
            }
        };
        for (cell, c) in board.occupied() {
            let m = l.centre(cell) - p;
            let m2 = m.length_squared();
            let b = m.dot(d);
            if m2 < reach * reach {
                // Already touching (only possible at the launcher).
                offer(0.0, Event::Creature(c.id, cell, c.kind));
                continue;
            }
            let h2 = m2 - b * b;
            if b <= 0.0 || h2 > (reach + tol) * (reach + tol) {
                continue;
            }
            let t = b - (reach * reach - h2).max(0.0).sqrt();
            offer(t.max(0.0), Event::Creature(c.id, cell, c.kind));
        }
        if d.y < 0.0 {
            offer(((l.ceiling_y + l.r - p.y) / d.y).max(0.0), Event::Ceiling);
        }
        if d.x < 0.0 {
            offer(((l.left + l.r - p.x) / d.x).max(0.0), Event::Wall);
        } else if d.x > 0.0 {
            offer(((l.right - l.r - p.x) / d.x).max(0.0), Event::Wall);
        }
        // The aim range guarantees upward motion, so the ceiling is always
        // a candidate.
        let (t, event) = best.expect("an upward shot always reaches the ceiling");
        let mut q = p + d * t;
        match event {
            Event::Wall => {
                // Land exactly on the plane, then reflect.
                q.x = if d.x < 0.0 {
                    l.left + l.r
                } else {
                    l.right - l.r
                };
                segments.push(Segment { from: p, to: q });
                p = q;
                d.x = -d.x;
            }
            Event::Ceiling => {
                q.y = l.ceiling_y + l.r;
                segments.push(Segment { from: p, to: q });
                return Ok(finish(
                    board,
                    angle,
                    segments,
                    Hit::Ceiling,
                    ceiling_attachment(board, q),
                ));
            }
            Event::Creature(id, cell, kind) => {
                segments.push(Segment { from: p, to: q });
                let attachment = if kind == Kind::Angry {
                    None
                } else {
                    creature_attachment(board, cell, q)
                };
                return Ok(finish(
                    board,
                    angle,
                    segments,
                    Hit::Creature { id, cell, kind },
                    attachment,
                ));
            }
        }
    }
    Err(PlanError::TooManyBounces)
}

fn finish(
    board: &Board,
    angle: f64,
    segments: Vec<Segment>,
    hit: Hit,
    attachment: Option<Cell>,
) -> ShotPlan {
    let travel = segments.iter().map(Segment::length).sum();
    ShotPlan {
        angle,
        segments,
        hit,
        attachment,
        travel,
        revision: board.revision(),
    }
}

/// Whether the projectile can slide from `q` to `to` without overlapping
/// any creature except `except` (whose own rule is checked by the caller).
fn snap_clear(board: &Board, q: DVec2, to: DVec2, except: Option<Cell>) -> bool {
    let l = &board.layout;
    let limit = 2.0 * l.r - l.tolerance();
    board
        .occupied()
        .filter(|(cell, _)| Some(*cell) != except)
        .all(|(cell, _)| segment_distance(q, to, l.centre(cell)) >= limit)
}

fn creature_attachment(board: &Board, struck: Cell, q: DVec2) -> Option<Cell> {
    let l = &board.layout;
    let tc = l.centre(struck);
    let u = (q - tc).normalize_or_zero();
    let mut best: Option<(f64, Cell)> = None;
    for cell in l.neighbours(struck) {
        if !board.is_empty(cell) {
            continue;
        }
        let cc = l.centre(cell);
        // At most one step round the struck creature: 60° (cos 0.5).
        if u.dot((cc - tc) / (2.0 * l.r)) < 0.5 - 1e-9 {
            continue;
        }
        if !snap_clear(board, q, cc, Some(struck)) {
            continue;
        }
        let d = cc.distance_squared(q);
        // Neighbours come in row-major order: strict `<` keeps the first.
        if best.is_none_or(|(bd, _)| d < bd - l.tolerance() * l.r) {
            best = Some((d, cell));
        }
    }
    best.map(|(_, c)| c)
}

fn ceiling_attachment(board: &Board, q: DVec2) -> Option<Cell> {
    let l = &board.layout;
    let mut best: Option<(f64, Cell)> = None;
    for col in 0..l.row_len(0) {
        let cell = Cell::new(0, col);
        let cc = l.centre(cell);
        let dx = (cc.x - q.x).abs();
        if !board.is_empty(cell) || dx > 2.0 * l.r + l.tolerance() {
            continue;
        }
        if !snap_clear(board, q, cc, None) {
            continue;
        }
        if best.is_none_or(|(bd, _)| dx < bd - l.tolerance()) {
            best = Some((dx, cell));
        }
    }
    best.map(|(_, c)| c)
}

/// A fired projectile: the plan it follows and how far it has come.
#[derive(Clone, Debug, PartialEq)]
pub struct Flight {
    pub plan: ShotPlan,
    pub travelled: f64,
}

impl Flight {
    pub fn new(plan: ShotPlan) -> Flight {
        Flight {
            plan,
            travelled: 0.0,
        }
    }

    /// Moves `distance` along the path, through as many bounces as fit.
    /// Returns the number of bounces passed and whether it arrived.
    pub fn advance(&mut self, distance: f64) -> (usize, bool) {
        let before = self.plan.bounces_before(self.travelled);
        self.travelled = (self.travelled + distance).min(self.plan.travel);
        let after = self.plan.bounces_before(self.travelled);
        (after - before, self.travelled >= self.plan.travel)
    }

    pub fn position(&self) -> DVec2 {
        self.plan.position_at(self.travelled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grin::emotion::{Family, FAMILIES};
    use crate::grin::geometry::Layout;
    use crate::grin::rng::Rng;

    const HEX: Kind = Kind::Ordinary(Family::Hex);

    fn board(columns: u16, zoom: f64, cells: &[(u16, u16)]) -> Board {
        let mut b = Board::new(Layout::new(columns, zoom).unwrap());
        for &(r, c) in cells {
            b.place(Cell::new(r, c), HEX, false).unwrap();
        }
        b
    }

    /// The angle that sends the projectile straight at `p`.
    fn angle_to(p: DVec2) -> f64 {
        let d = p - LAUNCHER;
        (-d.y).atan2(d.x).to_degrees()
    }

    fn random_board(rng: &mut Rng, columns: u16, zoom: f64) -> Board {
        let mut b = Board::new(Layout::new(columns, zoom).unwrap());
        let rows = 3 + rng.below(6) as u16;
        for row in 0..rows {
            for col in 0..b.layout.row_len(row) {
                if rng.below(10) < 6 {
                    let kind = if rng.below(12) == 0 {
                        Kind::Angry
                    } else {
                        Kind::Ordinary(FAMILIES[rng.below(6) as usize])
                    };
                    b.place(Cell::new(row, col), kind, false);
                }
            }
        }
        b
    }

    /// A reference: march the centre in tiny steps, reflecting at the
    /// sides, and report the first overlap. Slow but obviously right.
    fn march(b: &Board, angle: f64) -> Hit {
        let l = &b.layout;
        let h = l.r / 200.0;
        let reach2 = 4.0 * l.r * l.r;
        // Smallest id first, so the first overlap found wins a tie.
        let mut all: Vec<(CreatureId, Cell, Kind, DVec2)> = b
            .occupied()
            .map(|(cell, c)| (c.id, cell, c.kind, l.centre(cell)))
            .collect();
        all.sort_by_key(|x| x.0);
        let mut p = LAUNCHER;
        let mut d = direction(angle);
        loop {
            p += d * h;
            if p.x < l.left + l.r {
                p.x = 2.0 * (l.left + l.r) - p.x;
                d.x = -d.x;
            } else if p.x > l.right - l.r {
                p.x = 2.0 * (l.right - l.r) - p.x;
                d.x = -d.x;
            }
            if let Some(&(id, cell, kind, _)) =
                all.iter().find(|x| x.3.distance_squared(p) < reach2)
            {
                return Hit::Creature { id, cell, kind };
            }
            if p.y < l.ceiling_y + l.r {
                return Hit::Ceiling;
            }
        }
    }

    /// Whether some other creature comes within a hair of touching the
    /// path (where marching and exact contact may legitimately disagree).
    fn grazes(b: &Board, plan: &ShotPlan) -> bool {
        let l = &b.layout;
        b.occupied().any(|(cell, c)| {
            Some(c.id) != plan.first_hit()
                && plan.segments.iter().any(|s| {
                    let d = segment_distance(s.from, s.to, l.centre(cell));
                    (d - 2.0 * l.r).abs() < 0.03 * l.r
                })
        }) || plan.segments.iter().any(|s| {
            (s.to.y - (l.ceiling_y + l.r)).abs() < 0.03 * l.r && plan.first_hit().is_some()
        })
    }

    /// G04/G05: the planned first contact is what a fine-step simulation of
    /// the same flight hits, straight or after any number of bounces.
    #[test]
    fn plan_matches_a_marched_flight() {
        let mut rng = Rng::new(2026);
        let mut checked = 0;
        let mut bounced = 0;
        for (columns, zoom) in [(6, 1.0), (8, 1.0), (12, 0.75), (18, 1.0), (18, 0.75)] {
            for _ in 0..30 {
                let b = random_board(&mut rng, columns, zoom);
                for _ in 0..10 {
                    let angle = AIM_MIN + (AIM_MAX - AIM_MIN) * (rng.below(1_000_000) as f64 / 1e6);
                    let p = plan(&b, angle).unwrap();
                    if grazes(&b, &p) {
                        continue;
                    }
                    assert_eq!(
                        march(&b, angle),
                        p.hit,
                        "{columns} cols, zoom {zoom}, angle {angle}"
                    );
                    checked += 1;
                    bounced += (p.bounces() > 0) as u32;
                }
            }
        }
        assert!(checked > 1200, "{checked}");
        assert!(bounced > 300, "{bounced}");
    }

    /// Parity and symmetry: a mirrored board and mirrored aim give the
    /// mirrored hit and path.
    #[test]
    fn mirrored_boards_give_mirrored_shots() {
        let mut rng = Rng::new(5);
        for columns in [6, 7, 8, 18] {
            let l = Layout::new(columns, 1.0).unwrap();
            for _ in 0..10 {
                let b = random_board(&mut rng, columns, 1.0);
                let mut m = Board::new(l);
                for (cell, c) in b.occupied() {
                    let mirrored = Cell::new(cell.row, l.row_len(cell.row) - 1 - cell.col);
                    m.place(mirrored, c.kind, false);
                }
                for _ in 0..5 {
                    let angle = 12.0 + 156.0 * (rng.below(10_000) as f64 / 1e4);
                    let (p, q) = (plan(&b, angle).unwrap(), plan(&m, 180.0 - angle).unwrap());
                    if grazes(&b, &p) {
                        continue;
                    }
                    let cell = |h: Hit| match h {
                        Hit::Creature { cell, .. } => Some(cell),
                        Hit::Ceiling => None,
                    };
                    let mirror = |c: Cell| Cell::new(c.row, l.row_len(c.row) - 1 - c.col);
                    assert_eq!(cell(p.hit).map(mirror), cell(q.hit));
                    assert_eq!(p.bounces(), q.bounces());
                    assert!((p.travel - q.travel).abs() < 1e-6);
                    let end = p.segments.last().unwrap().to;
                    let mend = q.segments.last().unwrap().to;
                    assert!((end.x - (l.left + l.right - mend.x)).abs() < 1e-6);
                }
            }
        }
    }

    #[test]
    fn direct_shot_hits_and_attaches_below() {
        let b = board(8, 1.0, &[(0, 3), (0, 4)]);
        let target = b.layout.centre(Cell::new(0, 3));
        let p = plan(&b, angle_to(target)).unwrap();
        assert_eq!(p.bounces(), 0);
        let id = b.get(Cell::new(0, 3)).unwrap().id;
        assert_eq!(p.first_hit(), Some(id));
        // Coming from below, it settles in one of the cells under the target.
        assert!(
            matches!(p.attachment, Some(c) if c.row == 1 && (c.col == 2 || c.col == 3)),
            "{:?}",
            p.attachment
        );
        // The stop point touches the target exactly.
        let end = p.segments[0].to;
        assert!((end.distance(target) - 2.0 * b.layout.r).abs() < 1e-6);
    }

    /// G06: a tangent graze counts; just missing does not.
    #[test]
    fn tangent_touch_counts_as_contact() {
        let b = board(8, 1.0, &[(0, 3)]);
        let l = b.layout;
        let c = l.centre(Cell::new(0, 3));
        // A vertical shot whose centre line passes exactly 2r to the side.
        // Aim is fixed at the launcher, so find the angle whose ray passes
        // at distance 2r from c: solve by bisection on the miss distance.
        let miss = |angle: f64| {
            let d = direction(angle);
            let m = c - LAUNCHER;
            (m - d * m.dot(d)).length()
        };
        let (mut lo, mut hi) = (angle_to(c), AIM_MIN);
        for _ in 0..200 {
            let mid = (lo + hi) / 2.0;
            if miss(mid) < 2.0 * l.r {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let tangent = lo;
        assert!((miss(tangent) - 2.0 * l.r).abs() < 1e-9);
        assert!(
            plan(&b, tangent).unwrap().first_hit().is_some(),
            "tangent counts"
        );
        let further = tangent - 0.01;
        assert!(miss(further) > 2.0 * l.r + 1e-3);
        let p = plan(&b, further).unwrap();
        assert!(
            p.first_hit().is_none() || p.bounces() > 0,
            "a clear miss on the direct path"
        );
    }

    /// G06: two creatures touched at the same moment: the smaller id wins.
    #[test]
    fn simultaneous_contacts_go_to_the_smaller_id() {
        // Two top-row creatures side by side, shot straight up between them:
        // both are first touched at the same moment.
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        let right = b.place(Cell::new(0, 4), HEX, false).unwrap();
        let left = b.place(Cell::new(0, 3), HEX, false).unwrap();
        assert!(right < left);
        let between = (b.layout.centre(Cell::new(0, 3)) + b.layout.centre(Cell::new(0, 4))) / 2.0;
        assert!(
            (between.x - LAUNCHER.x).abs() < 1e-9,
            "the launcher is in the middle"
        );
        let p = plan(&b, 90.0).unwrap();
        assert_eq!(p.first_hit(), Some(right));
        // It settles below, in the short row cell between them.
        assert_eq!(p.attachment, Some(Cell::new(1, 3)));
    }

    /// G10: a ceiling hit attaches locally into a top-row gap.
    #[test]
    fn ceiling_hit_attaches_in_the_nearest_gap() {
        let l = Layout::new(8, 1.0).unwrap();
        let mut cells: Vec<(u16, u16)> = (0..8)
            .filter(|&c| c != 3 && c != 4)
            .map(|c| (0, c))
            .collect();
        cells.push((1, 0));
        let b = board(8, 1.0, &cells);
        let p = plan(&b, 90.0).unwrap();
        assert_eq!(p.hit, Hit::Ceiling);
        // Straight up from x = 270: halfway between columns 3 and 4: the
        // smaller column wins the tie.
        assert_eq!(p.attachment, Some(Cell::new(0, 3)));
        let end = p.segments.last().unwrap().to;
        assert!((end.y - (l.ceiling_y + l.r)).abs() < 1e-9);
    }

    /// G11/F08: when no neighbour cell within reach exists, the shot is
    /// blocked rather than placed somewhere far away.
    #[test]
    fn blocked_when_no_local_cell_exists() {
        let l = Layout::new(18, 1.0).unwrap();
        let mut b = Board::new(l);
        // Only the last row: hit from below, its lower neighbours do not
        // exist, its side neighbours are taken and the cells above are more
        // than one step round the struck creature.
        let last = l.rows - 1;
        for col in 0..l.row_len(last) {
            b.place(Cell::new(last, col), HEX, false);
        }
        let below_launcher = l.centre(Cell::new(last, 0)).y < LAUNCHER.y - 2.0 * l.r;
        assert!(below_launcher, "the last row is above the launcher");
        let p = plan(&b, 90.0).unwrap();
        assert!(
            matches!(p.hit, Hit::Creature { cell, .. } if cell.row == last),
            "{:?}",
            p.hit
        );
        assert_eq!(p.attachment, None);
    }

    /// G09 and X01: Angry first means no attachment and nothing behind it
    /// is the target.
    #[test]
    fn angry_first_hides_what_is_behind() {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        b.place(Cell::new(0, 3), HEX, false);
        let angry = b.place(Cell::new(1, 3), Kind::Angry, false).unwrap();
        let p = plan(&b, angle_to(b.layout.centre(Cell::new(1, 3)))).unwrap();
        assert_eq!(p.first_hit(), Some(angry));
        assert_eq!(p.attachment, None, "a direct Angry hit needs no cell");
    }

    /// Attachments never overwrite and always touch the struck creature.
    #[test]
    fn attachments_are_local_and_empty() {
        let mut rng = Rng::new(11);
        for (columns, zoom) in [(6, 1.0), (8, 0.75), (18, 1.0)] {
            for _ in 0..20 {
                let b = random_board(&mut rng, columns, zoom);
                for _ in 0..10 {
                    let angle = AIM_MIN + 160.0 * (rng.below(100_000) as f64 / 1e5);
                    let p = plan(&b, angle).unwrap();
                    let Some(cell) = p.attachment else { continue };
                    assert!(b.is_empty(cell));
                    match p.hit {
                        Hit::Creature { cell: struck, .. } => {
                            assert!(b.layout.neighbours(struck).contains(&cell));
                        }
                        Hit::Ceiling => assert_eq!(cell.row, 0),
                    }
                }
            }
        }
    }

    /// G15: even the flattest shot on the narrowest board needs few bounces.
    #[test]
    fn flattest_shots_stay_well_inside_the_bounce_limit() {
        for (columns, zoom) in [(6, 0.75), (18, 0.75), (6, 1.0), (18, 1.0)] {
            let b = Board::new(Layout::new(columns, zoom).unwrap());
            for angle in [AIM_MIN, AIM_MAX] {
                let p = plan(&b, angle).unwrap();
                assert_eq!(p.hit, Hit::Ceiling);
                assert!(p.bounces() < 24, "{} bounces", p.bounces());
                assert!(p.travel.is_finite());
            }
        }
    }

    /// G07/G08: however fast the flight, it passes every bounce and stops
    /// once, at the planned contact.
    #[test]
    fn flight_passes_bounces_and_stops_exactly_once() {
        let b = board(8, 1.0, &[(0, 0), (0, 1)]);
        let p = plan(&b, 15.0).unwrap();
        assert!(p.bounces() >= 2);
        for step in [0.5, 4.0, 100.0, 5000.0] {
            let mut f = Flight::new(p.clone());
            let (mut bounces, mut arrivals) = (0, 0);
            for _ in 0..100_000 {
                let (n, arrived) = f.advance(step);
                bounces += n;
                if arrived {
                    arrivals += 1;
                    break;
                }
            }
            assert_eq!((bounces, arrivals), (p.bounces(), 1), "step {step}");
            assert_eq!(f.position(), p.segments.last().unwrap().to);
        }
    }

    #[test]
    fn angles_are_clamped_and_finite() {
        let b = Board::new(Layout::new(8, 1.0).unwrap());
        for a in [-30.0, 0.0, 5.0, 175.0, 200.0] {
            let p = plan(&b, a).unwrap();
            assert!(p.travel.is_finite());
            assert!(p.segments.iter().all(|s| s.to.is_finite()));
        }
    }
}
