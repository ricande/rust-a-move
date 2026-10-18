//! GRIN (working title): a Snood-inspired aiming and matching game with
//! grumpy geometric creatures. See docs/GAME_RULES.md and docs/game/.
//!
//! A round starts with a game-local setup screen (rule P13: the template's
//! `Game::new(level)` has no options hook). It offers the shooting range
//! (milestone M2, `range.rs`: aiming by fear, shots, attachment) and the
//! character animation lab (M1, `lab.rs`). Matching and a real ending come
//! in M3.
//!
//! `GRIN_DEV=1` turns on development overlays (colliders, shot path) in the
//! range; normal play never shows a trajectory.
//!
//! `handle` only records what was pressed; everything changes in `update`,
//! because the framework may call `update` several times (or not at all)
//! after one `handle`.

mod board;
mod emotion;
mod geometry;
mod lab;
mod range;
mod render;
mod rng;
mod shot;

use macroquad::math::{Rect, Vec2};

use crate::audio::Sfx;
use crate::game::{Game, RoundResult};
use crate::i18n::Text;
use crate::t;
use crate::ui::{Input, Key};
use crate::view::{draw_button, with_alpha, View, FIELD_W, NEON, TEXT};

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
    ("setup.range", "Shooting range (M2)", "Skjutbana (M2)"),
    ("setup.lab", "Animation lab (M1)", "Animationsverkstad (M1)"),
    (
        "setup.help",
        "Up/Down and Space, or the mouse. In both: Enter ends the round.",
        "Upp/Ned och Mellanslag, eller musen. I båda: Enter avslutar rundan.",
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
        "Angry hit! (The blast comes in M4.)",
        "Träff på Arg! (Explosionen kommer i M4.)",
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

/// The setup screen's buttons: shooting range, animation lab.
pub const SETUP_BUTTONS: [Rect; 2] = [
    Rect {
        x: 110.0,
        y: 430.0,
        w: 320.0,
        h: 56.0,
    },
    Rect {
        x: 110.0,
        y: 500.0,
        w: 320.0,
        h: 56.0,
    },
];

enum Mode {
    /// Choosing; `focus` is the highlighted button.
    Setup {
        focus: usize,
    },
    Range(Box<range::Range>),
    Lab(Box<lab::Lab>),
}

pub struct GrinGame {
    level: u32,
    dev: bool,
    mode: Mode,
    /// Setup and lab presses since the last update, applied there in order.
    pending: Vec<Key>,
    /// A setup click since the last update.
    click: Option<Vec2>,
    pointer: Option<Vec2>,
    finish_requested: bool,
    done: bool,
}

impl GrinGame {
    fn with_dev(level: u32, dev: bool) -> GrinGame {
        GrinGame {
            level,
            dev,
            mode: Mode::Setup { focus: 0 },
            pending: Vec::new(),
            click: None,
            pointer: None,
            finish_requested: false,
            done: false,
        }
    }

    fn start(&mut self, choice: usize) {
        self.mode = if choice == 0 {
            Mode::Range(Box::new(range::Range::new(self.level, self.dev)))
        } else {
            Mode::Lab(Box::default())
        };
    }

    fn update_setup(&mut self, mut focus: usize) {
        let mut chosen = None;
        for key in std::mem::take(&mut self.pending) {
            match key {
                Key::Up | Key::Down => focus = 1 - focus,
                Key::Space | Key::Enter => chosen = Some(focus),
                _ => {}
            }
        }
        if let Some(p) = self.click.take() {
            if let Some(i) = SETUP_BUTTONS.iter().position(|b| b.contains(p)) {
                chosen = Some(i);
            }
        }
        match chosen {
            Some(i) => self.start(i),
            None => self.mode = Mode::Setup { focus },
        }
    }

    fn draw_setup(&self, v: &View, focus: usize) {
        v.text_centered("GRIN", FIELD_W / 2.0, 250.0, 72.0, NEON);
        v.text_centered(t!("menu.subtitle"), FIELD_W / 2.0, 300.0, 22.0, TEXT);
        let labels = [t!("setup.range"), t!("setup.lab")];
        for (i, (rect, label)) in SETUP_BUTTONS.iter().zip(labels).enumerate() {
            let hover = self.pointer.is_some_and(|p| rect.contains(p));
            draw_button(v, *rect, label, hover || focus == i, true);
        }
        v.text_wrapped_centered(t!("setup.help"), 640.0, 16.0, with_alpha(TEXT, 0.7));
    }
}

impl Game for GrinGame {
    const TEXTS: &'static [Text] = TEXTS;

    fn new(level: u32) -> Self {
        let dev = crate::config::env_str("DEV").as_deref() == Some("1");
        GrinGame::with_dev(level, dev)
    }

    fn handle(&mut self, input: &Input) {
        if self.done {
            return;
        }
        self.pointer = input.pointer;
        match &mut self.mode {
            Mode::Setup { .. } => {
                self.pending.extend(input.keys.iter().copied());
                if input.left_pressed {
                    self.click = input.pointer.or(self.click);
                }
            }
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
            Mode::Setup { focus } => {
                let focus = *focus;
                self.update_setup(focus);
            }
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
            Mode::Setup { focus } => self.draw_setup(view, *focus),
            Mode::Range(r) => r.draw(view),
            Mode::Lab(lab) => lab.draw(view),
        }
    }

    fn take_sounds(&mut self) -> Vec<Sfx> {
        match &mut self.mode {
            Mode::Range(r) => r.take_sounds(),
            _ => Vec::new(),
        }
    }

    fn finished(&self) -> Option<RoundResult> {
        let score = match &self.mode {
            Mode::Range(r) => r.shots,
            _ => 0,
        };
        self.done.then_some(RoundResult { score })
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

    fn lab_game() -> GrinGame {
        let mut g = GrinGame::with_dev(1, false);
        g.handle(&click(SETUP_BUTTONS[1].center()));
        g.update(FIXED_DT);
        assert!(matches!(g.mode, Mode::Lab(_)));
        g
    }

    fn lab(g: &GrinGame) -> &lab::Lab {
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
    fn enter_finishes_and_stays_finished() {
        let mut g = lab_game();
        g.handle(&press(Key::Enter));
        assert_eq!(g.finished(), None);
        g.update(FIXED_DT);
        assert_eq!(g.finished(), Some(RoundResult { score: 0 }));
        let t = lab(&g).time();
        g.handle(&press(Key::X));
        g.update(FIXED_DT);
        assert_eq!(lab(&g).time(), t, "frozen");
        assert_eq!(g.finished(), Some(RoundResult { score: 0 }));
    }

    #[test]
    fn setup_chooses_by_keys_and_never_fires() {
        let mut g = GrinGame::with_dev(1, false);
        g.handle(&press(Key::Down));
        g.update(FIXED_DT);
        assert!(matches!(g.mode, Mode::Setup { focus: 1 }));
        g.handle(&press(Key::Up));
        g.handle(&press(Key::Space));
        g.update(FIXED_DT);
        let Mode::Range(r) = &g.mode else {
            panic!("range expected")
        };
        assert_eq!(r.shots, 0, "the choosing press does not fire");
        // Clicking the range button with the mouse does not fire either.
        let mut g = GrinGame::with_dev(1, false);
        g.handle(&click(SETUP_BUTTONS[0].center()));
        for _ in 0..10 {
            g.update(FIXED_DT);
        }
        let Mode::Range(r) = &g.mode else {
            panic!("range expected")
        };
        assert_eq!(r.shots, 0);
        // A click beside the buttons does nothing.
        let mut g = GrinGame::with_dev(1, false);
        g.handle(&click(Vec2::new(20.0, 20.0)));
        g.update(FIXED_DT);
        assert!(matches!(g.mode, Mode::Setup { .. }));
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
}
