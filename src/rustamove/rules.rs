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
    /// Blast reach in hex-graph steps (1–3); a level may override it.
    pub explosion_radius: u8,
    /// Angry's smile before its burst.
    pub blast_smile_ticks: u64,
    /// Delay between the waves of a chain.
    pub blast_wave_ticks: u64,
    /// Flash, ring and fragments of one burst.
    pub blast_present_ticks: u64,
    /// At most this many extra sound accents for later waves of a chain.
    pub blast_accents: u32,
    /// A runner leaves on open aiming tick number max(1, this): 90 is
    /// 0.75 s, so it is still there on tick 89 and gone on tick 90.
    pub runner_struggle_ticks: u32,
    /// The same next to an Angry: 0 means no extra delay, it leaves on its
    /// first open tick.
    pub panic_struggle_ticks: u32,
    /// A runner's dash off the board; input stays locked meanwhile.
    pub escape_present_ticks: u64,
    /// Points for a runner that gets away.
    pub escape_score: u64,
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
            explosion_radius: 1,
            blast_smile_ticks: 7,
            blast_wave_ticks: 12,
            blast_present_ticks: 42,
            blast_accents: 3,
            runner_struggle_ticks: 90,
            panic_struggle_ticks: 0,
            escape_present_ticks: 30,
            escape_score: 10,
        }
    }
}

/// Allowed range for a level's `ceiling_shots` override.
pub const CEILING_SHOTS_RANGE: std::ops::RangeInclusive<u32> = 2..=20;
/// Allowed explosion radius (1 is the default; 2 and 3 are for testing).
pub const EXPLOSION_RADIUS_RANGE: std::ops::RangeInclusive<u8> = 1..=3;
