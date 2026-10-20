//! Rust a move: a Snood-inspired aiming and matching game with
//! grumpy geometric creatures. See docs/GAME_RULES.md and docs/game/.
//!
//! A round starts with a game-local setup screen (rule P13: the template's
//! `Game::new(level)` has no options hook). From there the player starts
//! the classic round (`round.rs`) with the board width, zoom and reduced
//! motion chosen there (saved in `persistence.rs`), or opens the shooting
//! range (`range.rs`) or the character animation lab (`lab.rs`).
//!
//! `RUST_A_MOVE_DEV=1` turns on development overlays (colliders, shot path) in the
//! range; normal play never shows a trajectory.
//!
//! `handle` only records what was pressed; everything changes in `update`,
//! because the framework may call `update` several times (or not at all)
//! after one `handle`.

mod aim;
mod blast;
mod board;
mod content;
mod emotion;
mod geometry;
mod lab;
mod persistence;
mod range;
mod render;
mod rng;
mod round;
mod rules;
mod sfx;
mod shot;

use std::path::PathBuf;

use macroquad::math::{Rect, Vec2};

use persistence::{Options, Warning};

use crate::audio::Sfx;
use crate::game::{Game, Outcome, RoundResult};
use crate::i18n::Text;
use crate::t;
use crate::ui::{Input, Key};
use crate::view::{draw_button, with_alpha, View, ACCENT, ERROR, FIELD_W, NEON, TEXT};

const TEXTS: &[Text] = &[
    (
        "menu.subtitle",
        "Aim with fear. Match with spite.",
        "Sikta med skräck. Matcha med trots.",
    ),
    (
        "lab.title",
        "Lab {n}/{total}: {name}",
        "Verkstad {n}/{total}: {name}",
    ),
    ("lab.s.cast", "The cast", "Ensemblen"),
    ("lab.s.target", "One target", "Ett mål"),
    (
        "lab.s.ring",
        "Angry and six nervous neighbours",
        "Arg och sex nervösa grannar",
    ),
    ("lab.s.angry_aim", "Aiming at Angry", "Sikte på Arg"),
    ("lab.s.routes", "Runner routes", "Rymmarens vägar"),
    ("lab.s.panic", "Runner next to Angry", "Rymmare bredvid Arg"),
    ("lab.s.blast", "Blasts and falls", "Explosioner och ras"),
    ("lab.s.widths", "Board widths", "Brädets bredd"),
    (
        "lab.d.size",
        "Size: radius {r}   Space: aim at the next one",
        "Storlek: radie {r}   Mellanslag: sikta på nästa",
    ),
    (
        "lab.d.target",
        "{family}   aim {aim}   Space: aim on/off",
        "{family}   sikte {aim}   Mellanslag: sikte av/på",
    ),
    ("lab.on", "on", "på"),
    ("lab.off", "off", "av"),
    (
        "lab.d.ring_none",
        "Forced aim: none   Up/Down or Space: aim a neighbour",
        "Tvingat sikte: inget   Upp/Ned eller Mellanslag: sikta på en granne",
    ),
    (
        "lab.d.ring",
        "Forced aim: neighbour {n} of 6",
        "Tvingat sikte: granne {n} av 6",
    ),
    (
        "lab.d.angry_aimed",
        "Aim: left Angry. Same motion as its twin; nobody behind reacts",
        "Sikte: vänstra Arg. Rör sig som tvillingen; ingen bakom reagerar",
    ),
    (
        "lab.d.angry_free",
        "Aim: none. Space: aim at the left Angry",
        "Sikte: inget. Mellanslag: sikta på vänstra Arg",
    ),
    (
        "lab.d.route",
        "{setup}: {route}   Space: restart",
        "{setup}: {route}   Mellanslag: börja om",
    ),
    (
        "lab.d.panic",
        "{route}   Space: pop the left blocker",
        "{route}   Mellanslag: spräng vänstra hindret",
    ),
    ("lab.d.blocked", "no way out", "ingen väg ut"),
    ("lab.d.left", "leaving left", "drar åt vänster"),
    ("lab.d.right", "leaving right", "drar åt höger"),
    ("lab.d.escaped", "escaped", "rymt"),
    ("lab.p.boxed", "Boxed in", "Instängd"),
    ("lab.p.left", "Left open", "Öppet åt vänster"),
    ("lab.p.right", "Right open", "Öppet åt höger"),
    ("lab.p.both", "Both open", "Båda öppna"),
    (
        "lab.p.direct",
        "Direct hit on Angry   Space: fire",
        "Direktträff på Arg   Mellanslag: skjut",
    ),
    (
        "lab.p.chain",
        "Chain of three Angry   Space: fire",
        "Kedja av tre Arga   Mellanslag: skjut",
    ),
    (
        "lab.p.fall",
        "Match of three, cluster falls   Space: fire",
        "Tre i rad, klungan rasar   Mellanslag: skjut",
    ),
    (
        "lab.d.width",
        "{columns} columns, zoom {zoom}%   Space: move the aim",
        "{columns} kolumner, zoom {zoom} %   Mellanslag: flytta siktet",
    ),
    ("lab.ending_hold", "Ending hold", "Avslutningspaus"),
    ("lab.slow", "Speed: slow (1/4)", "Fart: långsam (1/4)"),
    ("lab.normal", "Speed: normal", "Fart: normal"),
    ("lab.reduced_on", "Reduced motion: on", "Mindre rörelse: på"),
    ("lab.reduced_off", "Reduced motion: off", "Mindre rörelse: av"),
    (
        "lab.help",
        "Z/X scenario   Up/Down setting   Left/Right slow/normal   Space event\nD colliders   A reduced motion   Enter finish   Esc pause",
        "Z/X scenario   Upp/Ned inställning   Vänster/Höger långsam/normal   Mellanslag händelse\nD kolliderare   A mindre rörelse   Enter avsluta   Esc paus",
    ),
    ("lab.k.round", "Round", "Rund"),
    ("lab.k.block", "Block", "Kloss"),
    ("lab.k.diamond", "Diamond", "Diamant"),
    ("lab.k.hex", "Hex", "Hexa"),
    ("lab.k.triangle", "Triangle", "Triangel"),
    ("lab.k.oval", "Oval", "Oval"),
    ("lab.k.runner", "Runner", "Rymmare"),
    ("lab.k.angry", "Angry", "Arg"),
    ("setup.level", "Level {level}", "Nivå {level}"),
    ("setup.play", "Play", "Spela"),
    ("setup.columns", "<  Width: {n} columns  >", "<  Bredd: {n} kolumner  >"),
    ("setup.zoom", "<  Zoom: {percent}%  >", "<  Zoom: {percent} %  >"),
    ("setup.motion_off", "Reduced motion: off", "Mindre rörelse: av"),
    ("setup.motion_on", "Reduced motion: on", "Mindre rörelse: på"),
    ("setup.flashes_off", "Reduced flashes: off", "Mindre blixtar: av"),
    ("setup.flashes_on", "Reduced flashes: on", "Mindre blixtar: på"),
    ("setup.range", "Shooting range (test)", "Skjutbana (test)"),
    ("setup.lab", "Animation lab", "Animationsverkstad"),
    (
        "setup.help",
        "Up/Down choose, Left/Right change, Space start. Width and zoom apply from the next round. In the range and lab, Enter ends the round.",
        "Upp/Ned väljer, Vänster/Höger ändrar, Mellanslag startar. Bredd och zoom gäller från nästa omgång. I skjutbanan och verkstaden avslutar Enter omgången.",
    ),
    (
        "setup.warn_invalid",
        "The saved game options could not be read; using the defaults. Starting a round saves new ones.",
        "De sparade spelinställningarna gick inte att läsa; standardvärden används. När en omgång startar sparas nya.",
    ),
    (
        "setup.warn_newer",
        "The saved game options are from a newer version; using the defaults.",
        "De sparade spelinställningarna kommer från en nyare version; standardvärden används.",
    ),
    (
        "setup.save_failed",
        "Could not save the game options: {error}",
        "Kunde inte spara spelinställningarna: {error}",
    ),
    ("round.level", "Level {level}", "Nivå {level}"),
    ("round.score", "Score {score}", "Poäng {score}"),
    ("round.next", "next", "nästa"),
    ("round.ceiling", "Ceiling drops in", "Taket sjunker om"),
    ("round.won", "Board cleared!", "Brädet är tomt!"),
    ("round.lost_danger", "Too low: the danger line!", "För lågt: dödslinjen!"),
    ("round.lost_blocked", "No room: the board is blocked!", "Ingen plats: brädet är blockerat!"),
    ("round.lost_fault", "Something went wrong with that shot.", "Något gick fel med det skottet."),
    ("round.final", "Score {score}", "Poäng {score}"),
    (
        "round.help",
        "Mouse or Left/Right aim   Click or Space fire   Esc pause",
        "Musen eller Vänster/Höger siktar   Klick eller Mellanslag skjuter   Esc paus",
    ),
    ("range.title", "Range: {name}", "Skjutbana: {name}"),
    ("range.f.mixed", "Mixed", "Blandat"),
    ("range.f.gaps", "Ceiling gaps", "Luckor i taket"),
    ("range.f.curtain", "Curtain (bank shots)", "Ridå (vallskott)"),
    ("range.f.pocket", "Pocket", "Ficka"),
    (
        "range.detail",
        "{columns} columns, zoom {zoom}%   Shots {shots}",
        "{columns} kolumner, zoom {zoom} %   Skott {shots}",
    ),
    ("range.next", "next", "nästa"),
    (
        "range.m.danger",
        "Danger line reached: new board. (A loss from M3.)",
        "Dödslinjen nådd: nytt bräde. (Förlust från M3.)",
    ),
    (
        "range.m.angry",
        "Angry hit! (Blasts happen in the game, not on the range.)",
        "Träff på Arg! (Explosioner sker i spelet, inte på skjutbanan.)",
    ),
    (
        "range.m.blocked",
        "No room here: the board is blocked. (A loss from M3.)",
        "Ingen plats här: brädet är blockerat. (Förlust från M3.)",
    ),
    (
        "range.help",
        "Mouse or Left/Right (A/D) aim   Click or Space fire\nZ/X board   Up/Down width   Enter finish   Esc pause",
        "Musen eller Vänster/Höger (A/D) siktar   Klick eller Mellanslag skjuter\nZ/X bräde   Upp/Ned bredd   Enter avslutar   Esc paus",
    ),
    (
        "range.dev",
        "DEV: S colliders   W shot path",
        "DEV: S kolliderare   W skottbana",
    ),
];

/// Setup screen rows, top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Play,
    Columns,
    Zoom,
    Motion,
    Flashes,
    Range,
    Lab,
}

const ROWS: [Row; 7] = [
    Row::Play,
    Row::Columns,
    Row::Zoom,
    Row::Motion,
    Row::Flashes,
    Row::Range,
    Row::Lab,
];

/// Where each setup row is, shared by input and drawing.
fn row_rect(i: usize) -> Rect {
    let gap = if i >= 5 { 20.0 } else { 0.0 };
    Rect::new(110.0, 320.0 + i as f32 * 60.0 + gap, 320.0, 50.0)
}

struct Setup {
    focus: usize,
    options: Options,
    /// Options have been read (once, at the first input).
    loaded: bool,
    warning: Option<Warning>,
}

enum Mode {
    Setup(Setup),
    Play(Box<round::Round>),
    Range(Box<range::Range>),
    Lab(Box<lab::Lab>),
}

pub struct RustAMove {
    level: u32,
    dev: bool,
    /// The options file; `None` in tests, which never touch user files.
    options_path: Option<PathBuf>,
    mode: Mode,
    /// Setup and lab presses since the last update, applied there in order.
    pending: Vec<Key>,
    /// A setup click since the last update.
    click: Option<Vec2>,
    pointer: Option<Vec2>,
    /// A problem saving the options, shown briefly in the round.
    save_error: Option<String>,
    finish_requested: bool,
    done: bool,
}

impl RustAMove {
    fn with(level: u32, dev: bool, options_path: Option<PathBuf>) -> RustAMove {
        RustAMove {
            level,
            dev,
            options_path,
            mode: Mode::Setup(Setup {
                focus: 0,
                options: Options::default(),
                loaded: false,
                warning: None,
            }),
            pending: Vec::new(),
            click: None,
            pointer: None,
            save_error: None,
            finish_requested: false,
            done: false,
        }
    }

    /// Reads the saved options once, at the setup screen's first input.
    fn load_options(&mut self) {
        let Mode::Setup(setup) = &mut self.mode else {
            return;
        };
        if setup.loaded {
            return;
        }
        setup.loaded = true;
        if let Some(path) = &self.options_path {
            let (options, warning) = persistence::load(path);
            setup.options = options;
            setup.warning = warning;
        }
    }

    fn start(&mut self, row: Row, options: Options) {
        self.mode = match row {
            Row::Play => {
                // Saved only when a round starts with them.
                if let Some(path) = &self.options_path {
                    self.save_error = persistence::save(path, &options)
                        .err()
                        .map(|e| e.to_string());
                }
                let layout =
                    geometry::Layout::new(options.columns, options.zoom()).unwrap_or_else(|_| {
                        geometry::Layout::new(8, 1.0).expect("the default layout is valid")
                    });
                {
                    let mut round = round::Round::new(self.level, layout, options.reduced_motion);
                    round.reduced_flashes = options.reduced_flashes;
                    Mode::Play(Box::new(round))
                }
            }
            Row::Range => {
                let mut r = range::Range::new(self.level, self.dev);
                r.reduced_motion = options.reduced_motion;
                Mode::Range(Box::new(r))
            }
            _ => {
                let mut l = lab::Lab::new();
                l.reduced_motion = options.reduced_motion;
                Mode::Lab(Box::new(l))
            }
        };
    }

    fn update_setup(&mut self) {
        let Mode::Setup(setup) = &mut self.mode else {
            return;
        };
        let mut chosen = None;
        let n = ROWS.len();
        for key in std::mem::take(&mut self.pending) {
            let row = ROWS[setup.focus];
            match key {
                Key::Up => setup.focus = (setup.focus + n - 1) % n,
                Key::Down => setup.focus = (setup.focus + 1) % n,
                Key::Left | Key::Right => {
                    let step = if key == Key::Left { -1 } else { 1 };
                    match row {
                        Row::Columns => setup.options.step_columns(step),
                        Row::Zoom => setup.options.step_zoom(step),
                        Row::Motion => setup.options.reduced_motion ^= true,
                        Row::Flashes => setup.options.reduced_flashes ^= true,
                        _ => {}
                    }
                }
                Key::Space | Key::Enter => match row {
                    Row::Motion => setup.options.reduced_motion ^= true,
                    Row::Flashes => setup.options.reduced_flashes ^= true,
                    Row::Columns | Row::Zoom => {}
                    _ => chosen = Some(row),
                },
                _ => {}
            }
        }
        if let Some(p) = self.click.take() {
            if let Some(i) = (0..n).find(|&i| row_rect(i).contains(p)) {
                setup.focus = i;
                let right_half = p.x >= row_rect(i).center().x;
                let step = if right_half { 1 } else { -1 };
                match ROWS[i] {
                    Row::Columns => setup.options.step_columns(step),
                    Row::Zoom => setup.options.step_zoom(step),
                    Row::Motion => setup.options.reduced_motion ^= true,
                    Row::Flashes => setup.options.reduced_flashes ^= true,
                    row => chosen = Some(row),
                }
            }
        }
        if let Some(row) = chosen {
            let options = setup.options;
            self.start(row, options);
        }
    }

    fn draw_setup(&self, v: &View, setup: &Setup) {
        v.text_centered(crate::config::APP_NAME, FIELD_W / 2.0, 170.0, 64.0, NEON);
        v.text_centered(t!("menu.subtitle"), FIELD_W / 2.0, 214.0, 22.0, TEXT);
        v.text_centered(
            &t!("setup.level", level = self.level),
            FIELD_W / 2.0,
            280.0,
            28.0,
            ACCENT,
        );
        let o = &setup.options;
        for (i, row) in ROWS.iter().enumerate() {
            let label = match row {
                Row::Play => t!("setup.play").to_owned(),
                Row::Columns => t!("setup.columns", n = o.columns),
                Row::Zoom => t!("setup.zoom", percent = o.board_zoom),
                Row::Motion => {
                    if o.reduced_motion {
                        t!("setup.motion_on").to_owned()
                    } else {
                        t!("setup.motion_off").to_owned()
                    }
                }
                Row::Flashes => {
                    if o.reduced_flashes {
                        t!("setup.flashes_on").to_owned()
                    } else {
                        t!("setup.flashes_off").to_owned()
                    }
                }
                Row::Range => t!("setup.range").to_owned(),
                Row::Lab => t!("setup.lab").to_owned(),
            };
            let rect = row_rect(i);
            let hover = self.pointer.is_some_and(|p| rect.contains(p));
            draw_button(v, rect, &label, hover || setup.focus == i, true);
        }
        if let Some(w) = &setup.warning {
            let text = match w {
                Warning::Unreadable | Warning::Invalid => t!("setup.warn_invalid"),
                Warning::NewerVersion => t!("setup.warn_newer"),
            };
            v.text_wrapped_centered(text, 820.0, 15.0, ERROR);
        }
        v.text_wrapped_centered(t!("setup.help"), 880.0, 15.0, with_alpha(TEXT, 0.7));
    }
}

impl Game for RustAMove {
    const TEXTS: &'static [Text] = TEXTS;
    const SOUNDS: &'static [&'static [u8]] = sfx::SOUNDS;

    fn new(level: u32) -> Self {
        let dev = crate::config::env_str("DEV").as_deref() == Some("1");
        RustAMove::with(level, dev, Some(persistence::default_path()))
    }

    fn handle(&mut self, input: &Input) {
        if self.done {
            return;
        }
        self.pointer = input.pointer;
        match &mut self.mode {
            Mode::Setup(_) => {
                self.load_options();
                self.pending.extend(input.keys.iter().copied());
                if input.left_pressed {
                    self.click = input.pointer.or(self.click);
                }
            }
            Mode::Play(r) => r.handle(input),
            Mode::Range(r) => {
                if input.key(Key::Enter) {
                    self.finish_requested = true;
                }
                r.handle(input);
            }
            Mode::Lab(_) => {
                for &key in &input.keys {
                    if key == Key::Enter {
                        self.finish_requested = true;
                    } else {
                        self.pending.push(key);
                    }
                }
            }
        }
    }

    fn update(&mut self, _dt: f32) {
        if self.done {
            return;
        }
        if self.finish_requested {
            self.done = true;
            return;
        }
        match &mut self.mode {
            Mode::Setup(_) => self.update_setup(),
            Mode::Play(r) => r.update(),
            Mode::Range(r) => r.update(),
            Mode::Lab(lab) => {
                for key in std::mem::take(&mut self.pending) {
                    lab.press(key);
                }
                lab.update();
            }
        }
    }

    fn draw(&self, view: &View) {
        match &self.mode {
            Mode::Setup(setup) => self.draw_setup(view, setup),
            Mode::Play(r) => {
                r.draw(view);
                if let Some(e) = &self.save_error {
                    if r.ticks < 360 {
                        view.text_wrapped_centered(
                            &t!("setup.save_failed", error = e),
                            96.0,
                            14.0,
                            ERROR,
                        );
                    }
                }
            }
            Mode::Range(r) => r.draw(view),
            Mode::Lab(lab) => lab.draw(view),
        }
    }

    fn take_sounds(&mut self) -> Vec<Sfx> {
        match &mut self.mode {
            Mode::Play(r) => r.take_sounds(),
            Mode::Range(r) => r.take_sounds(),
            _ => Vec::new(),
        }
    }

    fn finished(&self) -> Option<RoundResult> {
        if let Mode::Play(r) = &self.mode {
            return r
                .finished()
                .map(|(score, outcome)| RoundResult { score, outcome });
        }
        let score = match &self.mode {
            Mode::Range(r) => r.shots,
            _ => 0,
        };
        self.done.then_some(RoundResult {
            score,
            outcome: Outcome::Won,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::FIXED_DT;

    fn press(k: Key) -> Input {
        Input {
            keys: vec![k],
            ..Default::default()
        }
    }

    fn click(p: Vec2) -> Input {
        Input {
            pointer: Some(p),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        }
    }

    fn game() -> RustAMove {
        RustAMove::with(1, false, None)
    }

    fn row_index(row: Row) -> usize {
        ROWS.iter().position(|&r| r == row).unwrap()
    }

    fn lab_game() -> RustAMove {
        let mut g = game();
        g.handle(&click(row_rect(row_index(Row::Lab)).center()));
        g.update(FIXED_DT);
        assert!(matches!(g.mode, Mode::Lab(_)));
        g
    }

    fn lab(g: &RustAMove) -> &lab::Lab {
        match &g.mode {
            Mode::Lab(l) => l,
            _ => panic!("not in the lab"),
        }
    }

    /// F02/F03: a press is applied once, in the next update, however many
    /// updates follow one `handle`.
    #[test]
    fn presses_apply_once_in_update() {
        let mut g = lab_game();
        g.handle(&press(Key::X));
        assert_eq!(lab(&g).scenario, 0, "handle changes nothing");
        g.update(FIXED_DT);
        g.update(FIXED_DT);
        g.update(FIXED_DT);
        assert_eq!(lab(&g).scenario, 1);
        g.handle(&press(Key::X));
        g.handle(&Input::default());
        assert_eq!(lab(&g).scenario, 1);
        g.update(FIXED_DT);
        assert_eq!(lab(&g).scenario, 2);
    }

    #[test]
    fn enter_finishes_the_lab_and_stays_finished() {
        let mut g = lab_game();
        g.handle(&press(Key::Enter));
        assert_eq!(g.finished(), None);
        g.update(FIXED_DT);
        let won = Some(RoundResult {
            score: 0,
            outcome: Outcome::Won,
        });
        assert_eq!(g.finished(), won);
        let t = lab(&g).time();
        g.handle(&press(Key::X));
        g.update(FIXED_DT);
        assert_eq!(lab(&g).time(), t, "frozen");
        assert_eq!(g.finished(), won);
    }

    #[test]
    fn setup_starts_play_by_default_and_never_fires() {
        let mut g = game();
        g.handle(&press(Key::Space));
        g.update(FIXED_DT);
        let Mode::Play(r) = &g.mode else {
            panic!("play expected")
        };
        assert_eq!(r.shots, 0, "the choosing press does not fire");
        for _ in 0..30 {
            g.update(FIXED_DT);
        }
        let Mode::Play(r) = &g.mode else {
            panic!("play expected")
        };
        assert_eq!(r.shots, 0);
        // A click beside the rows does nothing.
        let mut g = game();
        g.handle(&click(Vec2::new(20.0, 20.0)));
        g.update(FIXED_DT);
        assert!(matches!(g.mode, Mode::Setup(_)));
    }

    #[test]
    fn setup_options_change_the_board_and_are_saved_on_start() {
        let dir = crate::tests::temp_dir("setup-options");
        let path = dir.join("game-settings.json");
        let mut g = RustAMove::with(2, false, Some(path.clone()));
        // Columns: two to the right with the keyboard, zoom: one down with
        // the mouse (left half).
        g.handle(&press(Key::Down));
        g.handle(&press(Key::Right));
        g.handle(&press(Key::Right));
        g.update(FIXED_DT);
        let zoom = row_rect(row_index(Row::Zoom));
        g.handle(&click(Vec2::new(zoom.x + 10.0, zoom.center().y)));
        g.update(FIXED_DT);
        g.handle(&click(row_rect(row_index(Row::Play)).center()));
        g.update(FIXED_DT);
        let Mode::Play(r) = &g.mode else {
            panic!("play expected")
        };
        assert_eq!((r.board.layout.columns, r.board.layout.zoom), (10, 0.95));
        let saved = persistence::load(&path).0;
        assert_eq!((saved.columns, saved.board_zoom), (10, 95));
        // The next round (or a retry) reads them back: same level, same
        // options, same board.
        let mut again = RustAMove::with(2, false, Some(path));
        again.handle(&press(Key::Space));
        again.update(FIXED_DT);
        let Mode::Play(r2) = &again.mode else {
            panic!("play expected")
        };
        let cells = |r: &round::Round| {
            r.board
                .occupied()
                .map(|(c, x)| (c, x.kind))
                .collect::<Vec<_>>()
        };
        assert_eq!(cells(r), cells(r2));
        assert_eq!((r.loaded, r.next), (r2.loaded, r2.next));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn range_and_lab_open_from_setup() {
        let mut g = game();
        g.handle(&click(row_rect(row_index(Row::Range)).center()));
        g.update(FIXED_DT);
        let Mode::Range(r) = &g.mode else {
            panic!("range expected")
        };
        assert_eq!(r.shots, 0);
        lab_game();
    }

    #[test]
    fn time_advances_only_in_update() {
        let mut g = lab_game();
        for _ in 0..10 {
            g.handle(&Input::default());
        }
        assert_eq!(lab(&g).time(), 0.0);
        for _ in 0..120 {
            g.update(FIXED_DT);
        }
        assert!((lab(&g).time() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn setup_rows_do_not_overlap_and_fit() {
        for i in 0..ROWS.len() {
            let r = row_rect(i);
            assert!(r.y + r.h < 820.0 && r.x >= 0.0 && r.x + r.w <= FIELD_W);
            if i > 0 {
                assert!(row_rect(i - 1).y + row_rect(i - 1).h < r.y);
            }
        }
    }
}
