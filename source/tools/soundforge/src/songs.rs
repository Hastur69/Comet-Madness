//! Original compositions for Comet Craze. All in F minor (i–VI–III–VII: Fm, Db, Ab, Eb).
//! Written from scratch for this project — no melodies from the 1985 soundtrack are used.

use crate::dsp::*;
use crate::instruments::*;

/// Song timing helper.
struct Grid {
    bpm: f32,
}
impl Grid {
    fn beat(&self) -> f32 {
        60.0 / self.bpm
    }
    fn step(&self) -> f32 {
        self.beat() / 4.0
    }
    /// sample index of (bar, 16th step) — steps may be fractional
    fn at(&self, bar: f32, step: f32) -> usize {
        ((bar * 16.0 + step) * self.step() * SR) as usize
    }
    fn secs_steps(&self, steps: f32) -> f32 {
        steps * self.step()
    }
    fn bar_len(&self, bars: f32) -> usize {
        (bars * 16.0 * self.step() * SR) as usize
    }
}

/// Mix buses for one song.
struct Mix {
    drums: Buf,
    bass: Buf,
    lead: Buf,
    pad: Buf,
    fx: Buf,
    verb_send: Buf,
    delay_send: Buf,
    kicks: Vec<usize>,
}
impl Mix {
    fn new(len: usize) -> Self {
        Mix {
            drums: Buf::new(len),
            bass: Buf::new(len),
            lead: Buf::new(len),
            pad: Buf::new(len),
            fx: Buf::new(len),
            verb_send: Buf::new(len),
            delay_send: Buf::new(len),
            kicks: vec![],
        }
    }
}

/// Sidechain ducking envelope built from kick positions.
fn duck(len: usize, kicks: &[usize], depth: f32, release_s: f32) -> Vec<f32> {
    let mut g = vec![1.0f32; len];
    let rel = (release_s * SR) as usize;
    let att = (0.004 * SR) as usize;
    for &k in kicks {
        for i in 0..rel {
            let idx = k + i;
            if idx >= len {
                break;
            }
            let d = if i < att {
                depth * (i as f32 / att as f32)
            } else {
                let x = (i - att) as f32 / (rel - att) as f32;
                depth * (1.0 - x).powi(2)
            };
            g[idx] = g[idx].min(1.0 - d);
        }
    }
    g
}

fn apply_gain(buf: &mut Buf, g: &[f32]) {
    for i in 0..buf.len().min(g.len()) {
        buf.l[i] *= g[i];
        buf.r[i] *= g[i];
    }
}

fn finish(mut m: Mix, grid: &Grid, loop_len: Option<usize>, drive: f32, lead_gain: f32, air: f32) -> Buf {
    let len = m.drums.len();
    let d_bass = duck(len, &m.kicks, 0.88, grid.beat() * 0.55);
    let d_pad = duck(len, &m.kicks, 0.65, grid.beat() * 0.7);
    let d_lead = duck(len, &m.kicks, 0.35, grid.beat() * 0.5);
    apply_gain(&mut m.bass, &d_bass);
    apply_gain(&mut m.pad, &d_pad);
    apply_gain(&mut m.lead, &d_lead);

    let mut verb = reverb(&m.verb_send, 0.82, 0.35, 0);
    highpass(&mut verb, 250.0);
    apply_gain(&mut verb, &d_pad);
    let mut dly = ping_pong(&m.delay_send, grid.beat() * 0.75, 0.42, 4500.0, 0);
    highpass(&mut dly, 300.0);
    apply_gain(&mut dly, &d_lead);

    let mut out = Buf::new(len);
    out.mix(&m.drums, 1.0);
    out.mix(&m.bass, 0.9);
    out.mix(&m.lead, 0.75 * lead_gain);
    out.mix(&m.pad, 0.7);
    out.mix(&m.fx, 0.8);
    out.mix(&verb, 0.9);
    out.mix(&dly, 0.35);
    if let Some(l) = loop_len {
        out.fold_loop(l);
    }
    // tilt: high-shelf 'air' boost + DC/rumble removal
    {
        let mut sl = crate::dsp::Svf::default();
        let mut sr = crate::dsp::Svf::default();
        for i in 0..out.len() {
            let hl = sl.hp(out.l[i], 2800.0, 0.6);
            let hr = sr.hp(out.r[i], 2800.0, 0.6);
            out.l[i] += hl * air;
            out.r[i] += hr * air;
        }
        highpass(&mut out, 28.0);
    }
    let p = out.peak().max(1e-6);
    out.scale(1.0 / p);
    master(&mut out, drive, 0.95);
    out
}

// Chord data: (pad voicing, bass root in octave 2)
const CHORDS: [([f32; 4], f32); 4] = [
    ([53.0, 56.0, 60.0, 65.0], 41.0), // Fm
    ([49.0, 53.0, 56.0, 61.0], 37.0), // Db
    ([48.0, 51.0, 56.0, 60.0], 44.0), // Ab
    ([46.0, 51.0, 55.0, 58.0], 39.0), // Eb
];

/// Main drop hook for "Comet Rush" (step, len, midi) per bar, 4 bars.
const HOOK: [&[(f32, f32, f32)]; 4] = [
    &[(0.0, 3.0, 77.0), (3.0, 3.0, 72.0), (6.0, 2.0, 68.0), (8.0, 2.0, 72.0), (10.0, 2.0, 77.0), (12.0, 2.0, 79.0), (14.0, 2.0, 80.0)],
    &[(0.0, 3.0, 80.0), (3.0, 3.0, 77.0), (6.0, 2.0, 73.0), (8.0, 2.0, 77.0), (10.0, 2.0, 80.0), (12.0, 4.0, 82.0)],
    &[(0.0, 3.0, 84.0), (3.0, 3.0, 80.0), (6.0, 2.0, 75.0), (8.0, 2.0, 80.0), (10.0, 2.0, 84.0), (12.0, 2.0, 82.0), (14.0, 2.0, 80.0)],
    &[(0.0, 4.0, 79.0), (4.0, 2.0, 75.0), (6.0, 2.0, 79.0), (8.0, 6.0, 82.0), (14.0, 2.0, 79.0)],
];
/// Variation ending for the 2nd half of an 8-bar phrase
const HOOK_END: &[(f32, f32, f32)] = &[(0.0, 2.0, 79.0), (2.0, 2.0, 82.0), (4.0, 2.0, 84.0), (6.0, 2.0, 87.0), (8.0, 8.0, 89.0)];

/// Slow SID-flavoured melody used on the title screen.
const TITLE_MEL: [&[(f32, f32, f32)]; 8] = [
    &[(0.0, 4.0, 72.0), (4.0, 2.0, 68.0), (6.0, 2.0, 72.0), (8.0, 8.0, 77.0)],
    &[(0.0, 4.0, 75.0), (4.0, 2.0, 73.0), (6.0, 2.0, 72.0), (8.0, 6.0, 73.0), (14.0, 2.0, 77.0)],
    &[(0.0, 6.0, 75.0), (6.0, 2.0, 72.0), (8.0, 4.0, 68.0), (12.0, 4.0, 72.0)],
    &[(0.0, 8.0, 70.0), (8.0, 4.0, 67.0), (12.0, 4.0, 70.0)],
    &[(0.0, 4.0, 72.0), (4.0, 4.0, 77.0), (8.0, 4.0, 79.0), (12.0, 4.0, 80.0)],
    &[(0.0, 4.0, 79.0), (4.0, 4.0, 77.0), (8.0, 8.0, 73.0)],
    &[(0.0, 4.0, 75.0), (4.0, 4.0, 80.0), (8.0, 4.0, 79.0), (12.0, 4.0, 75.0)],
    &[(0.0, 12.0, 77.0), (12.0, 2.0, 75.0), (14.0, 2.0, 72.0)],
];

fn arp_bar(m: &mut Mix, g: &Grid, bar: f32, chord: usize, vel: f32, oct: f32) {
    let c = CHORDS[chord].0;
    let order = [0usize, 1, 2, 3, 2, 1, 2, 3, 0, 2, 1, 3, 2, 3, 1, 2];
    for s in 0..16 {
        let n = c[order[s]] + oct;
        let pos = g.at(bar, s as f32);
        let pan = if s % 2 == 0 { -0.4 } else { 0.4 };
        pluck(&mut m.lead, pos, g.secs_steps(0.8), n, vel, pan);
        pluck(&mut m.verb_send, pos, g.secs_steps(0.8), n, vel * 0.35, pan);
        if s % 4 == 2 {
            pluck(&mut m.delay_send, pos, g.secs_steps(0.8), n, vel * 0.5, pan);
        }
    }
}

fn kick_at(m: &mut Mix, pos: usize, vel: f32, hard: bool) {
    kick(&mut m.drums, pos, vel, hard);
    m.kicks.push(pos);
}

// =====================================================================
// TITLE — "Orbital Drift" (128 BPM, 16-bar seamless loop)
// =====================================================================
pub fn title() -> Buf {
    let g = Grid { bpm: 128.0 };
    let bars = 16.0;
    let loop_len = g.bar_len(bars);
    let mut m = Mix::new(loop_len + g.bar_len(2.0));
    for b in 0..16 {
        let bar = b as f32;
        let ch = b % 4;
        let (voicing, root) = CHORDS[ch];
        let cut = if b < 8 { 700.0 + b as f32 * 180.0 } else { 2400.0 };
        pad(&mut m.pad, g.at(bar, 0.0), g.secs_steps(15.5), &voicing, 0.9, cut);
        pad(&mut m.verb_send, g.at(bar, 0.0), g.secs_steps(15.5), &voicing, 0.4, cut);
        arp_bar(&mut m, &g, bar, ch, if b < 4 { 0.35 } else { 0.55 }, 12.0);
        // four-on-the-floor, gentler in the first half
        for q in 0..4 {
            kick_at(&mut m, g.at(bar, q as f32 * 4.0), if b < 8 { 0.55 } else { 0.8 }, false);
            hat(&mut m.drums, g.at(bar, q as f32 * 4.0 + 2.0), 0.5, b >= 8, 0.2);
        }
        if b >= 4 {
            clap(&mut m.drums, g.at(bar, 4.0), 0.55);
            clap(&mut m.drums, g.at(bar, 12.0), 0.55);
            clap(&mut m.verb_send, g.at(bar, 12.0), 0.25);
        }
        for s in [0.0, 6.0, 10.0] {
            house_bass(&mut m.bass, g.at(bar, s), g.secs_steps(if s == 0.0 { 5.0 } else { 3.0 }), root - 12.0 + 12.0, 0.5);
        }
        if b >= 8 {
            for &(st, ln, n) in TITLE_MEL[b - 8] {
                chip_lead(&mut m.lead, g.at(bar, st), g.secs_steps(ln), n, 0.9, 0.1);
                chip_lead(&mut m.delay_send, g.at(bar, st), g.secs_steps(ln), n, 0.7, 0.0);
                chip_lead(&mut m.verb_send, g.at(bar, st), g.secs_steps(ln), n, 0.4, 0.0);
            }
        }
    }
    finish(m, &g, Some(loop_len), 1.5, 1.0, 0.5)
}

// =====================================================================
// STAGE A — "Comet Rush" (128 BPM electro / big-room, 48-bar loop)
// =====================================================================
pub fn comet_rush() -> Buf {
    let g = Grid { bpm: 128.0 };
    let loop_len = g.bar_len(48.0);
    let mut m = Mix::new(loop_len + g.bar_len(2.0));

    let lead_bar = |m: &mut Mix, bar: usize, phrase_bar: usize, bright: f32, vel: f32, oct: f32| {
        let notes: &[(f32, f32, f32)] = if phrase_bar == 7 { HOOK_END } else { HOOK[phrase_bar % 4] };
        for (k, &(st, ln, n)) in notes.iter().enumerate() {
            let pos = g.at(bar as f32, st);
            let dur = g.secs_steps(ln * 0.92);
            supersaw(&mut m.lead, pos, dur, n + oct, vel, bright, (bar * 31 + k) as u64);
            supersaw(&mut m.verb_send, pos, dur, n + oct, vel * 0.3, bright * 0.6, (bar * 17 + k) as u64);
            supersaw(&mut m.delay_send, pos, dur, n + oct, vel * 0.4, bright * 0.5, (bar * 7 + k) as u64);
        }
    };

    for b in 0..48usize {
        let bar = b as f32;
        let ch = b % 4;
        let (voicing, root) = CHORDS[ch];
        let section = match b {
            0..=7 => 0,   // intro groove
            8..=15 => 1,  // build
            16..=31 => 2, // drop A
            32..=35 => 3, // breakdown
            36..=39 => 4, // build 2
            _ => 5,       // drop B
        };
        let drop = section == 2 || section == 5;

        // ---- kick
        let kick_on = !(b >= 12 && b <= 15) && !(b >= 32 && b <= 39 && b % 2 == 1);
        if kick_on {
            for q in 0..4 {
                if section == 3 && q != 0 {
                    continue;
                }
                kick_at(&mut m, g.at(bar, q as f32 * 4.0), if drop { 1.0 } else { 0.85 }, drop);
            }
        }
        // ---- claps / hats
        if section != 3 {
            clap(&mut m.drums, g.at(bar, 4.0), 0.75);
            clap(&mut m.drums, g.at(bar, 12.0), 0.75);
            clap(&mut m.verb_send, g.at(bar, 12.0), 0.25);
        }
        for s in 0..16 {
            let st = s as f32;
            if s % 4 == 2 {
                hat(&mut m.drums, g.at(bar, st), 0.8, drop, 0.15);
            } else if drop || section == 0 {
                hat(&mut m.drums, g.at(bar, st), if s % 2 == 0 { 0.35 } else { 0.22 }, false, -0.3);
            }
        }
        // ---- bass: offbeat 8ths + octave pop
        if section != 3 && !(b >= 14 && b <= 15) {
            for (s, o) in [(2.0, 0.0), (6.0, 0.0), (10.0, 0.0), (14.0, 12.0)] {
                house_bass(&mut m.bass, g.at(bar, s), g.secs_steps(1.6), root + o, if drop { 1.0 } else { 0.8 });
            }
        }
        // ---- pads
        let pad_cut = match section {
            0 => 900.0,
            1 | 4 => 900.0 + (b % 8) as f32 * 450.0,
            3 => 2000.0,
            _ => 3200.0,
        };
        pad(&mut m.pad, g.at(bar, 0.0), g.secs_steps(15.0), &voicing, if drop { 0.55 } else { 0.8 }, pad_cut);
        pad(&mut m.verb_send, g.at(bar, 0.0), g.secs_steps(15.0), &voicing, 0.3, pad_cut);

        // ---- leads
        match section {
            1 => lead_bar(&mut m, b, b - 8, 0.08 + 0.07 * (b - 8) as f32, 0.55, 0.0),
            2 => lead_bar(&mut m, b, (b - 16) % 8, 1.0, 0.85, 0.0),
            5 => {
                lead_bar(&mut m, b, (b - 40) % 8, 1.0, 0.75, 0.0);
                lead_bar(&mut m, b, (b - 40) % 8, 0.8, 0.35, 12.0);
            }
            3 | 4 => {
                arp_bar(&mut m, &g, bar, ch, 0.5, 12.0);
                if section == 3 {
                    for &(st, ln, n) in TITLE_MEL[(b - 32) + 4] {
                        chip_lead(&mut m.lead, g.at(bar, st), g.secs_steps(ln), n, 0.8, 0.0);
                        chip_lead(&mut m.delay_send, g.at(bar, st), g.secs_steps(ln), n, 0.6, 0.0);
                    }
                }
            }
            0 => {
                if b >= 4 {
                    arp_bar(&mut m, &g, bar, ch, 0.32, 12.0);
                }
            }
            _ => {}
        }

        // ---- build machinery: snare rolls + risers
        let roll_base = if (12..=15).contains(&b) { Some(b - 12) } else if (36..=39).contains(&b) { Some(b - 36) } else { None };
        if let Some(rb) = roll_base {
            let div = [4.0f32, 2.0, 1.0, 0.5][rb];
            let mut s = 0.0;
            while s < 16.0 {
                if !(rb == 3 && s >= 14.0) {
                    let v = 0.35 + 0.5 * ((rb as f32 * 16.0 + s) / 64.0);
                    snare(&mut m.drums, g.at(bar, s), v, false);
                    snare(&mut m.verb_send, g.at(bar, s), v * 0.3, false);
                }
                s += div;
            }
            if rb == 0 {
                riser(&mut m.fx, g.at(bar, 0.0), g.secs_steps(62.0), 0.9);
            }
        }
        if b == 16 || b == 40 || b == 24 || b == 0 {
            crash(&mut m.drums, g.at(bar, 0.0), 1.0);
            crash(&mut m.verb_send, g.at(bar, 0.0), 0.3);
        }
        if b == 16 || b == 40 {
            impact(&mut m.fx, g.at(bar, 0.0), 0.9);
            downlifter(&mut m.fx, g.at(bar, 0.0), g.secs_steps(32.0), 0.5);
        }
    }
    finish(m, &g, Some(loop_len), 1.8, 1.6, 1.0)
}

// =====================================================================
// STAGE B — "Supernova Cascade" (140 BPM dubstep, 48-bar loop)
// =====================================================================
pub fn supernova_cascade() -> Buf {
    let g = Grid { bpm: 140.0 };
    let loop_len = g.bar_len(48.0);
    let mut m = Mix::new(loop_len + g.bar_len(2.0));

    // Wobble phrases: (step, len, note, lfo, glide_from) over 2 bars (32 steps)
    let f1 = 29.0; // F1
    let db1 = 25.0;
    let ab1 = 32.0;
    let eb1 = 27.0;
    let phrase_a: Vec<(f32, f32, f32, Lfo, Option<f32>)> = vec![
        (0.0, 6.0, f1, Lfo::Sine(2.0), None),
        (6.0, 2.0, f1 + 12.0, Lfo::Hold(0.95), None),
        (8.0, 8.0, f1, Lfo::Sine(4.0), None),
        (16.0, 4.0, db1, Lfo::Saw(2.0), Some(f1)),
        (20.0, 3.0, db1 + 12.0, Lfo::Sine(1.0), None),
        (23.0, 1.0, db1 + 12.0, Lfo::Hold(0.8), None),
        (24.0, 4.0, f1, Lfo::Step(4.0), None),
        (28.0, 4.0, ab1, Lfo::Sine(3.0), Some(f1)),
    ];
    let phrase_b: Vec<(f32, f32, f32, Lfo, Option<f32>)> = vec![
        (0.0, 4.0, ab1, Lfo::Sine(2.0), None),
        (4.0, 2.0, ab1 + 12.0, Lfo::Hold(1.0), None),
        (6.0, 2.0, ab1, Lfo::Saw(4.0), None),
        (8.0, 8.0, ab1, Lfo::Sine(6.0), None),
        (16.0, 6.0, eb1, Lfo::Sine(1.5), Some(eb1 + 12.0)),
        (22.0, 2.0, eb1 + 12.0, Lfo::Hold(0.9), None),
        (24.0, 2.0, eb1, Lfo::Step(8.0), None),
        (26.0, 2.0, eb1 + 7.0, Lfo::Hold(0.7), None),
        (28.0, 4.0, f1, Lfo::Saw(1.0), Some(eb1)),
    ];
    let phrase_c: Vec<(f32, f32, f32, Lfo, Option<f32>)> = vec![
        // triplet-heavy switch-up
        (0.0, 8.0, f1, Lfo::Sine(3.0), None),
        (8.0, 4.0, f1 + 12.0, Lfo::Sine(6.0), None),
        (12.0, 4.0, f1, Lfo::Step(6.0), None),
        (16.0, 8.0, db1, Lfo::Sine(1.5), Some(f1)),
        (24.0, 2.0, ab1, Lfo::Hold(1.0), None),
        (26.0, 2.0, ab1 - 2.0, Lfo::Hold(0.6), None),
        (28.0, 4.0, f1, Lfo::Saw(3.0), Some(ab1)),
    ];

    for b in 0..48usize {
        let bar = b as f32;
        let ch = b % 4;
        let (voicing, _) = CHORDS[ch];
        let section = match b {
            0..=7 => 0,
            8..=15 => 1,
            16..=31 => 2,
            32..=39 => 3,
            _ => 4,
        };
        let drop = section >= 2;

        // ---- halftime drums
        if !(12..=15).contains(&b) {
            if drop {
                let pat: &[f32] = match b % 4 {
                    0 => &[0.0, 10.0],
                    1 => &[0.0, 3.0, 11.0],
                    2 => &[0.0, 10.0, 14.0],
                    _ => &[0.0, 6.0, 11.0],
                };
                for &s in pat {
                    kick_at(&mut m, g.at(bar, s), 1.0, true);
                }
                snare(&mut m.drums, g.at(bar, 8.0), 1.0, true);
                clap(&mut m.drums, g.at(bar, 8.0), 0.6);
                snare(&mut m.verb_send, g.at(bar, 8.0), 0.35, true);
                if b % 4 == 3 {
                    snare(&mut m.drums, g.at(bar, 15.0), 0.45, false);
                }
                for s in 0..16 {
                    let v = if s % 4 == 2 { 0.55 } else if s % 2 == 0 { 0.3 } else { 0.18 };
                    hat(&mut m.drums, g.at(bar, s as f32), v, s == 14 && b % 2 == 1, if s % 2 == 0 { 0.3 } else { -0.3 });
                }
            } else {
                kick_at(&mut m, g.at(bar, 0.0), 0.85, false);
                if section == 0 && b >= 2 {
                    snare(&mut m.drums, g.at(bar, 8.0), 0.7, true);
                    snare(&mut m.verb_send, g.at(bar, 8.0), 0.4, true);
                }
                if section == 1 && b < 12 {
                    snare(&mut m.drums, g.at(bar, 8.0), 0.8, true);
                    kick_at(&mut m, g.at(bar, 10.0), 0.7, false);
                }
                for s in (0..16).step_by(2) {
                    hat(&mut m.drums, g.at(bar, s as f32), if s % 4 == 2 { 0.45 } else { 0.25 }, false, 0.2);
                }
            }
        }

        // ---- pads + atmosphere
        let pcut = if drop { 1600.0 } else { 700.0 + (b % 8) as f32 * 200.0 };
        pad(&mut m.pad, g.at(bar, 0.0), g.secs_steps(15.5), &voicing, if drop { 0.35 } else { 0.85 }, pcut);
        pad(&mut m.verb_send, g.at(bar, 0.0), g.secs_steps(15.5), &voicing, 0.35, pcut);

        if section == 0 || section == 1 {
            // eerie chip arp in the intro (half speed 8ths)
            let c = voicing;
            for s in (0..16).step_by(2) {
                let n = c[(s / 2) % 4] + 24.0;
                chip_lead(&mut m.lead, g.at(bar, s as f32), g.secs_steps(1.6), n, 0.35, if s % 4 == 0 { -0.5 } else { 0.5 });
                chip_lead(&mut m.delay_send, g.at(bar, s as f32), g.secs_steps(1.6), n, 0.4, 0.0);
            }
            if section == 0 {
                // sub drone
                if b % 2 == 0 {
                    wobble(&mut m.bass, g.at(bar, 0.0), g.secs_steps(30.0), f1, None, 0.45, Lfo::Hold(0.08), 140.0, 0.2, b as u64);
                }
            } else if b < 12 {
                // filtered teaser of the drop bass
                let ph = if b % 2 == 0 { &phrase_a } else { &phrase_b };
                for (k, &(st, ln, n, _l, gl)) in ph.iter().enumerate() {
                    if st < 16.0 {
                        wobble(&mut m.bass, g.at(bar, st), g.secs_steps(ln), n, gl, 0.6, Lfo::Hold(0.2 + 0.05 * (b - 8) as f32), 140.0, 0.4, (b * 100 + k) as u64);
                    }
                }
            }
        }

        // ---- build
        if (12..=15).contains(&b) {
            let rb = b - 12;
            let div = [4.0f32, 2.0, 1.0, 0.5][rb];
            let mut s = 0.0;
            while s < 16.0 {
                if !(rb == 3 && s >= 13.0) {
                    let v = 0.35 + 0.55 * ((rb as f32 * 16.0 + s) / 64.0);
                    snare(&mut m.drums, g.at(bar, s), v, false);
                    snare(&mut m.verb_send, g.at(bar, s), v * 0.3, false);
                }
                s += div;
            }
            if rb == 0 {
                riser(&mut m.fx, g.at(bar, 0.0), g.secs_steps(61.0), 1.0);
            }
            // siren lead
            for &(st, ln, n) in &[(0.0f32, 8.0f32, 77.0f32), (8.0, 8.0, 80.0)] {
                supersaw(&mut m.lead, g.at(bar, st), g.secs_steps(ln), n + rb as f32, 0.25 + 0.1 * rb as f32, 0.3 + 0.15 * rb as f32, b as u64);
            }
        }

        // ---- DROPS: wobble bass
        if drop {
            let pair = (b - 16) / 2;
            let half = (b % 2) as f32 * 16.0;
            let ph = match section {
                3 => &phrase_c,
                _ => {
                    if pair % 2 == 0 {
                        &phrase_a
                    } else {
                        &phrase_b
                    }
                }
            };
            let growl = match section {
                2 => 0.6,
                3 => 1.0,
                _ => 1.25,
            };
            for (k, &(st, ln, n, l, gl)) in ph.iter().enumerate() {
                if st >= half && st < half + 16.0 {
                    wobble(&mut m.bass, g.at(bar, st - half), g.secs_steps(ln), n, gl, 1.0, l, 140.0, growl, (b * 100 + k) as u64);
                }
            }
            // drop 2: supersaw chord stabs on the off-beats
            if section == 4 {
                for s in [3.0f32, 6.0, 11.0] {
                    for (vi, &n) in voicing.iter().enumerate() {
                        supersaw(&mut m.lead, g.at(bar, s), g.secs_steps(1.2), n + 12.0, 0.22, 1.0, (b * 10 + vi) as u64);
                    }
                }
                for &(st, ln, n) in HOOK[ch] {
                    if st < 8.0 {
                        chip_lead(&mut m.lead, g.at(bar, st), g.secs_steps(ln), n, 0.5, 0.0);
                        chip_lead(&mut m.delay_send, g.at(bar, st), g.secs_steps(ln), n, 0.4, 0.0);
                    }
                }
            }
        }

        if b == 16 || b == 32 || b == 40 || b == 0 {
            crash(&mut m.drums, g.at(bar, 0.0), 1.0);
            crash(&mut m.verb_send, g.at(bar, 0.0), 0.3);
        }
        if b == 16 || b == 40 {
            impact(&mut m.fx, g.at(bar, 0.0), 1.0);
            downlifter(&mut m.fx, g.at(bar, 0.0), g.secs_steps(32.0), 0.5);
        }
    }
    finish(m, &g, Some(loop_len), 2.0, 1.2, 0.6)
}

// =====================================================================
// GAME OVER sting
// =====================================================================
pub fn game_over() -> Buf {
    let g = Grid { bpm: 100.0 };
    let len = (6.5 * SR) as usize;
    let mut m = Mix::new(len);
    impact(&mut m.fx, 0, 0.9);
    pad(&mut m.pad, 0, 4.0, &[53.0, 56.0, 60.0, 65.0], 0.9, 1400.0);
    pad(&mut m.verb_send, 0, 4.0, &[53.0, 56.0, 60.0, 65.0], 0.5, 1400.0);
    let mel = [(0.0f32, 72.0f32), (2.0, 68.0), (4.0, 65.0), (6.0, 60.0), (8.0, 61.0), (12.0, 60.0)];
    for (i, &(st, n)) in mel.iter().enumerate() {
        let d = if i == mel.len() - 1 { g.secs_steps(12.0) } else { g.secs_steps(1.8) };
        chip_lead(&mut m.lead, g.at(0.0, st), d, n, 0.9, 0.0);
        chip_lead(&mut m.verb_send, g.at(0.0, st), d, n, 0.5, 0.0);
        chip_lead(&mut m.delay_send, g.at(0.0, st), d, n, 0.5, 0.0);
    }
    wobble(&mut m.bass, 0, 2.5, 29.0, Some(41.0), 0.7, Lfo::Saw(0.5), 100.0, 0.8, 7);
    let mut out = finish(m, &g, None, 1.2, 1.0, 0.4);
    let n = out.len();
    out.gain_curve(|i| {
        let x = i as f32 / n as f32;
        if x > 0.8 { (1.0 - x) / 0.2 } else { 1.0 }
    });
    out
}
