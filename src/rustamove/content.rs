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
use super::rules::{Rules, CEILING_SHOTS_RANGE, EXPLOSION_RADIUS_RANGE};

/// Largest accepted level file.
const MAX_LEVEL_BYTES: usize = 64 * 1024;
/// The only schema this build reads.
const SCHEMA_VERSION: u32 = 1;

/// Generated levels contain Angry creatures from this level on …
const ANGRY_FROM_LEVEL: u32 = 3;
/// … each cell with this chance, in percent.
const ANGRY_PERCENT: u32 = 4;

/// Generated levels contain runners from this level on …
const RUNNER_FROM_LEVEL: u32 = 5;
/// … each ordinary cell with this chance, in percent, …
const RUNNER_PERCENT: u32 = 6;
/// … at most this share of the cells, in percent (at least one).
const RUNNER_CAP_PERCENT: u32 = 8;

/// Curated levels by (columns, file). Level n uses the n-th entry of its
/// width.
const CURATED: &[(u16, &str)] = &[
    (8, include_str!("../../assets/levels/8-01-first-grin.json")),
    (
        8,
        include_str!("../../assets/levels/8-02-off-the-wall.json"),
    ),
    (
        8,
        include_str!("../../assets/levels/8-03-angry-neighbours.json"),
    ),
    (
        8,
        include_str!("../../assets/levels/8-04-chain-reaction.json"),
    ),
    (
        8,
        include_str!("../../assets/levels/8-05-runner-intro.json"),
    ),
    (
        8,
        include_str!("../../assets/levels/8-06-runner-panic.json"),
    ),
    (8, include_str!("../../assets/levels/8-07-full-house.json")),
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
    explosion_radius: Option<u8>,
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
    pub explosion_radius: Option<u8>,
}

/// The level-file token of a creature (the inverse of `token`).
fn token_of(kind: Kind, runner: bool) -> String {
    let name = |f: Family| match f {
        Family::Round => "round",
        Family::Block => "block",
        Family::Diamond => "diamond",
        Family::Hex => "hex",
        Family::Triangle => "triangle",
        Family::Oval => "oval",
    };
    match (kind, runner) {
        (Kind::Angry, _) => "angry".into(),
        (Kind::Ordinary(f), true) => format!("runner:{}", name(f)),
        (Kind::Ordinary(f), false) => name(f).into(),
    }
}

/// Writes a level in the file format `parse` reads, one row per line (the
/// layout of the files in `assets/levels/`). Rows run down to the last
/// occupied one.
pub fn to_json(level: &Level) -> String {
    let rows = level.cells.iter().map(|c| c.0.row + 1).max().unwrap_or(0);
    let mut lines = Vec::new();
    for row in 0..rows {
        let len = if row % 2 == 0 {
            level.columns
        } else {
            level.columns - 1
        };
        let tokens: Vec<String> = (0..len)
            .map(|col| {
                level
                    .cells
                    .iter()
                    .find(|c| c.0 == Cell::new(row, col))
                    .map_or("null".into(), |&(_, k, r)| {
                        format!("\"{}\"", token_of(k, r))
                    })
            })
            .collect();
        lines.push(format!("    [{}]", tokens.join(", ")));
    }
    let mut overrides = Vec::new();
    if let Some(n) = level.ceiling_shots {
        overrides.push(format!("\"ceiling_shots\": {n}"));
    }
    if let Some(n) = level.explosion_radius {
        overrides.push(format!("\"explosion_radius\": {n}"));
    }
    let rules = if overrides.is_empty() {
        String::new()
    } else {
        format!(",\n  \"rules_override\": {{ {} }}", overrides.join(", "))
    };
    format!(
        "{{\n  \"schema_version\": {SCHEMA_VERSION},\n  \"id\": {},\n  \"columns\": {},\n  \"seed\": {},\n  \"rows\": [\n{}\n  ]{rules}\n}}\n",
        serde_json::to_string(&level.id).expect("a string serializes"),
        level.columns,
        level.seed,
        lines.join(",\n"),
    )
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
    let (ceiling_shots, explosion_radius) = file
        .rules_override
        .map_or((None, None), |o| (o.ceiling_shots, o.explosion_radius));
    if let Some(n) = ceiling_shots {
        if !CEILING_SHOTS_RANGE.contains(&n) {
            return Err(LevelError::Override(format!("ceiling_shots {n}")));
        }
    }
    if let Some(n) = explosion_radius {
        if !EXPLOSION_RADIUS_RANGE.contains(&n) {
            return Err(LevelError::Override(format!("explosion_radius {n}")));
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
        explosion_radius,
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

/// A connected board for `number` on `layout`, deterministic from `seed`.
/// Higher levels get more rows and families, from level 3 a few Angry
/// creatures and from level 5 a few runners.
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
    // A hard cap: about 5 % of the cells, at least one.
    let cells: u32 = (0..rows).map(|r| layout.row_len(r) as u32).sum();
    let angry_cap = (cells * 5 / 100).max(1);
    let mut angry = 0;
    let runner_cap = (cells * RUNNER_CAP_PERCENT / 100).max(1);
    let mut runners = 0;
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
            // From level 3, a few Angry ones (rule of thumb: at most 5 %).
            if number >= ANGRY_FROM_LEVEL && angry < angry_cap && rng.below(100) < ANGRY_PERCENT {
                angry += 1;
                line.push(None);
                b.place(Cell::new(row, col), Kind::Angry, false);
                continue;
            }
            line.push(Some(f));
            let runner = number >= RUNNER_FROM_LEVEL
                && runners < runner_cap
                && rng.below(100) < RUNNER_PERCENT;
            runners += runner as u32;
            b.place(Cell::new(row, col), Kind::Ordinary(FAMILIES[f]), runner);
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
    /// The generator's result failed its checks; a simple known board.
    Fallback,
    /// A level from the level editor, by its id.
    Custom(String),
}

/// A usable start: enough creatures, all hanging from the ceiling, clear
/// of the danger line.
fn valid_start(b: &Board) -> bool {
    b.count() > b.layout.columns as usize && b.anchored().len() == b.count() && !b.reaches_danger()
}

/// Three full rows of ordinary creatures in a fixed pattern: valid at every
/// supported width and zoom (tested).
pub fn fallback(layout: Layout) -> Board {
    let mut b = Board::new(layout);
    for row in 0..3u16 {
        for col in 0..layout.row_len(row) {
            let f = FAMILIES[((col / 2) as usize + row as usize) % 4];
            b.place(Cell::new(row, col), Kind::Ordinary(f), false);
        }
    }
    b
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
            if let Some(n) = level.explosion_radius {
                rules.explosion_radius = n;
            }
            return (board, rules, level.seed, Source::Curated(level.id));
        }
    }
    let seed = super::emotion::splitmix(
        number as u64 ^ (layout.columns as u64) << 32 ^ ((layout.zoom * 1000.0) as u64) << 40,
    );
    let board = generate(layout, number, seed);
    if valid_start(&board) {
        (board, rules, seed, Source::Generated)
    } else {
        // Never expected; a known valid board rather than a broken start.
        (fallback(layout), rules, seed, Source::Fallback)
    }
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
        assert!(n >= 7);
        assert!(curated(8, 1).is_some() && curated(8, 8).is_none() && curated(6, 1).is_none());
        // The progression: Angry first appears in level 3, a chain in 4.
        let angry = |n: u32| {
            curated(8, n)
                .unwrap()
                .cells
                .iter()
                .filter(|c| c.1 == Kind::Angry)
                .count()
        };
        assert_eq!((angry(1), angry(2), angry(3), angry(4)), (0, 0, 1, 3));
        // Runners first appear in level 5, next to an Angry in 6.
        let runners = |n: u32| curated(8, n).unwrap().cells.iter().filter(|c| c.2).count();
        assert_eq!(
            (runners(4), runners(5), runners(6), runners(7)),
            (0, 1, 1, 1)
        );
        assert!(curated(8, 0).is_none());
    }

    /// Writing a level and reading it back gives the same level, for every
    /// curated level, and the text has the curated files' layout.
    #[test]
    fn levels_round_trip_through_json() {
        for &(_, text) in CURATED {
            let level = parse(text).unwrap();
            let written = to_json(&level);
            assert_eq!(parse(&written).unwrap(), level);
        }
        let level = Level {
            id: "custom \"1\"".into(),
            columns: 7,
            seed: 9,
            cells: vec![
                (Cell::new(0, 6), Kind::Angry, false),
                (Cell::new(1, 2), Kind::Ordinary(Family::Oval), true),
            ],
            ceiling_shots: Some(7),
            explosion_radius: Some(2),
        };
        let written = to_json(&level);
        assert_eq!(parse(&written).unwrap(), level, "{written}");
        assert!(written.contains("\"runner:oval\""));
        assert!(written.contains("[null, null, null, null, null, null, \"angry\"]"));
    }

    #[test]
    fn the_fallback_board_is_valid_everywhere() {
        for columns in super::super::geometry::MIN_COLUMNS..=super::super::geometry::MAX_COLUMNS {
            for zoom in [75u32, 80, 85, 90, 95, 100] {
                let l = Layout::new(columns, zoom as f64 / 100.0).unwrap();
                assert!(valid_start(&fallback(l)), "{columns} {zoom}");
            }
        }
    }

    /// Every width and zoom the setup screen offers gets a valid first board.
    #[test]
    fn every_setting_gets_a_valid_board() {
        for columns in super::super::geometry::MIN_COLUMNS..=super::super::geometry::MAX_COLUMNS {
            for zoom in [75u32, 80, 85, 90, 95, 100] {
                let l = Layout::new(columns, zoom as f64 / 100.0).unwrap();
                for number in [1, 3, 5, 9] {
                    let (b, _, _, source) = level_board(l, number);
                    assert!(valid_start(&b), "{columns} {zoom} {number}");
                    assert_ne!(source, Source::Fallback, "the generator suffices");
                }
            }
        }
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
                    let (a, _, seed, source) = level_board(l, number);
                    let (b, _, seed2, _) = level_board(l, number);
                    assert_eq!(seed, seed2);
                    let cells =
                        |b: &Board| b.occupied().map(|(c, x)| (c, x.kind)).collect::<Vec<_>>();
                    assert_eq!(cells(&a), cells(&b), "same level, same board");
                    assert!(a.count() > columns as usize, "{columns} {number}");
                    assert_eq!(a.anchored().len(), a.count(), "connected");
                    assert!(!a.reaches_danger());
                    let runners = a.occupied().filter(|(_, c)| c.runner).count();
                    if source == Source::Generated {
                        let angry = a.occupied().filter(|(_, c)| c.kind == Kind::Angry).count();
                        if number < ANGRY_FROM_LEVEL {
                            assert_eq!(angry, 0);
                        }
                        if number < RUNNER_FROM_LEVEL {
                            assert_eq!(runners, 0);
                        }
                        assert!(runners * 100 <= a.count() * RUNNER_CAP_PERCENT as usize + 100);
                        assert!(
                            angry * 100 <= a.count() * 8 + 100,
                            "Angry stays rare: {angry} of {}",
                            a.count()
                        );
                    }
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
