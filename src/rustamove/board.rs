//! The logical board: which creature sits in which cell. Cells are stored
//! row-major in a bounded `Vec`, so every traversal has a stable order (no
//! hash maps in the rules). `revision` changes with every change of
//! occupancy, so a shot plan can tell whether it is still current.

use super::emotion::{Kind, Mood};
use super::geometry::{Cell, Layout};

/// A creature's identity for its whole life on the board, given out in
/// increasing order per round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreatureId(pub u64);

#[derive(Clone, Debug, PartialEq)]
pub struct Creature {
    pub id: CreatureId,
    pub kind: Kind,
    /// Runner trait; only on ordinary creatures.
    pub runner: bool,
    /// A runner's current route and how long it has struggled on it.
    pub struggle: Option<Struggle>,
    /// Presentation only: never read by the rules.
    pub mood: Mood,
}

/// A runner pulling towards an open exit (rule R09).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Struggle {
    /// −1 left, +1 right.
    pub dir: i8,
    /// Aiming ticks spent on this route, this one included.
    pub ticks: u32,
}

impl Creature {
    /// Seed for cosmetic phases (blinks, tremor).
    pub fn seed(&self) -> u64 {
        self.id.0.wrapping_mul(7919)
    }
}

#[derive(Clone, Debug)]
pub struct Board {
    pub layout: Layout,
    cells: Vec<Option<Creature>>,
    next_id: u64,
    revision: u64,
}

impl Board {
    pub fn new(layout: Layout) -> Board {
        Board {
            cells: vec![None; layout.cell_count()],
            layout,
            next_id: 1,
            revision: 0,
        }
    }

    fn index(&self, cell: Cell) -> Option<usize> {
        if !self.layout.contains(cell) {
            return None;
        }
        // Rows before `cell.row`: pairs of a long and a short row.
        let r = cell.row as usize;
        let n = self.layout.columns as usize;
        Some((r / 2) * (2 * n - 1) + (r % 2) * n + cell.col as usize)
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn get(&self, cell: Cell) -> Option<&Creature> {
        self.index(cell).and_then(|i| self.cells[i].as_ref())
    }

    pub fn get_mut(&mut self, cell: Cell) -> Option<&mut Creature> {
        self.index(cell).and_then(|i| self.cells[i].as_mut())
    }

    pub fn is_empty(&self, cell: Cell) -> bool {
        self.layout.contains(cell) && self.get(cell).is_none()
    }

    /// Puts a new creature in an empty cell. Runner is ignored for Angry.
    pub fn place(&mut self, cell: Cell, kind: Kind, runner: bool) -> Option<CreatureId> {
        let i = self.index(cell)?;
        if self.cells[i].is_some() {
            return None;
        }
        let id = CreatureId(self.next_id);
        self.next_id += 1;
        self.cells[i] = Some(Creature {
            id,
            kind,
            runner: runner && kind != Kind::Angry,
            struggle: None,
            mood: Mood::default(),
        });
        self.revision += 1;
        Some(id)
    }

    #[allow(dead_code)] // Matches and blasts remove creatures from M3.
    pub fn remove(&mut self, cell: Cell) -> Option<Creature> {
        let i = self.index(cell)?;
        let c = self.cells[i].take();
        if c.is_some() {
            self.revision += 1;
        }
        c
    }

    /// Occupied cells and their creatures, in row-major order.
    pub fn occupied(&self) -> impl Iterator<Item = (Cell, &Creature)> + '_ {
        self.layout
            .cells()
            .zip(self.cells.iter())
            .filter_map(|(cell, c)| c.as_ref().map(|c| (cell, c)))
    }

    pub fn occupied_mut(&mut self) -> impl Iterator<Item = (Cell, &mut Creature)> + '_ {
        let layout = self.layout;
        layout
            .cells()
            .collect::<Vec<_>>()
            .into_iter()
            .zip(self.cells.iter_mut())
            .filter_map(|(cell, c)| c.as_mut().map(|c| (cell, c)))
    }

    #[allow(dead_code)] // Used by tests now, by the rules from M3.
    pub fn cell_of(&self, id: CreatureId) -> Option<Cell> {
        self.occupied()
            .find(|(_, c)| c.id == id)
            .map(|(cell, _)| cell)
    }

    /// Lowers the ceiling, and with it the whole board, by one row pitch
    /// (rule R11). Rows and their parity stay; only the origin moves.
    pub fn lower_ceiling(&mut self) {
        self.layout.ceiling_y += self.layout.pitch();
        self.revision += 1;
    }

    /// Whether any collider's bottom reaches the danger line (rule R11).
    pub fn reaches_danger(&self) -> bool {
        let l = &self.layout;
        self.occupied()
            .any(|(cell, _)| l.centre(cell).y + l.r >= super::geometry::DANGER_Y)
    }

    /// Occupied cells connected to an occupied top-row cell (rule R07),
    /// in row-major order.
    pub fn anchored(&self) -> Vec<Cell> {
        let mut seen = vec![false; self.cells.len()];
        let mut queue: Vec<Cell> = Vec::new();
        for (cell, _) in self.occupied().filter(|(c, _)| c.row == 0) {
            seen[self.index(cell).unwrap()] = true;
            queue.push(cell);
        }
        let mut i = 0;
        while i < queue.len() {
            for n in self.layout.neighbours(queue[i]) {
                let k = self.index(n).unwrap();
                if !seen[k] && self.cells[k].is_some() {
                    seen[k] = true;
                    queue.push(n);
                }
            }
            i += 1;
        }
        let mut out: Vec<Cell> = queue;
        out.sort();
        out
    }

    #[allow(dead_code)] // Used by tests now, by the rules from M3.
    pub fn count(&self) -> usize {
        self.cells.iter().filter(|c| c.is_some()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustamove::emotion::Family;

    #[test]
    fn index_covers_every_cell_once() {
        for columns in [6, 7, 8, 18] {
            let b = Board::new(Layout::new(columns, 1.0).unwrap());
            let idx: Vec<usize> = b.layout.cells().map(|c| b.index(c).unwrap()).collect();
            assert_eq!(idx, (0..b.layout.cell_count()).collect::<Vec<_>>());
            assert_eq!(b.index(Cell::new(1, columns - 1)), None, "short row");
        }
    }

    #[test]
    fn anchored_follows_the_top_row() {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        let k = Kind::Ordinary(Family::Oval);
        for (r, c) in [(0, 0), (1, 0), (2, 0), (2, 5), (3, 5), (0, 7)] {
            b.place(Cell::new(r, c), k, false);
        }
        let a = b.anchored();
        assert_eq!(
            a,
            vec![
                Cell::new(0, 0),
                Cell::new(0, 7),
                Cell::new(1, 0),
                Cell::new(2, 0)
            ]
        );
    }

    #[test]
    fn lowering_the_ceiling_moves_everything_one_pitch() {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        b.place(Cell::new(1, 2), Kind::Ordinary(Family::Hex), false);
        let before = b.layout.centre(Cell::new(1, 2));
        let rev = b.revision();
        b.lower_ceiling();
        let after = b.layout.centre(Cell::new(1, 2));
        assert!((after.y - before.y - b.layout.pitch()).abs() < 1e-9);
        assert_eq!(after.x, before.x, "parity kept");
        assert_eq!(b.revision(), rev + 1, "plans must be remade");
    }

    #[test]
    fn danger_counts_the_collider_bottom() {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        let l = b.layout;
        // The last row whose bottom is still above the line.
        let safe = (0..l.rows)
            .rev()
            .find(|&r| l.centre(Cell::new(r, 0)).y + l.r < super::super::geometry::DANGER_Y)
            .unwrap();
        b.place(Cell::new(safe, 0), Kind::Ordinary(Family::Hex), false);
        assert!(!b.reaches_danger());
        b.place(Cell::new(safe + 1, 0), Kind::Ordinary(Family::Hex), false);
        assert!(b.reaches_danger());
    }

    #[test]
    fn place_and_remove_track_ids_and_revision() {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        let k = Kind::Ordinary(Family::Hex);
        let a = b.place(Cell::new(0, 1), k, false).unwrap();
        let c = b.place(Cell::new(2, 3), Kind::Angry, true).unwrap();
        assert!(a < c);
        assert!(
            !b.get(Cell::new(2, 3)).unwrap().runner,
            "Angry is never a runner"
        );
        assert_eq!(b.place(Cell::new(0, 1), k, false), None, "never overwrites");
        assert_eq!(b.revision(), 2);
        assert_eq!(b.cell_of(c), Some(Cell::new(2, 3)));
        let order: Vec<CreatureId> = b.occupied().map(|(_, c)| c.id).collect();
        assert_eq!(order, vec![a, c]);
        assert!(b.remove(Cell::new(0, 1)).is_some());
        assert!(b.remove(Cell::new(0, 1)).is_none());
        assert_eq!((b.revision(), b.count()), (3, 1));
    }
}
