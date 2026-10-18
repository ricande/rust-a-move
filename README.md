# GRIN (working title)

GRIN is an original, Snood-inspired portrait aiming-and-matching game with
grumpy geometric creatures. Shoot a creature into the hanging board; three or
more of one family disappear and unsupported creatures fall. There is **no
aiming line**: the creature you are about to hit shows its fear instead. The
Angry creature is always furious and gives nothing away, its neighbours are
nervous, and Runners try to escape sideways when a path opens.

The name GRIN is provisional. This is a local prototype; nothing has been
published.

**Status:** M1 – procedural character animation lab. The playable game
(M2–M7) is not built yet; see
[docs/game/06_IMPLEMENTATION_PLAN.md](docs/game/06_IMPLEMENTATION_PLAN.md).

GRIN is built on the [RustyHusky](https://github.com/ricande/RustyHusky)
template 0.3.0 (commit `91c257d8e60ca381768dde0258e04d745a30d184`): portrait
540 × 960 view, menus, settings, pause, English/Swedish texts and automatic
1080 × 1920 60 fps gameplay recording via FFmpeg. Changes to the framework
files are listed in [docs/FRAMEWORK_PATCHES.md](docs/FRAMEWORK_PATCHES.md).

## The animation lab (M1)

Starting a round opens the character lab. Keys:

| Key | Action |
| --- | --- |
| Z / X | previous / next scenario |
| Up / Down | change the scenario's parameter (size, aim, route …) |
| Left / Right | change playback speed (slow / normal) |
| Space | trigger the scenario's event (aim on/off, open route, blast …) |
| D | toggle the collider overlay |
| A | toggle reduced motion |
| Enter | finish the round (results and video) |
| Esc / P | pause (framework) |

The scenarios follow the animation bible in
[docs/game/04_CHARACTERS_ANIMATION.md](docs/game/04_CHARACTERS_ANIMATION.md).

## Template features kept

- **Portrait 9:16 gameplay** – a logical 540 × 960 view, scaled to any window
  without cropping or stretching, with correct mouse coordinates.
- **Automatic gameplay recording with audio** – every round is recorded
  while you play, including the game's sound effects.
- **1080 × 1920 MP4 video at 60 fps** – H.264 video with AAC stereo sound,
  independent of the window size.
- **Save or discard the recording after a level** – nothing is written to
  your video folder until you choose Save video.
- **A reusable foundation for quickly building new games** – menus,
  settings, pause, results, scaling and recording are done; you replace one
  game module.
- **English by default, with i18n support** – all texts go through one
  translation catalog; Swedish is included.

Breakout-style game; its general parts were kept and the Breakout parts
removed. The framework draws everything in code and synthesizes its sounds;
a game's images and sounds in `assets/` are built into the program, so it
needs no files at run time.


## Requirements

- Rust (stable, 1.87 or later) via [rustup](https://rustup.rs)
- Linux: development packages for X11, ALSA and OpenGL, e.g. on Debian/Ubuntu:
  `sudo apt install libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev`
- Windows and macOS need nothing extra to build.
- To record videos: [FFmpeg](https://ffmpeg.org) with the libx264 encoder, in
  `PATH` (Debian/Ubuntu: `sudo apt install ffmpeg`; macOS: `brew install
  ffmpeg`; Windows: an FFmpeg build such as the one from gyan.dev, with its
  `bin` folder in `PATH`). It is only needed at run time, not to build.
  Without it everything works except recording, and the settings and results
  screens say why.

## Running

```bash
cargo run --release
```

Tests (flow, settings, translations, scaling, recording pipeline) run without
a window; the video tests are skipped when FFmpeg is missing:

```bash
cargo test
```

## Playing

| Screen   | Keys / mouse |
| -------- | ------------ |
| Menus    | Mouse, or ↑/↓ and Enter (Space also chooses). Esc goes back. |
| Settings | ←/→ change the focused setting; click the left or right half of the volume button to lower or raise it. |
| Round    | Esc or P pauses. In the pause menu: Resume (also Esc/P) or Abort round. |
| Lab      | See [The animation lab](#the-animation-lab-m1). |

Flow: **main menu → round → results → next round or main menu**. An aborted
round also ends on the results screen (without "Next round").

## Settings

Saved as `settings.json` in the data folder and loaded at the next start:

| Setting   | Values                    | Default |
| --------- | ------------------------- | ------- |
| Language  | English, Svenska          | English |
| Volume    | 0–100 % in steps of 10 %  | 80 %    |
| Recording mode | Prioritize recording, Prioritize gameplay, Off | Prioritize recording |

A change of recording mode applies from the next round. Files from earlier
versions with `"recording": true` / `false` are read as Prioritize recording /
Off; a saved `recording_mode` is always kept.

Data folder: `$XDG_DATA_HOME/rustyhusky` (normally
`~/.local/share/rustyhusky`) on Linux, `~/Library/Application
Support/RustyHusky` on macOS, `%APPDATA%\RustyHusky` on Windows. A missing or
broken file gives the defaults.

## Video recording

Unless the recording mode is Off and as long as FFmpeg is available, every
round is recorded. The two modes differ in what gives way when the computer
cannot keep up:

| | Prioritize gameplay | Prioritize recording (default) |
| --- | --- | --- |
| Game speed | Real time, always | Follows the video: each video frame is exactly two physics steps (2 × 1/120 s). If frames cannot be produced 60 times per second, the game runs slower. |
| Video | 60 fps on the game's clock. Pictures are repeated or skipped to match the display's frame rate, so motion is only as smooth as the display rate. | Exactly one new picture per video frame, so motion is always smooth and at normal speed. |
| Encoder too slow | The recording stops with a message; the round goes on. | The game waits for it ("Waiting for the video encoder…" after 1 s). The window, pause and abort keep working. |
| Encoder hung | Stopped once its queue (about 0.4 s) is full. | Stopped after 5 s without progress; the round goes on in real time. |

Example: if the computer only manages 30 video frames per second in
Prioritize recording, a minute of play takes about two minutes, and the saved
file is one minute long and plays smoothly at 60 fps.

Both modes:

- **What:** only the game view, rendered separately at 1080 × 1920 (9:16),
  60 fps, H.264 – independent of the window size, without window borders,
  letterbox bars, the REC marker, the pause menu or the results screen.
- **When:** the first video frame (time 0) shows the round before any
  physics step; the last one shows the frame the level was finished (or the
  last frame before an abort). The video is exactly as long as its frames:
  number of frames / 60 s. Paused time adds nothing to game time, video or
  sound.
- **Length limit:** a video is at most 3:00 (`MAX_VIDEO_SECONDS` in
  `src/config.rs`; `None` removes the limit), the longest a YouTube Short may
  be. If a round lasts longer, recording stops at exactly 3:00 and the game
  goes on; the window shows "Video full (3:00)" and the results explain it.
  A game decides its own level length; to fit a whole level in a Short, keep
  it a little under 3:00.
- **Afterwards:** the results screen shows "Finishing the video…" while the
  encoder finishes, then **Save video** and **Discard video**. Until you save,
  the video is only a temporary file in the system's temp folder. Leaving the
  results screen without saving discards it.
- **Where:** `~/Videos/RustyHusky/rustyhusky-<date>-<time>.mp4` (UTC; the
  Linux folder follows `XDG_VIDEOS_DIR`, macOS uses `~/Movies`, Windows
  `%USERPROFILE%\Videos`). Existing files are never overwritten; `-2`, `-3` …
  is added instead. The results screen shows the saved path.
- **REC marker:** while recording, the window shows "● REC 0:42 / 3:00",
  the length of the video so far and its limit, in yellow for the last ten
  seconds. It is never part of the video.
- **Sound:** the video has a stereo AAC sound track with the game's sound
  effects, placed by game time (in Prioritize recording, the time of the
  video frame in which they happened), so it stays in sync however long the
  recording took. It is
  always mixed at full effect volume, whatever the volume setting. Sounds
  from outside the game (other programs, the microphone) are not recorded,
  and menu sounds are never part of the video.
- **Errors:** a missing FFmpeg, an encoder that crashes, hangs or is too
  slow, or a file that cannot be written is shown as a message on the
  results screen; the game keeps working. A failed save can be retried.
- **Never stuck:** frames go through a bounded queue of 24 pictures (about
  0.4 s, at most about 200 MB). Frames are never dropped, so a saved video
  is always complete and in sync, or there is none. When a recording fails,
  the window shows "Recording stopped", the results show why, FFmpeg is
  stopped and the round continues in real time. The FFmpeg check at startup
  is limited to 5 s, finishing and saving a video to 60 s.
- **Clean-up:** temporary files are removed on Discard, on leaving the
  results, and when the program closes (also via the window's close button,
  which stops any running FFmpeg at once, including a save in progress).
  Leftovers after a crash are removed at the next start once they are a day
  old.

## Template updates

GRIN changes some RustyHusky framework files. Do not run
`scripts/update-framework.sh` blindly: it overwrites them. Follow
[docs/FRAMEWORK_PATCHES.md](docs/FRAMEWORK_PATCHES.md) and the template's
[docs/NEW_GAME.md](docs/NEW_GAME.md).

## Environment variables

Handy for testing without touching your real settings and videos. The
prefix is `config::ENV_PREFIX` (`GRIN` here):

| Variable               | Effect                                            |
| ---------------------- | ------------------------------------------------- |
| `GRIN_DATA_DIR`        | folder for `settings.json`                        |
| `GRIN_VIDEO_DIR`       | folder for saved videos                           |
| `GRIN_LANG`            | `en` or `sv`, overrides the saved language for this run |
| `GRIN_FFMPEG`          | path to the ffmpeg program (a wrong path simulates missing FFmpeg) |
| `GRIN_WINDOW`          | force a window size, e.g. `1000x640`              |
| `GRIN_SCRIPT`          | run a UI script instead of real input (see [AGENTS.md](AGENTS.md)) |
| `GRIN_SHOTS`           | folder for the script's screenshots (default `shots`) |

## Documentation

| Document | Contents |
| -------- | -------- |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Modules, coordinates, time step, i18n, recording flow |
| [docs/GAME_RULES.md](docs/GAME_RULES.md) | GRIN's authoritative game rules |
| [docs/game/](docs/game/) | GRIN design package: decisions, technical design, characters, configuration, plan, tests, template audit ([Swedish overview](docs/game/README_SV.md)) |
| [docs/FRAMEWORK_PATCHES.md](docs/FRAMEWORK_PATCHES.md) | Changes to RustyHusky framework files, to reapply after a template update |
| [docs/NEW_GAME.md](docs/NEW_GAME.md) | The template's guide to building a game on it |
| [docs/UPSTREAM_FIXES.md](docs/UPSTREAM_FIXES.md) | The patched miniquad: symptom, cause, verification, how to remove it |
| [AGENTS.md](AGENTS.md) | Rules and checks for contributors and AI agents |
| [CHANGELOG.md](CHANGELOG.md) | Changes per version |
| [LICENSE](LICENSE) | GPL-3.0-or-later |
| [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) | RustOut (MIT), dependencies, FFmpeg, recorded videos |

## Known limitations

- Recording needs FFmpeg with libx264 at run time.
- Recording reads each frame back from the GPU (8 MB per frame), which costs
  some frame time while recording. The video is still exactly 60 fps
  (pictures are repeated or skipped to match game time).
- In Prioritize gameplay, recording needs a CPU that encodes 1080 × 1920
  H.264 (`veryfast`) at 60 fps in real time; on a slower or very busy machine
  it is stopped with a message rather than slowing the game. Prioritize
  recording works on slower machines, at the cost of a slower game while
  recording. The frame queue can use up to about 200 MB while the encoder is
  behind.
- In Prioritize gameplay, motion in the video is only as smooth as the
  display's frame rate: at 30 Hz every picture is shown twice. The file is
  60 fps either way.
- RustyHusky builds against a patched copy of miniquad 0.4.11 in `vendor/`
  that fixes black text rectangles (macroquad#1006); see
  [docs/UPSTREAM_FIXES.md](docs/UPSTREAM_FIXES.md). Building it prints one
  or two warnings (depending on the platform) from miniquad's own, unchanged
  code.
- The video sound contains only the game's synthesized effects.
- The start window size is fitted to the screen on X11/XWayland (via `xprop`
  and `xrandr`); elsewhere it starts at 360 × 640. It can always be resized.
- The built-in font covers Latin text (English and Swedish), not every script.

## License

GRIN, like RustyHusky, is free software, licensed under the GNU General Public License,
version 3 or (at your option) any later version (GPL-3.0-or-later). The full
text is in [LICENSE](LICENSE).

- Code reused from [RustOut](https://github.com/ricande/RustOut) keeps its
  original MIT copyright and permission notice, in
  [LICENSES/MIT-RustOut.txt](LICENSES/MIT-RustOut.txt).
- The Rust dependencies and FFmpeg keep their own licenses. See
  [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- Gameplay videos you record are not automatically covered by the code
  license. Separate rights to any assets you use (graphics, music, sounds,
  fonts) still apply.
