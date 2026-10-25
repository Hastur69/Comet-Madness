//! Music + sound effect playback. All audio is embedded in the executable.

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    Title,
    Rush,
    Cascade,
    GameOver,
}

impl Track {
    pub fn bpm(self) -> f32 {
        match self {
            Track::Title | Track::Rush => 128.0,
            Track::Cascade => 140.0,
            Track::GameOver => 100.0,
        }
    }
    pub fn looped(self) -> bool {
        true
    }
    pub fn name(self) -> &'static str {
        match self {
            Track::Title => "ORBITAL DRIFT",
            Track::Rush => "COMET RUSH",
            Track::Cascade => "SUPERNOVA CASCADE",
            Track::GameOver => "LAST LIGHT",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(usize)]
pub enum Sfx {
    Shoot,
    Hit,
    ExplodeSmall,
    ExplodeMedium,
    ExplodeBig,
    Crazy,
    Nova,
    Pickup,
    ExtraLife,
    LevelClear,
    PlayerDie,
    Bomb,
    UiSelect,
    SatelliteLaunch,
    CapsuleSpawn,
    Warp,
}
const SFX_COUNT: usize = 16;

pub const MUSIC_FILES: [(Track, &[u8]); 4] = [
    (Track::Title, include_bytes!("../assets/music/title_orbital_drift.ogg")),
    (Track::Rush, include_bytes!("../assets/music/stage_comet_rush.ogg")),
    (Track::Cascade, include_bytes!("../assets/music/stage_supernova_cascade.ogg")),
    (Track::GameOver, include_bytes!("../assets/music/game_over.ogg")),
];

const SFX_FILES: [(Sfx, &[u8], f32, f32); SFX_COUNT] = [
    // (id, data, base volume, min retrigger interval)
    (Sfx::Shoot, include_bytes!("../assets/sfx/shoot.wav"), 0.22, 0.05),
    (Sfx::Hit, include_bytes!("../assets/sfx/hit.wav"), 0.35, 0.03),
    (Sfx::ExplodeSmall, include_bytes!("../assets/sfx/explode_small.wav"), 0.6, 0.04),
    (Sfx::ExplodeMedium, include_bytes!("../assets/sfx/explode_medium.wav"), 0.75, 0.05),
    (Sfx::ExplodeBig, include_bytes!("../assets/sfx/explode_big.wav"), 0.95, 0.08),
    (Sfx::Crazy, include_bytes!("../assets/sfx/crazy.wav"), 0.45, 0.25),
    (Sfx::Nova, include_bytes!("../assets/sfx/nova.wav"), 0.7, 0.3),
    (Sfx::Pickup, include_bytes!("../assets/sfx/pickup.wav"), 0.7, 0.05),
    (Sfx::ExtraLife, include_bytes!("../assets/sfx/extra_life.wav"), 0.8, 0.2),
    (Sfx::LevelClear, include_bytes!("../assets/sfx/level_clear.wav"), 0.8, 0.5),
    (Sfx::PlayerDie, include_bytes!("../assets/sfx/player_die.wav"), 1.0, 0.5),
    (Sfx::Bomb, include_bytes!("../assets/sfx/bomb.wav"), 1.0, 0.3),
    (Sfx::UiSelect, include_bytes!("../assets/sfx/ui_select.wav"), 0.5, 0.03),
    (Sfx::SatelliteLaunch, include_bytes!("../assets/sfx/satellite_launch.wav"), 0.4, 0.1),
    (Sfx::CapsuleSpawn, include_bytes!("../assets/sfx/capsule_spawn.wav"), 0.5, 0.2),
    (Sfx::Warp, include_bytes!("../assets/sfx/warp.wav"), 0.9, 2.0),
];

struct Playing {
    track: Track,
    fade: f32, // current fade gain 0..1
    fading_out: bool,
}

pub struct Audio {
    sfx: Vec<(Sound, f32, f32)>,
    last_play: [f64; SFX_COUNT],
    music: Vec<(Track, Sound)>,
    playing: Vec<Playing>,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub duck: f32,
    pub current: Option<Track>,
    current_started: f64,
    /// per-track bass envelopes, BASS_FPS values per second, each 0..1
    bass: Vec<(Track, Vec<f32>)>,
}

/// Resolution of the pre-computed bass envelopes (values per second).
pub const BASS_FPS: f32 = 60.0;

/// Decode an OGG track and build a smoothed 0..1 envelope of its low end (< ~150 Hz).
/// Macroquad can't tap audio as it plays, so this is done once at load time and
/// looked up by play position each frame.
pub fn analyze_bass(data: &[u8]) -> Vec<f32> {
    use lewton::inside_ogg::OggStreamReader;
    let mut rdr = match OggStreamReader::new(std::io::Cursor::new(data)) {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let ch = rdr.ident_hdr.audio_channels.max(1) as usize;
    let sr = rdr.ident_hdr.audio_sample_rate as f32;
    let hop = (sr / BASS_FPS) as usize;
    // two cascaded one-pole low-passes at ~150 Hz
    let k = 1.0 - (-2.0 * std::f32::consts::PI * 150.0 / sr).exp();
    let (mut lp1, mut lp2) = (0.0f32, 0.0f32);
    let (mut acc, mut n) = (0.0f32, 0usize);
    let mut rms = Vec::new();
    while let Ok(Some(pkt)) = rdr.read_dec_packet_itl() {
        for frame in pkt.chunks(ch) {
            let mono = frame.iter().map(|&s| s as f32).sum::<f32>() / (ch as f32 * 32768.0);
            lp1 += (mono - lp1) * k;
            lp2 += (lp1 - lp2) * k;
            acc += lp2 * lp2;
            n += 1;
            if n == hop {
                rms.push((acc / n as f32).sqrt());
                acc = 0.0;
                n = 0;
            }
        }
    }
    if rms.is_empty() {
        return rms;
    }
    // instant attack, ~0.12 s release, so hits snap out and ease back in
    let rel = (-1.0 / (0.12 * BASS_FPS)).exp();
    let mut env = 0.0f32;
    for v in rms.iter_mut() {
        env = if *v > env { *v } else { *v + (env - *v) * rel };
        *v = env;
    }
    // normalise between the quiet floor and the loud peaks of this track
    let mut sorted = rms.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let lo = sorted[sorted.len() * 15 / 100];
    let hi = sorted[sorted.len() * 98 / 100].max(lo + 1e-6);
    for v in rms.iter_mut() {
        *v = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0).powf(1.4);
    }
    rms
}

impl Audio {
    pub async fn load_sfx() -> Vec<(Sound, f32, f32)> {
        let mut v = vec![];
        for (_, data, vol, gap) in SFX_FILES.iter() {
            v.push((load_sound_from_bytes(data).await.expect("sfx decode"), *vol, *gap));
        }
        v
    }

    pub fn new(sfx: Vec<(Sound, f32, f32)>, music: Vec<(Track, Sound)>, bass: Vec<(Track, Vec<f32>)>, music_volume: f32, sfx_volume: f32) -> Self {
        Audio {
            sfx,
            last_play: [-10.0; SFX_COUNT],
            music,
            playing: vec![],
            music_volume,
            sfx_volume,
            duck: 1.0,
            current: None,
            current_started: 0.0,
            bass,
        }
    }

    fn sound_of(&self, t: Track) -> &Sound {
        &self.music.iter().find(|(k, _)| *k == t).unwrap().1
    }

    pub fn play(&mut self, s: Sfx) {
        self.play_vol(s, 1.0);
    }

    pub fn play_vol(&mut self, s: Sfx, v: f32) {
        let i = s as usize;
        let now = macroquad::time::get_time();
        let (snd, base, gap) = &self.sfx[i];
        if now - self.last_play[i] < *gap as f64 || self.sfx_volume <= 0.001 {
            return;
        }
        self.last_play[i] = now;
        play_sound(snd, PlaySoundParams { looped: false, volume: (base * v * self.sfx_volume).min(1.0) });
    }

    /// Switch music with a short crossfade. No-op if that track is already current.
    pub fn music(&mut self, t: Track) {
        if self.current == Some(t) {
            return;
        }
        let now = macroquad::time::get_time();
        for p in self.playing.iter_mut() {
            p.fading_out = true;
        }
        // If the same track is fading out, stop it hard before restarting.
        if let Some(idx) = self.playing.iter().position(|p| p.track == t) {
            stop_sound(self.sound_of(t));
            self.playing.remove(idx);
        }
        let vol = self.music_volume * self.duck;
        play_sound(self.sound_of(t), PlaySoundParams { looped: t.looped(), volume: vol });
        self.playing.push(Playing { track: t, fade: 1.0, fading_out: false });
        self.current = Some(t);
        self.current_started = now;
    }

    #[allow(dead_code)]
    pub fn stop_music(&mut self) {
        for p in self.playing.iter_mut() {
            p.fading_out = true;
        }
        self.current = None;
    }

    pub fn update(&mut self, dt: f32) {
        let mut remove = vec![];
        for (i, p) in self.playing.iter_mut().enumerate() {
            if p.fading_out {
                p.fade -= dt / 0.7;
                if p.fade <= 0.0 {
                    remove.push(i);
                    continue;
                }
            }
        }
        for &i in remove.iter().rev() {
            let t = self.playing[i].track;
            stop_sound(self.sound_of(t));
            self.playing.remove(i);
        }
        let mv = self.music_volume * self.duck;
        for p in self.playing.iter() {
            let s = &self.music.iter().find(|(k, _)| *k == p.track).unwrap().1;
            set_sound_volume(s, (mv * p.fade).max(0.0));
        }
    }

    /// 0..1 phase within the current beat, plus beat index — drives the visual pulse.
    /// Current bass level 0..1 of the playing track (0 when nothing is playing).
    pub fn bass(&self) -> f32 {
        let Some(t) = self.current else { return 0.0 };
        let Some((_, env)) = self.bass.iter().find(|(k, _)| *k == t) else { return 0.0 };
        if env.is_empty() {
            return 0.0;
        }
        let el = (macroquad::time::get_time() - self.current_started).max(0.0) as f32;
        let i = (el * BASS_FPS) as usize;
        let i = if t.looped() { i % env.len() } else { i.min(env.len() - 1) };
        env[i]
    }

    pub fn beat(&self) -> (f32, u64) {
        match self.current {
            Some(t) => {
                let el = (macroquad::time::get_time() - self.current_started) as f32;
                let b = el * t.bpm() / 60.0;
                (b.fract(), b as u64)
            }
            None => (0.5, 0),
        }
    }
}
