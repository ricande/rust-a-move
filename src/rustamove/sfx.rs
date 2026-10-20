//! The game's sound effects: original WAV files made by
//! `scripts/make_sfx.py`, embedded in the program. A resolution plays at
//! most one sound of each kind (one pop for a whole group, one fall for a
//! whole cluster), never one per creature.

use crate::audio::Sfx;

/// `Game::SOUNDS`, in the order of the constants below.
pub const SOUNDS: &[&[u8]] = &[
    include_bytes!("../../assets/sfx/launch.wav"),
    include_bytes!("../../assets/sfx/bounce.wav"),
    include_bytes!("../../assets/sfx/attach.wav"),
    include_bytes!("../../assets/sfx/pop.wav"),
    include_bytes!("../../assets/sfx/fall.wav"),
    include_bytes!("../../assets/sfx/ceiling.wav"),
    include_bytes!("../../assets/sfx/blast.wav"),
    include_bytes!("../../assets/sfx/chain.wav"),
];

pub const LAUNCH: Sfx = Sfx::Game(0);
pub const BOUNCE: Sfx = Sfx::Game(1);
pub const ATTACH: Sfx = Sfx::Game(2);
pub const POP: Sfx = Sfx::Game(3);
pub const FALL: Sfx = Sfx::Game(4);
pub const CEILING: Sfx = Sfx::Game(5);
pub const BLAST: Sfx = Sfx::Game(6);
pub const CHAIN: Sfx = Sfx::Game(7);

#[cfg(test)]
mod tests {
    #[test]
    fn every_sound_decodes() {
        for (i, bytes) in super::SOUNDS.iter().enumerate() {
            let samples =
                crate::audio::decode_wav(bytes).unwrap_or_else(|e| panic!("sound {i}: {e}"));
            assert!(!samples.is_empty());
            assert!(samples.len() < 22_050, "sound {i} is short");
        }
    }
}
