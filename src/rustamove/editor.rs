//! The level editor: paint creatures on a board, check that the level is
//! playable, test-play it with the real rules and come back, and save or
//! load it in nine slots, in the same JSON format as `assets/levels/`.
//!
//! Like the rest of the game, `handle` only records input and everything
//! happens in `update`. Files are read and written only on an explicit Save
//! or Load, never while drawing.

use std::path::PathBuf;

use macroquad::color::Color;
use macroquad::math::{vec2, Rect, Vec2};

use super::board::Board;
use super::content::{self, Level, LevelError, Source};
use super::emotion::{self, Cues, Family, Kind, Mood};
use super::geometry::{Cell, Layout, DANGER_Y};
use super::persistence::Options;
use super::range::draw_frame;
use super::render;
use super::round::Round;
use super::rules::{Rules, CEILING_SHOTS_RANGE, EXPLOSION_RADIUS_RANGE};
use crate::audio::Sfx;
use crate::game::{Outcome, FIXED_DT};
use crate::t;
use crate::ui::{Input, Key};
use crate::view::{draw_button, with_alpha, View, ACCENT, ERROR, FIELD_W, NEON, TEXT};

/// Save slots 1..=SLOTS.
pub const SLOTS: u8 = 9;
/// Undo steps kept.
const UNDO: usize = 100;

/// What a click or Space does to a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Paint(Family),
    Angry,
    Erase,
}

/// The palette, left to right.
pub const TOOLS: [Tool; 8] = [
    Tool::Paint(Family::Round),
    Tool::Paint(Family::Block),
    Tool::Paint(Family::Diamond),
    Tool::Paint(Family::Hex),
    Tool::Paint(Family::Triangle),
    Tool::Paint(Family::Oval),
    Tool::Angry,
    Tool::Erase,
];

/// The editor's buttons. Rectangles come from `button_rect`, shared by
/// input and drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Slot,
    Ceiling,
    Radius,
    Tool(usize),
    Runner,
    Test,
    Save,
    Load,
    Undo,
    Clear,
    Exit,
}

const BUTTONS: [Button; 18] = [
    Button::Slot,
    Button::Ceiling,
    Button::Radius,
    Button::Tool(0),
    Button::Tool(1),
    Button::Tool(2),
    Button::Tool(3),
    Button::Tool(4),
    Button::Tool(5),
    Button::Tool(6),
    Button::Tool(7),
    Button::Runner,
    Button::Test,
    Button::Save,
    Button::Load,
    Button::Undo,
    Button::Clear,
    Button::Exit,
];

/// Where a button is, in logical coordinates.
pub fn button_rect(b: Button) -> Rect {
    match b {
        Button::Slot => Rect::new(18.0, 52.0, 160.0, 36.0),
        Button::Ceiling => Rect::new(190.0, 52.0, 160.0, 36.0),
        Button::Radius => Rect::new(362.0, 52.0, 160.0, 36.0),
        Button::Tool(i) => Rect::new(14.0 + i as f32 * 57.0, 772.0, 52.0, 52.0),
        Button::Runner => Rect::new(14.0 + 8.0 * 57.0, 772.0, 52.0, 52.0),
        Button::Test => Rect::new(14.0, 834.0, 96.0, 40.0),
        Button::Save => Rect::new(117.0, 834.0, 96.0, 40.0),
        Button::Load => Rect::new(220.0, 834.0, 96.0, 40.0),
        Button::Undo => Rect::new(323.0, 834.0, 96.0, 40.0),
        Button::Clear => Rect::new(426.0, 834.0, 100.0, 40.0),
        Button::Exit => Rect::new(18.0, 6.0, 90.0, 36.0),
    }
}

/// A short-lived line in the status bar.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    Saved(u8),
    Loaded(u8),
    Empty(u8),
    Unreadable(u8, String),
    NotSaved(String),
    CannotTest,
    Tested(Outcome, u32),
}

/// Input gathered by `handle`, used by the next `update`.
#[derive(Clone, Debug, Default)]
struct Intent {
    keys: Vec<Key>,
    pointer: Option<Vec2>,
    pressed: bool,
    down: bool,
}

pub struct Editor {
    pub board: Board,
    pub options: Options,
    pub slot: u8,
    pub ceiling_shots: u32,
    pub explosion_radius: u8,
    pub tool: usize,
    pub runner: bool,
    /// The keyboard cursor.
    pub cursor: Cell,
    undo: Vec<Board>,
    /// The cell last painted in the current mouse stroke.
    stroke: Option<Cell>,
    /// The board before the current stroke, saved for undo at its first
    /// change, so a whole stroke is one undo step.
    stroke_undo: Option<Board>,
    intent: Intent,
    pointer: Option<Vec2>,
    /// Where slots are saved; `None` means no files (tests).
    dir: Option<PathBuf>,
    /// A level being test-played.
    pub test: Option<Box<Round>>,
    pub note: Option<(Note, u64)>,
    /// Set when the player leaves the editor.
    pub exit: bool,
    ticks: u64,
}

impl Editor {
    /// An empty editor for a board with the options' width and zoom.
    pub fn new(options: Options, dir: Option<PathBuf>) -> Editor {
        let layout = Layout::new(options.columns, options.zoom())
            .unwrap_or_else(|_| Layout::new(8, 1.0).expect("the default layout is valid"));
        Editor {
            board: Board::new(layout),
            options,
            slot: 1,
            ceiling_shots: Rules::default().ceiling_shots,
            explosion_radius: Rules::default().explosion_radius,
            tool: 0,
            runner: false,
            cursor: Cell::new(0, 0),
            undo: Vec::new(),
            stroke: None,
            stroke_undo: None,
            intent: Intent::default(),
            pointer: None,
            dir,
            test: None,
            note: None,
            exit: false,
            ticks: 0,
        }
    }

    fn layout(&self) -> Layout {
        self.board.layout
    }

    /// Cells a creature may be painted in: on the board and clear of the
    /// danger line.
    pub fn editable(&self, cell: Cell) -> bool {
        let l = self.layout();
        l.contains(cell) && l.centre(cell).y + l.r < DANGER_Y
    }

    /// The editable cell under a logical position.
    fn cell_at(&self, p: Vec2) -> Option<Cell> {
        let l = self.layout();
        let cell = l.nearest_cell(macroquad::math::dvec2(p.x as f64, p.y as f64))?;
        let inside = l
            .centre(cell)
            .distance(macroquad::math::dvec2(p.x as f64, p.y as f64))
            <= l.r;
        (inside && self.editable(cell)).then_some(cell)
    }

    fn remember(&mut self) {
        self.undo.push(self.board.clone());
        if self.undo.len() > UNDO {
            self.undo.remove(0);
        }
    }

    /// Applies the current tool to `cell`. Returns whether anything changed.
    fn apply(&mut self, cell: Cell) -> bool {
        if !self.editable(cell) {
            return false;
        }
        let want = match TOOLS[self.tool] {
            Tool::Paint(f) => Some((Kind::Ordinary(f), self.runner)),
            Tool::Angry => Some((Kind::Angry, false)),
            Tool::Erase => None,
        };
        let have = self.board.get(cell).map(|c| (c.kind, c.runner));
        if have == want {
            return false;
        }
        self.board.remove(cell);
        if let Some((kind, runner)) = want {
            self.board.place(cell, kind, runner);
        }
        true
    }

    pub fn handle(&mut self, input: &Input) {
        if let Some(r) = &mut self.test {
            if input.key(Key::Enter) {
                self.intent.keys.push(Key::Enter);
            }
            r.handle(input);
            return;
        }
        self.intent.keys.extend(input.keys.iter().copied());
        self.intent.pointer = input.pointer;
        self.intent.pressed |= input.left_pressed;
        self.intent.down = input.left_down;
    }

    pub fn update(&mut self) {
        self.ticks += 1;
        let intent = std::mem::take(&mut self.intent);
        self.intent.down = intent.down;
        self.intent.pointer = intent.pointer;
        if self.test.is_some() {
            self.update_test(&intent);
            return;
        }
        self.pointer = intent.pointer;
        for &key in &intent.keys {
            self.key(key);
        }
        self.mouse(&intent);
        if self
            .note
            .as_ref()
            .is_some_and(|(_, until)| self.ticks >= *until)
        {
            self.note = None;
        }
    }

    fn update_test(&mut self, intent: &Intent) {
        let stop = intent.keys.contains(&Key::Enter);
        let Some(r) = self.test.as_mut() else { return };
        r.update();
        let result = r.finished();
        if let Some((score, outcome)) = result {
            self.say(Note::Tested(outcome, score));
            self.test = None;
        } else if stop {
            self.test = None;
        }
    }

    fn say(&mut self, note: Note) {
        self.note = Some((note, self.ticks + 240));
    }

    fn key(&mut self, key: Key) {
        let l = self.layout();
        let c = self.cursor;
        match key {
            Key::Up | Key::Down | Key::Left | Key::Right => {
                let (mut row, mut col) = (c.row as i32, c.col as i32);
                match key {
                    Key::Up => row -= 1,
                    Key::Down => row += 1,
                    Key::Left => col -= 1,
                    _ => col += 1,
                }
                let row = row.clamp(0, l.rows as i32 - 1) as u16;
                let col = col.clamp(0, l.row_len(row) as i32 - 1) as u16;
                let next = Cell::new(row, col);
                if self.editable(next) {
                    self.cursor = next;
                }
            }
            Key::Space => {
                let before = self.board.clone();
                if self.apply(c) {
                    self.push_undo(before);
                }
            }
            Key::D => {
                if self.board.get(c).is_some() {
                    self.remember();
                    self.board.remove(c);
                }
            }
            Key::Z => self.tool = (self.tool + TOOLS.len() - 1) % TOOLS.len(),
            Key::X => self.tool = (self.tool + 1) % TOOLS.len(),
            Key::A => self.runner ^= true,
            Key::W => self.press(Button::Undo, false),
            Key::Enter => self.press(Button::Test, false),
            _ => {}
        }
    }

    fn push_undo(&mut self, before: Board) {
        self.undo.push(before);
        if self.undo.len() > UNDO {
            self.undo.remove(0);
        }
    }

    fn mouse(&mut self, intent: &Intent) {
        let Some(p) = intent.pointer else {
            self.stroke = None;
            return;
        };
        if intent.pressed {
            if let Some(b) = BUTTONS
                .iter()
                .copied()
                .find(|&b| button_rect(b).contains(p))
            {
                let right_half = p.x >= button_rect(b).center().x;
                self.press(b, right_half);
                return;
            }
            if let Some(cell) = self.cell_at(p) {
                // A new stroke: one undo step for all of it.
                self.stroke_undo = Some(self.board.clone());
                self.stroke = Some(cell);
                self.paint_stroke(cell);
            }
            return;
        }
        if intent.down && self.stroke.is_some() {
            if let Some(cell) = self.cell_at(p) {
                if Some(cell) != self.stroke {
                    self.stroke = Some(cell);
                    self.paint_stroke(cell);
                }
            }
        } else if !intent.down {
            self.stroke = None;
            self.stroke_undo = None;
        }
    }

    fn paint_stroke(&mut self, cell: Cell) {
        self.cursor = cell;
        let before = self.stroke_undo.clone();
        if self.apply(cell) {
            if let Some(b) = before {
                self.push_undo(b);
                self.stroke_undo = None;
            }
        }
    }

    /// A button. `up` is true for the right half of a `< value >` button.
    fn press(&mut self, b: Button, up: bool) {
        let step = if up { 1 } else { -1 };
        match b {
            Button::Slot => {
                self.slot = ((self.slot as i32 - 1 + step).rem_euclid(SLOTS as i32) + 1) as u8
            }
            Button::Ceiling => {
                let n = (self.ceiling_shots as i32 + step) as u32;
                self.ceiling_shots =
                    n.clamp(*CEILING_SHOTS_RANGE.start(), *CEILING_SHOTS_RANGE.end());
            }
            Button::Radius => {
                let n = (self.explosion_radius as i32 + step) as u8;
                self.explosion_radius = n.clamp(
                    *EXPLOSION_RADIUS_RANGE.start(),
                    *EXPLOSION_RADIUS_RANGE.end(),
                );
            }
            Button::Tool(i) => self.tool = i,
            Button::Runner => self.runner ^= true,
            Button::Test => self.start_test(),
            Button::Save => self.save(),
            Button::Load => self.load(),
            Button::Undo => {
                if let Some(b) = self.undo.pop() {
                    self.board = b;
                }
            }
            Button::Clear => {
                if self.board.count() > 0 {
                    self.remember();
                    self.board = Board::new(self.layout());
                }
            }
            Button::Exit => self.exit = true,
        }
    }

    /// The level as it would be saved.
    pub fn level(&self) -> Level {
        let defaults = Rules::default();
        Level {
            id: format!("custom-{}", self.slot),
            columns: self.layout().columns,
            seed: 1000 + self.slot as u64,
            cells: self
                .board
                .occupied()
                .map(|(cell, c)| (cell, c.kind, c.runner))
                .collect(),
            ceiling_shots: (self.ceiling_shots != defaults.ceiling_shots)
                .then_some(self.ceiling_shots),
            explosion_radius: (self.explosion_radius != defaults.explosion_radius)
                .then_some(self.explosion_radius),
        }
    }

    /// Why the level cannot be played yet, if it cannot.
    pub fn problem(&self) -> Option<LevelError> {
        let level = self.level();
        if level.cells.is_empty() {
            return Some(LevelError::Empty);
        }
        content::build(&level, self.layout()).err()
    }

    /// Creatures that do not hang from the ceiling.
    fn loose(&self) -> Vec<Cell> {
        let anchored = self.board.anchored();
        self.board
            .occupied()
            .map(|(c, _)| c)
            .filter(|c| !anchored.contains(c))
            .collect()
    }

    fn start_test(&mut self) {
        let level = self.level();
        // Same checks as the status line: not empty, all hanging, clear of
        // the danger line.
        let board = match self.problem() {
            None => content::build(&level, self.layout()).ok(),
            Some(_) => None,
        };
        let Some(board) = board else {
            self.say(Note::CannotTest);
            return;
        };
        let rules = Rules {
            ceiling_shots: self.ceiling_shots,
            explosion_radius: self.explosion_radius,
            ..Rules::default()
        };
        let mut r = Round::with_board(
            1,
            board,
            rules,
            level.seed,
            Source::Custom(level.id),
            self.options.reduced_motion,
        );
        r.reduced_flashes = self.options.reduced_flashes;
        r.quality = self.options.visual_quality;
        self.test = Some(Box::new(r));
    }

    fn slot_path(&self) -> Option<PathBuf> {
        self.dir
            .as_ref()
            .map(|d| d.join(format!("custom-{}.json", self.slot)))
    }

    fn save(&mut self) {
        let Some(path) = self.slot_path() else { return };
        let text = content::to_json(&self.level());
        let write = || -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, text)?;
            std::fs::rename(&tmp, &path)
        };
        match write() {
            Ok(()) => self.say(Note::Saved(self.slot)),
            Err(e) => self.say(Note::NotSaved(e.to_string())),
        }
    }

    fn load(&mut self) {
        let Some(path) = self.slot_path() else { return };
        let slot = self.slot;
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return self.say(Note::Empty(slot))
            }
            Err(e) => return self.say(Note::Unreadable(slot, e.to_string())),
        };
        let level = match content::parse(&text) {
            Ok(l) => l,
            Err(e) => return self.say(Note::Unreadable(slot, format!("{e:?}"))),
        };
        let Ok(layout) = Layout::new(level.columns, self.layout().zoom) else {
            return self.say(Note::Unreadable(slot, format!("{} columns", level.columns)));
        };
        let mut board = Board::new(layout);
        for &(cell, kind, runner) in &level.cells {
            if layout.contains(cell) {
                board.place(cell, kind, runner);
            }
        }
        self.remember();
        self.board = board;
        self.ceiling_shots = level
            .ceiling_shots
            .unwrap_or(Rules::default().ceiling_shots);
        self.explosion_radius = level
            .explosion_radius
            .unwrap_or(Rules::default().explosion_radius);
        self.cursor = Cell::new(0, 0);
        self.say(Note::Loaded(slot));
    }

    pub fn take_sounds(&mut self) -> Vec<Sfx> {
        self.test
            .as_mut()
            .map_or_else(Vec::new, |r| r.take_sounds())
    }

    pub fn draw(&self, v: &View) {
        if let Some(r) = &self.test {
            r.draw(v);
            // On a dark strip of its own at the very bottom.
            v.rect(0.0, 938.0, FIELD_W, 22.0, Color::new(0.0, 0.0, 0.0, 0.75));
            v.text_centered(t!("editor.testing"), FIELD_W / 2.0, 954.0, 14.0, ACCENT);
            return;
        }
        let l = self.layout();
        let r = l.r as f32;
        draw_frame(v, &l);
        let t = self.ticks as f64 * FIXED_DT as f64;
        // Empty editable cells as faint rings.
        let ring = with_alpha(TEXT, 0.12);
        for cell in l
            .cells()
            .filter(|&c| self.editable(c) && self.board.is_empty(c))
        {
            let p = l.centre(cell);
            v.circle_lines(vec2(p.x as f32, p.y as f32), r * 0.85, 1.0, ring);
        }
        let loose = self.loose();
        for (cell, c) in self.board.occupied() {
            let p = l.centre(cell);
            let at = vec2(p.x as f32, p.y as f32);
            let pose = emotion::pose(
                c.kind,
                &Cues::default(),
                &Mood::default(),
                t,
                c.seed(),
                self.options.reduced_motion,
            );
            render::draw_creature(v, at, r, c.kind, c.runner, &pose, 1.0);
            if loose.contains(&cell) {
                v.circle_lines(at, r * 1.05, 2.5, ERROR);
            }
        }
        // Hover and cursor.
        if let Some(cell) = self.pointer.and_then(|p| self.cell_at(p)) {
            let p = l.centre(cell);
            v.circle_lines(vec2(p.x as f32, p.y as f32), r, 2.0, with_alpha(NEON, 0.6));
        }
        let p = l.centre(self.cursor);
        v.circle_lines(vec2(p.x as f32, p.y as f32), r * 1.1, 2.0, ACCENT);
        self.draw_bar(v);
    }

    fn draw_bar(&self, v: &View) {
        let hot = |b: Button| self.pointer.is_some_and(|p| button_rect(b).contains(p));
        draw_button(
            v,
            button_rect(Button::Exit),
            t!("editor.exit"),
            hot(Button::Exit),
            true,
        );
        v.text(t!("editor.title"), 122.0, 32.0, 22.0, NEON);
        let status = match self.problem() {
            None => (t!("editor.ok").to_owned(), ACCENT),
            Some(LevelError::Empty) => (t!("editor.empty").to_owned(), with_alpha(TEXT, 0.7)),
            Some(LevelError::Loose(_)) => (t!("editor.loose", n = self.loose().len()), ERROR),
            Some(LevelError::Danger(_)) => (t!("editor.danger").to_owned(), ERROR),
            Some(_) => (t!("editor.invalid").to_owned(), ERROR),
        };
        let (text, colour) = match &self.note {
            Some((note, _)) => (self.note_text(note), ACCENT),
            None => status,
        };
        let w = v.text_width(&text, 15.0);
        v.text(&text, (FIELD_W - 16.0 - w).max(240.0), 30.0, 15.0, colour);
        let labels = [
            (Button::Slot, t!("editor.slot", n = self.slot)),
            (
                Button::Ceiling,
                t!("editor.ceiling", n = self.ceiling_shots),
            ),
            (
                Button::Radius,
                t!("editor.radius", n = self.explosion_radius),
            ),
        ];
        for (b, label) in labels {
            draw_button(v, button_rect(b), &label, hot(b), true);
        }
        // Palette: the creature itself on each button.
        let t = self.ticks as f64 * FIXED_DT as f64;
        for (i, tool) in TOOLS.iter().enumerate() {
            let rect = button_rect(Button::Tool(i));
            let chosen = self.tool == i;
            draw_button(v, rect, "", chosen || hot(Button::Tool(i)), true);
            if chosen {
                v.rect_lines(
                    rect.x - 2.0,
                    rect.y - 2.0,
                    rect.w + 4.0,
                    rect.h + 4.0,
                    3.0,
                    ACCENT,
                );
            }
            let c = rect.center();
            match tool {
                Tool::Paint(f) => {
                    let kind = Kind::Ordinary(*f);
                    let pose =
                        emotion::pose(kind, &Cues::default(), &Mood::default(), t, i as u64, true);
                    render::draw_creature(v, c, 18.0, kind, self.runner, &pose, 1.0);
                }
                Tool::Angry => {
                    let pose =
                        emotion::pose(Kind::Angry, &Cues::default(), &Mood::default(), t, 9, true);
                    render::draw_creature(v, c, 18.0, Kind::Angry, false, &pose, 1.0);
                }
                Tool::Erase => {
                    let x = Color::new(1.0, 0.45, 0.45, 1.0);
                    v.line(c - vec2(12.0, 12.0), c + vec2(12.0, 12.0), 4.0, x);
                    v.line(c - vec2(12.0, -12.0), c + vec2(12.0, -12.0), 4.0, x);
                }
            }
        }
        let rr = button_rect(Button::Runner);
        draw_button(v, rr, "", self.runner || hot(Button::Runner), true);
        if self.runner {
            v.rect_lines(rr.x - 2.0, rr.y - 2.0, rr.w + 4.0, rr.h + 4.0, 3.0, ACCENT);
        }
        // A sweatband: the runner sign.
        let c = rr.center();
        v.rect(
            c.x - 16.0,
            c.y - 6.0,
            32.0,
            12.0,
            Color::new(0.97, 0.97, 0.97, 1.0),
        );
        v.rect(
            c.x - 16.0,
            c.y - 1.5,
            32.0,
            3.0,
            Color::new(0.9, 0.15, 0.2, 1.0),
        );
        v.text_centered(t!("editor.runner"), c.x, rr.y + rr.h - 4.0, 11.0, TEXT);
        let actions = [
            (Button::Test, t!("editor.test")),
            (Button::Save, t!("editor.save")),
            (Button::Load, t!("editor.load")),
            (Button::Undo, t!("editor.undo")),
            (Button::Clear, t!("editor.clear")),
        ];
        for (b, label) in actions {
            draw_button(v, button_rect(b), label, hot(b), true);
        }
        v.text_wrapped_centered(t!("editor.help"), 900.0, 13.0, with_alpha(TEXT, 0.6));
    }

    fn note_text(&self, note: &Note) -> String {
        match note {
            Note::Saved(n) => t!("editor.saved", n = n),
            Note::Loaded(n) => t!("editor.loaded", n = n),
            Note::Empty(n) => t!("editor.slot_empty", n = n),
            Note::Unreadable(n, e) => t!("editor.unreadable", n = n, error = e),
            Note::NotSaved(e) => t!("editor.not_saved", error = e),
            Note::CannotTest => t!("editor.cannot_test").to_owned(),
            Note::Tested(Outcome::Won, s) => t!("editor.test_won", score = s),
            Note::Tested(Outcome::Lost, s) => t!("editor.test_lost", score = s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> Editor {
        Editor::new(Options::default(), None)
    }

    fn click(p: Vec2) -> Input {
        Input {
            pointer: Some(p),
            left_pressed: true,
            left_down: true,
            ..Default::default()
        }
    }

    fn drag(p: Vec2) -> Input {
        Input {
            pointer: Some(p),
            left_down: true,
            ..Default::default()
        }
    }

    fn press(k: Key) -> Input {
        Input {
            keys: vec![k],
            ..Default::default()
        }
    }

    fn step(e: &mut Editor, input: Input) {
        e.handle(&input);
        e.update();
    }

    fn centre(e: &Editor, cell: Cell) -> Vec2 {
        let p = e.layout().centre(cell);
        vec2(p.x as f32, p.y as f32)
    }

    fn tool_index(tool: Tool) -> usize {
        TOOLS.iter().position(|&t| t == tool).unwrap()
    }

    #[test]
    fn clicking_paints_and_erases_and_undo_restores() {
        let mut e = editor();
        step(
            &mut e,
            click(button_rect(Button::Tool(tool_index(Tool::Paint(Family::Hex)))).center()),
        );
        let p = centre(&e, Cell::new(0, 2));
        step(&mut e, click(p));
        assert_eq!(
            e.board.get(Cell::new(0, 2)).unwrap().kind,
            Kind::Ordinary(Family::Hex)
        );
        // A different tool on the same cell replaces it.
        step(
            &mut e,
            click(button_rect(Button::Tool(tool_index(Tool::Angry))).center()),
        );
        let p = centre(&e, Cell::new(0, 2));
        step(&mut e, click(p));
        assert_eq!(e.board.get(Cell::new(0, 2)).unwrap().kind, Kind::Angry);
        step(
            &mut e,
            click(button_rect(Button::Tool(tool_index(Tool::Erase))).center()),
        );
        let p = centre(&e, Cell::new(0, 2));
        step(&mut e, click(p));
        assert!(e.board.get(Cell::new(0, 2)).is_none());
        // Undo, step by step.
        step(&mut e, click(button_rect(Button::Undo).center()));
        assert_eq!(e.board.get(Cell::new(0, 2)).unwrap().kind, Kind::Angry);
        step(&mut e, press(Key::W));
        assert_eq!(
            e.board.get(Cell::new(0, 2)).unwrap().kind,
            Kind::Ordinary(Family::Hex)
        );
        step(&mut e, press(Key::W));
        assert_eq!(e.board.count(), 0);
    }

    #[test]
    fn dragging_paints_a_stroke_undone_at_once() {
        let mut e = editor();
        let p = centre(&e, Cell::new(0, 0));
        step(&mut e, click(p));
        for col in 1..6 {
            let p = centre(&e, Cell::new(0, col));
            step(&mut e, drag(p));
        }
        step(&mut e, Input::default());
        assert_eq!(e.board.count(), 6);
        // Moving without the button down paints nothing.
        let p = centre(&e, Cell::new(0, 7));
        step(
            &mut e,
            Input {
                pointer: Some(p),
                ..Default::default()
            },
        );
        assert_eq!(e.board.count(), 6);
        step(&mut e, press(Key::W));
        assert_eq!(e.board.count(), 0, "the stroke is one undo step");
    }

    #[test]
    fn runners_and_keyboard_painting() {
        let mut e = editor();
        step(&mut e, press(Key::A)); // runner on
        step(&mut e, press(Key::X)); // block
        step(&mut e, press(Key::Right));
        step(&mut e, press(Key::Space));
        let c = e.board.get(Cell::new(0, 1)).unwrap();
        assert_eq!((c.kind, c.runner), (Kind::Ordinary(Family::Block), true));
        step(&mut e, press(Key::D));
        assert!(e.board.get(Cell::new(0, 1)).is_none());
        // Angry never becomes a runner.
        e.tool = tool_index(Tool::Angry);
        step(&mut e, press(Key::Space));
        assert!(!e.board.get(Cell::new(0, 1)).unwrap().runner);
    }

    #[test]
    fn nothing_goes_below_the_danger_line_or_off_the_board() {
        let mut e = editor();
        for _ in 0..40 {
            step(&mut e, press(Key::Down));
        }
        assert!(e.editable(e.cursor));
        let below = Cell::new(e.cursor.row + 1, 0);
        assert!(!e.editable(below));
        let p = centre(&e, below);
        step(&mut e, click(p));
        assert_eq!(e.board.count(), 0);
        step(&mut e, click(vec2(5.0, 300.0)));
        assert_eq!(e.board.count(), 0);
    }

    #[test]
    fn problems_are_reported_and_block_testing() {
        let mut e = editor();
        assert_eq!(e.problem(), Some(LevelError::Empty));
        step(&mut e, press(Key::Enter));
        assert!(e.test.is_none());
        assert_eq!(e.note.as_ref().map(|n| n.0.clone()), Some(Note::CannotTest));
        // A creature hanging from nothing.
        e.board
            .place(Cell::new(2, 2), Kind::Ordinary(Family::Oval), false);
        assert!(matches!(e.problem(), Some(LevelError::Loose(_))));
        assert_eq!(e.loose(), vec![Cell::new(2, 2)]);
        e.board
            .place(Cell::new(1, 2), Kind::Ordinary(Family::Oval), false);
        e.board
            .place(Cell::new(0, 2), Kind::Ordinary(Family::Hex), false);
        assert_eq!(e.problem(), None);
    }

    /// Testing plays the level with the real rules; when the round ends the
    /// editor is back with the level untouched.
    #[test]
    fn test_play_returns_to_the_editor() {
        let mut e = editor();
        for col in 0..3 {
            e.board
                .place(Cell::new(0, col), Kind::Ordinary(Family::Hex), false);
        }
        let before: Vec<_> = e.board.occupied().map(|(c, x)| (c, x.kind)).collect();
        step(&mut e, press(Key::Enter));
        assert!(e.test.is_some());
        // The only family on the board is hex, so every shot is hex: aim at
        // the row and fire until the round ends.
        let target = centre(&e, Cell::new(0, 1));
        for _ in 0..3000 {
            if e.test.is_none() {
                break;
            }
            step(&mut e, click(target));
        }
        assert!(e.test.is_none(), "the test ended");
        assert!(matches!(
            e.note.as_ref().map(|n| n.0.clone()),
            Some(Note::Tested(Outcome::Won, _))
        ));
        let after: Vec<_> = e.board.occupied().map(|(c, x)| (c, x.kind)).collect();
        assert_eq!(before, after, "the level is unchanged");
        // Enter during a test goes straight back.
        step(&mut e, press(Key::Enter));
        assert!(e.test.is_some());
        step(&mut e, press(Key::Enter));
        assert!(e.test.is_none());
    }

    #[test]
    fn slots_save_and_load_in_the_level_format() {
        let dir = crate::tests::temp_dir("editor-slots");
        let mut e = Editor::new(Options::default(), Some(dir.clone()));
        e.board
            .place(Cell::new(0, 0), Kind::Ordinary(Family::Round), false);
        e.board.place(Cell::new(0, 1), Kind::Angry, false);
        e.board
            .place(Cell::new(1, 0), Kind::Ordinary(Family::Oval), true);
        step(
            &mut e,
            click(button_rect(Button::Slot).center() + vec2(40.0, 0.0)),
        ); // slot 2
        step(
            &mut e,
            click(button_rect(Button::Ceiling).center() + vec2(40.0, 0.0)),
        ); // 6
        step(&mut e, click(button_rect(Button::Save).center()));
        assert_eq!(e.note.as_ref().map(|n| n.0.clone()), Some(Note::Saved(2)));
        let text = std::fs::read_to_string(dir.join("custom-2.json")).unwrap();
        let level = content::parse(&text).unwrap();
        assert_eq!(level, e.level());
        assert_eq!(level.ceiling_shots, Some(6));
        assert!(
            content::build(&level, e.layout()).is_ok(),
            "playable as a level file"
        );
        // Load it into a fresh editor.
        let mut f = Editor::new(Options::default(), Some(dir.clone()));
        step(
            &mut f,
            click(button_rect(Button::Slot).center() + vec2(40.0, 0.0)),
        );
        step(&mut f, click(button_rect(Button::Load).center()));
        assert_eq!(f.level(), e.level());
        assert_eq!(f.ceiling_shots, 6);
        // An empty slot says so and changes nothing.
        step(
            &mut f,
            click(button_rect(Button::Slot).center() + vec2(40.0, 0.0)),
        );
        step(&mut f, click(button_rect(Button::Load).center()));
        assert_eq!(f.note.as_ref().map(|n| n.0.clone()), Some(Note::Empty(3)));
        assert_eq!(f.board.count(), 3);
        // A broken file is reported, not loaded.
        std::fs::write(dir.join("custom-3.json"), "{").unwrap();
        step(&mut f, click(button_rect(Button::Load).center()));
        assert!(matches!(
            f.note.as_ref().map(|n| n.0.clone()),
            Some(Note::Unreadable(3, _))
        ));
        assert_eq!(f.board.count(), 3);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn values_stay_in_their_ranges_and_slots_wrap() {
        let mut e = editor();
        for _ in 0..30 {
            step(
                &mut e,
                click(button_rect(Button::Ceiling).center() + vec2(40.0, 0.0)),
            );
            step(
                &mut e,
                click(button_rect(Button::Radius).center() + vec2(40.0, 0.0)),
            );
        }
        assert_eq!((e.ceiling_shots, e.explosion_radius), (20, 3));
        step(
            &mut e,
            click(button_rect(Button::Slot).center() - vec2(40.0, 0.0)),
        );
        assert_eq!(e.slot, SLOTS, "1 wraps to the last slot");
    }

    #[test]
    fn buttons_fit_and_do_not_overlap() {
        for (i, &a) in BUTTONS.iter().enumerate() {
            let r = button_rect(a);
            assert!(
                r.x >= 0.0 && r.x + r.w <= FIELD_W && r.y >= 0.0 && r.y + r.h <= 960.0,
                "{a:?}"
            );
            for &b in &BUTTONS[i + 1..] {
                assert!(r.intersect(button_rect(b)).is_none(), "{a:?} {b:?}");
            }
        }
        // The board's editable area stays above the palette.
        let e = editor();
        for cell in e.layout().cells().filter(|&c| e.editable(c)) {
            let p = e.layout().centre(cell);
            assert!(p.y as f32 + e.layout().r as f32 <= button_rect(Button::Tool(0)).y);
        }
    }
}
