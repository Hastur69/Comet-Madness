//! Core DSP building blocks: buffers, oscillators, filters, envelopes, effects.

use std::f32::consts::PI;

pub const SR: f32 = 44100.0;
pub const TAU: f32 = PI * 2.0;

/// Stereo floating point buffer.
#[derive(Clone)]
pub struct Buf {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
}

impl Buf {
    pub fn new(len: usize) -> Self {
        Buf { l: vec![0.0; len], r: vec![0.0; len] }
    }
    pub fn len(&self) -> usize {
        self.l.len()
    }
    #[inline]
    pub fn add(&mut self, i: usize, l: f32, r: f32) {
        if i < self.l.len() {
            self.l[i] += l;
            self.r[i] += r;
        }
    }
    #[inline]
    pub fn add_mono(&mut self, i: usize, v: f32, pan: f32) {
        // pan -1..1 constant power
        let a = (pan.clamp(-1.0, 1.0) + 1.0) * 0.25 * PI;
        self.add(i, v * a.cos(), v * a.sin());
    }
    pub fn mix(&mut self, other: &Buf, gain: f32) {
        let n = self.len().min(other.len());
        for i in 0..n {
            self.l[i] += other.l[i] * gain;
            self.r[i] += other.r[i] * gain;
        }
    }
    pub fn scale(&mut self, g: f32) {
        for i in 0..self.len() {
            self.l[i] *= g;
            self.r[i] *= g;
        }
    }
    pub fn peak(&self) -> f32 {
        self.l.iter().chain(self.r.iter()).fold(0.0f32, |m, v| m.max(v.abs()))
    }
    /// Fold the region past `len` back onto the start (seamless loop tails), then truncate.
    pub fn fold_loop(&mut self, len: usize) {
        let total = self.len();
        if total > len {
            for i in len..total {
                let j = (i - len) % len;
                self.l[j] += self.l[i];
                self.r[j] += self.r[i];
            }
        }
        self.l.truncate(len);
        self.r.truncate(len);
    }
    pub fn gain_curve(&mut self, f: impl Fn(usize) -> f32) {
        for i in 0..self.len() {
            let g = f(i);
            self.l[i] *= g;
            self.r[i] *= g;
        }
    }
}

/// xorshift PRNG — deterministic so the soundtrack renders identically every time.
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }
    /// uniform in [-1, 1]
    pub fn bi(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    /// uniform in [0, 1]
    pub fn uni(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }
}

pub fn mtof(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

#[inline]
fn poly_blep(mut t: f32, dt: f32) -> f32 {
    if t < dt {
        t /= dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// Band-limited (polyBLEP) oscillator.
#[derive(Clone, Copy)]
pub struct Osc {
    pub phase: f32,
}
impl Osc {
    pub fn new(phase: f32) -> Self {
        Osc { phase }
    }
    #[inline]
    pub fn saw(&mut self, freq: f32) -> f32 {
        let dt = (freq / SR).min(0.49);
        let t = self.phase;
        let v = 2.0 * t - 1.0 - poly_blep(t, dt);
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        v
    }
    #[inline]
    pub fn square(&mut self, freq: f32, pw: f32) -> f32 {
        let dt = (freq / SR).min(0.49);
        let t = self.phase;
        let mut v = if t < pw { 1.0 } else { -1.0 };
        v += poly_blep(t, dt);
        let t2 = (t + 1.0 - pw) % 1.0;
        v -= poly_blep(t2, dt);
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        v
    }
    #[inline]
    pub fn sine(&mut self, freq: f32) -> f32 {
        let v = (self.phase * TAU).sin();
        self.phase += freq / SR;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        v
    }
    #[inline]
    pub fn tri(&mut self, freq: f32) -> f32 {
        let t = self.phase;
        let v = if t < 0.5 { 4.0 * t - 1.0 } else { 3.0 - 4.0 * t };
        self.phase += freq / SR;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        v
    }
}

/// Zavalishin TPT state-variable filter (stable under fast modulation).
#[derive(Clone, Copy, Default)]
pub struct Svf {
    ic1: f32,
    ic2: f32,
}
pub struct SvfOut {
    pub lp: f32,
    pub bp: f32,
    pub hp: f32,
}
impl Svf {
    #[inline]
    pub fn run(&mut self, x: f32, cutoff: f32, q: f32) -> SvfOut {
        let fc = cutoff.clamp(20.0, SR * 0.45);
        let g = (PI * fc / SR).tan();
        let k = 1.0 / q.max(0.05);
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;
        let v3 = x - self.ic2;
        let v1 = a1 * self.ic1 + a2 * v3;
        let v2 = self.ic2 + a2 * self.ic1 + a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        SvfOut { lp: v2, bp: v1, hp: x - k * v1 - v2 }
    }
    pub fn lp(&mut self, x: f32, c: f32, q: f32) -> f32 {
        self.run(x, c, q).lp
    }
    pub fn hp(&mut self, x: f32, c: f32, q: f32) -> f32 {
        self.run(x, c, q).hp
    }
    pub fn bp(&mut self, x: f32, c: f32, q: f32) -> f32 {
        self.run(x, c, q).bp
    }
}

/// Linear-attack, exponential-ish ADSR evaluated at time `t` for a note of length `dur`.
pub fn adsr(t: f32, dur: f32, a: f32, d: f32, s: f32, r: f32) -> f32 {
    let held = if t < a {
        t / a.max(1e-5)
    } else if t < a + d {
        1.0 - (1.0 - s) * ((t - a) / d.max(1e-5))
    } else {
        s
    };
    if t <= dur {
        held
    } else {
        let at_release = if dur < a {
            dur / a.max(1e-5)
        } else if dur < a + d {
            1.0 - (1.0 - s) * ((dur - a) / d.max(1e-5))
        } else {
            s
        };
        let rt = (t - dur) / r.max(1e-5);
        if rt >= 1.0 {
            0.0
        } else {
            at_release * (1.0 - rt) * (1.0 - rt)
        }
    }
}

pub fn soft_clip(x: f32, drive: f32) -> f32 {
    (x * drive).tanh() / drive.tanh()
}

// ---------------------------------------------------------------- effects

struct Comb {
    buf: Vec<f32>,
    idx: usize,
    store: f32,
}
impl Comb {
    fn new(n: usize) -> Self {
        Comb { buf: vec![0.0; n], idx: 0, store: 0.0 }
    }
    #[inline]
    fn run(&mut self, x: f32, fb: f32, damp: f32) -> f32 {
        let out = self.buf[self.idx];
        self.store = out * (1.0 - damp) + self.store * damp;
        self.buf[self.idx] = x + self.store * fb;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }
}
struct Allpass {
    buf: Vec<f32>,
    idx: usize,
}
impl Allpass {
    fn new(n: usize) -> Self {
        Allpass { buf: vec![0.0; n], idx: 0 }
    }
    #[inline]
    fn run(&mut self, x: f32) -> f32 {
        let b = self.buf[self.idx];
        let out = -x + b;
        self.buf[self.idx] = x + b * 0.5;
        self.idx = (self.idx + 1) % self.buf.len();
        out
    }
}

/// Freeverb-style stereo reverb. Returns the wet signal only.
pub fn reverb(input: &Buf, room: f32, damp: f32, extra_len: usize) -> Buf {
    const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    const APS: [usize; 4] = [556, 441, 341, 225];
    const SPREAD: usize = 23;
    let mut cl: Vec<Comb> = COMBS.iter().map(|&n| Comb::new(n)).collect();
    let mut cr: Vec<Comb> = COMBS.iter().map(|&n| Comb::new(n + SPREAD)).collect();
    let mut al: Vec<Allpass> = APS.iter().map(|&n| Allpass::new(n)).collect();
    let mut ar: Vec<Allpass> = APS.iter().map(|&n| Allpass::new(n + SPREAD)).collect();
    let fb = 0.7 + room * 0.28;
    let n = input.len() + extra_len;
    let mut out = Buf::new(n);
    for i in 0..n {
        let (xl, xr) = if i < input.len() { (input.l[i], input.r[i]) } else { (0.0, 0.0) };
        let x = (xl + xr) * 0.015;
        let mut ol = 0.0;
        let mut or = 0.0;
        for c in cl.iter_mut() {
            ol += c.run(x, fb, damp);
        }
        for c in cr.iter_mut() {
            or += c.run(x, fb, damp);
        }
        for a in al.iter_mut() {
            ol = a.run(ol);
        }
        for a in ar.iter_mut() {
            or = a.run(or);
        }
        out.l[i] = ol;
        out.r[i] = or;
    }
    out
}

/// Ping-pong delay, wet only.
pub fn ping_pong(input: &Buf, time_s: f32, fb: f32, lp_cut: f32, extra: usize) -> Buf {
    let d = (time_s * SR) as usize;
    let n = input.len() + extra;
    let mut out = Buf::new(n);
    let mut bl = vec![0.0f32; d.max(1)];
    let mut br = vec![0.0f32; d.max(1)];
    let mut idx = 0;
    let mut fl = Svf::default();
    let mut fr = Svf::default();
    for i in 0..n {
        let (xl, xr) = if i < input.len() { (input.l[i], input.r[i]) } else { (0.0, 0.0) };
        let dl = bl[idx];
        let dr = br[idx];
        let mono = (xl + xr) * 0.5;
        bl[idx] = fl.lp(mono + dr * fb, lp_cut, 0.7);
        br[idx] = fr.lp(dl * fb, lp_cut, 0.7);
        out.l[i] = dl;
        out.r[i] = dr;
        idx = (idx + 1) % bl.len();
    }
    out
}

/// High-pass a whole buffer (used to keep reverb/pads out of the sub range).
pub fn highpass(buf: &mut Buf, cut: f32) {
    let mut a = Svf::default();
    let mut b = Svf::default();
    for i in 0..buf.len() {
        buf.l[i] = a.hp(buf.l[i], cut, 0.707);
        buf.r[i] = b.hp(buf.r[i], cut, 0.707);
    }
}

/// Simple look-ahead peak limiter + final soft clip, normalizing to `ceiling`.
pub fn master(buf: &mut Buf, pre_gain: f32, ceiling: f32) {
    buf.scale(pre_gain);
    let look = (0.004 * SR) as usize;
    let n = buf.len();
    // peak envelope over look-ahead window
    let mut peaks = vec![0.0f32; n];
    for i in 0..n {
        peaks[i] = buf.l[i].abs().max(buf.r[i].abs());
    }
    // sliding max over [i-look, i], then shift so each sample sees `look` samples ahead
    let mut smax = vec![0.0f32; n];
    let mut dq: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
    for i in 0..n {
        while let Some(&b) = dq.back() {
            if peaks[b] <= peaks[i] {
                dq.pop_back();
            } else {
                break;
            }
        }
        dq.push_back(i);
        while let Some(&f) = dq.front() {
            if f + look < i {
                dq.pop_front();
            } else {
                break;
            }
        }
        smax[i] = peaks[*dq.front().unwrap()];
    }
    let win: Vec<f32> = (0..n).map(|o| smax[(o + look).min(n - 1)]).collect();
    let thresh = 0.9f32;
    let rel = (-1.0 / (0.08 * SR)).exp();
    let mut g = 1.0f32;
    for i in 0..n {
        let target = if win[i] > thresh { thresh / win[i] } else { 1.0 };
        if target < g {
            g = target;
        } else {
            g = target + (g - target) * rel;
        }
        buf.l[i] = soft_clip(buf.l[i] * g, 1.2);
        buf.r[i] = soft_clip(buf.r[i] * g, 1.2);
    }
    let p = buf.peak();
    if p > 0.0 {
        buf.scale(ceiling / p);
    }
}

pub fn write_wav(path: &str, buf: &Buf, stereo: bool) -> std::io::Result<()> {
    use std::io::Write;
    let ch: u16 = if stereo { 2 } else { 1 };
    let n = buf.len();
    let data_len = (n * ch as usize * 2) as u32;
    let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
    f.write_all(b"RIFF")?;
    f.write_all(&(36 + data_len).to_le_bytes())?;
    f.write_all(b"WAVEfmt ")?;
    f.write_all(&16u32.to_le_bytes())?;
    f.write_all(&1u16.to_le_bytes())?;
    f.write_all(&ch.to_le_bytes())?;
    f.write_all(&(SR as u32).to_le_bytes())?;
    f.write_all(&(SR as u32 * ch as u32 * 2).to_le_bytes())?;
    f.write_all(&(ch * 2).to_le_bytes())?;
    f.write_all(&16u16.to_le_bytes())?;
    f.write_all(b"data")?;
    f.write_all(&data_len.to_le_bytes())?;
    for i in 0..n {
        let to16 = |v: f32| (v.clamp(-1.0, 1.0) * 32767.0) as i16;
        if stereo {
            f.write_all(&to16(buf.l[i]).to_le_bytes())?;
            f.write_all(&to16(buf.r[i]).to_le_bytes())?;
        } else {
            f.write_all(&to16((buf.l[i] + buf.r[i]) * 0.5).to_le_bytes())?;
        }
    }
    Ok(())
}
