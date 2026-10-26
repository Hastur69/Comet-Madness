//! Instruments: each renders one event into a stereo buffer at a sample position.

use crate::dsp::*;

fn secs(n: f32) -> usize {
    (n * SR) as usize
}

// ------------------------------------------------------------------ drums

/// Punchy EDM kick: pitch-swept sine + click + saturation.
pub fn kick(buf: &mut Buf, pos: usize, vel: f32, hard: bool) {
    let len = secs(if hard { 0.55 } else { 0.42 });
    let mut ph = 0.0f32;
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ pos as u64);
    let mut f_click = Svf::default();
    for i in 0..len {
        let t = i as f32 / SR;
        let f = 42.0 + (if hard { 190.0 } else { 140.0 }) * (-t * 32.0).exp() + 40.0 * (-t * 9.0).exp();
        ph += f / SR;
        let body = (ph * TAU).sin();
        let amp = (-t * if hard { 5.5 } else { 7.5 }).exp() * (1.0 - (-t * 900.0).exp());
        let click = f_click.bp(rng.bi(), 3500.0, 0.9) * (-t * 260.0).exp() * 0.9;
        let v = soft_clip(body * amp * if hard { 2.6 } else { 1.8 }, 1.6) + click;
        buf.add_mono(pos + i, v * vel, 0.0);
    }
}

/// Snare: tonal body + bright noise. `big` adds a long tail for dubstep halftime.
pub fn snare(buf: &mut Buf, pos: usize, vel: f32, big: bool) {
    let len = secs(if big { 0.6 } else { 0.3 });
    let mut rng = Rng(0xA5A5_1234 ^ (pos as u64).wrapping_mul(31));
    let mut o1 = Osc::new(0.0);
    let mut o2 = Osc::new(0.3);
    let mut hp = Svf::default();
    let mut bp = Svf::default();
    for i in 0..len {
        let t = i as f32 / SR;
        let tone = (o1.sine(190.0 + 60.0 * (-t * 40.0).exp()) * 0.6 + o2.tri(330.0) * 0.25) * (-t * 22.0).exp();
        let n = rng.bi();
        let decay = if big { 7.5 } else { 16.0 };
        let noise = (hp.hp(n, 900.0, 0.7) * 0.7 + bp.bp(n, 3800.0, 1.2) * 0.6) * (-t * decay).exp();
        let v = soft_clip((tone + noise) * 1.4, 1.3);
        buf.add_mono(pos + i, v * vel, 0.0);
    }
}

/// Classic layered clap.
pub fn clap(buf: &mut Buf, pos: usize, vel: f32) {
    let len = secs(0.35);
    let mut rng = Rng(0xC1A9 ^ (pos as u64) << 3);
    let mut bp = Svf::default();
    for i in 0..len {
        let t = i as f32 / SR;
        let bursts = [0.0f32, 0.011, 0.022, 0.031];
        let mut env = 0.0f32;
        for &b in bursts.iter() {
            if t >= b {
                env = env.max((-(t - b) * 180.0).exp());
            }
        }
        env = env.max(if t > 0.031 { (-(t - 0.031) * 13.0).exp() * 0.6 } else { 0.0 });
        let v = bp.bp(rng.bi(), 1350.0, 1.6) * env * 2.2;
        buf.add(pos + i, v * vel * 0.9, v * vel);
    }
}

pub fn hat(buf: &mut Buf, pos: usize, vel: f32, open: bool, pan: f32) {
    let len = secs(if open { 0.32 } else { 0.06 });
    let mut rng = Rng(0x4A7 ^ (pos as u64) << 5);
    let mut hp = Svf::default();
    // metallic partials
    let mut oscs = [Osc::new(0.1), Osc::new(0.5), Osc::new(0.7), Osc::new(0.2)];
    let fr = [3527.0f32, 4731.0, 6153.0, 8211.0];
    for i in 0..len {
        let t = i as f32 / SR;
        let mut metal = 0.0;
        for (o, f) in oscs.iter_mut().zip(fr.iter()) {
            metal += o.square(*f, 0.5);
        }
        let x = rng.bi() * 0.7 + metal * 0.08;
        let env = (-t * if open { 11.0 } else { 70.0 }).exp();
        let v = hp.hp(x, 7500.0, 0.8) * env;
        buf.add_mono(pos + i, v * vel * 0.55, pan);
    }
}

pub fn crash(buf: &mut Buf, pos: usize, vel: f32) {
    let len = secs(2.4);
    let mut rng = Rng(0xC4A5 ^ (pos as u64));
    let mut hpl = Svf::default();
    let mut hpr = Svf::default();
    for i in 0..len {
        let t = i as f32 / SR;
        let env = (-t * 1.9).exp() * (1.0 - (-t * 400.0).exp());
        let l = hpl.hp(rng.bi(), 4200.0, 0.6) * env;
        let r = hpr.hp(rng.bi(), 4200.0, 0.6) * env;
        buf.add(pos + i, l * vel * 0.45, r * vel * 0.45);
    }
}

/// Deep cinematic impact for drops.
pub fn impact(buf: &mut Buf, pos: usize, vel: f32) {
    let len = secs(2.5);
    let mut ph = 0.0f32;
    let mut rng = Rng(0x1111 ^ pos as u64);
    let mut lp = Svf::default();
    for i in 0..len {
        let t = i as f32 / SR;
        let f = 30.0 + 70.0 * (-t * 6.0).exp();
        ph += f / SR;
        let boom = (ph * TAU).sin() * (-t * 1.6).exp();
        let rumble = lp.lp(rng.bi(), 300.0 + 3000.0 * (-t * 5.0).exp(), 0.8) * (-t * 2.5).exp();
        let v = soft_clip(boom * 1.6 + rumble * 0.8, 1.5);
        buf.add_mono(pos + i, v * vel, 0.0);
    }
}

/// White-noise riser with sweeping band-pass + rising detuned saw.
pub fn riser(buf: &mut Buf, pos: usize, dur: f32, vel: f32) {
    let len = secs(dur);
    let mut rng = Rng(0x5EED ^ pos as u64);
    let mut fl = Svf::default();
    let mut fr = Svf::default();
    let mut o1 = Osc::new(0.0);
    let mut o2 = Osc::new(0.5);
    for i in 0..len {
        let x = i as f32 / len as f32;
        let cut = 250.0 * (40.0f32).powf(x);
        let amp = x * x * vel;
        let f = mtof(48.0 + 30.0 * x * x);
        let saw = (o1.saw(f * 1.005) + o2.saw(f * 0.995)) * 0.12 * x;
        let l = fl.bp(rng.bi(), cut, 2.5) * 0.8 + saw;
        let r = fr.bp(rng.bi(), cut * 1.03, 2.5) * 0.8 + saw;
        buf.add(pos + i, l * amp, r * amp);
    }
}

/// Reverse-ish downlifter after a drop.
pub fn downlifter(buf: &mut Buf, pos: usize, dur: f32, vel: f32) {
    let len = secs(dur);
    let mut rng = Rng(0xD0D0 ^ pos as u64);
    let mut f = Svf::default();
    for i in 0..len {
        let x = i as f32 / len as f32;
        let cut = 9000.0 * (0.02f32).powf(x);
        let amp = (1.0 - x).powi(2) * vel;
        let v = f.bp(rng.bi(), cut, 1.5) * amp;
        buf.add_mono(pos + i, v, (x * 12.0).sin() * 0.6);
    }
}

// ------------------------------------------------------------------ synths

/// 7-voice supersaw lead with filter envelope.
pub fn supersaw(buf: &mut Buf, pos: usize, dur: f32, note: f32, vel: f32, bright: f32, seed: u64) {
    let rel = 0.18;
    let len = secs(dur + rel);
    let detune = [-0.19f32, -0.11, -0.045, 0.0, 0.05, 0.12, 0.2];
    let pans = [-0.9f32, -0.6, -0.25, 0.0, 0.3, 0.65, 0.95];
    let mut rng = Rng(0xABCD ^ seed);
    let mut oscs: Vec<Osc> = (0..7).map(|_| Osc::new(rng.uni())).collect();
    let mut sub = Osc::new(0.0);
    let mut fl = Svf::default();
    let mut fr = Svf::default();
    let base = mtof(note);
    for i in 0..len {
        let t = i as f32 / SR;
        let env = adsr(t, dur, 0.004, 0.25, 0.75, rel);
        let mut l = 0.0;
        let mut r = 0.0;
        for k in 0..7 {
            let f = base * 2f32.powf(detune[k] / 12.0);
            let s = oscs[k].saw(f);
            let a = (pans[k] + 1.0) * 0.25 * std::f32::consts::PI;
            l += s * a.cos();
            r += s * a.sin();
        }
        let s = sub.square(base * 0.5, 0.5) * 0.25;
        l += s;
        r += s;
        let cut = 900.0 + bright * 9000.0 * (0.35 + 0.65 * (-t * 5.0).exp());
        let l = fl.lp(l * 0.22, cut, 0.9);
        let r = fr.lp(r * 0.22, cut, 0.9);
        buf.add(pos + i, l * env * vel, r * env * vel);
    }
}

/// Bright pluck for arps.
pub fn pluck(buf: &mut Buf, pos: usize, dur: f32, note: f32, vel: f32, pan: f32) {
    let len = secs(dur + 0.25);
    let mut o1 = Osc::new(0.0);
    let mut o2 = Osc::new(0.33);
    let mut f = Svf::default();
    let fr = mtof(note);
    for i in 0..len {
        let t = i as f32 / SR;
        let env = adsr(t, dur, 0.002, 0.12, 0.25, 0.22);
        let x = o1.saw(fr * 1.003) * 0.5 + o2.square(fr * 0.997, 0.3) * 0.4;
        let cut = 500.0 + 6500.0 * (-t * 18.0).exp();
        let v = f.lp(x, cut, 2.2) * env * vel;
        buf.add_mono(pos + i, v * 0.5, pan);
    }
}

/// Wide pad chord (slow attack supersaw, dark filter).
pub fn pad(buf: &mut Buf, pos: usize, dur: f32, notes: &[f32], vel: f32, cut: f32) {
    let rel = 0.9;
    let len = secs(dur + rel);
    for (ni, &n) in notes.iter().enumerate() {
        let mut rng = Rng(0x9AD ^ (ni as u64 * 977) ^ pos as u64);
        let mut oscs: Vec<Osc> = (0..4).map(|_| Osc::new(rng.uni())).collect();
        let det = [-0.13f32, -0.04, 0.05, 0.14];
        let mut fl = Svf::default();
        let mut fr = Svf::default();
        let base = mtof(n);
        for i in 0..len {
            let t = i as f32 / SR;
            let env = adsr(t, dur, 0.35, 0.4, 0.85, rel);
            let lfo = (t * 0.7 + ni as f32).sin() * 0.15;
            let mut l = 0.0;
            let mut r = 0.0;
            for k in 0..4 {
                let s = oscs[k].saw(base * 2f32.powf(det[k] / 12.0));
                if k % 2 == 0 {
                    l += s;
                    r += s * 0.4;
                } else {
                    r += s;
                    l += s * 0.4;
                }
            }
            let c = cut * (1.0 + lfo);
            let l = fl.lp(l * 0.12, c, 0.8);
            let r = fr.lp(r * 0.12, c, 0.8);
            buf.add(pos + i, l * env * vel, r * env * vel);
        }
    }
}

/// Electro/house bass: saw + square an octave down, filter env, clean sub.
pub fn house_bass(buf: &mut Buf, pos: usize, dur: f32, note: f32, vel: f32) {
    let len = secs(dur + 0.05);
    let mut o1 = Osc::new(0.0);
    let mut o2 = Osc::new(0.5);
    let mut sub = Osc::new(0.0);
    let mut f = Svf::default();
    let fr = mtof(note);
    for i in 0..len {
        let t = i as f32 / SR;
        let env = adsr(t, dur, 0.003, 0.1, 0.8, 0.04);
        let x = o1.saw(fr) * 0.6 + o2.square(fr * 1.002, 0.5) * 0.4;
        let cut = 220.0 + 2600.0 * (-t * 14.0).exp();
        let mid = soft_clip(f.lp(x, cut, 1.8) * 1.5, 2.0);
        let s = sub.sine(fr * 0.5) * 0.9;
        buf.add_mono(pos + i, (mid * 0.55 + s) * env * vel, 0.0);
    }
}

/// LFO shapes for the wobble bass.
#[derive(Clone, Copy)]
pub enum Lfo {
    /// cycles per beat
    Sine(f32),
    /// saw-down "wub" (cycles per beat)
    Saw(f32),
    /// stepped random (gates per beat)
    Step(f32),
    /// fixed open filter value 0..1
    Hold(f32),
}

/// Dubstep growl/wobble bass. FM + dual saw through a resonant LFO-driven
/// filter, hard-saturated; with a separate clean sub for the low end.
pub fn wobble(
    buf: &mut Buf,
    pos: usize,
    dur: f32,
    note: f32,
    glide_from: Option<f32>,
    vel: f32,
    lfo: Lfo,
    bpm: f32,
    growl: f32,
    seed: u64,
) {
    let len = secs(dur + 0.02);
    let beat = 60.0 / bpm;
    let mut c1 = Osc::new(0.0);
    let mut c2 = Osc::new(0.37);
    let mut c3 = Osc::new(0.11);
    let mut m = Osc::new(0.0);
    let mut sub = Osc::new(0.0);
    let mut f1 = Svf::default();
    let mut f2 = Svf::default();
    let mut hp = Svf::default();
    let mut sublp = Svf::default();
    let mut rng = Rng(0xB055 ^ seed);
    let mut step_val = rng.uni();
    let mut last_step = -1i64;
    for i in 0..len {
        let t = i as f32 / SR;
        let n = match glide_from {
            Some(g) => {
                let k = (t / 0.09).min(1.0);
                g + (note - g) * (1.0 - (1.0 - k).powi(3))
            }
            None => note,
        };
        let fr = mtof(n);
        let l = match lfo {
            Lfo::Sine(rate) => 0.5 - 0.5 * (t / beat * rate * TAU).cos(),
            Lfo::Saw(rate) => 1.0 - ((t / beat * rate) % 1.0),
            Lfo::Step(rate) => {
                let s = (t / beat * rate) as i64;
                if s != last_step {
                    last_step = s;
                    step_val = 0.15 + 0.85 * rng.uni();
                }
                step_val
            }
            Lfo::Hold(v) => v,
        };
        // FM growl: modulator at half the pitch, index swept by the LFO
        let idx = growl * (0.4 + 2.6 * l);
        let frm = fr * 2.0;
        let mv = m.sine(frm * 0.5) * idx;
        c1.phase = (c1.phase + mv * 0.004).rem_euclid(1.0);
        let x = c1.saw(frm) * 0.55 + c2.saw(frm * 1.007) * 0.45 + c3.square(fr, 0.5) * 0.3;
        let cut = 90.0 + 4200.0 * l * l;
        let a = f1.run(x, cut, 5.5);
        let b = f2.run(a.lp, cut * 1.8, 3.0);
        let raw = a.lp * 0.8 + b.bp * 0.6 + a.bp * 0.3 * l;
        let dist = soft_clip(raw * 3.5, 2.5);
        let mid = hp.hp(dist, 110.0, 0.7);
        let env = adsr(t, dur, 0.004, 0.05, 1.0, 0.02);
        let s = sublp.lp(sub.sine(fr), 120.0, 0.7) * 0.95;
        let v = (mid * 0.62 + s * 0.85) * env * vel;
        // slight stereo widening on the mids
        let w = mid * 0.08 * env * vel;
        buf.add(pos + i, v + w, v - w);
    }
}

/// Square-wave "SID-style" lead with vibrato — a nod to the C64 roots.
pub fn chip_lead(buf: &mut Buf, pos: usize, dur: f32, note: f32, vel: f32, pan: f32) {
    let len = secs(dur + 0.1);
    let mut o = Osc::new(0.0);
    let mut f = Svf::default();
    for i in 0..len {
        let t = i as f32 / SR;
        let vib = if t > 0.12 { (t * 6.0 * TAU).sin() * 0.18 } else { 0.0 };
        let fr = mtof(note + vib);
        let pw = 0.25 + 0.2 * (t * 2.3).sin();
        let env = adsr(t, dur, 0.003, 0.08, 0.7, 0.08);
        let v = f.lp(o.square(fr, pw), 5000.0, 0.7) * env * vel * 0.35;
        buf.add_mono(pos + i, v, pan);
    }
}
