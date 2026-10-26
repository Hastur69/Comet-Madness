//! soundforge — renders every audio asset used by Comet Craze.
//!
//! Usage:  cargo run --release -- <output_dir>
//! Writes WAV files; `build_assets.sh` then encodes the music to OGG Vorbis.

mod dsp;
mod instruments;
mod sfx;
mod songs;

use dsp::{write_wav, Buf};

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "out".to_string());
    let music = format!("{out}/music");
    let fx = format!("{out}/sfx");
    std::fs::create_dir_all(&music).unwrap();
    std::fs::create_dir_all(&fx).unwrap();

    let only = std::env::args().nth(2);
    let want = |name: &str| only.as_deref().map(|o| o == name || o == "all").unwrap_or(true);

    let songs: Vec<(&str, fn() -> Buf)> = vec![
        ("title_orbital_drift", songs::title),
        ("stage_comet_rush", songs::comet_rush),
        ("stage_supernova_cascade", songs::supernova_cascade),
        ("game_over", songs::game_over),
    ];
    for (name, f) in songs {
        if !want(name) {
            continue;
        }
        let t = std::time::Instant::now();
        let b = f();
        write_wav(&format!("{music}/{name}.wav"), &b, true).unwrap();
        println!("music {name:28} {:6.1}s audio  rendered in {:.1}s", b.len() as f32 / dsp::SR, t.elapsed().as_secs_f32());
    }

    let effects: Vec<(&str, Box<dyn Fn() -> Buf>)> = vec![
        ("shoot", Box::new(sfx::shoot)),
        ("hit", Box::new(sfx::hit)),
        ("explode_small", Box::new(|| sfx::explode(0.15, 11))),
        ("explode_medium", Box::new(|| sfx::explode(0.5, 12))),
        ("explode_big", Box::new(|| sfx::explode(1.0, 13))),
        ("crazy", Box::new(sfx::crazy_alarm)),
        ("nova", Box::new(sfx::nova)),
        ("pickup", Box::new(sfx::pickup)),
        ("extra_life", Box::new(sfx::extra_life)),
        ("level_clear", Box::new(sfx::level_clear)),
        ("player_die", Box::new(sfx::player_die)),
        ("bomb", Box::new(sfx::bomb)),
        ("ui_select", Box::new(sfx::ui_select)),
        ("satellite_launch", Box::new(sfx::satellite_launch)),
        ("capsule_spawn", Box::new(sfx::capsule_spawn)),
        ("warp", Box::new(sfx::warp)),
        ("warp_fire", Box::new(sfx::warp_fire)),
        ("warp_reverse", Box::new(sfx::warp_reverse)),
    ];
    for (name, f) in effects {
        if !want(name) && only.as_deref() != Some("sfx") {
            continue;
        }
        let b = f();
        write_wav(&format!("{fx}/{name}.wav"), &b, true).unwrap();
        println!("sfx   {name:28} {:6.2}s", b.len() as f32 / dsp::SR);
    }
}
