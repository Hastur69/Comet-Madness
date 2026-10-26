//! Core gameplay, modelled on the 1985 C64 original rather than its arcade parent:
//!
//!  * your ship never rotates — it is seen from behind, pointing *into* the screen,
//!    and always fires straight up; you line up shots by flying under your targets;
//!  * planets come hurtling in from the sides, start small and grow while they
//!    bounce and curve around the screen. They can't steer freely: they change
//!    direction by bouncing off the edges or bending along a curve — bending ever
//!    harder toward you as they grow, until big ones home in like missiles;
//!  * small (distant) planets can't hurt you; big ones take many hits;
//!  * WORLDS (Earth-like) simply explode once hit enough;
//!  * NOVAS (cratered) start glowing angrily when shot and, moments later, go wild
//!    and home in on you very fast — finish them quickly;
//!  * fully-grown planets gain an orbiting satellite that breaks off to attack;
//!  * from round 2 small comets streak about and chase you if left alone; small
//!    rockets drift out with them — fly over one for 200 points (shooting does nothing);
//!  * extra life every 10,000; Perfect Round bonus if every planet is destroyed
//!    before it grows large and no life is lost; no firing between rounds.

use crate::audio::{Sfx, Track};
use crate::fx::Fx;
use crate::gfx::Gfx;
use crate::util::*;
use macroquad::prelude::*;
use std::collections::VecDeque;

#[derive(Default, Clone, Copy)]
pub struct Input {
    pub mv: Vec2,
    pub fire: bool,
}

pub struct Player {
    pub pos: Vec2,
    pub vel: Vec2,
    pub alive: bool,
    pub respawn_t: f32,
    pub invuln: f32,
    fire_cd: f32,
    gun: bool,
    pub bank: f32,
}

pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub start_y: f32,
    pub life: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    World,
    Nova,
}

pub struct Comet {
    pub id: u32,
    pub kind: Kind,
    pub pos: Vec2,
    pub vel: Vec2,
    pub growth: f32,
    pub damage: f32,
    /// Nova has been shot and is glowing angrily (fuse running)
    pub angry: bool,
    pub fuse: f32,
    /// Nova has gone wild — homing at high speed
    pub crazy: bool,
    pub map: usize,
    pub rot: f32,
    pub rot_speed: f32,
    pub hue_off: f32,
    pub flash: f32,
    pub phase: f32,
    pub curve: f32,
    pub curve_t: f32,
    pub hp_show: f32,
    pub alive: bool,
    pub spawn_t: f32,
    pub grown: bool,
    trail: VecDeque<Vec2>,
}

pub struct Satellite {
    pub parent: Option<u32>,
    pub orbit_r: f32,
    pub orbit_a: f32,
    pub orbit_speed: f32,
    pub pos: Vec2,
    pub vel: Vec2,
    pub attacking: bool,
    pub detach_t: f32,
    pub life: f32,
    pub alive: bool,
}

pub struct Mini {
    pub pos: Vec2,
    pub vel: Vec2,
    pub curve: f32,
    pub age: f32,
    pub trail: VecDeque<Vec2>,
    pub alive: bool,
}

pub struct Capsule {
    pub pos: Vec2,
    pub vel: Vec2,
    pub t: f32,
    pub alive: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Phase {
    Intro,
    Playing,
    Clear,
}

pub struct World {
    pub player: Player,
    pub bullets: Vec<Bullet>,
    pub comets: Vec<Comet>,
    pub sats: Vec<Satellite>,
    pub minis: Vec<Mini>,
    pub capsules: Vec<Capsule>,
    pub level: u32,
    pub score: u64,
    pub lives: u32,
    next_life: u64,
    pub to_spawn: u32,
    spawn_t: f32,
    mini_t: f32,
    pub combo: u32,
    pub combo_t: f32,
    pub best_combo: u32,
    perfect: bool,
    pub phase: Phase,
    pub phase_t: f32,
    pub time_scale: f32,
    slowmo_t: f32,
    pub game_over: bool,
    pub game_over_t: f32,
    pub t: f32,
    next_id: u32,
    pub sounds: Vec<(Sfx, f32)>,
    pub kills: u32,
    pub hue: f32,
}

pub const EXTRA_LIFE_EVERY: u64 = 10_000;
const PLAYER_R: f32 = 12.0;
const TOP: f32 = 62.0; // below the HUD bar

pub fn track_for_level(l: u32) -> Track {
    if (l.max(1) - 1) % 10 >= 8 {
        return Track::Crazy;
    }
    if ((l.max(1) - 1) / 4) % 2 == 0 {
        Track::Rush
    } else {
        Track::Cascade
    }
}

struct LevelParams {
    count: u32,
    max_alive: usize,
    interval: f32,
    growth: f32,
    speed: f32,
    nova_chance: f32,
}

fn params(l: u32) -> LevelParams {
    let lf = l as f32;
    LevelParams {
        // gentle, endless ramp: rounds keep coming forever, getting steadily nastier
        count: (6 + l).min(22),
        max_alive: (2 + (l / 3) as usize).min(6),
        interval: (2.4 - 0.08 * lf).max(0.9),
        growth: (0.042 + 0.005 * lf).min(0.13),
        speed: (1.0 + 0.035 * lf).min(1.8),
        nova_chance: (0.2 + 0.03 * lf).min(0.5),
    }
}

/// Rotate a vector by an angle.
fn rot(v: Vec2, a: f32) -> Vec2 {
    let (s, c) = a.sin_cos();
    vec2(v.x * c - v.y * s, v.x * s + v.y * c)
}

impl Comet {
    pub fn radius(&self) -> f32 {
        8.0 + 54.0 * self.growth.powf(0.85)
    }
    /// Distant (small) planets fly "behind" the ship and can't hurt it.
    pub fn harmful(&self) -> bool {
        self.growth >= 0.3 || self.crazy
    }
    fn max_hp(&self, level: u32) -> f32 {
        1.0 + (self.growth * 11.0).floor() + (level / 4) as f32
    }
}

impl World {
    pub fn new(start_level: u32) -> Self {
        let mut w = World {
            player: Player {
                pos: vec2(VW * 0.5, VH * 0.78),
                vel: Vec2::ZERO,
                alive: true,
                respawn_t: 0.0,
                invuln: 2.0,
                fire_cd: 0.0,
                gun: false,
                bank: 0.0,
            },
            bullets: vec![],
            comets: vec![],
            sats: vec![],
            minis: vec![],
            capsules: vec![],
            level: start_level.max(1),
            score: 0,
            lives: 3,
            next_life: EXTRA_LIFE_EVERY,
            to_spawn: 0,
            spawn_t: 0.0,
            mini_t: 0.0,
            combo: 0,
            combo_t: 0.0,
            best_combo: 0,
            perfect: true,
            phase: Phase::Intro,
            phase_t: 0.0,
            time_scale: 1.0,
            slowmo_t: 0.0,
            game_over: false,
            game_over_t: 0.0,
            t: 0.0,
            next_id: 1,
            sounds: vec![],
            kills: 0,
            hue: 0.0,
        };
        w.begin_level(None);
        w
    }

    fn sfx(&mut self, s: Sfx, v: f32) {
        self.sounds.push((s, v));
    }

    fn begin_level(&mut self, fx: Option<&mut Fx>) {
        let p = params(self.level);
        self.to_spawn = p.count;
        self.spawn_t = 0.4;
        self.mini_t = rnd(5.0, 8.0);
        self.perfect = true;
        self.phase = Phase::Intro;
        self.phase_t = 2.0;
        // the original changes the planets' colours every level
        self.hue = 280.0 + self.level as f32 * 53.0;
        if let Some(fx) = fx {
            self.first_banner(fx);
        }
    }

    pub fn first_banner(&mut self, fx: &mut Fx) {
        let sub = match self.level {
            1 => "SHOOT THE PLANETS WHILE THEY ARE SMALL",
            2 => "COMETS AND LOST ROCKETS INBOUND",
            _ => "THE UNIVERSE IS COLLAPSING",
        };
        fx.banner(&format!("ROUND {}", self.level), sub, hsv(self.hue, 0.5, 1.0, 1.0), 2.2);
    }

    pub fn comets_remaining(&self) -> u32 {
        self.to_spawn + self.comets.iter().filter(|c| c.alive).count() as u32
    }

    fn mult(&self) -> u64 {
        (1 + self.combo / 4).min(4) as u64
    }

    fn add_score(&mut self, pts: u64, fx: &mut Fx) {
        self.score += pts;
        while self.score >= self.next_life {
            self.next_life += EXTRA_LIFE_EVERY;
            self.lives = (self.lives + 1).min(9);
            self.sfx(Sfx::ExtraLife, 1.0);
            fx.popup(self.player.pos + vec2(0.0, -46.0), "EXTRA LIFE!".into(), Color::new(0.4, 1.0, 0.5, 1.0), 24.0);
        }
    }

    fn register_kill(&mut self, base: u64, pos: Vec2, fx: &mut Fx, color: Color) {
        self.combo += 1;
        self.combo_t = 1.6;
        self.best_combo = self.best_combo.max(self.combo);
        self.kills += 1;
        let m = self.mult();
        let pts = base * m;
        self.add_score(pts, fx);
        let txt = if m > 1 { format!("{} x{}", pts / m, m) } else { pts.to_string() };
        fx.popup(pos, txt, color, if base >= 1000 { 26.0 } else { 18.0 });
    }

    /// Planets come hurtling in from the sides of the screen.
    fn spawn_comet(&mut self, fx: &mut Fx) {
        let p = params(self.level);
        let left = chance(0.5);
        let y = rnd(TOP + 40.0, VH * 0.62);
        let pos = vec2(if left { -10.0 } else { VW + 10.0 }, y);
        let ang = rnd(-0.45, 0.45) + if left { 0.0 } else { std::f32::consts::PI };
        let kind = if chance(p.nova_chance) { Kind::Nova } else { Kind::World };
        let id = self.next_id;
        self.next_id += 1;
        self.comets.push(Comet {
            id,
            kind,
            pos,
            vel: from_angle(ang) * rnd(300.0, 380.0) * p.speed,
            growth: 0.04,
            damage: 0.0,
            angry: false,
            fuse: 0.0,
            crazy: false,
            map: if kind == Kind::World { 0 } else { 1 },
            rot: rnd(0.0, 1.0),
            rot_speed: rnd(0.05, 0.12) * if left { -1.0 } else { 1.0 },
            hue_off: rnd(-25.0, 25.0),
            flash: 0.0,
            phase: rnd(0.0, 6.28),
            curve: rnd(-0.6, 0.6),
            curve_t: rnd(0.8, 1.8),
            hp_show: 0.0,
            alive: true,
            spawn_t: 0.0,
            grown: false,
            trail: VecDeque::new(),
        });
        let _ = fx;
    }

    fn spawn_mini(&mut self) {
        let side = rndi(0, 3);
        let (pos, dir) = match side {
            0 => (vec2(-30.0, rnd(TOP + 30.0, VH * 0.5)), vec2(1.0, rnd(-0.2, 0.6))),
            1 => (vec2(VW + 30.0, rnd(TOP + 30.0, VH * 0.5)), vec2(-1.0, rnd(-0.2, 0.6))),
            _ => (vec2(rnd(120.0, VW - 120.0), TOP - 20.0), vec2(rnd(-0.6, 0.6), 1.0)),
        };
        let sp = rnd(430.0, 520.0) * (1.0 + 0.03 * self.level as f32).min(1.4);
        self.minis.push(Mini { pos, vel: dir.normalize() * sp, curve: rnd(-1.2, 1.2), age: 0.0, trail: VecDeque::new(), alive: true });
        // a small rocket often drifts out with the comet
        if chance(0.6) && self.capsules.len() < 2 {

            // Original left side entry
            //let fromleft = chance(0.5);
            //let cp = vec2(if fromleft { -30.0 } else { VW + 30.0 }, rnd(VH * 0.35, VH - 60.0));
            //let cv = vec2(if fromleft { 1.0 } else { -1.0 } * rnd(70.0, 110.0), rnd(-20.0, 20.0));

            // rockets always enter from the top edge and fall nose-first
            let cp = vec2(rnd(80.0, VW - 80.0), -30.0);
            let cv = vec2(rnd(-25.0, 25.0), rnd(90.0, 130.0));

            self.capsules.push(Capsule { pos: cp, vel: cv, t: 0.0, alive: true });
            self.sfx(Sfx::CapsuleSpawn, 1.0);
        }
    }

    fn spawn_satellite(&mut self, parent: u32, i: usize) {
        self.sats.push(Satellite {
            parent: Some(parent),
            orbit_r: 90.0 + i as f32 * 20.0,
            orbit_a: rnd(0.0, 6.28),
            orbit_speed: rnd(1.6, 2.4) * if i % 2 == 0 { 1.0 } else { -1.0 },
            pos: Vec2::ZERO,
            vel: Vec2::ZERO,
            attacking: false,
            detach_t: rnd(2.5, 4.5) - (self.level as f32 * 0.12).min(1.5),
            life: 0.0,
            alive: true,
        });
    }

    /// Developer showcase: one of everything (used for screenshots).
    pub fn showcase(&mut self, fx: &mut Fx) {
        self.phase = Phase::Playing;
        self.to_spawn = 50;
        let setups = [(0.12, Kind::World), (0.4, Kind::World), (0.65, Kind::Nova), (1.0, Kind::World)];
        for (i, &(g, k)) in setups.iter().enumerate() {
            self.spawn_comet(fx);
            let c = self.comets.last_mut().unwrap();
            c.growth = g;
            c.kind = k;
            c.map = if k == Kind::World { 0 } else { 1 };
            c.spawn_t = 5.0;
            c.vel = vec2(0.0, 0.0);
            c.pos = vec2(180.0 + i as f32 * 300.0, 200.0 + (i % 2) as f32 * 110.0);
        }
        self.comets[2].angry = true;
        self.comets[2].crazy = true;
        self.comets[3].grown = true;
        self.comets[3].hp_show = 5.0;
        self.comets[3].damage = 5.0;
        let id = self.comets[3].id;
        self.spawn_satellite(id, 0);
        self.spawn_mini();
        self.minis[0].pos = vec2(300.0, 470.0);
        self.minis[0].vel = vec2(500.0, -80.0);
        self.capsules.push(Capsule { pos: vec2(900.0, 560.0), vel: Vec2::ZERO, t: 0.0, alive: true });
        self.player.pos = vec2(640.0, 600.0);
        self.player.invuln = 0.0;
        for k in 0..5 {
            self.bullets.push(Bullet { pos: vec2(633.0 + (k % 2) as f32 * 14.0, 560.0 - k as f32 * 95.0), vel: vec2(0.0, -1000.0), start_y: 580.0, life: 1.0 });
        }
    }

    // ---------------------------------------------------------------- update

    pub fn update(&mut self, raw_dt: f32, input: &Input, fx: &mut Fx) {
        if self.slowmo_t > 0.0 {
            self.slowmo_t -= raw_dt;
            self.time_scale = approach(self.time_scale, 0.3, 12.0, raw_dt);
        } else {
            self.time_scale = approach(self.time_scale, 1.0, 6.0, raw_dt);
        }
        let dt = raw_dt * self.time_scale;
        self.t += dt;
        if self.combo_t > 0.0 {
            self.combo_t -= dt;
            if self.combo_t <= 0.0 {
                self.combo = 0;
            }
        }

        self.update_phase(dt, fx);
        self.update_player(dt, input, fx);
        for b in self.bullets.iter_mut() {
            b.pos += b.vel * dt;
            b.life -= dt;
            if b.pos.y < TOP - 10.0 {
                b.life = 0.0;
            }
        }
        self.update_comets(dt, fx);
        self.update_sats(dt, fx);
        self.update_minis(dt);
        for c in self.capsules.iter_mut() {
            c.t += dt;
            //c.pos += c.vel * dt + vec2(0.0, (c.t * 2.2).sin() * 20.0 * dt);
            //if c.pos.x < -80.0 || c.pos.x > VW + 80.0 {
            c.pos += c.vel * dt + vec2((c.t * 2.2).sin() * 20.0 * dt, 0.0);
            if c.pos.y > VH + 60.0 || c.pos.x < -80.0 || c.pos.x > VW + 80.0 {
                c.alive = false;
            }
        }
        self.collide(fx);

        self.bullets.retain(|b| b.life > 0.0);
        self.comets.retain(|c| c.alive);
        self.sats.retain(|s| s.alive);
        self.minis.retain(|m| m.alive);
        self.capsules.retain(|c| c.alive);

        if self.game_over {
            self.game_over_t += raw_dt;
        }
    }

    fn update_phase(&mut self, dt: f32, fx: &mut Fx) {
        match self.phase {
            Phase::Intro => {
                self.phase_t -= dt;
                if self.phase_t <= 0.0 {
                    self.phase = Phase::Playing;
                }
            }
            Phase::Playing => {
                if self.game_over {
                    return;
                }
                let p = params(self.level);
                let alive = self.comets.len();
                self.spawn_t -= dt;
                if self.to_spawn > 0 && alive < p.max_alive && (self.spawn_t <= 0.0 || alive == 0) {
                    self.spawn_comet(fx);
                    self.to_spawn -= 1;
                    self.spawn_t = p.interval * rnd(0.8, 1.25);
                }
                if self.level >= 2 {
                    self.mini_t -= dt;
                    if self.mini_t <= 0.0 {
                        self.mini_t = (rnd(7.0, 11.0) - 0.25 * self.level as f32).max(3.5);
                        self.spawn_mini();
                    }
                }
                if self.to_spawn == 0 && self.comets.is_empty() && self.player.alive {
                    self.finish_level(fx);
                }
            }
            Phase::Clear => {
                self.phase_t -= dt;
                if self.phase_t <= 0.0 {
                    self.level += 1;
                    self.begin_level(Some(fx));
                }
            }
        }
    }

    fn finish_level(&mut self, fx: &mut Fx) {
        self.phase = Phase::Clear;
        self.phase_t = 3.2;
        self.sfx(Sfx::LevelClear, 1.0);
        for s in self.sats.iter_mut() {
            fx.explode(s.pos, 0.1, Color::new(1.0, 0.5, 0.3, 1.0), WHITE);
            s.alive = false;
        }
        for m in self.minis.iter_mut() {
            m.alive = false;
        }
        let mut sub = String::new();
        if self.perfect {
            let b = 1000 * self.level as u64;
            self.add_score(b, fx);
            sub = format!("PERFECT ROUND BONUS  +{}", fmt_score(b));
        }
        fx.banner(&format!("ROUND {} CLEAR", self.level), &sub, Color::new(0.5, 1.0, 0.8, 1.0), 3.0);
    }

    fn update_player(&mut self, dt: f32, input: &Input, fx: &mut Fx) {
        let pl = &mut self.player;
        if !pl.alive {
            if self.game_over {
                return;
            }
            pl.respawn_t -= dt;
            if pl.respawn_t <= 0.0 {
                pl.alive = true;
                pl.pos = vec2(VW * 0.5, VH * 0.78);
                pl.vel = Vec2::ZERO;
                pl.invuln = 2.5;
                for c in self.comets.iter_mut() {
                    let d = c.pos - pl.pos;
                    if d.length() < 280.0 {
                        c.vel = d.normalize_or_zero() * c.vel.length().max(250.0);
                    }
                }
                for s in self.sats.iter_mut() {
                    if s.attacking {
                        s.alive = false;
                    }
                }
                fx.ring(pl.pos, 600.0, 0.5, 4.0, Color::new(0.4, 0.9, 1.0, 1.0));
            }
            return;
        }
        pl.invuln = (pl.invuln - dt).max(0.0);

        // joystick-style movement with a little inertia
        let mv = if input.mv.length() > 1.0 { input.mv.normalize() } else { input.mv };
        pl.vel += mv * 3800.0 * dt;
        pl.vel *= (-9.0 * dt).exp();
        let max_speed = 470.0;
        if pl.vel.length() > max_speed {
            pl.vel = pl.vel.normalize() * max_speed;
        }
        pl.pos += pl.vel * dt;
        pl.pos.x = pl.pos.x.clamp(26.0, VW - 26.0);
        pl.pos.y = pl.pos.y.clamp(TOP + 30.0, VH - 26.0);
        pl.bank = approach(pl.bank, (pl.vel.x / max_speed).clamp(-1.0, 1.0), 12.0, dt);

        // engine glow particles drift toward the viewer (down the screen)
        if chance(0.6) {
            for sx in [-9.0f32, 9.0] {
                fx.add(crate::fx::Particle {
                    pos: pl.pos + vec2(sx, 12.0),
                    vel: vec2(rnd(-20.0, 20.0), rnd(90.0, 170.0)) + pl.vel * 0.2,
                    life: 0.0,
                    max_life: rnd(0.12, 0.25),
                    size: rnd(1.5, 2.5),
                    color: Color::new(0.35, 0.75, 1.0, 1.0),
                    kind: crate::fx::PKind::Spark,
                    drag: 3.0,
                    spin: 0.0,
                    ang: 0.0,
                });
            }
        }

        // auto-fire, always straight up — into the screen
        pl.fire_cd -= dt;
        if input.fire && self.phase == Phase::Playing && pl.fire_cd <= 0.0 {
            pl.fire_cd = 0.075;
            pl.gun = !pl.gun;
            let off = if pl.gun { 8.0 } else { -8.0 };
            let muzzle = pl.pos + vec2(off, -16.0);
            self.bullets.push(Bullet { pos: muzzle, vel: vec2(0.0, -1050.0), start_y: muzzle.y, life: 1.2 });
            fx.sparks(muzzle, vec2(0.0, -1.0), 1, Color::new(0.6, 0.95, 1.0, 1.0), 220.0);
            self.sounds.push((Sfx::Shoot, 1.0));
        }
    }

    fn update_comets(&mut self, dt: f32, fx: &mut Fx) {
        let p = params(self.level);
        let ppos = self.player.pos;
        let palive = self.player.alive;
        let playing = self.phase == Phase::Playing;
        let mut new_sats = vec![];
        let mut wild = vec![];
        for c in self.comets.iter_mut() {
            c.phase += dt;
            c.spawn_t += dt;
            c.rot += c.rot_speed * dt * if c.crazy { 5.0 } else { 1.0 };
            c.flash = (c.flash - dt * 6.0).max(0.0);
            c.hp_show = (c.hp_show - dt).max(0.0);
            if playing && !c.crazy {
                c.growth = (c.growth + p.growth * dt).min(1.0);
                if c.growth > 0.55 {
                    self.perfect = false;
                }
                if c.growth >= 1.0 && !c.grown {
                    c.grown = true;
                    new_sats.push(c.id);
                }
            }
            // an angry nova's fuse burns down, then it goes wild
            if c.angry && !c.crazy {
                c.fuse -= dt;
                if c.fuse <= 0.0 {
                    c.crazy = true;
                    wild.push(c.pos);
                }
            }

            // Planets can't steer freely — they bend along curves, and the bigger
            // they get, the harder those curves bend toward the player.
            let speed_target = if c.crazy {
                500.0 * p.speed
            } else {
                (330.0 - 120.0 * c.growth) * p.speed
            };
            let sp = c.vel.length().max(1.0);
            let new_sp = approach(sp, speed_target, 1.5, dt);
            let mut dir = c.vel / sp;
            c.curve_t -= dt;
            if c.curve_t <= 0.0 {
                c.curve_t = rnd(0.7, 1.8);
                c.curve = rnd(-0.9, 0.9);
            }
            let to_p = (ppos - c.pos).normalize_or_zero();
            let side = dir.perp_dot(to_p); // >0: player is to the left of travel
            let homing = if !palive {
                0.0
            } else if c.crazy {
                3.4
            } else if c.growth < 0.3 {
                0.15
            } else {
                0.3 + (1.1 + 0.04 * self.level.min(12) as f32) * (c.growth - 0.3) / 0.7
            };
            let turn = c.curve * (1.0 - c.growth * 0.6) + homing * side.signum() * side.abs().sqrt();
            dir = rot(dir, turn * dt);
            c.vel = dir * new_sp;
            c.pos += c.vel * dt;

            // bounce off the screen edges (after the planet has fully entered)
            let r = c.radius();
            let entered = c.spawn_t > 0.6;
            if entered {
                if c.pos.x < r {
                    c.pos.x = r;
                    c.vel.x = c.vel.x.abs();
                }
                if c.pos.x > VW - r {
                    c.pos.x = VW - r;
                    c.vel.x = -c.vel.x.abs();
                }
            }
            if c.pos.y < TOP + r * 0.6 {
                c.pos.y = TOP + r * 0.6;
                c.vel.y = c.vel.y.abs();
            }
            if c.pos.y > VH - r * 0.6 {
                c.pos.y = VH - r * 0.6;
                c.vel.y = -c.vel.y.abs();
            }
            if c.crazy || c.angry {
                c.trail.push_front(c.pos);
                if c.trail.len() > 8 {
                    c.trail.pop_back();
                }
            }
        }
        for id in new_sats {
            let n = if self.level >= 5 { 2 } else { 1 };
            for i in 0..n {
                self.spawn_satellite(id, i);
            }
        }
        for pos in wild {
            self.sounds.push((Sfx::Nova, 1.0));
            fx.shake(0.25);
            fx.ring(pos, 420.0, 0.5, 6.0, Color::new(1.0, 0.5, 0.2, 1.0));
            fx.popup(pos + vec2(0.0, -70.0), "NOVA!".into(), Color::new(1.0, 0.55, 0.25, 1.0), 28.0);
        }
    }

    fn update_sats(&mut self, dt: f32, fx: &mut Fx) {
        let ppos = self.player.pos;
        let mut launched = false;
        for i in 0..self.sats.len() {
            let parent_pos = self.sats[i].parent.and_then(|id| self.comets.iter().find(|c| c.id == id && c.alive).map(|c| c.pos));
            let s = &mut self.sats[i];
            s.life += dt;
            if !s.attacking {
                match parent_pos {
                    Some(pp) => {
                        s.orbit_a += s.orbit_speed * dt;
                        let target = pp + from_angle(s.orbit_a) * s.orbit_r;
                        s.pos = if s.life < 0.5 { pp + (target - pp) * (s.life / 0.5) } else { target };
                        s.detach_t -= dt;
                        if s.detach_t <= 0.0 {
                            s.attacking = true;
                            s.parent = None;
                            s.vel = (ppos - s.pos).normalize_or_zero() * 300.0;
                            launched = true;
                        }
                    }
                    None => {
                        s.attacking = true;
                        s.vel = (ppos - s.pos).normalize_or_zero() * 260.0;
                    }
                }
            } else {
                s.pos += s.vel * dt;
                if s.pos.x < -60.0 || s.pos.x > VW + 60.0 || s.pos.y < -60.0 || s.pos.y > VH + 60.0 {
                    s.alive = false;
                }
                if chance(0.5) {
                    fx.add(crate::fx::Particle {
                        pos: s.pos,
                        vel: -s.vel * 0.2 + rnd_dir() * 20.0,
                        life: 0.0,
                        max_life: 0.3,
                        size: 2.0,
                        color: Color::new(1.0, 0.4, 0.3, 1.0),
                        kind: crate::fx::PKind::Spark,
                        drag: 2.0,
                        spin: 0.0,
                        ang: 0.0,
                    });
                }
            }
        }
        if launched {
            self.sfx(Sfx::SatelliteLaunch, 1.0);
        }
    }

    fn update_minis(&mut self, dt: f32) {
        let ppos = self.player.pos;
        let palive = self.player.alive;
        for m in self.minis.iter_mut() {
            m.age += dt;
            let sp = m.vel.length();
            let mut dir = m.vel / sp.max(1.0);
            // left alone for a few seconds, a comet starts chasing you
            let turn = if m.age > 2.5 && m.age < 9.0 && palive {
                let to_p = (ppos - m.pos).normalize_or_zero();
                dir.perp_dot(to_p).signum() * 2.0
            } else {
                m.curve
            };
            dir = rot(dir, turn * dt);
            m.vel = dir * sp;
            m.pos += m.vel * dt;
            m.trail.push_front(m.pos);
            if m.trail.len() > 22 {
                m.trail.pop_back();
            }
            if m.age < 9.0 {
                // bounce inside the arena while hunting
                if m.pos.x < 10.0 && m.vel.x < 0.0 && m.age > 0.8 {
                    m.vel.x = -m.vel.x;
                }
                if m.pos.x > VW - 10.0 && m.vel.x > 0.0 && m.age > 0.8 {
                    m.vel.x = -m.vel.x;
                }
                if m.pos.y < TOP && m.vel.y < 0.0 && m.age > 0.8 {
                    m.vel.y = -m.vel.y;
                }
                if m.pos.y > VH - 10.0 && m.vel.y > 0.0 {
                    m.vel.y = -m.vel.y;
                }
            } else if m.pos.x < -120.0 || m.pos.x > VW + 120.0 || m.pos.y < -120.0 || m.pos.y > VH + 120.0 {
                m.alive = false;
            }
        }
    }

    // ---------------------------------------------------------------- combat

    fn kill_comet(&mut self, i: usize, fx: &mut Fx) {
        if !self.comets[i].alive {
            return;
        }
        let col = self.comet_rim(&self.comets[i]);
        let c = &mut self.comets[i];
        c.alive = false;
        let (pos, growth, crazy, kind) = (c.pos, c.growth, c.crazy, c.kind);
        let base = ((100.0 + growth * 500.0) / 10.0).round() as u64 * 10 * if crazy { 3 } else if kind == Kind::Nova { 2 } else { 1 };
        let size = 0.15 + growth * 0.85;
        fx.explode(pos, size, col, if kind == Kind::Nova { Color::new(1.0, 0.6, 0.2, 1.0) } else { WHITE });
        fx.shake(0.06 + size * 0.3);
        if growth > 0.8 || crazy {
            fx.flash(0.25, col);
            self.sfx(Sfx::ExplodeBig, 1.0);
        } else if growth > 0.45 {
            self.sfx(Sfx::ExplodeMedium, 1.0);
        } else {
            self.sfx(Sfx::ExplodeSmall, 1.0);
        }
        self.register_kill(base, pos + vec2(0.0, -20.0), fx, mix_color(col, WHITE, 0.5));
    }

    fn hit_comet(&mut self, i: usize, from: Vec2, fx: &mut Fx) {
        let level = self.level;
        let c = &mut self.comets[i];
        c.damage += 1.0;
        c.flash = 0.6;
        c.hp_show = 1.2;
        // shots knock planets slightly back "into the screen"
        c.vel.y -= 18.0 / (0.4 + c.growth);
        let mut angered = false;
        if c.kind == Kind::Nova && !c.angry && c.growth >= 0.25 {
            c.angry = true;
            c.fuse = 1.8;
            angered = true;
        }
        let dead = c.damage >= c.max_hp(level);
        let pos = c.pos;
        let r = c.radius();
        self.score += 10;
        fx.sparks(vec2(from.x, pos.y + r * 0.8), vec2(0.0, 1.0), 4, Color::new(1.0, 0.9, 0.6, 1.0), 300.0);
        self.sounds.push((Sfx::Hit, 1.0));
        if angered {
            self.sounds.push((Sfx::Crazy, 1.0));
        }
        if dead {
            self.kill_comet(i, fx);
        }
    }

    fn kill_player(&mut self, fx: &mut Fx) {
        let pl = &mut self.player;
        if !pl.alive || pl.invuln > 0.0 {
            return;
        }
        pl.alive = false;
        pl.respawn_t = 2.2;
        let pos = pl.pos;
        self.combo = 0;
        self.perfect = false;
        fx.explode(pos, 1.0, Color::new(0.3, 0.9, 1.0, 1.0), WHITE);
        fx.shake(1.0);
        fx.flash(0.5, Color::new(1.0, 0.3, 0.4, 1.0));
        self.slowmo_t = 0.6;
        self.sfx(Sfx::PlayerDie, 1.0);
        self.lives = self.lives.saturating_sub(1);
        if self.lives == 0 {
            self.game_over = true;
            self.game_over_t = 0.0;
        }
    }

    fn collide(&mut self, fx: &mut Fx) {
        for bi in 0..self.bullets.len() {
            if self.bullets[bi].life <= 0.0 {
                continue;
            }
            let bp = self.bullets[bi].pos;
            let mut consumed = false;
            // hit the nearest target along the shot (largest y first is automatic since shots move up)
            for ci in 0..self.comets.len() {
                if !self.comets[ci].alive {
                    continue;
                }
                let r = self.comets[ci].radius() + 3.0;
                if self.comets[ci].pos.distance_squared(bp) < r * r {
                    self.hit_comet(ci, bp, fx);
                    consumed = true;
                    break;
                }
            }
            if !consumed {
                for si in 0..self.sats.len() {
                    if self.sats[si].alive && self.sats[si].pos.distance(bp) < 15.0 {
                        self.sats[si].alive = false;
                        let sp = self.sats[si].pos;
                        let v = if self.sats[si].attacking { 200 } else { 100 };
                        fx.explode(sp, 0.12, Color::new(1.0, 0.5, 0.3, 1.0), WHITE);
                        self.sfx(Sfx::ExplodeSmall, 0.7);
                        self.register_kill(v, sp, fx, Color::new(1.0, 0.7, 0.5, 1.0));
                        consumed = true;
                        break;
                    }
                }
            }
            if !consumed {
                for mi in 0..self.minis.len() {
                    if self.minis[mi].alive && self.minis[mi].pos.distance(bp) < 18.0 {
                        self.minis[mi].alive = false;
                        let mp = self.minis[mi].pos;
                        fx.explode(mp, 0.25, Color::new(0.5, 0.9, 1.0, 1.0), WHITE);
                        self.sfx(Sfx::ExplodeSmall, 0.9);
                        self.register_kill(250, mp, fx, Color::new(0.6, 0.95, 1.0, 1.0));
                        consumed = true;
                        break;
                    }
                }
            }
            // shots pass straight through the rockets
            if consumed {
                self.bullets[bi].life = 0.0;
            }
        }

        if !self.player.alive {
            return;
        }
        let pp = self.player.pos;
        for ci in 0..self.comets.len() {
            let c = &self.comets[ci];
            if !c.alive || !c.harmful() {
                continue;
            }
            // forgiving hitbox: just clipping the edge with the ship's nose is survivable
            let r = c.radius() * 0.8 + PLAYER_R;
            if c.pos.distance_squared(pp) < r * r {
                self.kill_player(fx);
                return;
            }
        }
        for si in 0..self.sats.len() {
            if self.sats[si].alive && self.sats[si].attacking && self.sats[si].pos.distance(pp) < 9.0 + PLAYER_R {
                self.kill_player(fx);
                return;
            }
        }
        for mi in 0..self.minis.len() {
            if self.minis[mi].alive && self.minis[mi].pos.distance(pp) < 10.0 + PLAYER_R {
                self.kill_player(fx);
                return;
            }
        }
        for ki in 0..self.capsules.len() {
            if self.capsules[ki].alive && self.capsules[ki].pos.distance(pp) < 22.0 + PLAYER_R {
                self.capsules[ki].alive = false;
                let pos = self.capsules[ki].pos;
                self.add_score(200, fx);
                self.sfx(Sfx::Pickup, 1.0);
                fx.ring(pos, 300.0, 0.4, 4.0, Color::new(1.0, 0.9, 0.4, 1.0));
                fx.popup(pos + vec2(0.0, -24.0), "200".into(), Color::new(1.0, 0.9, 0.4, 1.0), 20.0);
            }
        }
    }

    // ---------------------------------------------------------------- drawing

    fn comet_rim(&self, c: &Comet) -> Color {
        if c.crazy {
            hsv(self.t * 900.0 + c.id as f32 * 47.0 + 60.0, 1.0, 1.0, 1.0)
        } else if c.angry {
            Color::new(1.0, 0.45 + 0.25 * (self.t * 30.0).sin(), 0.15, 1.0)
        } else {
            hsv(self.hue + c.hue_off, 0.85, 1.0, 1.0)
        }
    }

    pub fn draw(&self, g: &Gfx, fx: &Fx, pulse: f32) {
        // far (small) planets first: they are "behind" the ship
        let mut order: Vec<usize> = (0..self.comets.len()).collect();
        order.sort_by(|&a, &b| self.comets[a].growth.partial_cmp(&self.comets[b].growth).unwrap());

        for c in self.capsules.iter() {
            self.draw_capsule(g, c);
        }
        for &i in order.iter() {
            if !self.comets[i].harmful() {
                self.draw_comet(g, &self.comets[i], pulse);
            }
        }
        // bullets fly into the screen: they shrink as they travel up
        gl_use_material(&g.additive);
        for b in self.bullets.iter() {
            let d = ((b.start_y - b.pos.y) / (b.start_y - TOP).max(1.0)).clamp(0.0, 1.0);
            let s = 1.0 - 0.45 * d;
            g.streak(b.pos, -std::f32::consts::FRAC_PI_2, 40.0 * s, 16.0 * s, Color::new(0.3, 0.8, 1.0, 0.9));
            g.streak(b.pos, -std::f32::consts::FRAC_PI_2, 22.0 * s, 6.0 * s, WHITE);
        }
        gl_use_default_material();
        for &i in order.iter() {
            if self.comets[i].harmful() {
                self.draw_comet(g, &self.comets[i], pulse);
            }
        }
        for s in self.sats.iter() {
            let col = if s.attacking { Color::new(1.0, 0.35, 0.25, 1.0) } else { Color::new(0.8, 0.85, 1.0, 1.0) };
            gl_use_material(&g.additive);
            g.glow_at(s.pos, if s.attacking { 30.0 } else { 22.0 }, with_alpha(col, 0.5));
            gl_use_default_material();
            g.planet(2, s.pos, 9.0, self.t * 0.5 + s.orbit_a, mix_color(col, WHITE, 0.4), col, if s.attacking { 0.2 } else { 0.0 }, 0.0, 1.0);
        }
        gl_use_material(&g.additive);
        for m in self.minis.iter() {
            let n = m.trail.len();
            let hunting = m.age > 2.5 && m.age < 9.0;
            for (k, p) in m.trail.iter().enumerate() {
                let f = 1.0 - k as f32 / n as f32;
                let c = if hunting { Color::new(1.0, 0.5 * f + 0.3, 0.6, 0.35 * f) } else { Color::new(0.4, 0.8 * f + 0.2, 1.0, 0.35 * f) };
                g.glow_at(*p, 4.0 + 16.0 * f, c);
            }
            g.glow_at(m.pos, 26.0, if hunting { Color::new(1.0, 0.6, 0.8, 0.9) } else { Color::new(0.6, 0.9, 1.0, 0.9) });
            g.glow_at(m.pos, 10.0, WHITE);
        }
        gl_use_default_material();

        self.draw_player(g);
        fx.draw_particles(g);
    }

    fn draw_comet(&self, g: &Gfx, c: &Comet, pulse: f32) {
        let r = c.radius();
        let near = (c.growth / 0.3).clamp(0.0, 1.0);
        let fog = if c.crazy { 0.0 } else { (1.0 - near) * 0.6 };
        let appear = (c.spawn_t / 0.3).min(1.0);
        let alpha = (0.45 + 0.55 * near) * appear;
        let rim = self.comet_rim(c);
        let tint = if c.crazy {
            hsv(self.t * 900.0 + c.id as f32 * 47.0, 0.5, 1.0, 1.0)
        } else if c.kind == Kind::World {
            // Worlds keep Earth's colours, lightly tinted by the round's palette
            mix_color(WHITE, hsv(self.hue + c.hue_off, 0.5, 1.0, 1.0), 0.25)
        } else {
            mix_color(WHITE, hsv(self.hue + c.hue_off, 0.7, 1.0, 1.0), 0.5)
        };
        let flash = c.flash.max(if c.angry && !c.crazy { (0.15 + 0.25 * (self.t * 24.0).sin()).max(0.0) } else { 0.0 });

        gl_use_material(&g.additive);
        if c.crazy || c.angry {
            for (k, p) in c.trail.iter().enumerate().skip(1) {
                let f = 1.0 - k as f32 / c.trail.len() as f32;
                g.glow_at(*p, r * 1.4, with_alpha(rim, 0.2 * f));
            }
        }
        let halo = if c.angry || c.crazy { 2.8 + pulse * 0.4 } else { 2.0 };
        g.glow_at(c.pos, r * halo, with_alpha(rim, (0.28 + 0.15 * pulse) * alpha));
        if c.grown && !c.crazy {
            for k in 0..8 {
                let a = self.t * 0.5 + k as f32 * std::f32::consts::TAU / 8.0;
                let len = r * (1.6 + 0.25 * (self.t * 3.0 + k as f32).sin());
                g.streak(c.pos + from_angle(a) * len * 0.5, a, len, 12.0, with_alpha(rim, 0.18));
            }
        }
        gl_use_default_material();

        g.planet(c.map, c.pos, r, c.rot, tint, rim, flash, fog, alpha);

        if c.hp_show > 0.0 && c.growth > 0.25 {
            let mh = c.max_hp(self.level);
            let frac = (1.0 - c.damage / mh).clamp(0.0, 1.0);
            let a = c.hp_show.min(1.0) * 0.8;
            let segs = 40;
            let shown = (segs as f32 * frac) as i32;
            for k in 0..shown {
                let a0 = -std::f32::consts::FRAC_PI_2 + k as f32 / segs as f32 * std::f32::consts::TAU;
                let a1 = a0 + std::f32::consts::TAU / segs as f32 * 0.7;
                let rr = r + 8.0;
                draw_line(
                    c.pos.x + a0.cos() * rr,
                    c.pos.y + a0.sin() * rr,
                    c.pos.x + a1.cos() * rr,
                    c.pos.y + a1.sin() * rr,
                    3.0,
                    with_alpha(if c.angry { Color::new(1.0, 0.5, 0.2, 1.0) } else { Color::new(0.6, 1.0, 0.9, 1.0) }, a),
                );
            }
        }
    }

    /// The lost rockets: little white craft falling nose-down from the top — fly over them.
    fn draw_capsule(&self, g: &Gfx, c: &Capsule) {
        let blink = ((c.t * 6.0).sin() > 0.0) as i32 as f32;
        let p = c.pos;
        // pt(along, across): `along` points down the screen (the way the rocket faces)
        let pt = |along: f32, across: f32| vec2(p.x + across, p.y + along);
        gl_use_material(&g.additive);
        g.glow_at(p, 36.0, Color::new(1.0, 0.9, 0.5, 0.25 + 0.15 * blink));
        // exhaust flame trails upward, behind the rocket
        g.streak(pt(-22.0, 0.0), std::f32::consts::FRAC_PI_2, 22.0, 9.0, Color::new(1.0, 0.6, 0.3, 0.8));
        gl_use_default_material();
        let body = Color::new(0.92, 0.95, 1.0, 1.0);
        let fin = Color::new(1.0, 0.45, 0.4, 1.0);
        let (a, b) = (pt(-14.0, 0.0), pt(8.0, 0.0));
        draw_line(a.x, a.y, b.x, b.y, 12.0, body);
        draw_triangle(pt(8.0, -6.0), pt(8.0, 6.0), pt(20.0, 0.0), body); // nose
        draw_triangle(pt(-14.0, -6.0), pt(-6.0, -6.0), pt(-16.0, -14.0), fin); // left fin
        draw_triangle(pt(-14.0, 6.0), pt(-6.0, 6.0), pt(-16.0, 14.0), fin); // right fin
        let w = pt(3.0, 0.0);
        draw_circle(w.x, w.y, 3.0, Color::new(0.3, 0.6, 1.0, 1.0)); // porthole
        if blink > 0.0 {
            gl_use_material(&g.additive);
            g.glow_at(pt(20.0, 0.0), 8.0, WHITE);
            gl_use_default_material();
        }
    }

    /// The ship seen from behind, pointing into the screen.
    pub fn draw_ship(g: &Gfx, pos: Vec2, bank: f32, scale: f32, line: Color, glow: bool) {
        let s = scale;
        // banking: the wing on the side we turn toward dips, the other rises
        let tilt = bank * 7.0 * s;
        let wl = pos + vec2(-28.0 * s, 8.0 * s + tilt);
        let wr = pos + vec2(28.0 * s, 8.0 * s - tilt);
        let nose = pos + vec2(bank * 3.0 * s, -18.0 * s);
        let tail = pos + vec2(0.0, 10.0 * s);
        let body = Color::new(0.06, 0.1, 0.22, 1.0);
        if glow {
            gl_use_material(&g.additive);
            g.glow_at(pos, 40.0 * s, with_alpha(line, 0.3));
            gl_use_default_material();
        }
        // wings
        draw_triangle(wl, nose, tail, body);
        draw_triangle(wr, nose, tail, body);
        let w = 2.2 * s;
        draw_line(wl.x, wl.y, nose.x, nose.y, w, line);
        draw_line(wr.x, wr.y, nose.x, nose.y, w, line);
        draw_line(wl.x, wl.y, tail.x, tail.y, w, line);
        draw_line(wr.x, wr.y, tail.x, tail.y, w, line);
        // tail fin and canopy
        let fin = pos + vec2(bank * 2.0 * s, -26.0 * s);
        draw_line(tail.x, tail.y - 4.0 * s, fin.x, fin.y, 2.6 * s, line);
        draw_circle(pos.x + bank * 2.0 * s, pos.y - 4.0 * s, 4.0 * s, Color::new(0.5, 0.95, 1.0, 1.0));
        // twin engines facing the viewer
        if glow {
            gl_use_material(&g.additive);
            for ex in [-9.0f32, 9.0] {
                let e = pos + vec2(ex * s, 9.0 * s + if ex < 0.0 { tilt * 0.3 } else { -tilt * 0.3 });
                g.glow_at(e, 9.0 * s * rnd(0.85, 1.15), Color::new(0.4, 0.8, 1.0, 0.95));
                g.glow_at(e, 3.5 * s, WHITE);
            }
            gl_use_default_material();
        } else {
            for ex in [-9.0f32, 9.0] {
                draw_circle(pos.x + ex * s, pos.y + 9.0 * s, 3.0 * s, Color::new(0.4, 0.8, 1.0, 1.0));
            }
        }
    }

    fn draw_player(&self, g: &Gfx) {
        let pl = &self.player;
        if !pl.alive {
            return;
        }
        if pl.invuln > 0.0 && (pl.invuln * 14.0).sin() < -0.2 {
            return;
        }
        Self::draw_ship(g, pl.pos, pl.bank, 1.15, Color::new(0.4, 0.95, 1.0, 1.0), true);
    }

    pub fn draw_hud(&self, g: &Gfx, hiscore: u64, pulse: f32) {
        draw_rectangle(0.0, 0.0, VW, 56.0, Color::new(0.0, 0.0, 0.05, 0.55));
        draw_line(0.0, 56.0, VW, 56.0, 1.5, hsv(self.hue, 0.6, 1.0, 0.35 + pulse * 0.2));
        let dim = Color::new(0.7, 0.8, 1.0, 0.7);
        g.text("SCORE", 20.0, 20.0, 12.0, dim, false);
        g.text(&fmt_score(self.score), 20.0, 46.0, 26.0, WHITE, true);
        g.text_center("HI-SCORE", VW * 0.5, 20.0, 12.0, dim, false);
        g.text_center(&fmt_score(hiscore.max(self.score)), VW * 0.5, 46.0, 24.0, Color::new(1.0, 0.85, 0.4, 1.0), true);
        g.text(&format!("ROUND {}", self.level), 290.0, 20.0, 12.0, dim, false);
        g.text(&format!("{} LEFT", self.comets_remaining()), 290.0, 44.0, 18.0, hsv(self.hue, 0.5, 1.0, 1.0), true);
        for i in 0..self.lives.min(8) {
            let x = VW - 34.0 - i as f32 * 40.0;
            Self::draw_ship(g, vec2(x, 32.0), 0.0, 0.55, Color::new(0.4, 0.95, 1.0, 1.0), false);
        }
        if self.combo >= 4 {
            let m = self.mult();
            let c = hsv(self.t * 200.0, 0.6, 1.0, 1.0);
            g.neon_text_center(&format!("COMBO x{}", m), 900.0, 42.0, 22.0 + pulse * 3.0, c, true);
            let w = 140.0 * (self.combo_t / 1.6).clamp(0.0, 1.0);
            draw_rectangle(830.0, 48.0, w, 3.0, c);
        }
    }
}
