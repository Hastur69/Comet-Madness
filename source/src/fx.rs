//! Particles, shockwaves, floating score popups, starfield and screen shake.

use crate::gfx::Gfx;
use crate::util::*;
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum PKind {
    Spark,
    Glow,
    Debris,
}

#[derive(Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: Color,
    pub kind: PKind,
    pub drag: f32,
    pub spin: f32,
    pub ang: f32,
}

pub struct Ring {
    pub pos: Vec2,
    pub r: f32,
    pub speed: f32,
    pub life: f32,
    pub max_life: f32,
    pub width: f32,
    pub color: Color,
}

pub struct Popup {
    pub pos: Vec2,
    pub text: String,
    pub life: f32,
    pub color: Color,
    pub size: f32,
}

pub struct Banner {
    pub text: String,
    pub sub: String,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
}

struct Star {
    p: Vec3,
    hue: f32,
}

pub struct Fx {
    pub particles: Vec<Particle>,
    pub rings: Vec<Ring>,
    pub popups: Vec<Popup>,
    pub banners: Vec<Banner>,
    stars: Vec<Star>,
    pub trauma: f32,
    pub flash: f32,
    pub flash_col: Color,
    pub nebula_scroll: Vec2,
    pub time: f32,
    /// 0..1 nebula visibility (fades out on the way into the void)
    pub nebula_amt: f32,
    /// 0..1 starfield visibility (0 in the void rounds)
    pub star_amt: f32,
    /// 0..1 hyperspace warp intensity (between galaxy sectors)
    pub warp: f32,
    /// 0 = normal blue-white warp, 1 = fire warp, 2 = reverse warp
    pub warp_kind: u8,
    /// 0..1 blend toward a fiery / icy-blue nebula (rounds 7 and 8)
    pub neb_fire: f32,
    pub neb_blue: f32,
    ring_phase: f32,
    /// -1..1 sideways motion of the ship; the starfield rolls the opposite way
    pub roll_input: f32,
    /// current roll speed (radians/second) — eases in and out for an inertia feel
    roll_speed: f32,
}

const MAX_PARTICLES: usize = 4000;

impl Fx {
    pub fn new() -> Self {
        let stars = (0..300).map(|_| Star { p: vec3(rnd(-1.0, 1.0), rnd(-1.0, 1.0), rnd(0.05, 1.0)), hue: rnd(180.0, 320.0) }).collect();
        Fx {
            particles: Vec::with_capacity(MAX_PARTICLES),
            rings: vec![],
            popups: vec![],
            banners: vec![],
            stars,
            trauma: 0.0,
            flash: 0.0,
            flash_col: WHITE,
            nebula_scroll: Vec2::ZERO,
            time: 0.0,
            nebula_amt: 1.0,
            star_amt: 1.0,
            warp: 0.0,
            warp_kind: 0,
            neb_fire: 0.0,
            neb_blue: 0.0,
            ring_phase: 0.0,
            roll_input: 0.0,
            roll_speed: 0.0,
        }
    }

    pub fn clear(&mut self) {
        self.particles.clear();
        self.rings.clear();
        self.popups.clear();
        self.banners.clear();
        self.trauma = 0.0;
        self.flash = 0.0;
    }

    pub fn add(&mut self, p: Particle) {
        if self.particles.len() < MAX_PARTICLES {
            self.particles.push(p);
        }
    }

    pub fn shake(&mut self, amt: f32) {
        self.trauma = (self.trauma + amt).min(1.0);
    }

    pub fn flash(&mut self, amt: f32, c: Color) {
        if amt > self.flash {
            self.flash = amt;
            self.flash_col = c;
        }
    }

    pub fn popup(&mut self, pos: Vec2, text: String, color: Color, size: f32) {
        self.popups.push(Popup { pos, text, life: 1.1, color, size });
    }

    pub fn banner(&mut self, text: &str, sub: &str, color: Color, dur: f32) {
        self.banners.push(Banner { text: text.into(), sub: sub.into(), life: dur, max_life: dur, color });
    }

    pub fn ring(&mut self, pos: Vec2, speed: f32, life: f32, width: f32, color: Color) {
        self.rings.push(Ring { pos, r: 4.0, speed, life, max_life: life, width, color });
    }

    /// Big colourful explosion scaled by `size` (0..1+).
    pub fn explode(&mut self, pos: Vec2, size: f32, c1: Color, c2: Color) {
        let n = (30.0 + size * 160.0) as usize;
        for i in 0..n {
            let d = rnd_dir();
            let sp = rnd(80.0, 520.0) * (0.6 + size * 0.9);
            let c = if i % 3 == 0 { c2 } else { c1 };
            self.add(Particle {
                pos: pos + d * rnd(0.0, 10.0 + size * 30.0),
                vel: d * sp,
                life: 0.0,
                max_life: rnd(0.35, 0.9) * (0.7 + size * 0.6),
                size: rnd(2.0, 4.5) * (0.8 + size * 0.6),
                color: c,
                kind: PKind::Spark,
                drag: 2.6,
                spin: 0.0,
                ang: 0.0,
            });
        }
        for _ in 0..(6.0 + size * 20.0) as usize {
            let d = rnd_dir();
            self.add(Particle {
                pos: pos + d * rnd(0.0, 20.0 * size),
                vel: d * rnd(20.0, 120.0),
                life: 0.0,
                max_life: rnd(0.4, 1.1),
                size: rnd(14.0, 36.0) * (0.6 + size),
                color: with_alpha(mix_color(c1, c2, rnd(0.0, 1.0)), 0.55),
                kind: PKind::Glow,
                drag: 1.6,
                spin: 0.0,
                ang: 0.0,
            });
        }
        for _ in 0..(4.0 + size * 14.0) as usize {
            let d = rnd_dir();
            self.add(Particle {
                pos,
                vel: d * rnd(60.0, 300.0),
                life: 0.0,
                max_life: rnd(0.8, 1.6),
                size: rnd(3.0, 7.0) * (0.7 + size * 0.5),
                color: mix_color(c1, WHITE, 0.3),
                kind: PKind::Debris,
                drag: 0.8,
                spin: rnd(-12.0, 12.0),
                ang: rnd(0.0, 6.28),
            });
        }
        self.ring(pos, 380.0 + size * 500.0, 0.45 + size * 0.25, 3.0 + size * 8.0, c1);
        if size > 0.6 {
            self.ring(pos, 220.0 + size * 260.0, 0.7, 2.0 + size * 4.0, c2);
        }
        self.add(Particle {
            pos,
            vel: Vec2::ZERO,
            life: 0.0,
            max_life: 0.25,
            size: 60.0 + size * 160.0,
            color: Color::new(1.0, 0.95, 0.9, 0.9),
            kind: PKind::Glow,
            drag: 0.0,
            spin: 0.0,
            ang: 0.0,
        });
    }

    pub fn sparks(&mut self, pos: Vec2, dir: Vec2, n: usize, c: Color, speed: f32) {
        for _ in 0..n {
            let d = (dir + rnd_dir() * 0.8).normalize_or_zero();
            self.add(Particle {
                pos,
                vel: d * rnd(speed * 0.4, speed),
                life: 0.0,
                max_life: rnd(0.15, 0.4),
                size: rnd(1.5, 3.0),
                color: c,
                kind: PKind::Spark,
                drag: 4.0,
                spin: 0.0,
                ang: 0.0,
            });
        }
    }

    pub fn update(&mut self, dt: f32, star_speed: f32) {
        self.time += dt;
        for p in self.particles.iter_mut() {
            p.life += dt;
            p.pos += p.vel * dt;
            p.vel *= (-p.drag * dt).exp();
            p.ang += p.spin * dt;
        }
        self.particles.retain(|p| p.life < p.max_life);
        for r in self.rings.iter_mut() {
            r.life -= dt;
            r.r += r.speed * dt;
            r.speed *= (-2.5 * dt).exp();
        }
        self.rings.retain(|r| r.life > 0.0);
        for p in self.popups.iter_mut() {
            p.life -= dt;
            p.pos.y -= 40.0 * dt;
        }
        self.popups.retain(|p| p.life > 0.0);
        for b in self.banners.iter_mut() {
            b.life -= dt;
        }
        self.banners.retain(|b| b.life > 0.0);
        self.trauma = (self.trauma - dt * 1.6).max(0.0);
        self.flash = (self.flash - dt * 3.0).max(0.0);
        // Like the original: steering left rolls the starfield to the right and vice versa.
        // The roll builds up and coasts down smoothly; moving up/down leaves it alone.
        self.roll_speed = approach(self.roll_speed, -self.roll_input * 1.3, 2.2, dt);
        let (rs, rc) = (self.roll_speed * dt).sin_cos();
        let (ax, ay) = (VW * 0.32, VH * 0.42);
        for s in self.stars.iter_mut() {
            // rotate in screen space so the roll is circular, not squashed
            let (x, y) = (s.p.x * ax, s.p.y * ay);
            s.p.x = (x * rc - y * rs) / ax;
            s.p.y = (x * rs + y * rc) / ay;
            s.p.z -= dt * star_speed;
            if s.p.z <= 0.02 {
                s.p = vec3(rnd(-1.0, 1.0), rnd(-1.0, 1.0), 1.0);
            } else if s.p.z > 1.0 {
                // reverse warp: stars stream back in from the edges toward the centre
                s.p = vec3(rnd(-1.0, 1.0), rnd(-1.0, 1.0), rnd(0.03, 0.2));
            }
        }
        self.nebula_scroll += vec2(4.0, 1.5) * dt;
        // tunnel rings rush toward the viewer during a warp (away from it in reverse)
        let dir = if self.warp_kind == 2 { -1.0 } else { 1.0 };
        self.ring_phase = (self.ring_phase + dt * (0.25 + self.warp * 2.5 * dir)).rem_euclid(1.0);
    }

    pub fn shake_offset(&self, enabled: bool) -> Vec2 {
        if !enabled {
            return Vec2::ZERO;
        }
        let s = self.trauma * self.trauma * 22.0;
        let t = self.time * 60.0;
        vec2((t * 1.7).sin() + (t * 3.1).cos() * 0.5, (t * 2.3).cos() + (t * 1.3).sin() * 0.5) * s * 0.66
    }

    // --------------------------------------------------------------- drawing

    /// `bass` is the music's current low-end level (0..1); `bass_mix` blends the rings
    /// from beat-pulse mode (0) to bass-visualizer mode (1).
    pub fn draw_background(&self, g: &Gfx, hue: f32, pulse: f32, intensity: f32, star_streak: f32, bass: f32, bass_mix: f32) {
        // nebula: two offset layers, tinted per level, faded by `nebula_amt`.
        // During a warp it zooms past the camera and bleaches toward white-blue.
        let warp = self.warp;
        let neb = self.nebula_amt;
        let zoom = 1.0 + warp * warp * 0.9;
        let (warp_tint, warp_hue, warp_sat) = match self.warp_kind {
            1 => (Color::new(0.7, 0.25, 0.05, 1.0), 25.0, 0.85),
            2 => (Color::new(0.4, 0.2, 0.6, 1.0), 275.0, 0.5),
            _ => (Color::new(0.25, 0.3, 0.6, 1.0), -1.0, 0.0),
        };
        let mut t1 = hsv(hue, 0.65, 0.55 * intensity * neb, 1.0);
        let mut t2 = hsv(hue + 140.0, 0.6, 0.35 * intensity, 1.0);
        // Rounds 7 and 8: a faint fiery, then icy-blue, nebula
        t1 = mix_color(t1, hsv(12.0, 0.9, 0.6 * intensity * neb, 1.0), self.neb_fire);
        t2 = mix_color(t2, hsv(35.0, 0.9, 0.45 * intensity, 1.0), self.neb_fire);
        t1 = mix_color(t1, hsv(215.0, 0.8, 0.55 * intensity * neb, 1.0), self.neb_blue);
        t2 = mix_color(t2, hsv(185.0, 0.7, 0.4 * intensity, 1.0), self.neb_blue);
        let t1 = mix_color(t1, warp_tint, warp * 0.5 * neb);
        let off = self.nebula_scroll;
        let layer = |w: f32, h: f32, dx: f32, dy: f32| (VW * 0.5 - w * zoom * 0.5 + dx, VH * 0.5 - h * zoom * 0.5 + dy, vec2(w * zoom, h * zoom));
        let (x, y, size) = layer(VW + 120.0, VH + 80.0, 0.0, 0.0);
        draw_texture_ex(&g.nebula, x, y, t1, DrawTextureParams { dest_size: Some(size), ..Default::default() });
        gl_use_material(&g.additive);
        if neb > 0.001 {
            let (x, y, size) = layer(VW + 200.0, VH + 120.0, -(off.x * 0.3).sin() * 30.0, (off.y * 0.2).cos() * 20.0);
            draw_texture_ex(
                &g.nebula,
                x,
                y,
                with_alpha(t2, 0.8 * neb),
                DrawTextureParams { dest_size: Some(size), flip_x: true, flip_y: true, ..Default::default() },
            );
            let (x, y, size) = layer(VW + 200.0, VH + 120.0, (off.x * 0.6).sin() * 20.0, 0.0);
            draw_texture_ex(&g.nebula, x, y, with_alpha(t2, 0.6 * neb), DrawTextureParams { dest_size: Some(size), flip_x: true, ..Default::default() });
        }

        // depth rings — a tunnel pulsing to the beat, echoing the original's
        // "flying into the screen" perspective
        let c = vec2(VW * 0.5, VH * 0.5);
        for i in 0..7 {
            let z = ((i as f32 + self.ring_phase.fract()) / 7.0).max(0.001);
            let r = 40.0 / (1.05 - z);
            // beat mode: brightness throbs on each beat
            let a_beat = (z * z) * 0.07 * intensity * (1.0 + pulse * 1.5);
            // bass mode: rings swell outward and flare brighter/thicker with the low end
            let r_bass = r * (1.0 + bass * (0.15 + 0.28 * z));
            let a_bass = (z * z) * intensity * (0.05 + 0.4 * bass);
            let w_bass = 1.5 + z * 2.0 + bass * (2.0 + 4.0 * z);
            let r = lerp(r, r_bass, bass_mix);
            let a = lerp(a_beat, a_bass, bass_mix);
            let w = lerp(1.5 + z * 2.0, w_bass, bass_mix);
            let a = a * (1.0 + warp * 4.0);
            let w = w * (1.0 + warp * 1.5);
            let ring_col = if warp_hue >= 0.0 {
                mix_color(hsv(hue + 40.0, 0.7, 1.0, a), hsv(warp_hue, warp_sat, 1.0, a), warp)
            } else {
                hsv(hue + 40.0 - warp * 40.0, 0.7 - warp * 0.4, 1.0, a)
            };
            draw_circle_lines(c.x, c.y, r, w, ring_col);
        }

        // starfield streaks (perspective projection from screen centre)
        // a warp always streaks the stars, even out of a starless sector
        let star_vis = self.star_amt.max(self.warp);
        let stars = if star_vis > 0.001 { &self.stars[..] } else { &[] };
        for s in stars.iter() {
            let z = s.p.z;
            let sx = c.x + s.p.x / z * VW * 0.32;
            let sy = c.y + s.p.y / z * VH * 0.42;
            if sx < -20.0 || sx > VW + 20.0 || sy < -20.0 || sy > VH + 20.0 {
                continue;
            }
            // during a warp the streaks stretch into long hyperspace lines
            let z2 = (z + 0.012 + 0.03 * star_streak + pulse * 0.01 + warp * warp * 0.45).min(1.6);
            let px = c.x + s.p.x / z2 * VW * 0.32;
            let py = c.y + s.p.y / z2 * VH * 0.42;
            let b = (((1.0 - z) * 0.9).clamp(0.05, 0.75) + warp * 0.5) * intensity * star_vis;
            let col = if warp_hue >= 0.0 {
                mix_color(hsv(s.hue, 0.35, 1.0, b), hsv(warp_hue + s.hue * 0.1, warp_sat, 1.0, b), warp)
            } else {
                hsv(s.hue - warp * 60.0, 0.35 - warp * 0.2, 1.0, b)
            };
            draw_line(px, py, sx, sy, (1.0 - z) * (1.6 + warp * 1.5) + 0.5, col);
        }
        gl_use_default_material();
    }

    pub fn draw_particles(&self, g: &Gfx) {
        gl_use_material(&g.additive);
        for p in self.particles.iter() {
            let t = p.life / p.max_life;
            let fade = 1.0 - t;
            match p.kind {
                PKind::Spark => {
                    let v = p.vel.length();
                    let len = (v * 0.045).clamp(p.size, 40.0);
                    let ang = p.vel.y.atan2(p.vel.x);
                    g.streak(p.pos, ang, len * 1.6, p.size * 2.6, with_alpha(p.color, fade));
                }
                PKind::Glow => {
                    let s = p.size * (0.6 + 0.6 * t);
                    g.glow_at(p.pos, s, with_alpha(p.color, p.color.a * fade * fade));
                }
                PKind::Debris => {}
            }
        }
        for r in self.rings.iter() {
            let t = r.life / r.max_life;
            draw_circle_lines(r.pos.x, r.pos.y, r.r, r.width * t + 0.5, with_alpha(r.color, t * 0.9));
            draw_circle_lines(r.pos.x, r.pos.y, r.r * 0.96, r.width * t * 2.5, with_alpha(r.color, t * 0.25));
        }
        gl_use_default_material();
        for p in self.particles.iter() {
            if p.kind == PKind::Debris {
                let t = 1.0 - p.life / p.max_life;
                let d = from_angle(p.ang) * p.size;
                let e = from_angle(p.ang + 2.2) * p.size * 0.7;
                let f = from_angle(p.ang + 4.0) * p.size * 0.9;
                draw_triangle(p.pos + d, p.pos + e, p.pos + f, with_alpha(p.color, t));
            }
        }
    }

    pub fn draw_popups(&self, g: &Gfx) {
        for p in self.popups.iter() {
            let a = (p.life / 1.1).min(1.0);
            let s = p.size * (1.0 + (1.0 - a).powi(3) * 0.2);
            g.text_center(&p.text, p.pos.x, p.pos.y, s, with_alpha(p.color, a), true);
        }
    }

    pub fn draw_banners(&self, g: &Gfx) {
        if let Some(b) = self.banners.last() {
            let t = 1.0 - b.life / b.max_life;
            let a = if t < 0.12 { t / 0.12 } else if b.life < 0.4 { b.life / 0.4 } else { 1.0 };
            let zoom = 1.0 + (1.0 - ease_out(t / 0.15)) * 0.6;
            let y = VH * 0.42;
            draw_rectangle(0.0, y - 70.0, VW, 110.0, Color::new(0.0, 0.0, 0.0, 0.35 * a));
            draw_line(0.0, y - 70.0, VW, y - 70.0, 2.0, with_alpha(b.color, 0.6 * a));
            draw_line(0.0, y + 40.0, VW, y + 40.0, 2.0, with_alpha(b.color, 0.6 * a));
            g.neon_text_center(&b.text, VW * 0.5, y, 54.0 * zoom, with_alpha(b.color, a), true);
            if !b.sub.is_empty() {
                g.text_center(&b.sub, VW * 0.5, y + 30.0, 20.0, Color::new(1.0, 1.0, 1.0, 0.85 * a), false);
            }
        }
    }
}
