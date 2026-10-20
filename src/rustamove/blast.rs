//! Angry blasts and chain reactions (rule R06), as one pure calculation on a
//! snapshot of the board.
//!
//! A direct hit on an Angry creature removes every occupied cell within the
//! configured hex-graph radius of it, the Angry included; the radius counts
//! steps between cells and crosses empty ones. Every Angry inside a blast
//! adds its own blast, in waves, until no new Angry is reached. Each
//! creature is removed once and each Angry explodes once, so the closure
//! always ends. Falls are worked out only afterwards, by the caller, so an
//! Angry that would have lost its support still explodes.

use super::board::Board;
use super::emotion::Kind;
use super::geometry::{Cell, Layout};

/// The outcome of a blast, before anything is removed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Blast {
    /// Every removed cell with the wave that removes it (0 = the struck
    /// Angry's own blast), in removal order: by wave, then row-major.
    pub removed: Vec<(Cell, u32)>,
    /// The exploding Angry creatures with the wave of their own blast.
    pub sources: Vec<(Cell, u32)>,
}

impl Blast {
    pub fn waves(&self) -> u32 {
        self.sources.iter().map(|&(_, w)| w + 1).max().unwrap_or(0)
    }

    pub fn contains(&self, cell: Cell) -> bool {
        self.removed.iter().any(|&(c, _)| c == cell)
    }
}

/// Cells within `radius` neighbour steps of `centre`, through empty cells
/// too, in row-major order (the centre included).
pub fn area(layout: &Layout, centre: Cell, radius: u8) -> Vec<Cell> {
    let mut seen = vec![centre];
    let mut frontier = vec![centre];
    for _ in 0..radius {
        let mut next = Vec::new();
        for &c in &frontier {
            for n in layout.neighbours(c) {
                if !seen.contains(&n) {
                    seen.push(n);
                    next.push(n);
                }
            }
        }
        frontier = next;
    }
    seen.sort();
    seen
}

/// The full chain started by a direct hit on the Angry in `struck`.
pub fn closure(board: &Board, struck: Cell, radius: u8) -> Blast {
    let mut blast = Blast::default();
    if board.get(struck).map(|c| c.kind) != Some(Kind::Angry) {
        return blast;
    }
    let l = &board.layout;
    let mut wave_sources = vec![struck];
    let mut wave = 0u32;
    blast.sources.push((struck, 0));
    while !wave_sources.is_empty() {
        let mut next: Vec<Cell> = Vec::new();
        let mut this_wave: Vec<Cell> = Vec::new();
        for &source in &wave_sources {
            for cell in area(l, source, radius) {
                let Some(c) = board.get(cell) else { continue };
                let known = blast.contains(cell) || this_wave.contains(&cell);
                if c.kind == Kind::Angry && !blast.sources.iter().any(|&(s, _)| s == cell) {
                    // Explodes in the next wave; it disappears with its own
                    // blast, after its smile.
                    blast.sources.push((cell, wave + 1));
                    next.push(cell);
                } else if !known && c.kind != Kind::Angry {
                    this_wave.push(cell);
                }
            }
        }
        // The Angry creatures of this wave go with their own blast.
        for &source in &wave_sources {
            if !this_wave.contains(&source) {
                this_wave.push(source);
            }
        }
        this_wave.sort();
        blast
            .removed
            .extend(this_wave.into_iter().map(|c| (c, wave)));
        // Stable order: the next wave's sources by creature id.
        next.sort_by_key(|c| board.get(*c).map(|x| x.id));
        wave_sources = next;
        wave += 1;
    }
    blast
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rustamove::emotion::Family;

    const HEX: Kind = Kind::Ordinary(Family::Hex);

    fn board(cells: &[(u16, u16, Kind)]) -> Board {
        let mut b = Board::new(Layout::new(8, 1.0).unwrap());
        for &(r, c, k) in cells {
            b.place(Cell::new(r, c), k, false).unwrap();
        }
        b
    }

    fn cells(blast: &Blast) -> Vec<Cell> {
        let mut v: Vec<Cell> = blast.removed.iter().map(|&(c, _)| c).collect();
        v.sort();
        v
    }

    fn c(r: u16, col: u16) -> Cell {
        Cell::new(r, col)
    }

    #[test]
    fn area_counts_graph_steps_through_empty_cells() {
        let l = Layout::new(8, 1.0).unwrap();
        assert_eq!(area(&l, c(3, 3), 0), vec![c(3, 3)]);
        assert_eq!(area(&l, c(3, 3), 1).len(), 7);
        assert_eq!(area(&l, c(3, 3), 2).len(), 19);
        // At the edge the area is cut, never wrapped.
        assert_eq!(area(&l, c(0, 0), 1), vec![c(0, 0), c(0, 1), c(1, 0)]);
    }

    /// X02: radius 1 in the open: the Angry and its occupied neighbours.
    #[test]
    fn radius_one_takes_the_angry_and_its_neighbours() {
        let b = board(&[
            (2, 3, HEX),
            (2, 4, HEX),
            (3, 2, HEX),
            (3, 3, Kind::Angry),
            (3, 4, HEX),
            (4, 3, HEX),
            (4, 4, HEX),
            (5, 3, HEX), // two steps away: stays
        ]);
        let blast = closure(&b, c(3, 3), 1);
        assert_eq!(
            cells(&blast),
            vec![
                c(2, 3),
                c(2, 4),
                c(3, 2),
                c(3, 3),
                c(3, 4),
                c(4, 3),
                c(4, 4)
            ]
        );
        assert_eq!(blast.sources, vec![(c(3, 3), 0)]);
        assert_eq!(blast.waves(), 1);
    }

    /// X03: radius 2 reaches across an empty cell.
    #[test]
    fn radius_two_crosses_empty_cells() {
        let b = board(&[
            (3, 3, Kind::Angry),
            (5, 3, HEX),
            (1, 3, HEX),
            (3, 1, HEX),
            (3, 0, HEX),
        ]);
        let blast = closure(&b, c(3, 3), 2);
        assert_eq!(cells(&blast), vec![c(1, 3), c(3, 1), c(3, 3), c(5, 3)]);
        let blast3 = closure(&b, c(3, 3), 3);
        assert!(blast3.contains(c(3, 0)), "radius 3 reaches further");
    }

    /// X04/X06: a chain of three goes off in waves on the snapshot, also
    /// an Angry that only the first blast cut loose.
    #[test]
    fn chains_go_off_in_waves() {
        let b = board(&[
            (2, 4, Kind::Angry),
            (2, 3, Kind::Angry),
            (1, 2, Kind::Angry),
            (1, 3, HEX),
            (0, 2, HEX),
            (2, 6, HEX),
        ]);
        let blast = closure(&b, c(2, 4), 1);
        assert_eq!(
            blast.sources,
            vec![(c(2, 4), 0), (c(2, 3), 1), (c(1, 2), 2)]
        );
        assert_eq!(blast.waves(), 3);
        let wave_of = |cell: Cell| {
            blast
                .removed
                .iter()
                .find(|&&(x, _)| x == cell)
                .map(|&(_, w)| w)
        };
        assert_eq!(wave_of(c(1, 3)), Some(0), "removed by the first blast");
        assert_eq!(
            wave_of(c(2, 3)),
            Some(1),
            "an Angry goes with its own blast"
        );
        assert_eq!(wave_of(c(0, 2)), Some(2));
        assert_eq!(wave_of(c(2, 6)), None, "out of every blast");
    }

    /// X05: overlapping and cyclic blasts (every Angry is next to several
    /// others): each creature once, every Angry once, and it ends.
    #[test]
    fn overlapping_blasts_remove_each_creature_once() {
        let l = Layout::new(8, 1.0).unwrap();
        let mut list = Vec::new();
        for r in 0..3u16 {
            for col in 0..l.row_len(r) {
                list.push((r, col, Kind::Angry));
            }
        }
        for col in 0..l.row_len(3) {
            list.push((3, col, HEX));
        }
        list.push((4, 3, HEX)); // two steps below the Angry rows: stays
        let b = board(&list);
        let blast = closure(&b, c(2, 3), 1);
        let mut v = cells(&blast);
        let n = v.len();
        v.dedup();
        assert_eq!(v.len(), n, "no creature twice");
        assert_eq!(n, b.count() - 1, "everything but the far one");
        assert!(!blast.contains(c(4, 3)));
        let mut s: Vec<Cell> = blast.sources.iter().map(|&(c, _)| c).collect();
        s.sort();
        let ns = s.len();
        s.dedup();
        assert_eq!((s.len(), ns), (23, 23), "all 23 Angry, each once");
    }

    /// In a checkerboard each Angry touches only one other above and one
    /// below, so the chain climbs a zig-zag column and nothing else.
    #[test]
    fn a_checkerboard_chain_climbs_one_column() {
        let l = Layout::new(8, 1.0).unwrap();
        let mut list = Vec::new();
        for r in 0..6u16 {
            for col in 0..l.row_len(r) {
                let k = if (r + col) % 2 == 0 { Kind::Angry } else { HEX };
                list.push((r, col, k));
            }
        }
        let blast = closure(&board(&list), c(5, 3), 1);
        let sources: Vec<Cell> = blast.sources.iter().map(|&(c, _)| c).collect();
        assert_eq!(
            sources,
            vec![c(5, 3), c(4, 4), c(3, 3), c(2, 4), c(1, 3), c(0, 4)]
        );
        assert_eq!(blast.waves(), 6);
        assert_eq!(blast.removed.len(), 18);
    }

    #[test]
    fn only_a_struck_angry_explodes() {
        let b = board(&[(0, 0, HEX), (0, 1, Kind::Angry)]);
        assert_eq!(closure(&b, c(0, 0), 1), Blast::default());
        assert_eq!(closure(&b, c(3, 3), 1), Blast::default());
    }
}
