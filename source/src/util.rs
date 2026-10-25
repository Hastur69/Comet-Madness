//! Small math / colour helpers shared across the game.

use macroquad::prelude::*;

/// Virtual playfield size. All gameplay happens in these coordinates.
pub const VW: f32 = 1280.0;
pub const VH: f32 = 720.0;
/// Internal render resolution (supersampled; scaled to the window with letterboxing).
pub const RW: u32 = 1920;
pub const RH: u32 = 1080;
pub const RSCALE: f32 = RW as f32 / VW;

pub fn rnd(a: f32, b: f32) -> f32 {
    macroquad::rand::gen_range(a, b)
}
pub fn rndi(a: i32, b: i32) -> i32 {
    macroquad::rand::gen_range(a, b)
}
pub fn chance(p: f32) -> bool {
    rnd(0.0, 1.0) < p
}
pub fn rnd_dir() -> Vec2 {
    let a = rnd(0.0, std::f32::consts::TAU);
    vec2(a.cos(), a.sin())
}
pub fn from_angle(a: f32) -> Vec2 {
    vec2(a.cos(), a.sin())
}
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
pub fn approach(cur: f32, target: f32, rate: f32, dt: f32) -> f32 {
    target + (cur - target) * (-rate * dt).exp()
}
/// HSV (h in degrees) to colour.
pub fn hsv(h: f32, s: f32, v: f32, a: f32) -> Color {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    Color::new(r + m, g + m, b + m, a)
}

pub fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

pub fn mix_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(lerp(a.r, b.r, t), lerp(a.g, b.g, t), lerp(a.b, b.b, t), lerp(a.a, b.a, t))
}

/// Ease out cubic.
pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(3)
}

/// Format score with thousands separators.
pub fn fmt_score(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
