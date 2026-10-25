//! Attract-mode autopilot. The ship can only fire straight up, so the bot
//! slides underneath its chosen target while steering clear of threats.

use crate::game::{Input, World};
use crate::util::*;
use macroquad::prelude::*;

pub fn think(w: &World) -> Input {
    let p = w.player.pos;
    let mut push = Vec2::ZERO;
    let mut threat = |pos: Vec2, vel: Vec2, r: f32, weight: f32| {
        let ahead = pos + vel * 0.25;
        for q in [pos, ahead] {
            let d = p - q;
            let dist = (d.length() - r).max(8.0);
            if dist < 190.0 {
                push += d.normalize_or_zero() * weight * (190.0 - dist) / 190.0;
            }
        }
    };
    for c in w.comets.iter().filter(|c| c.harmful()) {
        threat(c.pos, c.vel, c.radius(), if c.crazy { 2.6 } else { 1.4 });
    }
    for s in w.sats.iter().filter(|s| s.attacking) {
        threat(s.pos, s.vel, 10.0, 1.6);
    }
    for m in w.minis.iter() {
        threat(m.pos, m.vel, 14.0, 2.0);
    }

    // pick a target: small planets first (cheap now, expensive later), then comets
    let mut best: Option<(f32, Vec2, f32)> = None;
    for c in w.comets.iter() {
        if c.pos.x < 0.0 || c.pos.x > VW || c.pos.y > p.y - 20.0 {
            continue;
        }
        let score = (c.pos.x - p.x).abs() * 0.7 + c.growth * 300.0 - if c.angry { 250.0 } else { 0.0 };
        // lead the target by the bullet's travel time
        let t = (p.y - c.pos.y) / 1050.0;
        let lead = c.pos.x + c.vel.x * t;
        if best.map_or(true, |(b, _, _)| score < b) {
            best = Some((score, vec2(lead, c.pos.y), c.radius()));
        }
    }
    for m in w.minis.iter() {
        if m.pos.y > p.y - 20.0 {
            continue;
        }
        let score = (m.pos.x - p.x).abs() * 0.8 + 60.0;
        let t = (p.y - m.pos.y) / 1050.0;
        if best.map_or(true, |(b, _, _)| score < b) {
            best = Some((score, vec2(m.pos.x + m.vel.x * t, m.pos.y), 12.0));
        }
    }

    let mut mv = Vec2::ZERO;
    let mut fire = false;
    if let Some((_, tgt, r)) = best {
        let dx = tgt.x - p.x;
        mv.x = (dx / 60.0).clamp(-1.0, 1.0);
        fire = dx.abs() < r + 14.0;
    }
    // hover in the lower part of the screen
    let home_y = VH * 0.8;
    mv.y = ((home_y - p.y) / 120.0).clamp(-1.0, 1.0);
    if push.length() > 0.15 {
        mv = (mv * 0.3 + push * 1.5).clamp_length_max(1.0);
    }
    // grab rockets when it's calm
    if push.length() < 0.2 {
        if let Some(c) = w.capsules.iter().min_by(|a, b| a.pos.distance(p).partial_cmp(&b.pos.distance(p)).unwrap()) {
            if c.pos.distance(p) < 300.0 {
                mv = (c.pos - p).normalize_or_zero();
            }
        }
    }
    if p.x < 60.0 {
        mv.x = mv.x.max(0.5);
    }
    if p.x > VW - 60.0 {
        mv.x = mv.x.min(-0.5);
    }
    Input { mv: mv.clamp_length_max(1.0), fire: fire || w.comets.iter().any(|c| (c.pos.x - p.x).abs() < c.radius() && c.pos.y < p.y) }
}
