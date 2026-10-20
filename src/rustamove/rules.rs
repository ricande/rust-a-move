//! The round's tunable rules, in one place (docs/game/05_CONFIGURATION_CONTENT.md).
//! All values are PROPOSED prototype defaults; times are in fixed 1/120 s
//! ticks so nothing depends on the frame rate.

/// Rules for one round. A level may override `ceiling_shots`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rules {
    /// Smallest group that disappears (fixed in v1).
    pub match_min: usize,
    /// Shots between ceiling drops; every shot counts.
    pub ceiling_shots: u32,
    /// Points per creature removed in a match or blast.
    pub match_score: u64,
    /// Points per creature that falls because nothing holds it.
    pub fall_score: u64,
    /// How long a matched creature pops before its fragments fly.
    pub pop_present_ticks: u64,
    /// Longest a falling creature is shown.
    pub fall_max_ticks: u64,
    /// The pause after the last important event before the round ends,
    /// so the recording shows the outcome.
    pub ending_hold_ticks: u64,
    /// The ceiling's visible slide when it drops.
    pub ceiling_slide_ticks: u64,
    /// Input lock after a shot that matched nothing.
    pub settle_ticks: u64,
}

impl Default for Rules {
    fn default() -> Self {
        Rules {
            match_min: 3,
            ceiling_shots: 5,
            match_score: 10,
            fall_score: 20,
            pop_present_ticks: 24,
            fall_max_ticks: 96,
            ending_hold_ticks: 96,
            ceiling_slide_ticks: 30,
            settle_ticks: 12,
        }
    }
}

/// Allowed range for a level's `ceiling_shots` override.
pub const CEILING_SHOTS_RANGE: std::ops::RangeInclusive<u32> = 2..=20;
