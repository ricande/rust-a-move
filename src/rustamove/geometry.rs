//! Board layout: which cells exist, where their centres are in the logical
//! 540 × 960 field and which cells are neighbours. See
//! docs/game/03_TECHNICAL_DESIGN.md §2 and docs/game/05_CONFIGURATION_CONTENT.md.
//!
//! Rows alternate between N cells (even rows) and N − 1 cells (odd rows,
//! shifted by one radius), so every cell has at most six neighbours. All
//! creatures share one circular collider of radius `r`, whatever their
//! drawn shape.

use macroquad::math::{dvec2, DVec2};

/// Smallest and largest number of cells in a long row.
pub const MIN_COLUMNS: u16 = 6;
pub const MAX_COLUMNS: u16 = 18;
/// Smallest and largest board zoom.
pub const MIN_ZOOM: f64 = 0.75;
pub const MAX_ZOOM: f64 = 1.0;

/// Safe horizontal board area at zoom 1.
const SAFE_LEFT: f64 = 30.0;
const SAFE_WIDTH: f64 = 480.0;
#[cfg(test)]
const FIELD_CENTRE_X: f64 = 270.0;
/// Initial ceiling, danger line and launcher; fixed while the board scales.
pub const INITIAL_CEILING_Y: f64 = 100.0;
pub const DANGER_Y: f64 = 760.0;
pub const LAUNCHER: DVec2 = DVec2::new(270.0, 850.0);
/// Hard limit on board cells, whatever the layout says.
const MAX_CELLS: usize = 1024;

/// A board cell. Even rows have `columns` cells, odd rows one fewer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell {
    pub row: u16,
    pub col: u16,
}

impl Cell {
    pub const fn new(row: u16, col: u16) -> Self {
        Cell { row, col }
    }
}

/// Why a layout cannot be used.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutError {
    Columns(u16),
    Zoom(f64),
    TooManyCells(usize),
}

/// The geometry of one board: fixed for a round.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub columns: u16,
    pub zoom: f64,
    /// Left and right board edges (the projectile's bounce planes are one
    /// radius inside them).
    pub left: f64,
    pub right: f64,
    /// Collider radius of every creature.
    pub r: f64,
    /// Top of the board; the top row's colliders touch it.
    pub ceiling_y: f64,
    /// How many rows can be addressed, including two below the danger
    /// line so an attachment there can lose normally.
    pub rows: u16,
}

impl Layout {
    pub fn new(columns: u16, zoom: f64) -> Result<Layout, LayoutError> {
        if !(MIN_COLUMNS..=MAX_COLUMNS).contains(&columns) {
            return Err(LayoutError::Columns(columns));
        }
        if !zoom.is_finite() || !(MIN_ZOOM..=MAX_ZOOM).contains(&zoom) {
            return Err(LayoutError::Zoom(zoom));
        }
        let width = SAFE_WIDTH * zoom;
        let left = SAFE_LEFT + (SAFE_WIDTH - width) / 2.0;
        let r = width / (2.0 * columns as f64);
        let pitch = 3f64.sqrt() * r;
        let rows = ((DANGER_Y - INITIAL_CEILING_Y) / pitch).ceil() as u16 + 2;
        let layout = Layout {
            columns,
            zoom,
            left,
            right: left + width,
            r,
            ceiling_y: INITIAL_CEILING_Y,
            rows,
        };
        let cells = layout.cell_count();
        if cells > MAX_CELLS {
            return Err(LayoutError::TooManyCells(cells));
        }
        Ok(layout)
    }

    /// Vertical distance between row centres.
    pub fn pitch(&self) -> f64 {
        3f64.sqrt() * self.r
    }

    /// Number of cells in `row`.
    pub fn row_len(&self, row: u16) -> u16 {
        if row.is_multiple_of(2) {
            self.columns
        } else {
            self.columns - 1
        }
    }

    pub fn contains(&self, cell: Cell) -> bool {
        cell.row < self.rows && cell.col < self.row_len(cell.row)
    }

    pub fn cell_count(&self) -> usize {
        (0..self.rows).map(|r| self.row_len(r) as usize).sum()
    }

    /// Every cell, in row-major order.
    #[allow(dead_code)] // Board setup and validation use it from M2.
    pub fn cells(&self) -> impl Iterator<Item = Cell> + '_ {
        (0..self.rows).flat_map(move |row| (0..self.row_len(row)).map(move |col| Cell { row, col }))
    }

    /// Centre of `cell`, always computed from the layout (never
    /// accumulated), so it does not drift.
    pub fn centre(&self, cell: Cell) -> DVec2 {
        let shift = if cell.row % 2 == 1 { self.r } else { 0.0 };
        dvec2(
            self.left + self.r + 2.0 * self.r * cell.col as f64 + shift,
            self.ceiling_y + self.r + cell.row as f64 * self.pitch(),
        )
    }

    /// The up to six neighbours of `cell`, in row-major order.
    pub fn neighbours(&self, cell: Cell) -> Vec<Cell> {
        let (row, col) = (cell.row as i32, cell.col as i32);
        // Diagonal columns in the rows above and below.
        let diag = if row % 2 == 0 {
            [col - 1, col]
        } else {
            [col, col + 1]
        };
        let mut out = Vec::with_capacity(6);
        let mut push = |r: i32, c: i32| {
            if r >= 0 && c >= 0 {
                let cell = Cell::new(r as u16, c as u16);
                if self.contains(cell) {
                    out.push(cell);
                }
            }
        };
        for c in diag {
            push(row - 1, c);
        }
        push(row, col - 1);
        push(row, col + 1);
        for c in diag {
            push(row + 1, c);
        }
        out
    }

    /// Used for attachment from M2.
    #[allow(dead_code)]
    /// The cell whose centre is nearest to `p`; ties go to the smaller row,
    /// then the smaller column. `None` above the ceiling or outside the
    /// side edges.
    pub fn nearest_cell(&self, p: DVec2) -> Option<Cell> {
        if !(self.left..=self.right).contains(&p.x) || p.y < self.ceiling_y {
            return None;
        }
        let approx = ((p.y - self.ceiling_y - self.r) / self.pitch()).round() as i32;
        let mut best: Option<(f64, Cell)> = None;
        for row in (approx - 1).max(0)..=(approx + 1) {
            for col in 0..self.columns as i32 {
                let cell = Cell::new(row as u16, col as u16);
                if !self.contains(cell) {
                    continue;
                }
                let d = self.centre(cell).distance_squared(p);
                // Cells are visited in row-major order, so `<` keeps the
                // first of equally near cells.
                if best.is_none_or(|(bd, _)| d < bd - self.tolerance()) {
                    best = Some((d, cell));
                }
            }
        }
        best.map(|(_, c)| c)
    }

    /// Whether a creature in `from` can slide straight sideways (`dir` −1
    /// left, +1 right) until it is fully past the board edge without its
    /// collider overlapping any cell in `occupied` (rule R09). Touching
    /// exactly is allowed.
    pub fn route_clear(&self, from: Cell, dir: i8, occupied: &[Cell]) -> bool {
        let start = self.centre(from);
        let end_x = if dir < 0 {
            self.left - self.r
        } else {
            self.right + self.r
        };
        let (lo, hi) = (start.x.min(end_x), start.x.max(end_x));
        let limit = 2.0 * self.r - self.tolerance();
        occupied.iter().filter(|&&c| c != from).all(|&c| {
            let p = self.centre(c);
            let nearest = dvec2(p.x.clamp(lo, hi), start.y);
            p.distance(nearest) >= limit
        })
    }

    /// The exit a runner in `from` would take: the open side with the
    /// shorter way out, left on a tie; `None` when both are blocked.
    pub fn chosen_exit(&self, from: Cell, occupied: &[Cell]) -> Option<i8> {
        let x = self.centre(from).x;
        let left = self
            .route_clear(from, -1, occupied)
            .then_some(x - self.left);
        let right = self
            .route_clear(from, 1, occupied)
            .then_some(self.right - x);
        match (left, right) {
            (Some(l), Some(r)) => Some(if r < l - self.tolerance() { 1 } else { -1 }),
            (Some(_), None) => Some(-1),
            (None, Some(_)) => Some(1),
            (None, None) => None,
        }
    }

    /// Geometric tolerance for contacts and ties: relative to the diameter.
    pub fn tolerance(&self) -> f64 {
        1e-7 * 2.0 * self.r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_layouts() -> Vec<Layout> {
        let mut v = Vec::new();
        for columns in [6, 8, 12, 18] {
            for zoom in [0.75, 1.0] {
                v.push(Layout::new(columns, zoom).unwrap());
            }
        }
        v
    }

    #[test]
    fn rejects_bad_settings() {
        assert_eq!(Layout::new(5, 1.0), Err(LayoutError::Columns(5)));
        assert_eq!(Layout::new(19, 1.0), Err(LayoutError::Columns(19)));
        assert!(matches!(Layout::new(8, 0.5), Err(LayoutError::Zoom(_))));
        assert!(matches!(
            Layout::new(8, f64::NAN),
            Err(LayoutError::Zoom(_))
        ));
        assert!(Layout::new(8, 1.0).is_ok());
    }

    /// G01: neighbours are reciprocal, at most six, never wrap round, and
    /// are exactly the cells two radii away.
    #[test]
    fn neighbours_are_reciprocal_and_touching() {
        for l in all_layouts() {
            for cell in l.cells() {
                let n = l.neighbours(cell);
                assert!(n.len() <= 6);
                for &m in &n {
                    assert!(l.neighbours(m).contains(&cell), "{cell:?} ↔ {m:?}");
                    let d = l.centre(cell).distance(l.centre(m));
                    assert!((d - 2.0 * l.r).abs() < 1e-9, "touching");
                }
                // No other cell is that close.
                let close = l
                    .cells()
                    .filter(|&m| {
                        m != cell && l.centre(cell).distance(l.centre(m)) < 2.0 * l.r + 1e-6
                    })
                    .count();
                assert_eq!(close, n.len());
            }
        }
    }

    #[test]
    fn interior_cells_have_six_neighbours_and_edges_fewer() {
        let l = Layout::new(8, 1.0).unwrap();
        assert_eq!(l.neighbours(Cell::new(2, 3)).len(), 6);
        assert_eq!(l.neighbours(Cell::new(3, 3)).len(), 6);
        // Top-left corner: right neighbour and one below.
        assert_eq!(
            l.neighbours(Cell::new(0, 0)),
            vec![Cell::new(0, 1), Cell::new(1, 0)]
        );
        // Long-row edge cell: up, side, down.
        assert_eq!(l.neighbours(Cell::new(2, 0)).len(), 3);
        // Short-row edge cell has all its diagonals.
        assert_eq!(l.neighbours(Cell::new(1, 0)).len(), 5);
    }

    /// G02: every cell fits inside the board's side edges and the safe
    /// area, and the launcher is well below the danger line.
    #[test]
    fn layouts_fit_the_board() {
        for l in all_layouts() {
            assert!(l.left >= SAFE_LEFT - 1e-9 && l.right <= SAFE_LEFT + SAFE_WIDTH + 1e-9);
            assert!(
                ((l.left + l.right) / 2.0 - FIELD_CENTRE_X).abs() < 1e-9,
                "centred"
            );
            for cell in l.cells() {
                let c = l.centre(cell);
                assert!(c.x - l.r >= l.left - 1e-9 && c.x + l.r <= l.right + 1e-9);
            }
            // Top row touches the ceiling.
            assert!((l.centre(Cell::new(0, 0)).y - l.r - l.ceiling_y).abs() < 1e-9);
            // Long rows fill the width exactly.
            let last = l.centre(Cell::new(0, l.columns - 1));
            assert!((last.x + l.r - l.right).abs() < 1e-9);
            assert!(LAUNCHER.y - DANGER_Y > 2.0 * l.r);
            // The rows reach past the danger line.
            let bottom = l.centre(Cell::new(l.rows - 1, 0)).y;
            assert!(bottom > DANGER_Y);
            assert!(l.cell_count() <= MAX_CELLS);
        }
    }

    #[test]
    fn radius_follows_columns_and_zoom() {
        let l = Layout::new(8, 1.0).unwrap();
        assert!((l.r - 30.0).abs() < 1e-12);
        let l = Layout::new(6, 0.75).unwrap();
        assert!((l.r - 30.0).abs() < 1e-12);
        let l = Layout::new(18, 0.75).unwrap();
        assert!((l.r - 10.0).abs() < 1e-12);
    }

    /// E01/E02: the corridor counts staggered rows; the nearer exit wins,
    /// left on a tie.
    #[test]
    fn routes_check_the_whole_corridor() {
        let l = Layout::new(8, 1.0).unwrap();
        let runner = Cell::new(0, 3);
        // Alone in the top row: both ways open; col 3 is nearer the left.
        assert_eq!(l.chosen_exit(runner, &[runner]), Some(-1));
        let mirrored = Cell::new(0, 4);
        assert_eq!(l.chosen_exit(mirrored, &[mirrored]), Some(1));
        // A same-row blocker far to the left blocks the left.
        let occ = [runner, Cell::new(0, 0)];
        assert_eq!(l.chosen_exit(runner, &occ), Some(1));
        // A staggered cell below the left path blocks it too …
        let occ = [runner, Cell::new(1, 1)];
        assert!(!l.route_clear(runner, -1, &occ));
        // … but the runner's own lower-right neighbour only blocks the right.
        let occ = [runner, Cell::new(1, 3)];
        assert!(l.route_clear(runner, -1, &occ));
        assert!(!l.route_clear(runner, 1, &occ));
        // Two rows down never touches the corridor.
        let occ = [runner, Cell::new(2, 1), Cell::new(2, 6)];
        assert_eq!(l.chosen_exit(runner, &occ), Some(-1));
        // Boxed in.
        let occ = [runner, Cell::new(0, 2), Cell::new(0, 4)];
        assert_eq!(l.chosen_exit(runner, &occ), None);
        // A tie between equally near exits goes left: the middle of an odd
        // row of a 7-wide short row (8 columns: row 1 has 7 cells).
        let mid = Cell::new(1, 3);
        assert_eq!(l.chosen_exit(mid, &[mid]), Some(-1));
    }

    /// G03: positions map back to their own cell; halfway points use the
    /// documented tie-break (smaller row, then smaller column).
    #[test]
    fn nearest_cell_maps_centres_and_breaks_ties() {
        for l in all_layouts() {
            for cell in l.cells() {
                assert_eq!(l.nearest_cell(l.centre(cell)), Some(cell));
            }
            let a = l.centre(Cell::new(0, 0));
            let b = l.centre(Cell::new(0, 1));
            assert_eq!(l.nearest_cell((a + b) / 2.0), Some(Cell::new(0, 0)));
            let below = l.centre(Cell::new(1, 0));
            assert_eq!(l.nearest_cell((a + below) / 2.0), Some(Cell::new(0, 0)));
            assert_eq!(l.nearest_cell(dvec2(l.left - 1.0, 300.0)), None);
            assert_eq!(l.nearest_cell(dvec2(270.0, l.ceiling_y - 1.0)), None);
        }
    }
}
