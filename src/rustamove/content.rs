//! Levels: the versioned JSON format, its validation, the curated levels
//! and a seeded generator for everything else
//! (docs/game/05_CONFIGURATION_CONTENT.md).
//!
//! A curated level is used when one exists for the board's width and the
//! level number; otherwise the generator makes a connected, valid board.
//! Valid is not the same as solvable: no level here claims to be solvable
//! beyond what playing it has shown.

use serde::Deserialize;

use super::board::Board;
use super::emotion::{Family, Kind, FAMILIES};
use super::geometry::{Cell, Layout};
use super::rng::Rng;
use super::rules::{Rules, CEILING_SHOTS_RANGE};

/// Largest accepted level file.
const MAX_LEVEL_BYTES: usize = 64 * 1024;
/// The only schema this build reads.
const SCHEMA_VERSION: u32 = 1;

/// Curated levels by (columns, file). Level n uses the n-th entry of its
/// width.
const CURATED: &[(u16, &str)] = &[
    (8, include_str!("../../assets/levels/8-01-first-grin.json")),
    (
        8,
        include_str!("../../assets/levels/8-02-off-the-wall.json"),
    ),
    (8, include_str!("../../assets/levels/8-03-full-house.json")),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LevelFile {
    schema_version: u32,
    id: String,
    columns: u16,
    seed: u64,
    rows: Vec<Vec<Option<String>>>,
    #[serde(default)]
    rules_override: Option<RulesOverride>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RulesOverride {
    ceiling_shots: Option<u32>,
}

/// Why a level cannot be used.
#[derive(Clone, Debug, PartialEq)]
pub enum LevelError {
    TooLarge,
    Json(String),
    Schema(u32),
    Columns {
        level: u16,
        board: u16,
    },
    TooManyRows(usize),
    RowLength {
        row: usize,
        len: usize,
        expected: u16,
    },
    Token {
        row: usize,
        col: usize,
        token: String,
    },
    Override(String),
    Empty,
    /// Some creature does not hang from the ceiling.
    Loose(Cell),
    /// Some creature already reaches the danger line.
    Danger(Cell),
}

/// A parsed, validated level.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub id: String,
    pub columns: u16,
    pub seed: u64,
    /// Occupied cells in row-major order.
    pub cells: Vec<(Cell, Kind, bool)>,
    pub ceiling_shots: Option<u32>,
}

fn token(t: &str) -> Option<(Kind, bool)> {
    let family = |name: &str| -> Option<Family> {
        Some(match name {
            "round" => Family::Round,
            "block" => Family::Block,
            "diamond" => Family::Diamond,
            "hex" => Family::Hex,
            "triangle" => Family::Triangle,
            "oval" => Family::Oval,
            _ => return None,
        })
    };
    if t == "angry" {
        return Some((Kind::Angry, false));
    }
    if let Some(f) = t.strip_prefix("runner:") {
        return family(f).map(|f| (Kind::Ordinary(f), true));
    }
    family(t).map(|f| (Kind::Ordinary(f), false))
}

/// Parses and checks the shape of a level file (row lengths, tokens,
/// overrides). Fit on a particular board is checked by `build`.
pub fn parse(text: &str) -> Result<Level, LevelError> {
    if text.len() > MAX_LEVEL_BYTES {
        return Err(LevelError::TooLarge);
    }
    let file: LevelFile =
        serde_json::from_str(text).map_err(|e| LevelError::Json(e.to_string()))?;
    if file.schema_version != SCHEMA_VERSION {
        return Err(LevelError::Schema(file.schema_version));
    }
    if file.rows.len() > 64 {
        return Err(LevelError::TooManyRows(file.rows.len()));
    }
    let mut cells = Vec::new();
    for (r, row) in file.rows.iter().enumerate() {
        let expected = if r % 2 == 0 {
            file.columns
        } else {
            file.columns.saturating_sub(1)
        };
        if row.len() != expected as usize {
            return Err(LevelError::RowLength {
                row: r,
                len: row.len(),
                expected,
            });
        }
        for (c, t) in row.iter().enumerate() {
            let Some(t) = t else { continue };
            let (kind, runner) = token(t).ok_or_else(|| LevelError::Token {
                row: r,
                col: c,
                token: t.chars().take(32).collect(),
            })?;
            cells.push((Cell::new(r as u16, c as u16), kind, runner));
        }
    }
    let ceiling_shots = file.rules_override.and_then(|o| o.ceiling_shots);
    if let Some(n) = ceiling_shots {
        if !CEILING_SHOTS_RANGE.contains(&n) {
            return Err(LevelError::Override(format!("ceiling_shots {n}")));
        }
    }
    if cells.is_empty() {
        return Err(LevelError::Empty);
    }
    Ok(Level {
        id: file.id.chars().take(64).collect(),
        columns: file.columns,
        seed: file.seed,
        cells,
        ceiling_shots,
    })
}

/// Puts a level on a board with `layout`, checking that it fits, hangs from
/// the ceiling and stays clear of the danger line. Nothing is clipped or
/// dropped silently.
pub fn build(level: &Level, layout: Layout) -> Result<Board, LevelError> {
    if level.columns != layout.columns {
        return Err(LevelError::Columns {
            level: level.columns,
            board: layout.columns,
        });
    }
    let mut b = Board::new(layout);
    for &(cell, kind, runner) in &level.cells {
        if !layout.contains(cell) {
            return Err(LevelError::TooManyRows(cell.row as usize + 1));
        }
        b.place(cell, kind, runner);
    }
    let anchored = b.anchored();
    if let Some((cell, _)) = b.occupied().find(|(c, _)| !anchored.contains(c)) {
        return Err(LevelError::Loose(cell));
    }
    if let Some((cell, _)) = b
        .occupied()
        .find(|(c, _)| layout.centre(*c).y + layout.r >= super::geometry::DANGER_Y)
    {
        return Err(LevelError::Danger(cell));
    }
    Ok(b)
}

/// The curated level `number` (from 1) for `columns`, if there is one.
pub fn curated(columns: u16, number: u32) -> Option<Level> {
    let text = CURATED
        .iter()
        .filter(|(c, _)| *c == columns)
        .nth(number.checked_sub(1)? as usize)?
        .1;
    Some(parse(text).expect("curated levels are valid (tested)"))
}

/// A connected board of ordinary creatures for `number` on `layout`,
/// deterministic from `seed`. Higher levels get more rows and families.
pub fn generate(layout: Layout, number: u32, seed: u64) -> Board {
    let mut rng = Rng::new(seed);
    let families = (3 + (number as usize).saturating_sub(1) / 2).min(FAMILIES.len());
    // Leave at least 45 % of the height between the board and the danger
    // line at the start.
    let room = (super::geometry::DANGER_Y - layout.ceiling_y) * 0.55;
    let max_rows = (((room - 2.0 * layout.r) / layout.pitch()).floor() as u16 + 1).max(2);
    let rows = (3 + number as u16 / 2).clamp(3, 9).min(max_rows);
    let mut b = Board::new(layout);
    let mut grid: Vec<Vec<Option<usize>>> = Vec::new();
    for row in 0..rows {
        let mut line = Vec::new();
        for col in 0..layout.row_len(row) {
            // The last row is ragged.
            if row == rows - 1 && row > 0 && rng.below(100) < 35 {
                line.push(None);
                continue;
            }
            // Clumps: often the family of the left or upper neighbour.
            let left = col
                .checked_sub(1)
                .and_then(|c| line.get(c as usize).copied().flatten());
            let up = row
                .checked_sub(1)
                .and_then(|r| grid.get(r as usize))
                .and_then(|prev| prev.get(col as usize).copied().flatten());
            let f = match rng.below(100) {
                0..=34 if left.is_some() => left.unwrap(),
                35..=54 if up.is_some() => up.unwrap(),
                _ => rng.below(families as u32) as usize,
            };
            line.push(Some(f));
            b.place(Cell::new(row, col), Kind::Ordinary(FAMILIES[f]), false);
        }
        grid.push(line);
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

/// Where a round's board came from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Curated(String),
    Generated,
}

/// The board, rules and seed for level `number` on `layout`. The seed
/// depends only on the level and the board settings, so Retry gives the
/// same challenge.
pub fn level_board(layout: Layout, number: u32) -> (Board, Rules, u64, Source) {
    let mut rules = Rules::default();
    if let Some(level) = curated(layout.columns, number) {
        if let Ok(board) = build(&level, layout) {
            if let Some(n) = level.ceiling_shots {
                rules.ceiling_shots = n;
            }
            return (board, rules, level.seed, Source::Curated(level.id));
        }
    }
    let seed = super::emotion::splitmix(
        number as u64 ^ (layout.columns as u64) << 32 ^ ((layout.zoom * 1000.0) as u64) << 40,
    );
    (
        generate(layout, number, seed),
        rules,
        seed,
        Source::Generated,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(columns: u16) -> Layout {
        Layout::new(columns, 1.0).unwrap()
    }

    #[test]
    fn curated_levels_are_valid_and_fit_at_every_zoom() {
        let mut n = 0;
        for &(columns, text) in CURATED {
            let level = parse(text).unwrap();
            assert_eq!(level.columns, columns);
            for zoom in [0.75, 1.0] {
                let b = build(&level, Layout::new(columns, zoom).unwrap()).unwrap();
                assert_eq!(b.count(), level.cells.len());
            }
            n += 1;
        }
        assert!(n >= 3);
        assert!(curated(8, 1).is_some() && curated(8, 4).is_none() && curated(6, 1).is_none());
        assert!(curated(8, 0).is_none());
    }

    #[test]
    fn bad_levels_are_rejected_with_a_reason() {
        let base = |rows: &str, extra: &str| {
            format!(r#"{{"schema_version":1,"id":"t","columns":6,"seed":1,"rows":{rows}{extra}}}"#)
        };
        let ok = r#"[["round","round","round","hex","hex","hex"]]"#;
        assert!(parse(&base(ok, "")).is_ok());
        assert!(matches!(
            parse(&base(r#"[["round"]]"#, "")),
            Err(LevelError::RowLength { .. })
        ));
        assert!(matches!(
            parse(&base(
                r#"[["round","round","round","hex","hex","purple"]]"#,
                ""
            )),
            Err(LevelError::Token { col: 5, .. })
        ));
        assert!(matches!(
            parse(&base(ok, r#","bonus":1"#)),
            Err(LevelError::Json(_))
        ));
        assert!(matches!(
            parse(&base(ok, r#","rules_override":{"ceiling_shots":0}"#)),
            Err(LevelError::Override(_))
        ));
        assert!(matches!(
            parse(&base(r#"[[null,null,null,null,null,null]]"#, "")),
            Err(LevelError::Empty)
        ));
        let v2 = r#"{"schema_version":2,"id":"t","columns":6,"seed":1,"rows":[]}"#;
        assert_eq!(parse(v2), Err(LevelError::Schema(2)));
        assert_eq!(
            parse(&"x".repeat(MAX_LEVEL_BYTES + 1)),
            Err(LevelError::TooLarge)
        );
        assert!(matches!(parse("{"), Err(LevelError::Json(_))));
        // Runners and Angry are understood.
        let special = base(r#"[["angry","runner:oval","round","hex","hex","hex"]]"#, "");
        let l = parse(&special).unwrap();
        assert_eq!(l.cells[0].1, Kind::Angry);
        assert!(l.cells[1].2);
    }

    #[test]
    fn build_checks_width_support_and_danger() {
        let text = r#"{"schema_version":1,"id":"t","columns":6,"seed":1,"rows":[
            ["round","round",null,null,"hex","hex"],
            [null,null,null,null,null],
            ["oval",null,null,null,null,null]]}"#;
        let level = parse(text).unwrap();
        assert_eq!(
            build(&level, layout(8)).unwrap_err(),
            LevelError::Columns { level: 6, board: 8 }
        );
        assert_eq!(
            build(&level, layout(6)).unwrap_err(),
            LevelError::Loose(Cell::new(2, 0))
        );
        // A row near the bottom reaches the danger line.
        let l = layout(6);
        let mut rows =
            vec!["[\"round\",\"round\",\"round\",\"round\",\"round\",\"round\"]".to_string()];
        for r in 1..l.rows {
            let n = l.row_len(r);
            rows.push(format!("[{}]", vec!["\"hex\""; n as usize].join(",")));
        }
        let deep = format!(
            r#"{{"schema_version":1,"id":"t","columns":6,"seed":1,"rows":[{}]}}"#,
            rows.join(",")
        );
        assert!(matches!(
            build(&parse(&deep).unwrap(), l),
            Err(LevelError::Danger(_))
        ));
    }

    #[test]
    fn generated_boards_are_valid_reproducible_and_grow() {
        for columns in [6, 8, 12, 18] {
            for zoom in [0.75, 1.0] {
                let l = Layout::new(columns, zoom).unwrap();
                for number in 1..12 {
                    let (a, _, seed, _) = level_board(l, number);
                    let (b, _, seed2, _) = level_board(l, number);
                    assert_eq!(seed, seed2);
                    let cells =
                        |b: &Board| b.occupied().map(|(c, x)| (c, x.kind)).collect::<Vec<_>>();
                    assert_eq!(cells(&a), cells(&b), "same level, same board");
                    assert!(a.count() > columns as usize, "{columns} {number}");
                    assert_eq!(a.anchored().len(), a.count(), "connected");
                    assert!(!a.reaches_danger());
                    assert!(a
                        .occupied()
                        .all(|(_, c)| c.kind != Kind::Angry && !c.runner));
                }
            }
        }
        let l = layout(12);
        let fams = |n: u32| {
            let mut f: Vec<Kind> = level_board(l, n)
                .0
                .occupied()
                .map(|(_, c)| c.kind)
                .collect();
            f.sort_by_key(|k| format!("{k:?}"));
            f.dedup();
            f.len()
        };
        assert!(fams(1) <= 3 && fams(11) > 3, "more families later");
        assert_ne!(
            level_board(l, 4).2,
            level_board(Layout::new(12, 0.75).unwrap(), 4).2,
            "settings change the challenge"
        );
    }
}
