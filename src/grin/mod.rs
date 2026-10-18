//! GRIN (working title): a Snood-inspired aiming and matching game with
//! grumpy geometric creatures. See docs/GAME_RULES.md and docs/game/.
//!
//! Milestone M1: a round opens the character animation lab (`lab.rs`).
//! The playable board follows in M2–M3; it will sit next to the lab behind
//! a game-local setup choice.
//!
//! `handle` only records what was pressed; everything changes in `update`,
//! because the framework may call `update` several times (or not at all)
//! after one `handle`.

mod emotion;
mod geometry;
mod lab;
mod render;

use crate::audio::Sfx;
use crate::game::{Game, RoundResult};
use crate::i18n::Text;
use crate::ui::{Input, Key};
use crate::view::View;

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
];

pub struct GrinGame {
    lab: lab::Lab,
    /// Presses since the last update, applied there in order.
    pending: Vec<Key>,
    finish_requested: bool,
    done: bool,
}

impl Game for GrinGame {
    const TEXTS: &'static [Text] = TEXTS;

    fn new(_level: u32) -> Self {
        GrinGame {
            lab: lab::Lab::new(),
            pending: Vec::new(),
            finish_requested: false,
            done: false,
        }
    }

    fn handle(&mut self, input: &Input) {
        if self.done {
            return;
        }
        for &key in &input.keys {
            if key == Key::Enter {
                self.finish_requested = true;
            } else {
                self.pending.push(key);
            }
        }
    }

    fn update(&mut self, _dt: f32) {
        if self.done {
            return;
        }
        for key in std::mem::take(&mut self.pending) {
            self.lab.press(key);
        }
        if self.finish_requested {
            self.done = true;
            return;
        }
        self.lab.update();
    }

    fn draw(&self, view: &View) {
        self.lab.draw(view);
    }

    fn take_sounds(&mut self) -> Vec<Sfx> {
        Vec::new()
    }

    fn finished(&self) -> Option<RoundResult> {
        self.done.then_some(RoundResult { score: 0 })
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

    /// F02/F03: a press is applied once, in the next update, however many
    /// updates follow one `handle`.
    #[test]
    fn presses_apply_once_in_update() {
        let mut g = GrinGame::new(1);
        g.handle(&press(Key::X));
        assert_eq!(g.lab.scenario, 0, "handle changes nothing");
        g.update(FIXED_DT);
        g.update(FIXED_DT);
        g.update(FIXED_DT);
        assert_eq!(g.lab.scenario, 1);
        // No update: nothing yet, then exactly one step.
        g.handle(&press(Key::X));
        g.handle(&Input::default());
        assert_eq!(g.lab.scenario, 1);
        g.update(FIXED_DT);
        assert_eq!(g.lab.scenario, 2);
    }

    #[test]
    fn enter_finishes_and_stays_finished() {
        let mut g = GrinGame::new(1);
        g.handle(&press(Key::Enter));
        assert_eq!(g.finished(), None);
        g.update(FIXED_DT);
        assert_eq!(g.finished(), Some(RoundResult { score: 0 }));
        let t = g.lab.time();
        g.handle(&press(Key::X));
        g.update(FIXED_DT);
        assert_eq!(g.lab.time(), t, "frozen");
        assert_eq!(g.finished(), Some(RoundResult { score: 0 }));
    }

    #[test]
    fn time_advances_only_in_update() {
        let mut g = GrinGame::new(1);
        for _ in 0..10 {
            g.handle(&Input::default());
        }
        assert_eq!(g.lab.time(), 0.0);
        for _ in 0..120 {
            g.update(FIXED_DT);
        }
        assert!((g.lab.time() - 1.0).abs() < 1e-9);
    }
}
