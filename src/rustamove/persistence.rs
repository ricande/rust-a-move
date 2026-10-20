//! The game's own options file, `game-settings.json` in the program's data
//! folder (the framework's `settings.json` stays the template's). Read once
//! when the setup screen first gets input; written only when the player
//! starts a round. A broken or newer file gives the defaults and a warning,
//! and is never overwritten just because it was read
//! (docs/game/05_CONFIGURATION_CONTENT.md).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::geometry::{MAX_COLUMNS, MAX_ZOOM, MIN_COLUMNS, MIN_ZOOM};

const SCHEMA_VERSION: u32 = 1;
/// Larger files are not read.
const MAX_BYTES: u64 = 16 * 1024;

/// Zoom steps offered on the setup screen, in percent.
pub const ZOOM_STEP: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    pub schema_version: u32,
    /// Cells in a long row.
    pub columns: u16,
    /// Board zoom in percent (75–100).
    pub board_zoom: u32,
    pub reduced_motion: bool,
    /// A ring and colour pulse instead of a white flash in blasts.
    pub reduced_flashes: bool,
    /// "normal" or "low": how many fragments effects draw.
    pub visual_quality: super::render::Quality,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            schema_version: SCHEMA_VERSION,
            columns: 8,
            board_zoom: 100,
            reduced_motion: false,
            reduced_flashes: false,
            visual_quality: super::render::Quality::Normal,
        }
    }
}

impl Options {
    pub fn zoom(&self) -> f64 {
        self.board_zoom as f64 / 100.0
    }

    /// Whether every value is in range.
    pub fn valid(&self) -> bool {
        (MIN_COLUMNS..=MAX_COLUMNS).contains(&self.columns)
            && (MIN_ZOOM..=MAX_ZOOM).contains(&self.zoom())
            && self.board_zoom.is_multiple_of(ZOOM_STEP)
    }

    /// Changes the columns by `steps`, staying in range.
    pub fn step_columns(&mut self, steps: i32) {
        let c = self.columns as i32 + steps;
        self.columns = c.clamp(MIN_COLUMNS as i32, MAX_COLUMNS as i32) as u16;
    }

    /// Changes the zoom by `steps` × 5 %, staying in range.
    pub fn step_zoom(&mut self, steps: i32) {
        let lo = (MIN_ZOOM * 100.0).round() as i32;
        let hi = (MAX_ZOOM * 100.0).round() as i32;
        let z = self.board_zoom as i32 + steps * ZOOM_STEP as i32;
        self.board_zoom = z.clamp(lo, hi) as u32;
    }
}

/// Why the saved options could not be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Warning {
    Unreadable,
    Invalid,
    NewerVersion,
}

pub fn default_path() -> PathBuf {
    crate::settings::data_dir().join("game-settings.json")
}

/// The saved options, or the defaults with a reason. A missing file is not
/// a problem.
pub fn load(path: &Path) -> (Options, Option<Warning>) {
    let Ok(meta) = std::fs::metadata(path) else {
        return (Options::default(), None);
    };
    if meta.len() > MAX_BYTES {
        return (Options::default(), Some(Warning::Unreadable));
    }
    let Ok(text) = std::fs::read_to_string(path) else {
        return (Options::default(), Some(Warning::Unreadable));
    };
    parse(&text)
}

pub fn parse(text: &str) -> (Options, Option<Warning>) {
    match serde_json::from_str::<Options>(text) {
        Err(_) => (Options::default(), Some(Warning::Invalid)),
        Ok(o) if o.schema_version > SCHEMA_VERSION => {
            (Options::default(), Some(Warning::NewerVersion))
        }
        Ok(o) if !o.valid() => (Options::default(), Some(Warning::Invalid)),
        Ok(mut o) => {
            o.schema_version = SCHEMA_VERSION;
            (o, None)
        }
    }
}

/// Writes the options atomically (temporary file, then rename).
pub fn save(path: &Path, options: &Options) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(options).expect("options serialize") + "\n";
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_round_trip_through_the_file() {
        let dir = crate::tests::temp_dir("game-options");
        let path = dir.join("game-settings.json");
        assert_eq!(
            load(&path),
            (Options::default(), None),
            "missing file: defaults"
        );
        let o = Options {
            columns: 12,
            board_zoom: 85,
            reduced_motion: true,
            ..Options::default()
        };
        save(&path, &o).unwrap();
        assert_eq!(load(&path), (o, None));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn bad_files_give_defaults_and_a_warning() {
        assert_eq!(parse("{"), (Options::default(), Some(Warning::Invalid)));
        assert_eq!(
            parse(r#"{"columns": 40}"#),
            (Options::default(), Some(Warning::Invalid))
        );
        assert_eq!(
            parse(r#"{"board_zoom": 50}"#),
            (Options::default(), Some(Warning::Invalid))
        );
        assert_eq!(parse(r#"{"board_zoom": 83}"#).1, Some(Warning::Invalid));
        assert_eq!(parse(r#"{"columns": "eight"}"#).1, Some(Warning::Invalid));
        assert_eq!(
            parse(r#"{"schema_version": 9, "columns": 6}"#),
            (Options::default(), Some(Warning::NewerVersion))
        );
        // Missing fields default; unknown fields are ignored.
        let (o, _) = parse(r#"{"visual_quality": "low"}"#);
        assert_eq!(o.visual_quality, super::super::render::Quality::Low);
        let (o, _) = parse(r#"{"visual_quality": "maximum"}"#);
        assert_eq!(o.visual_quality, super::super::render::Quality::Maximum);
        assert_eq!(
            parse(r#"{"visual_quality": "ultra"}"#).1,
            Some(Warning::Invalid)
        );
        let (o, w) = parse(r#"{"columns": 6, "future": true}"#);
        assert_eq!((o.columns, o.board_zoom, w), (6, 100, None));
    }

    #[test]
    fn reading_a_broken_file_never_rewrites_it() {
        let dir = crate::tests::temp_dir("game-options-broken");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("game-settings.json");
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load(&path).1, Some(Warning::Invalid));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn steps_stay_in_range() {
        let mut o = Options::default();
        o.step_columns(-10);
        assert_eq!(o.columns, MIN_COLUMNS);
        o.step_columns(100);
        assert_eq!(o.columns, MAX_COLUMNS);
        o.step_zoom(-1);
        assert_eq!(o.board_zoom, 95);
        o.step_zoom(-100);
        assert_eq!(o.board_zoom, 75);
        o.step_zoom(100);
        assert_eq!(o.board_zoom, 100);
        assert!(o.valid());
    }
}
