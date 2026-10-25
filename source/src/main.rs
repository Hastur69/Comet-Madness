//! COMET CRAZE — a modern neon reboot of Crazy Comets (Martech, 1985, C64).
//!
//! Original 1985 game: programmed by Simon Nicol, music by Rob Hubbard,
//! itself inspired by Gottlieb's 1983 arcade game Mad Planets.
//! This reboot is an independent fan tribute; all code, art and music are new.

#![cfg_attr(not(debug_console), windows_subsystem = "windows")]

mod audio;
mod bot;
mod fx;
mod game;
mod gfx;
mod persist;
mod util;

use audio::{Audio, Sfx, Track};
use fx::Fx;
use game::{Input, Phase, World};
use gfx::Gfx;
use macroquad::prelude::*;
use persist::{Entry, Settings};
use util::*;

/// Progress tracing for diagnostics (only compiled into console debug builds).
fn trace(_msg: &str) {
    #[cfg(debug_console)]
    eprintln!("[trace] {_msg}");
}

fn window_conf() -> Conf {
    // Write any crash to %APPDATA%\cometmadness\crash.log (a GUI app has no console).
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("COMET MADNESS crashed: {info}\n");
        persist::write_crash_log(&msg);
    }));
    let icon = macroquad::miniquad::conf::Icon {
        small: *include_bytes!("../assets/icon/icon_16.rgba"),
        medium: *include_bytes!("../assets/icon/icon_32.rgba"),
        big: *include_bytes!("../assets/icon/icon_64.rgba"),
    };
    let fullscreen = persist::load_settings().fullscreen && !std::env::args().any(|a| a == "--windowed");
    Conf {
        window_title: "COMET MADNESS".to_owned(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        fullscreen,
        high_dpi: false,
        sample_count: 4,
        icon: Some(icon),
        ..Default::default()
    }
}

enum State {
    Title { sel: usize, idle: f32 },
    Options { sel: usize },
    HowTo,
    Scores { highlight: Option<usize>, t: f32 },
    Play { world: World, paused: bool, pause_sel: usize, demo: bool, demo_t: f32 },
    GameOver { world: World, t: f32 },
    NameEntry { score: u64, level: u32, letters: [u8; 3], pos: usize, t: f32 },
}

struct Args {
    demo: bool,
    shot: Option<(f32, String)>,
    shots_every: Option<(f32, String)>,
    level: u32,
    quit_after: Option<f32>,
    start_play: bool,
    fps: bool,
    showcase: bool,
    fixed_dt: bool,
}

fn parse_args() -> Args {
    let v: Vec<String> = std::env::args().collect();
    let mut a = Args { demo: false, shot: None, shots_every: None, level: 1, quit_after: None, start_play: false, fps: false, fixed_dt: false, showcase: false };
    let mut i = 1;
    while i < v.len() {
        match v[i].as_str() {
            "--demo" => a.demo = true,
            "--play" => a.start_play = true,
            "--fps" => a.fps = true,
            "--showcase" => a.showcase = true,
            "--fixed-dt" => a.fixed_dt = true,
            "--level" if i + 1 < v.len() => {
                a.level = v[i + 1].parse().unwrap_or(1);
                i += 1;
            }
            "--shot" if i + 2 < v.len() => {
                a.shot = Some((v[i + 1].parse().unwrap_or(3.0), v[i + 2].clone()));
                i += 2;
            }
            "--shots-every" if i + 2 < v.len() => {
                a.shots_every = Some((v[i + 1].parse().unwrap_or(3.0), v[i + 2].clone()));
                i += 2;
            }
            "--quit-after" if i + 1 < v.len() => {
                a.quit_after = v[i + 1].parse().ok();
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    a
}

struct App {
    g: Gfx,
    audio: Audio,
    fx: Fx,
    settings: Settings,
    scores: Vec<Entry>,
    now_playing: (String, f32),
    demo_world_hint: f32,
}

fn mouse_virtual() -> Vec2 {
    let (mx, my) = mouse_position();
    let sw = screen_width();
    let sh = screen_height();
    let s = (sw / VW).min(sh / VH);
    let ox = (sw - VW * s) * 0.5;
    let oy = (sh - VH * s) * 0.5;
    vec2((mx - ox) / s, (my - oy) / s)
}

fn any_key_or_click() -> bool {
    get_last_key_pressed().is_some() || is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right)
}

fn menu_nav(sel: &mut usize, n: usize) -> bool {
    let mut moved = false;
    if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
        *sel = (*sel + 1) % n;
        moved = true;
    }
    if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
        *sel = (*sel + n - 1) % n;
        moved = true;
    }
    moved
}

fn confirm() -> bool {
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::KpEnter)
}

impl App {
    /// Classic joystick controls: move in 8 directions, hold fire for auto-fire.
    /// The ship always faces into the screen, so there is no aiming.
    fn play_input(&mut self) -> Input {
        let mut mv = Vec2::ZERO;
        if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
            mv.x -= 1.0;
        }
        if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
            mv.x += 1.0;
        }
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            mv.y -= 1.0;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            mv.y += 1.0;
        }
        let fire = is_key_down(KeyCode::Space)
            || is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl)
            || is_key_down(KeyCode::Z)
            || is_key_down(KeyCode::J)
            || is_mouse_button_down(MouseButton::Left);
        Input { mv, fire }
    }

    fn set_music(&mut self, t: Track) {
        if self.audio.current != Some(t) {
            self.audio.music(t);
            if t != Track::GameOver {
                self.now_playing = (format!("NOW PLAYING  -  {}", t.name()), 4.0);
            }
        }
    }

    fn draw_now_playing(&self) {
        let (s, t) = &self.now_playing;
        if *t > 0.0 {
            let a = t.min(1.0) * 0.8;
            self.g.text_right(s, VW - 20.0, VH - 18.0, 12.0, Color::new(0.7, 0.9, 1.0, a), false);
        }
    }
}

const TITLE_MENU: [&str; 5] = ["PLAY", "OPTIONS", "HIGH SCORES", "HOW TO PLAY", "QUIT"];
const PAUSE_MENU: [&str; 3] = ["RESUME", "RESTART ROUND 1", "QUIT TO TITLE"];

#[macroquad::main(window_conf)]
async fn main() {
    let args = parse_args();
    trace("main start");
    macroquad::rand::srand((get_time() * 1e6) as u64 ^ 0x5EED_C0DE);

    // ---- loading screen
    trace("creating gfx");
    let g = Gfx::new();
    trace("gfx ready");
    let mut bass = vec![];
    let mut music = vec![];
    let total = audio::MUSIC_FILES.len() + 1;
    for step in 0..total {
        set_camera(&Gfx::rt_camera(&g.scene, Vec2::ZERO));
        clear_background(Color::new(0.01, 0.0, 0.03, 1.0));
        g.neon_text_center("COMET MADNESS", VW * 0.5, VH * 0.45, 64.0, Color::new(1.0, 0.4, 0.85, 1.0), true);
        g.text_center("CHARGING THE SUPERNOVA...", VW * 0.5, VH * 0.56, 16.0, Color::new(0.7, 0.9, 1.0, 0.8), false);
        let w = 420.0;
        draw_rectangle_lines(VW * 0.5 - w * 0.5, VH * 0.62, w, 10.0, 2.0, Color::new(0.4, 0.9, 1.0, 0.8));
        draw_rectangle(VW * 0.5 - w * 0.5, VH * 0.62, w * step as f32 / total as f32, 10.0, Color::new(0.4, 0.9, 1.0, 1.0));
        g.present(false, 0.0, 0.0, 0.0, BLACK);
        next_frame().await;
        if step < audio::MUSIC_FILES.len() {
            let (t, data) = audio::MUSIC_FILES[step];
            music.push((t, macroquad::audio::load_sound_from_bytes(data).await.expect("music decode")));
            bass.push((t, audio::analyze_bass(data)));
        }
    }
    trace("music loaded");
    let sfx = Audio::load_sfx().await;
    trace("sfx loaded");
    let settings = persist::load_settings();
    let audio = Audio::new(sfx, music, bass, settings.music_volume, settings.sfx_volume);
    trace("audio ready");

    let mut app = App {
        g,
        audio,
        fx: Fx::new(),
        settings,
        scores: persist::load_scores(),
        now_playing: (String::new(), 0.0),
        demo_world_hint: 0.0,
    };

    let mut state = if args.demo || args.start_play || args.showcase {
        let mut w = World::new(args.level);
        if args.showcase {
            w.showcase(&mut app.fx);
        } else {
            w.first_banner(&mut app.fx);
        }
        app.set_music(game::track_for_level(args.level));
        State::Play { world: w, paused: false, pause_sel: 0, demo: args.demo, demo_t: 0.0 }
    } else {
        app.set_music(Track::Title);
        State::Title { sel: 0, idle: 0.0 }
    };
    trace("state ready");
    let start = get_time();
    let mut shot_done = false;
    let mut shot_n = 0u32;
    let mut next_shot = args.shots_every.as_ref().map(|(t, _)| *t).unwrap_or(0.0);

    let mut frames = 0u64;
    // 0 = rings pulse to the beat, 1 = rings act as a bass visualizer (blended smoothly)
    let mut ring_bass_mix = 0.0f32;
    // galaxy progression: warp between sectors every 4 rounds, plus a 10-round
    // nebula cycle (1-6 nebula, 7-9 fading into the void, 10 empty void)
    let mut last_level = 0u32;
    let mut warp_sfx_level = 0u32;
    loop {
        frames += 1;
        let dt = if args.fixed_dt { 1.0 / 60.0 } else { get_frame_time().min(1.0 / 30.0) };
        let elapsed = if args.fixed_dt { frames as f32 / 60.0 } else { (get_time() - start) as f32 };
        app.audio.update(dt);
        app.now_playing.1 -= dt;
        let (beat_phase, _) = app.audio.beat();
        let pulse = (1.0 - beat_phase).powi(3);

        // ---- global hotkeys
        if is_key_pressed(KeyCode::F11) || (is_key_pressed(KeyCode::Enter) && (is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt))) {
            app.settings.fullscreen = !app.settings.fullscreen;
            set_fullscreen(app.settings.fullscreen);
            persist::save_settings(&app.settings);
        }
        if is_key_pressed(KeyCode::F2) {
            app.settings.crt = !app.settings.crt;
            persist::save_settings(&app.settings);
        }
        if is_key_pressed(KeyCode::M) {
            app.audio.music_volume = if app.audio.music_volume > 0.0 { 0.0 } else { 0.7 };
            app.settings.music_volume = app.audio.music_volume;
            persist::save_settings(&app.settings);
        }

        // ---- galaxy progression visuals
        let cur_world = match &state {
            State::Play { world, .. } | State::GameOver { world, .. } => Some(world),
            _ => None,
        };
        let (neb_target, star_target, warp_target) = match cur_world {
            Some(w) => {
                let cyc = (w.level.max(1) - 1) % 10 + 1;
                let neb = match cyc {
                    1..=6 => 1.0,
                    7 => 0.6,
                    8 => 0.3,
                    9 => 0.08,
                    _ => 0.0,
                };
                let stars = if cyc == 10 { 0.0 } else { 1.0 };
                // warp builds while the "ROUND CLEAR" banner shows after rounds 4, 8, 12...
                let warp = if w.phase == Phase::Clear && w.level % 4 == 0 && !w.game_over {
                    let p = 1.0 - w.phase_t / 3.2;
                    ((p - 0.2) / 0.6).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                (neb, stars, warp)
            }
            None => (1.0, 1.0, 0.0),
        };
        // flash as we drop out of hyperspace into the new sector
        if let Some(w) = cur_world {
            if w.level != last_level && last_level > 0 && last_level % 4 == 0 && w.level == last_level + 1 {
                app.fx.flash(0.9, Color::new(0.85, 0.9, 1.0, 1.0));
            }
            last_level = w.level;
        } else {
            last_level = 0;
        }
        // warp sound: starts 0.75 s into the round-clear gap so its boom lands
        // right as we drop out of hyperspace into the next round
        if let Some(w) = cur_world {
            if w.phase == Phase::Clear && w.level % 4 == 0 && !w.game_over && w.phase_t <= 2.45 && warp_sfx_level != w.level {
                warp_sfx_level = w.level;
                app.audio.play(Sfx::Warp);
            }
        }
        app.fx.nebula_amt = approach(app.fx.nebula_amt, neb_target, 0.8, dt);
        app.fx.star_amt = approach(app.fx.star_amt, star_target, 1.2, dt);
        let rate = if warp_target > app.fx.warp { 6.0 } else { 2.5 };
        app.fx.warp = approach(app.fx.warp, warp_target, rate, dt);

        // ---- update
        let mut next: Option<State> = None;
        let mut quit = false;
        let mut star_speed = 0.3;
        let mut bg_intensity = 1.0;
        match &mut state {
            State::Title { sel, idle } => {
                app.audio.duck = 1.0;
                *idle += dt;
                if any_key_or_click() || mouse_delta_position().length() > 0.001 {
                    *idle = 0.0;
                }
                if menu_nav(sel, TITLE_MENU.len()) {
                    app.audio.play(Sfx::UiSelect);
                }
                // mouse hover selection
                let m = mouse_virtual();
                for (i, _) in TITLE_MENU.iter().enumerate() {
                    let y = 430.0 + i as f32 * 46.0;
                    if (m.y - (y - 10.0)).abs() < 20.0 && (m.x - VW * 0.5).abs() < 160.0 && mouse_delta_position().length() > 0.0 {
                        *sel = i;
                    }
                }
                let clicked = is_mouse_button_pressed(MouseButton::Left);
                if confirm() || clicked {
                    app.audio.play(Sfx::Pickup);
                    match *sel {
                        0 => {
                            let mut w = World::new(1);
                            app.fx.clear();
                            w.first_banner(&mut app.fx);
                            app.set_music(game::track_for_level(1));
                            next = Some(State::Play { world: w, paused: false, pause_sel: 0, demo: false, demo_t: 0.0 });
                        }
                        1 => next = Some(State::Options { sel: 0 }),
                        2 => next = Some(State::Scores { highlight: None, t: 0.0 }),
                        3 => next = Some(State::HowTo),
                        _ => quit = true,
                    }
                }
                if is_key_pressed(KeyCode::Escape) {
                    quit = true;
                }
                if *idle > 22.0 {
                    let mut w = World::new(rndi(1, 4) as u32);
                    app.fx.clear();
                    w.first_banner(&mut app.fx);
                    app.set_music(game::track_for_level(w.level));
                    next = Some(State::Play { world: w, paused: false, pause_sel: 0, demo: true, demo_t: 0.0 });
                }
            }
            State::Options { sel } => {
                let n = 6;
                if menu_nav(sel, n) {
                    app.audio.play(Sfx::UiSelect);
                }
                let left = is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A);
                let right = is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D);
                let toggle = left || right || confirm();
                let d = if left { -0.1 } else if right { 0.1 } else { 0.0 };
                match *sel {
                    0 if d != 0.0 => {
                        app.settings.music_volume = (app.settings.music_volume + d).clamp(0.0, 1.0);
                        app.audio.music_volume = app.settings.music_volume;
                    }
                    1 if d != 0.0 => {
                        app.settings.sfx_volume = (app.settings.sfx_volume + d).clamp(0.0, 1.0);
                        app.audio.sfx_volume = app.settings.sfx_volume;
                        app.audio.play(Sfx::Shoot);
                    }
                    2 if toggle => app.settings.crt = !app.settings.crt,
                    3 if toggle => app.settings.screen_shake = !app.settings.screen_shake,
                    4 if toggle => {
                        app.settings.fullscreen = !app.settings.fullscreen;
                        set_fullscreen(app.settings.fullscreen);
                    }
                    5 if confirm() => {
                        persist::save_settings(&app.settings);
                        next = Some(State::Title { sel: 1, idle: 0.0 });
                    }
                    _ => {}
                }
                if toggle && *sel >= 2 && *sel <= 4 {
                    app.audio.play(Sfx::UiSelect);
                }
                if is_key_pressed(KeyCode::Escape) {
                    persist::save_settings(&app.settings);
                    next = Some(State::Title { sel: 1, idle: 0.0 });
                }
            }
            State::HowTo => {
                if is_key_pressed(KeyCode::Escape) || confirm() || is_mouse_button_pressed(MouseButton::Left) {
                    next = Some(State::Title { sel: 3, idle: 0.0 });
                }
            }
            State::Scores { t, .. } => {
                *t += dt;
                if *t > 0.4 && (is_key_pressed(KeyCode::Escape) || confirm() || is_mouse_button_pressed(MouseButton::Left)) {
                    app.set_music(Track::Title);
                    next = Some(State::Title { sel: 0, idle: 0.0 });
                }
                if *t > 30.0 {
                    app.set_music(Track::Title);
                    next = Some(State::Title { sel: 0, idle: 0.0 });
                }
            }
            State::Play { world, paused, pause_sel, demo, demo_t } => {
                star_speed = if world.phase == Phase::Clear { 1.6 } else { 0.22 + pulse * 0.12 };
                star_speed = lerp(star_speed, 7.0, app.fx.warp);
                if *demo {
                    *demo_t += dt;
                    let inp = bot::think(world);
                    world.update(dt, &inp, &mut app.fx);
                    let quit_demo = (elapsed > 1.0 && any_key_or_click() && !args.demo) || *demo_t > 60.0 || world.game_over_t > 3.0;
                    if quit_demo && !args.demo {
                        app.fx.clear();
                        app.set_music(Track::Title);
                        next = Some(State::Title { sel: 0, idle: 0.0 });
                    }
                    if args.demo && world.game_over_t > 3.0 {
                        let mut w = World::new(1);
                        w.first_banner(&mut app.fx);
                        *world = w;
                    }
                } else {
                    if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::P) {
                        *paused = !*paused;
                        *pause_sel = 0;
                        app.audio.play(Sfx::UiSelect);
                    }
                    if *paused {
                        app.audio.duck = 0.35;
                        bg_intensity = 0.6;
                        star_speed = 0.05;
                        if menu_nav(pause_sel, PAUSE_MENU.len()) {
                            app.audio.play(Sfx::UiSelect);
                        }
                        if confirm() {
                            match *pause_sel {
                                0 => *paused = false,
                                1 => {
                                    let mut w = World::new(1);
                                    app.fx.clear();
                                    w.first_banner(&mut app.fx);
                                    app.set_music(game::track_for_level(1));
                                    *world = w;
                                    *paused = false;
                                }
                                _ => {
                                    app.fx.clear();
                                    app.set_music(Track::Title);
                                    next = Some(State::Title { sel: 0, idle: 0.0 });
                                }
                            }
                        }
                    } else {
                        app.audio.duck = 1.0;
                        let inp = app.play_input();
                        world.update(dt, &inp, &mut app.fx);
                    }
                }
                // world-driven audio
                for (s, v) in world.sounds.drain(..) {
                    app.audio.play_vol(s, v);
                }
                if world.phase == Phase::Intro && !world.game_over {
                    let t = game::track_for_level(world.level);
                    if app.audio.current != Some(t) {
                        app.set_music(t);
                    }
                }
                if !*demo && world.game_over && world.game_over_t > 2.2 {
                    app.set_music(Track::GameOver);
                    let w = std::mem::replace(world, World::new(1));
                    next = Some(State::GameOver { world: w, t: 0.0 });
                }
                if !*paused {
                    app.fx.update(dt * world.time_scale, star_speed);
                }
            }
            State::GameOver { world, t } => {
                *t += dt;
                star_speed = 0.1;
                bg_intensity = 0.7;
                if *t > 1.5 && (any_key_or_click() || *t > 8.0) {
                    if persist::qualifies(&app.scores, world.score) {
                        next = Some(State::NameEntry { score: world.score, level: world.level, letters: *b"AAA", pos: 0, t: 0.0 });
                    } else {
                        //app.set_music(Track::Title);
                        next = Some(State::Scores { highlight: None, t: 0.0 });
                    }
                }
            }
            State::NameEntry { score, level, letters, pos, t } => {
                *t += dt;
                //if app.audio.current != Some(Track::Title) && *t > 0.2 {
                //    app.set_music(Track::Title);
                //}
                // type letters directly
                while let Some(ch) = get_char_pressed() {
                    let c = ch.to_ascii_uppercase();
                    if (c.is_ascii_alphanumeric() || c == ' ') && *pos < 3 && *t > 0.3 {
                        letters[*pos] = c as u8;
                        *pos += 1;
                        app.audio.play(Sfx::UiSelect);
                    }
                }
                let p = (*pos).min(2);
                if is_key_pressed(KeyCode::Up) {
                    letters[p] = if letters[p] >= b'Z' || letters[p] < b'A' { b'A' } else { letters[p] + 1 };
                    app.audio.play(Sfx::UiSelect);
                }
                if is_key_pressed(KeyCode::Down) {
                    letters[p] = if letters[p] <= b'A' || letters[p] > b'Z' { b'Z' } else { letters[p] - 1 };
                    app.audio.play(Sfx::UiSelect);
                }
                if is_key_pressed(KeyCode::Right) && *pos < 2 {
                    *pos += 1;
                }
                if (is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Backspace)) && *pos > 0 {
                    *pos -= 1;
                }
                if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                    let name = String::from_utf8_lossy(letters).to_string();
                    let idx = persist::insert_score(&mut app.scores, Entry { name, score: *score, level: *level });
                    persist::save_scores(&app.scores);
                    app.audio.play(Sfx::ExtraLife);
                    next = Some(State::Scores { highlight: Some(idx), t: 0.0 });
                }
            }
        }
        if !matches!(state, State::Play { .. }) {
            app.fx.update(dt, star_speed);
        }
        let _ = app.demo_world_hint;

        if frames < 4 { trace(&format!("frame {frames} update done")); }
        // ---- draw
        let shake = app.fx.shake_offset(app.settings.screen_shake);
        app.g.begin_scene(shake);
        let hue = match &state {
            State::Play { world, .. } | State::GameOver { world, .. } => world.hue,
            _ => 280.0 + elapsed * 6.0,
        };
        // Ring style alternates every 3 rounds: 1-3 beat pulse, 4-6 bass visualizer, 7-9 beat...
        let bass_rounds = match &state {
            State::Play { world, .. } | State::GameOver { world, .. } => ((world.level.max(1) - 1) / 3) % 2 == 1,
            _ => false,
        };
        ring_bass_mix = approach(ring_bass_mix, if bass_rounds { 1.0 } else { 0.0 }, 3.0, dt);
        let bass = app.audio.bass();
        app.fx.draw_background(&app.g, hue, pulse, bg_intensity, ((star_speed - 0.2) / 1.4).clamp(0.0, 1.0), bass, ring_bass_mix);
        let hiscore = app.scores.first().map(|e| e.score).unwrap_or(0);
        match &state {
            State::Title { sel, .. } => draw_title(&app, *sel, pulse, elapsed),
            State::Options { sel } => draw_options(&app, *sel),
            State::HowTo => draw_howto(&app, elapsed),
            State::Scores { highlight, t } => draw_scores(&app, *highlight, *t, pulse),
            State::Play { world, paused, pause_sel, demo, .. } => {
                world.draw(&app.g, &app.fx, pulse);
                app.fx.draw_popups(&app.g);
                set_camera(&Gfx::rt_camera(&app.g.scene, Vec2::ZERO));
                world.draw_hud(&app.g, hiscore, pulse);
                app.fx.draw_banners(&app.g);
                if *paused {
                    draw_pause(&app, *pause_sel);
                }
                if *demo {
                    let a = 0.6 + 0.4 * (elapsed * 3.0).sin();
                    app.g.neon_text_center("DEMO MODE", VW * 0.5, VH - 60.0, 26.0, Color::new(1.0, 0.5, 0.9, a), true);
                    if !args.demo {
                        app.g.text_center("PRESS ANY KEY", VW * 0.5, VH - 32.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.7), false);
                    }
                }
            }
            State::GameOver { world, t } => {
                world.draw(&app.g, &app.fx, pulse * 0.3);
                set_camera(&Gfx::rt_camera(&app.g.scene, Vec2::ZERO));
                world.draw_hud(&app.g, hiscore, 0.0);
                draw_game_over(&app, world, *t);
            }
            State::NameEntry { score, letters, pos, t, .. } => draw_name_entry(&app, *score, letters, *pos, *t, pulse),
        }
        app.draw_now_playing();

        let flash = app.fx.flash;
        let aberr = app.fx.trauma * app.fx.trauma * 0.012 + pulse * 0.0012 + app.fx.warp * app.fx.warp * 0.014;
        if frames < 3 { trace("draw done"); }
        app.g.present(app.settings.crt, aberr, pulse, flash * 0.6, app.fx.flash_col);
        if frames < 3 { trace("present done"); }

        // ---- test hooks: screenshots / timed exit
        if let Some((t, path)) = &args.shot {
            if !shot_done && elapsed >= *t {
                get_screen_data().export_png(path);
                shot_done = true;
            }
        }
        if let Some((every, prefix)) = &args.shots_every {
            if elapsed >= next_shot {
                get_screen_data().export_png(&format!("{prefix}_{shot_n:03}.png"));
                shot_n += 1;
                next_shot += *every;
            }
        }
        if let Some(q) = args.quit_after {
            if elapsed >= q {
                break;
            }
        }
        if quit {
            break;
        }
        if let Some(n) = next {
            state = n;
        }
        if frames < 4 { trace("before next_frame"); }
        next_frame().await;
        if frames < 4 { trace("after next_frame"); }
    }
    if args.fps {
        println!("avg fps: {:.1} over {} frames", frames as f64 / (get_time() - start), frames);
    }
    persist::save_settings(&app.settings);
}

// ======================================================================== screens

fn draw_title(app: &App, sel: usize, pulse: f32, t: f32) {
    let g = &app.g;
    // showcase planets orbiting the logo
    for i in 0..5 {
        let a = t * 0.25 + i as f32 * std::f32::consts::TAU / 5.0;
        let depth = (a.sin() + 1.0) * 0.5;
        let p = vec2(VW * 0.5 + a.cos() * 540.0, 240.0 + a.sin() * 70.0);
        let r = 16.0 + depth * 30.0;
        let h = 280.0 + i as f32 * 60.0;
        let rim = hsv(h, 0.85, 1.0, 1.0);
        gl_use_material(&g.additive);
        g.glow_at(p, r * 2.2, with_alpha(rim, 0.25 + pulse * 0.1));
        gl_use_default_material();
        g.planet(i, p, r, t * 0.05 + i as f32, mix_color(WHITE, hsv(h, 0.6, 1.0, 1.0), 0.4), rim, 0.0, (1.0 - depth) * 0.5, 1.0);
    }

    // logo with per-letter bounce on the beat
    let title = "COMET MADNESS";
    let size = 92.0;
    let total = g.text_size(title, size, true).width;
    let mut x = VW * 0.5 - total * 0.5;
    for (i, ch) in title.chars().enumerate() {
        let s = ch.to_string();
        let w = g.text_size(&s, size, true).width;
        let bob = ((t * 3.0 + i as f32 * 0.5).sin()) * 4.0 - pulse * 6.0 * ((i % 2) as f32);
        let h = 300.0 + i as f32 * 12.0 + t * 30.0;
        let c = hsv(h, 0.65, 1.0, 1.0);
        gl_use_material(&g.additive);
        g.glow_at(vec2(x + w * 0.5, 230.0 + bob), 60.0 + pulse * 10.0, with_alpha(c, 0.22));
        gl_use_default_material();
        g.text(&s, x + 3.0, 262.0 + bob + 3.0, size, Color::new(0.0, 0.0, 0.1, 0.6), true);
        g.text(&s, x, 262.0 + bob, size, c, true);
        x += w;
    }
    g.text_center("A NEON REBOOT OF THE 1985 COMMODORE 64 CLASSIC", VW * 0.5, 310.0, 16.0, Color::new(0.75, 0.9, 1.0, 0.9), false);

    for (i, item) in TITLE_MENU.iter().enumerate() {
        let y = 430.0 + i as f32 * 46.0;
        if i == sel {
            let c = Color::new(1.0, 0.45, 0.9, 1.0);
            draw_rectangle(VW * 0.5 - 170.0, y - 30.0, 340.0, 40.0, Color::new(1.0, 0.3, 0.8, 0.12 + pulse * 0.08));
            draw_rectangle_lines(VW * 0.5 - 170.0, y - 30.0, 340.0, 40.0, 2.0, with_alpha(c, 0.8));
            g.neon_text_center(item, VW * 0.5, y, 24.0, WHITE, true);
        } else {
            g.text_center(item, VW * 0.5, y, 22.0, Color::new(0.6, 0.75, 1.0, 0.8), false);
        }
    }
    g.text_center(
        "ARROWS / WASD MOVE  -  HOLD SPACE / CTRL / Z TO FIRE  -  ESC PAUSE  -  F11 FULLSCREEN",
        VW * 0.5,
        VH - 46.0,
        11.0,
        Color::new(0.7, 0.8, 1.0, 0.65),
        false,
    );
    g.text_center(
        "TRIBUTE TO CRAZY COMETS (MARTECH 1985)  BY SIMON NICOL  /  MUSIC ROB HUBBARD  -  ALL-NEW CODE, ART AND MUSIC",
        VW * 0.5,
        VH - 24.0,
        10.0,
        Color::new(0.6, 0.6, 0.9, 0.55),
        false,
    );
}

fn draw_panel(x: f32, y: f32, w: f32, h: f32, c: Color) {
    draw_rectangle(x, y, w, h, Color::new(0.02, 0.01, 0.08, 0.78));
    draw_rectangle_lines(x, y, w, h, 2.0, c);
}

fn draw_options(app: &App, sel: usize) {
    let g = &app.g;
    draw_panel(VW * 0.5 - 330.0, 120.0, 660.0, 500.0, Color::new(0.4, 0.9, 1.0, 0.6));
    g.neon_text_center("OPTIONS", VW * 0.5, 185.0, 40.0, Color::new(0.5, 0.95, 1.0, 1.0), true);
    let s = &app.settings;
    let bar = |v: f32| -> String {
        let n = (v * 10.0).round() as usize;
        format!("{}{}", "|".repeat(n), ".".repeat(10 - n))
    };
    let rows = [
        ("MUSIC VOLUME", bar(s.music_volume)),
        ("SFX VOLUME", bar(s.sfx_volume)),
        ("CRT EFFECT", if s.crt { "ON".into() } else { "OFF".into() }),
        ("SCREEN SHAKE", if s.screen_shake { "ON".into() } else { "OFF".into() }),
        ("FULLSCREEN", if s.fullscreen { "ON".into() } else { "OFF".into() }),
        ("BACK", String::new()),
    ];
    for (i, (k, v)) in rows.iter().enumerate() {
        let y = 260.0 + i as f32 * 48.0;
        let on = i == sel;
        let c = if on { WHITE } else { Color::new(0.6, 0.75, 1.0, 0.85) };
        if on {
            draw_rectangle(VW * 0.5 - 300.0, y - 28.0, 600.0, 38.0, Color::new(0.3, 0.8, 1.0, 0.12));
        }
        g.text(k, VW * 0.5 - 280.0, y, 20.0, c, on);
        g.text_right(v, VW * 0.5 + 280.0, y, 20.0, if on { Color::new(1.0, 0.5, 0.9, 1.0) } else { c }, true);
    }
    g.text_center("UP/DOWN SELECT  -  LEFT/RIGHT ADJUST  -  ENTER TOGGLE  -  ESC BACK", VW * 0.5, 600.0, 12.0, Color::new(0.7, 0.8, 1.0, 0.6), false);
}

fn draw_howto(app: &App, t: f32) {
    let g = &app.g;
    draw_panel(80.0, 60.0, VW - 160.0, VH - 120.0, Color::new(1.0, 0.4, 0.85, 0.6));
    g.neon_text_center("HOW TO PLAY", VW * 0.5, 120.0, 40.0, Color::new(1.0, 0.5, 0.9, 1.0), true);
    let lines = [
        ("YOUR SHIP FACES INTO THE SCREEN.", "It never turns and always fires straight up. Fly under your targets to line up a shot."),
        ("KILL THEM SMALL.", "Planets hurtle in from the sides as distant dots and grow. Small ones can't hurt you."),
        ("THEY CURVE AND BOUNCE.", "Planets can only change course by curving or bouncing - and big ones curve in for you."),
        ("WORLDS", "Earth-like planets. Hit one enough times and it explodes. Bigger means more hits."),
        ("NOVAS", "Cratered planets. Shoot one and it glows angrily, then goes wild and hunts you. Finish it fast!"),
        ("COMETS AND ROCKETS", "From round 2 comets streak about and chase you. Fly over the little rockets for 200."),
        ("BONUSES", "Extra life every 10,000. Perfect Round bonus if no planet grows large and you don't die."),
    ];
    for (i, (h, d)) in lines.iter().enumerate() {
        let y = 190.0 + i as f32 * 58.0;
        g.text(h, 300.0, y, 17.0, hsv(300.0 + i as f32 * 25.0, 0.6, 1.0, 1.0), true);
        g.text(d, 300.0, y + 22.0, 13.0, Color::new(0.85, 0.9, 1.0, 0.9), false);
    }
    // little visual legend
    let x = 190.0;
    World::draw_ship(g, vec2(x, 186.0), (t * 2.0).sin() * 0.6, 0.8, Color::new(0.4, 0.95, 1.0, 1.0), true);
    g.planet(0, vec2(x - 20.0, 240.0), 8.0, t * 0.1, WHITE, Color::new(0.4, 0.8, 1.0, 1.0), 0.0, 0.5, 0.7);
    g.planet(0, vec2(x + 12.0, 244.0), 18.0, t * 0.1, WHITE, Color::new(0.4, 0.8, 1.0, 1.0), 0.0, 0.0, 1.0);
    for k in 0..6 {
        let a = t * 1.5 - k as f32 * 0.35;
        draw_circle(x + a.cos() * 26.0, 300.0 + a.sin() * 10.0, 3.0 - k as f32 * 0.4, Color::new(0.6, 0.8, 1.0, 0.8 - k as f32 * 0.12));
    }
    g.planet(0, vec2(x, 356.0), 24.0, t * 0.1, WHITE, Color::new(0.4, 0.8, 1.0, 1.0), 0.0, 0.0, 1.0);
    let ang = Color::new(1.0, 0.45 + 0.25 * (t * 30.0).sin(), 0.15, 1.0);
    g.planet(1, vec2(x, 414.0), 24.0, t * 0.3, WHITE, ang, (0.15 + 0.25 * (t * 24.0).sin()).max(0.0), 0.0, 1.0);
    gl_use_material(&g.additive);
    g.glow_at(vec2(x + 10.0, 466.0), 18.0, Color::new(0.6, 0.9, 1.0, 0.9));
    for k in 1..8 {
        g.glow_at(vec2(x + 10.0 - k as f32 * 8.0, 466.0), 9.0 - k as f32, Color::new(0.4, 0.8, 1.0, 0.3));
    }
    gl_use_default_material();
    draw_line(x - 30.0, 478.0, x - 12.0, 478.0, 8.0, Color::new(0.92, 0.95, 1.0, 1.0));
    g.text_center("PRESS ENTER", VW * 0.5, VH - 80.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.6), false);
}

fn draw_scores(app: &App, highlight: Option<usize>, t: f32, pulse: f32) {
    let g = &app.g;
    draw_panel(VW * 0.5 - 360.0, 70.0, 720.0, 580.0, Color::new(1.0, 0.85, 0.3, 0.6));
    g.neon_text_center("HALL OF FAME", VW * 0.5, 135.0, 44.0, Color::new(1.0, 0.85, 0.35, 1.0), true);
    g.text("RANK", VW * 0.5 - 300.0, 185.0, 12.0, Color::new(0.7, 0.8, 1.0, 0.6), false);
    g.text("NAME", VW * 0.5 - 170.0, 185.0, 12.0, Color::new(0.7, 0.8, 1.0, 0.6), false);
    g.text_right("SCORE", VW * 0.5 + 170.0, 185.0, 12.0, Color::new(0.7, 0.8, 1.0, 0.6), false);
    g.text_right("ROUND", VW * 0.5 + 300.0, 185.0, 12.0, Color::new(0.7, 0.8, 1.0, 0.6), false);
    for (i, e) in app.scores.iter().enumerate() {
        let appear = ((t * 8.0) - i as f32).clamp(0.0, 1.0);
        let y = 230.0 + i as f32 * 40.0;
        let hl = highlight == Some(i);
        let c = if hl { hsv(t * 300.0, 0.6, 1.0, appear) } else { with_alpha(hsv(40.0 + i as f32 * 25.0, 0.4, 1.0, 1.0), appear) };
        let dx = (1.0 - ease_out(appear)) * 60.0;
        if hl {
            draw_rectangle(VW * 0.5 - 320.0, y - 28.0, 640.0, 38.0, Color::new(1.0, 0.8, 0.3, 0.12 + pulse * 0.1));
        }
        g.text(&format!("{:>2}", i + 1), VW * 0.5 - 300.0 + dx, y, 22.0, c, true);
        g.text(&e.name, VW * 0.5 - 170.0 + dx, y, 22.0, c, true);
        g.text_right(&fmt_score(e.score), VW * 0.5 + 170.0 + dx, y, 22.0, c, true);
        g.text_right(&e.level.to_string(), VW * 0.5 + 300.0 + dx, y, 22.0, c, false);
    }
    g.text_center("PRESS ENTER", VW * 0.5, 630.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.6), false);
}

fn draw_pause(app: &App, sel: usize) {
    let g = &app.g;
    draw_rectangle(0.0, 0.0, VW, VH, Color::new(0.0, 0.0, 0.05, 0.55));
    draw_panel(VW * 0.5 - 220.0, 230.0, 440.0, 260.0, Color::new(0.4, 0.9, 1.0, 0.7));
    g.neon_text_center("PAUSED", VW * 0.5, 290.0, 40.0, Color::new(0.5, 0.95, 1.0, 1.0), true);
    for (i, item) in PAUSE_MENU.iter().enumerate() {
        let y = 360.0 + i as f32 * 42.0;
        if i == sel {
            g.neon_text_center(item, VW * 0.5, y, 22.0, WHITE, true);
        } else {
            g.text_center(item, VW * 0.5, y, 20.0, Color::new(0.6, 0.75, 1.0, 0.8), false);
        }
    }
}

fn draw_game_over(app: &App, w: &World, t: f32) {
    let g = &app.g;
    let a = (t / 0.6).min(1.0);
    draw_rectangle(0.0, 0.0, VW, VH, Color::new(0.02, 0.0, 0.05, 0.6 * a));
    let zoom = 1.0 + (1.0 - ease_out(t / 0.8)) * 0.8;
    g.neon_text_center("GAME OVER", VW * 0.5, 280.0, 80.0 * zoom, Color::new(1.0, 0.3, 0.5, a), true);
    let stats = [
        ("FINAL SCORE", fmt_score(w.score)),
        ("ROUND REACHED", w.level.to_string()),
        ("COMETS DESTROYED", w.kills.to_string()),
        ("BEST COMBO", format!("{}", w.best_combo)),
    ];
    for (i, (k, v)) in stats.iter().enumerate() {
        let ap = ((t - 0.6 - i as f32 * 0.2) / 0.3).clamp(0.0, 1.0);
        let y = 370.0 + i as f32 * 40.0;
        g.text_right(k, VW * 0.5 - 20.0, y, 18.0, Color::new(0.7, 0.8, 1.0, ap * 0.8), false);
        g.text(v, VW * 0.5 + 20.0, y, 22.0, Color::new(1.0, 1.0, 1.0, ap), true);
    }
    if t > 1.5 {
        g.text_center("PRESS ANY KEY", VW * 0.5, 580.0, 14.0, Color::new(1.0, 1.0, 1.0, 0.5 + 0.3 * (t * 4.0).sin()), false);
    }
}

fn draw_name_entry(app: &App, score: u64, letters: &[u8; 3], pos: usize, t: f32, pulse: f32) {
    let g = &app.g;
    draw_panel(VW * 0.5 - 340.0, 150.0, 680.0, 420.0, Color::new(1.0, 0.85, 0.3, 0.7));
    g.neon_text_center("NEW HIGH SCORE!", VW * 0.5, 220.0, 44.0, hsv(t * 200.0, 0.6, 1.0, 1.0), true);
    g.text_center(&fmt_score(score), VW * 0.5, 280.0, 32.0, WHITE, true);
    g.text_center("ENTER YOUR INITIALS", VW * 0.5, 330.0, 16.0, Color::new(0.7, 0.8, 1.0, 0.8), false);
    for i in 0..3 {
        let x = VW * 0.5 - 100.0 + i as f32 * 100.0;
        let on = i == pos.min(2);
        let c = if on { Color::new(1.0, 0.5, 0.9, 1.0) } else { Color::new(0.6, 0.85, 1.0, 1.0) };
        let ch = (letters[i] as char).to_string();
        if on {
            draw_rectangle(x - 34.0, 360.0, 68.0, 90.0, Color::new(1.0, 0.4, 0.9, 0.15 + pulse * 0.1));
        }
        draw_rectangle_lines(x - 34.0, 360.0, 68.0, 90.0, 2.0, c);
        g.neon_text_center(&ch, x, 428.0, 56.0, c, true);
    }
    g.text_center("TYPE OR USE ARROWS  -  ENTER TO CONFIRM", VW * 0.5, 520.0, 13.0, Color::new(0.8, 0.8, 1.0, 0.6), false);
}
