# COMET MADNESS

**A modern neon reboot of *Crazy Comets* (Martech, Commodore 64, 1985) — written in Rust, for Windows 11.**

Your ship faces into the screen and fires straight up. Planets hurtle in from the sides, curving and
bouncing as they grow — shoot them while they're small. It plays like the 1985 original, rebuilt with
rotating shaded 3D planets, bloom and CRT post-processing, beat-synced visuals and an all-new,
hard-hitting EDM / dubstep soundtrack.

\---

## Run it

1. Unzip anywhere.
2. Double-click **`CometMadnesse.exe`**. No installer, no runtime, no DLLs — every asset is embedded in the exe.

Windows SmartScreen may warn about an unsigned app from an unknown publisher: choose **More info → Run anyway**.
Requires a GPU with OpenGL 2.1+ (any Windows 11 PC).

High scores and settings are saved to `%APPDATA%\\CometMadness\\`.

## Controls

|Action|Keys|
|-|-|
|Move (8 directions)|**Arrow keys** or **W A S D**|
|Fire (hold for auto-fire)|**Space**, **Ctrl**, **Z** or **left mouse**|
|Pause|**Esc** / **P**|
|Fullscreen|**F11** or **Alt+Enter**|
|CRT effect on/off|**F2**|
|Music on/off|**M**|

Like the C64 game, the ship never rotates — it always fires straight up, into the screen.

## How to play

* **Line up from below.** You can't aim, so fly underneath whatever you want to hit.
* **Kill them small.** Planets hurtle in from the sides as distant dots and grow. Small ones fly "behind" you and can't hurt you; big ones take many hits.
* **They curve and bounce.** Planets can't steer freely — they change course by bending along curves or bouncing off the edges, and the bigger they get, the harder they curve in toward you.
* **Worlds** (Earth-like) explode once you've hit them enough.
* **Novas** (cratered) start glowing angrily when shot and moments later go wild, homing in fast. Finish them quickly.
* Fully grown planets pick up an orbiting **satellite** that breaks off to attack.
* From round 2, small **comets** streak about and start chasing you if left alone. Little **rockets** drift out with them — fly over one for 200 points (shooting them does nothing).
* Extra life every **10,000**. **Perfect Round** bonus if every planet dies before growing large and you don't lose a life. Quick kills build a small combo multiplier.
* Leave the title screen alone for \~20 seconds to watch the attract-mode **demo**.

## The soundtrack

All music is original, composed and synthesized in code for this project by `tools/soundforge` (a small Rust synthesizer and sequencer — no samples, no third-party audio). Everything is in F minor.

|Track|Style|Where|
|-|-|-|
|**Orbital Drift**|128 BPM melodic electro with a SID-style square lead (a nod to the C64)|Title, menus, high scores|
|**Comet Rush**|128 BPM big-room electro: pumping sidechained supersaws, snare-roll builds, drops|Rounds 1–2, 5–6, 9–10…|
|**Supernova Cascade**|140 BPM halftime dubstep: FM-growl wobble bass, risers, impacts, two drops|Rounds 3–4, 7–8, 11–12…|
|**Last Light**|Game-over sting|Game over|

The arena's tunnel, starfield, bloom and HUD all pulse with the beat of whichever track is playing.

No melody from Rob Hubbard's 1985 score is used — this is a tribute in spirit, not a cover.

## Building from source

The project is a standard Cargo project.

```text
CometMadness/
├─ Cargo.toml            game crate (macroquad 0.4)
├─ build.rs              embeds the .exe icon on Windows
├─ src/
│  ├─ main.rs            window, state machine, menus, screens
│  ├─ game.rs            gameplay: ship, worlds, novas, satellites, comets, rockets
│  ├─ gfx.rs             shaders (planet, bloom, composite/CRT), procedural textures, text
│  ├─ fx.rs              particles, shockwaves, starfield, screen shake, banners
│  ├─ audio.rs           music crossfades, beat clock, SFX
│  ├─ bot.rs             attract-mode autopilot
│  ├─ persist.rs         high scores \& settings
│  └─ util.rs            math / colour helpers
└─ assets/
   ├─ music/\*.ogg        rendered soundtrack (OGG Vorbis)
   ├─ sfx/\*.wav          rendered sound effects
   ├─ fonts/             Orbitron (SIL Open Font License, see OFL.txt)
   └─ icon/              app icon (.ico, .png, raw RGBA for the window icon)
```

### On Windows (simplest)

```powershell
# Install Rust from https://rustup.rs (MSVC toolchain is fine), then:
cargo build --release
# -> target\\release\\CometMadness.exe
```

### Cross-compiling from Linux (how the shipped exe was built)

```bash
rustup target add x86\_64-pc-windows-gnu
sudo apt install gcc-mingw-w64-x86-64
cargo build --release --target x86\_64-pc-windows-gnu
# -> target/x86\_64-pc-windows-gnu/release/CometMadness.exe
```

`.cargo/config.toml` already points the linker at `x86\_64-w64-mingw32-gcc`.
(The shipped binary was built with Rust 1.91 + MinGW-w64 GCC 13, using `-Zbuild-std` because the build machine had no prebuilt Windows standard library; with rustup you don't need that.)

### Developer flags

`--windowed` (ignore saved fullscreen), `--demo` (start in attract mode), `--play --level N` (jump straight into round N),
`--fixed-dt`, `--fps`, `--shot <seconds> <file.png>`, `--shots-every <seconds> <prefix>`, `--quit-after <seconds>`.

## Credits \& licences

* **Original *Crazy Comets* (1985):** design \& code by Simon Nicol, music \& sound by Rob Hubbard, published by Martech — itself inspired by Gottlieb's *Mad Planets* (1983, designed by Kan Yabumoto).
The default high-score table is a small roll-call of them.
* **COMET MADNESS:** all code, graphics (procedural) and music (synthesized) are new and original to this project.
* **Font:** Orbitron by The League of Moveable Type / The Orbitron Project Authors, SIL Open Font License 1.1 (`assets/fonts/OFL.txt`).
* **Engine:** [macroquad](https://github.com/not-fl3/macroquad) (MIT/Apache-2.0).

*COMET MADNESS is an unofficial, non-commercial fan tribute. It is not affiliated with or endorsed by the rights holders of Crazy Comets or Mad Planets.*



