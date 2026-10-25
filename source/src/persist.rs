//! High-score table and settings, stored as small text files in %APPDATA%\cometmadness
//! (falls back to the folder the executable lives in).

use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub score: u64,
    pub level: u32,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub crt: bool,
    pub fullscreen: bool,
    pub screen_shake: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { music_volume: 0.7, sfx_volume: 0.8, crt: true, fullscreen: false, screen_shake: true }
    }
}

fn data_dir() -> PathBuf {
    if let Ok(app) = std::env::var("APPDATA") {
        let p = PathBuf::from(app).join("cometmadness");
        if std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".cometmadness");
        if std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_scores() -> Vec<Entry> {
    // A little roll-call of the people and places behind the 1985 original.
    let seed = [
        ("ROB", 50000, 10), // Rob Hubbard — composer
        ("SIM", 40000, 8),  // Simon Nicol — programmer
        ("MTC", 30000, 6),  // Martech
        ("ZAP", 25000, 5),  // Zzap!64
        ("WER", 23956, 5),  // the C64-Wiki's listed record holder
        ("GOT", 20000, 4),  // Gottlieb (Mad Planets)
        ("KAN", 15000, 3),  // Kan Yabumoto — Mad Planets designer
        ("SID", 10000, 2),  // the chip that made it sing
        ("C64", 7500, 2),
        ("NOV", 5000, 1),
    ];
    seed.iter().map(|&(n, s, l)| Entry { name: n.to_string(), score: s, level: l }).collect()
}

pub fn load_scores() -> Vec<Entry> {
    let path = data_dir().join("scores.txt");
    let mut v: Vec<Entry> = match std::fs::read_to_string(&path) {
        Ok(txt) => txt
            .lines()
            .filter_map(|l| {
                let mut p = l.split('\t');
                let name = p.next()?.trim().to_string();
                let score = p.next()?.trim().parse().ok()?;
                let level = p.next()?.trim().parse().ok()?;
                Some(Entry { name, score, level })
            })
            .collect(),
        Err(_) => vec![],
    };
    if v.is_empty() {
        v = default_scores();
    }
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v
}

pub fn save_scores(v: &[Entry]) {
    let path = data_dir().join("scores.txt");
    let txt: String = v.iter().map(|e| format!("{}\t{}\t{}\n", e.name, e.score, e.level)).collect();
    let _ = std::fs::write(path, txt);
}

pub fn qualifies(v: &[Entry], score: u64) -> bool {
    score > 0 && (v.len() < 10 || v.iter().any(|e| score > e.score))
}

pub fn insert_score(v: &mut Vec<Entry>, e: Entry) -> usize {
    v.push(e.clone());
    v.sort_by(|a, b| b.score.cmp(&a.score));
    v.truncate(10);
    v.iter().position(|x| x.name == e.name && x.score == e.score).unwrap_or(0)
}

pub fn load_settings() -> Settings {
    let path = data_dir().join("settings.txt");
    let mut s = Settings::default();
    if let Ok(txt) = std::fs::read_to_string(path) {
        for line in txt.lines() {
            let mut kv = line.splitn(2, '=');
            let (k, v) = (kv.next().unwrap_or("").trim(), kv.next().unwrap_or("").trim());
            match k {
                "music_volume" => s.music_volume = v.parse().unwrap_or(s.music_volume),
                "sfx_volume" => s.sfx_volume = v.parse().unwrap_or(s.sfx_volume),
                "crt" => s.crt = v == "true",
                "fullscreen" => s.fullscreen = v == "true",
                "screen_shake" => s.screen_shake = v == "true",
                _ => {}
            }
        }
    }
    s.music_volume = s.music_volume.clamp(0.0, 1.0);
    s.sfx_volume = s.sfx_volume.clamp(0.0, 1.0);
    s
}

pub fn save_settings(s: &Settings) {
    let path = data_dir().join("settings.txt");
    let txt = format!(
        "music_volume={}\nsfx_volume={}\ncrt={}\nfullscreen={}\nscreen_shake={}\n",
        s.music_volume, s.sfx_volume, s.crt, s.fullscreen, s.screen_shake
    );
    let _ = std::fs::write(path, txt);
}

pub fn write_crash_log(msg: &str) {
    use std::io::Write;
    let path = data_dir().join("crash.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(msg.as_bytes());
    }
}
