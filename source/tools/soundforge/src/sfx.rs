//! Sound effects. Short, punchy, designed to sit on top of the music.

use crate::dsp::*;
use crate::instruments::*;

fn len(s: f32) -> usize {
    (s * SR) as usize
}

fn norm(mut b: Buf, peak: f32) -> Buf {
    let p = b.peak().max(1e-6);
    b.scale(peak / p);
    b
}

pub fn shoot() -> Buf {
    let n = len(0.09);
    let mut b = Buf::new(n);
    let mut o = Osc::new(0.0);
    let mut o2 = Osc::new(0.5);
    let mut f = Svf::default();
    for i in 0..n {
        let t = i as f32 / SR;
        let fr = 1900.0 * (-t * 22.0).exp() + 380.0;
        let env = (-t * 38.0).exp();
        let x = o.square(fr, 0.3) * 0.6 + o2.saw(fr * 1.5) * 0.3;
        let v = f.lp(x, 6000.0, 0.8) * env;
        b.add_mono(i, v, 0.0);
    }
    norm(b, 0.6)
}

pub fn hit() -> Buf {
    let n = len(0.06);
    let mut b = Buf::new(n);
    let mut r = Rng(77);
    let mut f = Svf::default();
    let mut o = Osc::new(0.0);
    for i in 0..n {
        let t = i as f32 / SR;
        let env = (-t * 70.0).exp();
        let x = f.bp(r.bi(), 2500.0, 1.5) * 0.7 + o.square(900.0 - t * 6000.0, 0.5) * 0.3;
        b.add_mono(i, x * env, 0.0);
    }
    norm(b, 0.55)
}

pub fn explode(size: f32, seed: u64) -> Buf {
    let dur = 0.35 + size * 1.1;
    let n = len(dur + 0.6);
    let mut dry = Buf::new(n);
    let mut r = Rng(seed);
    let mut fl = Svf::default();
    let mut fr = Svf::default();
    let mut ph = 0.0f32;
    let mut crush_hold = 0.0f32;
    for i in 0..len(dur) {
        let t = i as f32 / SR;
        let x = t / dur;
        let env = (1.0 - x).powf(1.6) * (1.0 - (-t * 600.0).exp());
        let cut = 300.0 + 7000.0 * (-t * (9.0 - size * 5.0)).exp();
        // bitcrushed noise for a retro crunch
        if i % (2 + (size * 6.0) as usize) == 0 {
            crush_hold = r.bi();
        }
        let l = fl.lp(crush_hold * 0.6 + r.bi() * 0.4, cut, 1.1);
        let rr = fr.lp(crush_hold * 0.6 + r.bi() * 0.4, cut * 0.92, 1.1);
        let f = 35.0 + (90.0 + 60.0 * size) * (-t * 10.0).exp();
        ph += f / SR;
        let boom = (ph * TAU).sin() * (-t * (6.0 - size * 3.5)).exp() * (0.5 + size);
        dry.add(i, soft_clip(l * env * 1.4 + boom, 1.5), soft_clip(rr * env * 1.4 + boom, 1.5));
    }
    let wet = reverb(&dry, 0.6, 0.4, 0);
    dry.mix(&wet, 0.5 + size * 0.5);
    norm(dry, 0.9)
}

pub fn crazy_alarm() -> Buf {
    let n = len(0.55);
    let mut b = Buf::new(n);
    let mut o = Osc::new(0.0);
    for i in 0..n {
        let t = i as f32 / SR;
        let step = ((t * 16.0) as i32) % 2;
        let fr = if step == 0 { 880.0 } else { 1175.0 } * (1.0 + t * 0.8);
        let env = adsr(t, 0.45, 0.005, 0.1, 0.8, 0.1);
        let v = o.square(fr, 0.25) * env * 0.5;
        b.add_mono(i, v, (t * 20.0).sin() * 0.5);
    }
    norm(b, 0.5)
}

pub fn nova() -> Buf {
    let n = len(1.6);
    let mut b = Buf::new(n);
    let mut r = Rng(9);
    let mut f = Svf::default();
    let mut o = Osc::new(0.0);
    let mut o2 = Osc::new(0.3);
    for i in 0..len(1.2) {
        let t = i as f32 / SR;
        let x = t / 1.2;
        let cut = 200.0 * (30.0f32).powf(x);
        let env = (x * 3.0).min(1.0) * (1.0 - x);
        let fr = mtof(36.0 + 24.0 * x);
        let v = f.bp(r.bi(), cut, 3.0) * 0.8 + (o.saw(fr) + o2.saw(fr * 1.01)) * 0.2;
        b.add_mono(i, v * env, (t * 4.0).sin() * 0.7);
    }
    let w = reverb(&b, 0.7, 0.3, 0);
    b.mix(&w, 0.6);
    norm(b, 0.7)
}

fn arpeggio(notes: &[f32], step: f32, wave_sq: bool) -> Buf {
    let total = step * notes.len() as f32 + 0.5;
    let mut b = Buf::new(len(total));
    for (k, &n) in notes.iter().enumerate() {
        let pos = len(step * k as f32);
        let mut o = Osc::new(0.0);
        let mut f = Svf::default();
        for i in 0..len(step * 2.2) {
            let t = i as f32 / SR;
            let env = adsr(t, step * 1.3, 0.002, 0.05, 0.6, 0.15);
            let fr = mtof(n);
            let x = if wave_sq { o.square(fr, 0.25) } else { o.saw(fr) };
            b.add_mono(pos + i, f.lp(x, 7000.0, 0.7) * env * 0.5, 0.0);
        }
    }
    let w = ping_pong(&b, 0.09, 0.35, 6000.0, 0);
    b.mix(&w, 0.4);
    norm(b, 0.6)
}

pub fn pickup() -> Buf {
    arpeggio(&[77.0, 81.0, 84.0, 89.0], 0.045, true)
}
pub fn extra_life() -> Buf {
    arpeggio(&[65.0, 69.0, 72.0, 77.0, 81.0, 84.0, 89.0], 0.06, true)
}
pub fn level_clear() -> Buf {
    let mut b = arpeggio(&[65.0, 68.0, 72.0, 77.0, 80.0, 84.0], 0.07, false);
    let mut chord = Buf::new(len(1.6));
    for &n in &[77.0f32, 80.0, 84.0] {
        supersaw(&mut chord, len(0.42), 0.7, n, 0.6, 1.0, n as u64);
    }
    let mut out = Buf::new(b.len().max(chord.len()));
    out.mix(&b, 1.0);
    out.mix(&chord, 0.8);
    b = out;
    let w = reverb(&b, 0.7, 0.3, 0);
    b.mix(&w, 0.5);
    norm(b, 0.75)
}

pub fn player_die() -> Buf {
    let n = len(2.2);
    let mut b = Buf::new(n);
    let mut o = Osc::new(0.0);
    let mut r = Rng(31);
    let mut f = Svf::default();
    for i in 0..len(1.5) {
        let t = i as f32 / SR;
        let x = t / 1.5;
        let fr = mtof(84.0 - 60.0 * x.sqrt());
        let env = (1.0 - x).powi(2);
        let v = o.square(fr, 0.5) * 0.35 + f.lp(r.bi(), 4000.0 * (1.0 - x) + 200.0, 1.0) * 0.6;
        b.add_mono(i, v * env, 0.0);
    }
    impact(&mut b, 0, 0.8);
    let w = reverb(&b, 0.75, 0.35, 0);
    b.mix(&w, 0.6);
    norm(b, 0.9)
}

pub fn bomb() -> Buf {
    let n = len(2.6);
    let mut b = Buf::new(n);
    riser(&mut b, 0, 0.35, 1.0);
    impact(&mut b, len(0.3), 1.0);
    crash(&mut b, len(0.3), 1.0);
    downlifter(&mut b, len(0.3), 1.6, 0.8);
    let w = reverb(&b, 0.8, 0.3, 0);
    b.mix(&w, 0.6);
    norm(b, 0.95)
}

pub fn ui_select() -> Buf {
    arpeggio(&[84.0, 91.0], 0.035, true)
}

pub fn satellite_launch() -> Buf {
    let n = len(0.3);
    let mut b = Buf::new(n);
    let mut o = Osc::new(0.0);
    for i in 0..n {
        let t = i as f32 / SR;
        let fr = 300.0 + 2500.0 * (t / 0.3);
        let env = (1.0 - t / 0.3).max(0.0);
        b.add_mono(i, o.tri(fr) * env * 0.5, 0.0);
    }
    norm(b, 0.45)
}

pub fn capsule_spawn() -> Buf {
    arpeggio(&[96.0, 91.0, 96.0, 103.0], 0.03, false)
}

/// Hyperspace jump: ~2.5 s rising engine whine + noise sweep, then a deep
/// "drop out of warp" boom with a shimmering tail.
pub fn warp() -> Buf {
    let rise = 2.5f32;
    let n = len(rise + 2.4);
    let mut b = Buf::new(n);
    let mut o1 = Osc::new(0.0);
    let mut o2 = Osc::new(0.37);
    let mut o3 = Osc::new(0.71);
    let mut fl = Svf::default();
    let mut fr = Svf::default();
    let mut nf = Svf::default();
    let mut r = Rng(0x3A2F);
    for i in 0..len(rise) {
        let t = i as f32 / SR;
        let x = t / rise;
        let curve = x * x * (3.0 - 2.0 * x); // smoothstep
        let f = mtof(33.0 + 52.0 * curve);
        let amp = (x * 1.2).min(1.0).powf(1.5) * (1.0 - ((x - 0.97) / 0.03).clamp(0.0, 1.0));
        // detuned saws through a rising resonant filter = engine spool-up
        let tone = o1.saw(f) * 0.4 + o2.saw(f * 1.012) * 0.4 + o3.square(f * 0.5, 0.5) * 0.3;
        let cut = 200.0 + 7000.0 * curve;
        let wob = (t * (4.0 + 18.0 * x) * TAU).sin() * 0.15 * x; // accelerating warble
        let tl = fl.lp(tone, cut * (1.0 + wob), 3.0);
        let tr = fr.lp(tone, cut * (1.0 - wob), 3.0);
        // rushing air: band-passed noise sweeping upward
        let air = nf.bp(r.bi(), 400.0 + 9000.0 * x * x, 2.0) * x * 0.9;
        let pan = (t * (1.0 + 6.0 * x) * TAU).sin() * 0.5 * x;
        b.add(i, (tl * 0.55 + air * (0.5 - pan * 0.3)) * amp, (tr * 0.55 + air * (0.5 + pan * 0.3)) * amp);
    }
    let drop = len(rise - 0.05);
    impact(&mut b, drop, 1.0);
    crash(&mut b, drop, 0.8);
    downlifter(&mut b, drop, 1.8, 0.7);
    let w = reverb(&b, 0.85, 0.3, 0);
    b.mix(&w, 0.55);
    norm(b, 0.95)
}

/// Fire warp: a deeper, roaring, distorted spool-up with crackle, ending in a fiery boom.
pub fn warp_fire() -> Buf {
    let rise = 2.5f32;
    let n = len(rise + 2.6);
    let mut b = Buf::new(n);
    let mut o1 = Osc::new(0.0);
    let mut o2 = Osc::new(0.5);
    let mut fl = Svf::default();
    let mut fr = Svf::default();
    let mut nf = Svf::default();
    let mut r = Rng(0xF14E);
    for i in 0..len(rise) {
        let t = i as f32 / SR;
        let x = t / rise;
        let curve = x * x * (3.0 - 2.0 * x);
        let f = mtof(24.0 + 40.0 * curve);
        let amp = (x * 1.2).min(1.0).powf(1.3) * (1.0 - ((x - 0.97) / 0.03).clamp(0.0, 1.0));
        let tone = o1.saw(f) * 0.5 + o2.saw(f * 1.5 * 1.01) * 0.35;
        let cut = 150.0 + 4000.0 * curve;
        let roar = fl.lp(tone, cut, 2.0);
        // fire: crackling low-passed noise bursts
        let crack = if r.uni() < 0.004 + 0.02 * x { r.bi() * 3.0 } else { 0.0 };
        let fire = nf.bp(r.bi() * 0.6 + crack, 600.0 + 3500.0 * x, 1.2) * (0.3 + 0.9 * x);
        let v = soft_clip(roar * 2.2 + fire, 2.0);
        let wob = (t * (3.0 + 10.0 * x) * TAU).sin() * 0.4 * x;
        let rr = fr.lp(v, 9000.0, 0.7);
        b.add(i, rr * amp * (1.0 - wob * 0.3), rr * amp * (1.0 + wob * 0.3));
    }
    let drop = len(rise - 0.05);
    impact(&mut b, drop, 1.0);
    impact(&mut b, drop + len(0.03), 0.6);
    crash(&mut b, drop, 0.9);
    downlifter(&mut b, drop, 2.0, 0.6);
    let w = reverb(&b, 0.85, 0.45, 0);
    b.mix(&w, 0.5);
    norm(b, 0.95)
}

/// Reverse warp: the normal warp played backwards — a swelling suck-in that slams
/// at the drop point, then a falling whine as we're pulled back home.
pub fn warp_reverse() -> Buf {
    let mut b = warp();
    b.l.reverse();
    b.r.reverse();
    // fade the very start in so the reversed reverb tail doesn't click
    let fade_in = len(0.4);
    for i in 0..fade_in.min(b.len()) {
        let g = i as f32 / fade_in as f32;
        b.l[i] *= g;
        b.r[i] *= g;
    }
    norm(b, 0.95)
}
