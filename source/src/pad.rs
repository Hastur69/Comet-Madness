//! Game controller support via the `gilrs` crate, with per-controller calibration.
//!
//! Xbox-style pads work out of the box. DirectInput / USB pads (e.g. Sega Genesis,
//! Mega Drive and Saturn USB controllers) report their D-pad in many different
//! ways, so Options has a CALIBRATE CONTROLLER screen that records exactly which
//! raw input your pad sends for up/down/left/right/fire/pause. Calibrations are
//! saved per controller in %APPDATA%\cometmadness\controllers.txt.
//!
//! We deliberately read RAW inputs: gilrs' default filters convert D-pad axes into
//! button events that share one ID for both directions, and the bundled SDL
//! mappings can turn half-axes into buttons — both confuse generic pads.
//!
//! Call `pad::update()` once per frame, then query from anywhere.

use crate::persist;
use gilrs::{Axis, Button, EventType, GamepadId, Gilrs, GilrsBuilder};
use macroquad::prelude::*;
use std::cell::RefCell;

const DEADZONE: f32 = 0.35;
// Direct Windows joystick readings for USB pads whose layout gilrs cannot identify.
// No extra crate/dependency: this uses the same winmm API as joy.cpl.
#[cfg(windows)]
mod winjoy {
    use super::Raw;
    #[repr(C)]
    #[derive(Default)]
    struct JoyInfoEx {
        size: u32, flags: u32, x: u32, y: u32, z: u32, r: u32,
        u: u32, v: u32, buttons: u32, button_number: u32,
        pov: u32, reserved1: u32, reserved2: u32,
    }
    #[repr(C)]
    #[derive(Clone)]
    struct JoyCapsW {
        mid: u16, pid: u16, name: [u16; 32],
        xmin: u32, xmax: u32, ymin: u32, ymax: u32,
        zmin: u32, zmax: u32, num_buttons: u32, period_min: u32,
        period_max: u32, rmin: u32, rmax: u32,
        umin: u32, umax: u32, vmin: u32, vmax: u32,
        caps: u32, max_axes: u32, num_axes: u32, max_buttons: u32,
        reg_key: [u16; 32], oem_vxd: [u16; 260],
    }
    impl Default for JoyCapsW {
        fn default() -> Self { unsafe { std::mem::zeroed() } }
    }
    #[link(name = "winmm")]
    extern "system" {
        fn joyGetNumDevs() -> u32;
        fn joyGetDevCapsW(id: usize, caps: *mut JoyCapsW, size: u32) -> u32;
        fn joyGetPosEx(id: u32, info: *mut JoyInfoEx) -> u32;
    }
    pub struct Device {
        pub uuid: String,
        pub name: String,
        pub raw: Vec<Raw>,
    }
    fn normalized(v: u32, min: u32, max: u32) -> f32 {
        if max <= min { return 0.0; }
        ((v.saturating_sub(min) as f32 / (max - min) as f32) * 2.0 - 1.0).clamp(-1.0, 1.0)
    }
    pub fn devices() -> Vec<Device> {
        let mut out = Vec::new();
        let count = unsafe { joyGetNumDevs() }.min(32);
        for id in 0..count {
            let mut caps = JoyCapsW::default();
            if unsafe { joyGetDevCapsW(id as usize, &mut caps, std::mem::size_of::<JoyCapsW>() as u32) } != 0 {
                continue;
            }
            let mut info = JoyInfoEx { size: std::mem::size_of::<JoyInfoEx>() as u32, flags: 0xff, ..Default::default() };
            if unsafe { joyGetPosEx(id, &mut info) } != 0 { continue; }
            let len = caps.name.iter().position(|&x| x == 0).unwrap_or(caps.name.len());
            let name = String::from_utf16_lossy(&caps.name[..len]);
            // Model-specific key (rather than a transient joystick slot).
            let uuid = format!("winmm-{:04x}{:04x}-{}", caps.mid, caps.pid,
                name.as_bytes().iter().map(|b| format!("{:02x}", b)).collect::<String>());
            let mut raw = Vec::new();
            for b in 0..caps.num_buttons.min(32) {
                raw.push(Raw { code: 20_000 + b, is_axis: false, value: if info.buttons & (1 << b) != 0 { 1.0 } else { 0.0 } });
            }
            raw.push(Raw { code: 10_000, is_axis: true,
                value: normalized(info.x, caps.xmin, caps.xmax) });
            raw.push(Raw { code: 10_001, is_axis: true,
                value: normalized(info.y, caps.ymin, caps.ymax) });
            // POV in hundredths of a degree, 0=north, 9000=east.
            // Represent the hat as two signed axes so calibration works unmodified.
            let pov = if caps.caps & 0x10 == 0 || info.pov > 35999 {
                None
            } else { Some(info.pov as f32 / 100.0 * std::f32::consts::PI / 180.0) };
            raw.push(Raw { code: 10_006, is_axis: true, value: pov.map_or(0.0, |a| a.sin()) });
            raw.push(Raw { code: 10_007, is_axis: true, value: pov.map_or(0.0, |a| -a.cos()) });
            out.push(Device { uuid, name, raw });
        }
        out
    }
}
#[cfg(windows)]
fn raw_active(raw: &[Raw], src: Src) -> bool {
    match src {
        Src::Btn(code) => raw.iter().any(|r| !r.is_axis && r.code == code && r.value > 0.5),
        Src::Axis(code, sign, threshold) =>
            raw.iter().any(|r| r.is_axis && r.code == code && sign * (r.value - threshold) > 0.0),
    }
}
#[cfg(windows)]
fn known_gilrs(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    ["xbox", "xinput", "microsoft", "playstation", "dualshock",
     "dualsense", "sony", "nintendo", "switch", "8bitdo"]
        .iter().any(|s| n.contains(s))
}




/// The calibration steps, in order.
pub const STEPS: [&str; 6] = ["UP", "DOWN", "LEFT", "RIGHT", "FIRE", "PAUSE  (START)"];

/// One raw input source on a controller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Src {
    /// a button, by raw code
    Btn(u32),
    /// an axis, by raw code: active when `sign * (value - threshold) > 0`
    Axis(u32, f32, f32),
}

impl Src {
    pub fn describe(&self) -> String {
        match *self {
            Src::Btn(c) => format!("BUTTON {}", c & 0xffff),
            Src::Axis(c, s, _) => format!("AXIS {} {}", c & 0xffff, if s > 0.0 { "+" } else { "-" }),
        }
    }
    fn encode(&self) -> String {
        match *self {
            Src::Btn(c) => format!("b:{c}"),
            Src::Axis(c, s, t) => format!("a:{c}:{s}:{t}"),
        }
    }
    fn decode(tok: &str) -> Option<Src> {
        let p: Vec<&str> = tok.split(':').collect();
        match p.as_slice() {
            ["b", c] => Some(Src::Btn(c.parse().ok()?)),
            ["a", c, s, t] => Some(Src::Axis(c.parse().ok()?, s.parse().ok()?, t.parse().ok()?)),
            _ => None,
        }
    }
}

/// Saved bindings for one controller: up, down, left, right, fire, start.
#[derive(Clone, Debug)]
pub struct Calib {
    pub uuid: String,
    pub binds: [Option<Src>; 6],
}

/// A raw input reading, used by the calibration screen.
#[derive(Clone, Copy, Debug)]
pub struct Raw {
    pub code: u32,
    pub is_axis: bool,
    pub value: f32,
}

struct Pad {
    gilrs: Option<Gilrs>,
    calibs: Vec<Calib>,
    action_pressed: bool,
    start_pressed: bool,
    back_pressed: bool,
    dirs: [bool; 4],
    prev_dirs: [bool; 4],
    prev_start_axis: bool,
    stick: Vec2,
    fire: bool,
    connected: Option<String>,
    #[cfg(windows)]
    win_prev: Vec<Raw>,
    #[cfg(windows)]
    win_name: Option<String>,
}

fn load_calibs() -> Vec<Calib> {
    persist::load_pad_calibration()
        .lines()
        .filter_map(|l| {
            let t: Vec<&str> = l.split_whitespace().collect();
            if t.len() != 7 {
                return None;
            }
            let mut binds = [None; 6];
            for i in 0..6 {
                binds[i] = Src::decode(t[i + 1]);
            }
            Some(Calib { uuid: t[0].to_string(), binds })
        })
        .collect()
}

fn save_calibs(v: &[Calib]) {
    let txt: String = v
        .iter()
        .map(|c| {
            let toks: Vec<String> = c.binds.iter().map(|b| b.map(|s| s.encode()).unwrap_or_else(|| "-".into())).collect();
            format!("{} {}\n", c.uuid, toks.join(" "))
        })
        .collect();
    persist::save_pad_calibration(&txt);
}

thread_local! {
    static PAD: RefCell<Pad> = RefCell::new(Pad {
        // If controller support can't start, the game simply runs without it.
        gilrs: GilrsBuilder::new()
            .with_default_filters(false)
            .add_included_mappings(false)
            .add_env_mappings(false)
            .build()
            .ok(),
        calibs: load_calibs(),
        action_pressed: false,
        start_pressed: false,
        back_pressed: false,
        dirs: [false; 4],
        prev_dirs: [false; 4],
        prev_start_axis: false,
        stick: Vec2::ZERO,
        fire: false,
        connected: None,
        #[cfg(windows)]
        win_prev: Vec::new(),
        #[cfg(windows)]
        win_name: None,
    });
}

fn uuid_hex(u: [u8; 16]) -> String {
    u.iter().map(|b| format!("{b:02x}")).collect()
}

fn src_active(gp: &gilrs::Gamepad, src: Src) -> bool {
    let st = gp.state();
    match src {
        Src::Btn(code) => st.buttons().any(|(c, d)| c.into_u32() == code && d.is_pressed()),
        Src::Axis(code, sign, th) => st.axes().any(|(c, d)| c.into_u32() == code && sign * (d.value() - th) > 0.0),
    }
}

fn bound_button(c: &Calib, code: u32) -> Option<usize> {
    c.binds.iter().position(|b| *b == Some(Src::Btn(code)))
}

/// Poll the controllers. Call exactly once at the start of every frame.
pub fn update() {
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        let p = &mut *p;
        p.prev_dirs = p.dirs;
        let (mut action, mut start, mut back) = (false, false, false);
        let mut connected = None;
        let mut stick = Vec2::ZERO;
        let mut fire = false;
        let mut start_axis = false;
        #[cfg(windows)]
        let win = winjoy::devices().into_iter().next();
        #[cfg(windows)]
        let win_prev = p.win_prev.clone();
        #[cfg(windows)]
        let mut win_current = Vec::new();
        #[cfg(windows)]
        if let Some(ref device) = win {
            if p.win_name.as_deref() != Some(device.uuid.as_str()) {
                p.connected = Some(format!("{}  -  CALIBRATE IN OPTIONS IF NEEDED", device.name.to_uppercase()));
                p.win_name = Some(device.uuid.clone());
                p.win_prev.clear();
            }
            win_current = device.raw.clone();
        } else {
            p.win_name = None;
        }
        let calibs = &p.calibs;
        if let Some(g) = p.gilrs.as_mut() {
            // ---- events: menu presses
            while let Some(ev) = g.next_event() {
                let id: GamepadId = ev.id;
                #[cfg(windows)]
                if win.is_some() { continue; } // winmm exclusively owns the generic USB pad
                let cal = calibs.iter().find(|c| c.uuid == uuid_hex(g.gamepad(id).uuid()));
                match ev.event {
                    EventType::ButtonPressed(b, code) => {
                        let code = code.into_u32();
                        match cal.and_then(|c| bound_button(c, code)) {
                            Some(0..=3) => {}         // a calibrated direction: handled below
                            Some(5) => start = true,  // calibrated pause
                            Some(_) => action = true, // calibrated fire
                            None => match b {
                                Button::Start if cal.map_or(true, |c| c.binds[5].is_none()) => start = true,
                                Button::Select | Button::Mode => back = true,
                                Button::DPadUp | Button::DPadDown | Button::DPadLeft | Button::DPadRight => {}
                                _ => action = true,
                            },
                        }
                    }
                    EventType::Connected => {
                        let gp = g.gamepad(id);
                        let calibrated = calibs.iter().any(|c| c.uuid == uuid_hex(gp.uuid()));
                        connected = Some(format!(
                            "{}{}",
                            gp.name().to_uppercase(),
                            if calibrated { "  (CALIBRATED)" } else { "  -  CALIBRATE IN OPTIONS IF NEEDED" }
                        ));
                    }
                    _ => {}
                }
            }
            // ---- held state: movement / fire
            for (_, gp) in g.gamepads() {
                #[cfg(windows)]
                if win.is_some() { continue; }
                let uuid = uuid_hex(gp.uuid());
                if let Some(c) = calibs.iter().find(|c| c.uuid == uuid) {
                    let on = |i: usize| c.binds[i].map_or(false, |s| src_active(&gp, s));
                    let v = vec2((on(3) as i32 - on(2) as i32) as f32, (on(1) as i32 - on(0) as i32) as f32);
                    stick += v;
                    if on(4) {
                        fire = true;
                    }
                    if let Some(Src::Axis(..)) = c.binds[5] {
                        start_axis |= on(5);
                    }
                    // any other unbound button also fires
                    let bound: Vec<u32> = c.binds.iter().filter_map(|b| if let Some(Src::Btn(x)) = b { Some(*x) } else { None }).collect();
                    for (code, d) in gp.state().buttons() {
                        if d.is_pressed() && !bound.contains(&code.into_u32()) {
                            fire = true;
                        }
                    }
                    continue;
                }
                // Uncalibrated. Xbox-style pads (Windows "Gamepad", the ones with rumble) use
                // up = positive Y; generic HID pads report down = positive on their raw axes.
                let y_sign = if gp.is_ff_supported() { -1.0 } else { 1.0 };
                let mut v = vec2(gp.value(Axis::LeftStickX), y_sign * gp.value(Axis::LeftStickY));
                let len = v.length();
                v = if len < DEADZONE { Vec2::ZERO } else { v / len * ((len - DEADZONE) / (1.0 - DEADZONE)).min(1.0) };
                // hats / switches: up is positive
                let hx = gp.value(Axis::DPadX);
                let hy = -gp.value(Axis::DPadY);
                if hx.abs() > 0.5 {
                    v.x += hx.signum();
                }
                if hy.abs() > 0.5 {
                    v.y += hy.signum();
                }
                if gp.is_pressed(Button::DPadLeft) {
                    v.x -= 1.0;
                }
                if gp.is_pressed(Button::DPadRight) {
                    v.x += 1.0;
                }
                if gp.is_pressed(Button::DPadUp) {
                    v.y -= 1.0;
                }
                if gp.is_pressed(Button::DPadDown) {
                    v.y += 1.0;
                }
                stick += v;
                let excluded: Vec<u32> = [
                    Button::Start,
                    Button::Select,
                    Button::Mode,
                    Button::DPadUp,
                    Button::DPadDown,
                    Button::DPadLeft,
                    Button::DPadRight,
                ]
                .iter()
                .filter_map(|&b| gp.button_code(b).map(|c| c.into_u32()))
                .collect();
                for (code, d) in gp.state().buttons() {
                    if d.is_pressed() && !excluded.contains(&code.into_u32()) {
                        fire = true;
                    }
                }
            }
        }
        #[cfg(windows)]
        if let Some(device) = win {
            let raw = &win_current;
            // Exactly as in Pumpkin King: -X left, +X right, -Y up, +Y down;
            // hat in hundredths of a degree, centred = no direction.
            let axis = |code| raw.iter().find(|r| r.is_axis && r.code == code).map_or(0.0, |r| r.value);
            let x = if axis(10_006).abs() > 0.5 { axis(10_006) } else { axis(10_000) };
            let y = if axis(10_007).abs() > 0.5 { axis(10_007) } else { axis(10_001) };
            stick += vec2(if x.abs() > DEADZONE { x } else { 0.0 },
                          if y.abs() > DEADZONE { y } else { 0.0 });
            if let Some(c) = calibs.iter().find(|c| c.uuid == device.uuid) {
                // Use calibrated action buttons; ignore historical direction bindings.
                let on = |i: usize| c.binds[i].map_or(false, |s| raw_active(raw, s));
                let was = |i: usize| c.binds[i].map_or(false, |s| raw_active(&win_prev, s));
                fire |= on(4);
                if on(4) && !was(4) { action = true; }
                if on(5) && !was(5) { start = true; }
            } else {
                // Calibrate buttons to avoid guessing the pad's Start/fire numbering.
                // Directions work immediately through winmm without calibration.
                // Do not infer action from arbitrary buttons (would misread directions).
            }
            p.win_prev = win_current;
        } else {
            p.win_prev.clear();
        }
        if start_axis && !p.prev_start_axis {
            start = true;
        }
        p.prev_start_axis = start_axis;
        p.stick = vec2(stick.x.clamp(-1.0, 1.0), stick.y.clamp(-1.0, 1.0));
        p.dirs = [p.stick.y < -0.5, p.stick.y > 0.5, p.stick.x < -0.5, p.stick.x > 0.5];
        p.action_pressed = action;
        p.start_pressed = start;
        p.back_pressed = back;
        p.fire = fire;
        if connected.is_some() {
            p.connected = connected;
        }
    });
}

/// Movement from the stick / D-pad, each axis -1..1 (screen coordinates, +y = down).
pub fn stick() -> Vec2 {
    PAD.with(|p| p.borrow().stick)
}

/// Fire: any action button held.
pub fn fire_held() -> bool {
    PAD.with(|p| p.borrow().fire)
}

/// Any action (non-Start, non-direction) button pressed this frame — menu "OK".
pub fn confirm() -> bool {
    PAD.with(|p| p.borrow().action_pressed)
}

/// Start / pause pressed this frame (pause in game, OK in menus).
pub fn start() -> bool {
    PAD.with(|p| p.borrow().start_pressed)
}

/// Select / Mode pressed this frame — menu "back".
pub fn back() -> bool {
    PAD.with(|p| p.borrow().back_pressed)
}

/// Anything at all pressed this frame.
pub fn any_pressed() -> bool {
    PAD.with(|p| {
        let p = p.borrow();
        p.action_pressed || p.start_pressed || p.back_pressed || (0..4).any(|i| p.dirs[i] && !p.prev_dirs[i])
    })
}

/// Name of a controller that connected since the last call (for an on-screen notice).
pub fn take_connected() -> Option<String> {
    PAD.with(|p| p.borrow_mut().connected.take())
}

fn dir_pressed(i: usize) -> bool {
    PAD.with(|p| {
        let p = p.borrow();
        p.dirs[i] && !p.prev_dirs[i]
    })
}
pub fn menu_up() -> bool {
    dir_pressed(0)
}
pub fn menu_down() -> bool {
    dir_pressed(1)
}
pub fn menu_left() -> bool {
    dir_pressed(2)
}
pub fn menu_right() -> bool {
    dir_pressed(3)
}

// ------------------------------------------------------------------ calibration support

/// The first connected controller: (uuid, display name, calibrated?).
pub fn first_pad() -> Option<(String, String, bool)> {
    PAD.with(|p| {
        let p = p.borrow();
        #[cfg(windows)]
        if let Some(d) = winjoy::devices().into_iter().next() {
            let cal = p.calibs.iter().any(|c| c.uuid == d.uuid);
            return Some((d.uuid, d.name.to_uppercase(), cal));
        }
        let g = p.gilrs.as_ref()?;
        let (_, gp) = g.gamepads().next()?;
        let uuid = uuid_hex(gp.uuid());
        let cal = p.calibs.iter().any(|c| c.uuid == uuid);
        Some((uuid, gp.name().to_uppercase(), cal))
    })
}

/// Every raw button and axis the controller with this uuid has reported so far.
pub fn raw_snapshot(uuid: &str) -> Vec<Raw> {
    PAD.with(|p| {
        let p = p.borrow();
        let mut v = vec![];
        #[cfg(windows)]
        if let Some(d) = winjoy::devices().into_iter().find(|d| d.uuid == uuid) {
            return d.raw;
        }
        if let Some(g) = p.gilrs.as_ref() {
            for (_, gp) in g.gamepads() {
                if uuid_hex(gp.uuid()) != uuid {
                    continue;
                }
                for (code, d) in gp.state().buttons() {
                    v.push(Raw { code: code.into_u32(), is_axis: false, value: if d.is_pressed() { 1.0 } else { 0.0 } });
                }
                for (code, d) in gp.state().axes() {
                    v.push(Raw { code: code.into_u32(), is_axis: true, value: d.value() });
                }
            }
        }
        v
    })
}

/// Compare a snapshot against the resting snapshot and return the input that moved, if any.
/// Buttons win over axes; for axes the biggest movement wins.
pub fn detect(rest: &[Raw], now: &[Raw]) -> Option<Src> {
    let rest_of = |code: u32, is_axis: bool| rest.iter().find(|r| r.code == code && r.is_axis == is_axis).map_or(0.0, |r| r.value);
    for r in now.iter().filter(|r| !r.is_axis) {
        if r.value > 0.5 && rest_of(r.code, false) < 0.5 {
            return Some(Src::Btn(r.code));
        }
    }
    let mut best: Option<(f32, Src)> = None;
    for r in now.iter().filter(|r| r.is_axis) {
        let base = rest_of(r.code, true);
        let d = r.value - base;
        if d.abs() > 0.5 && best.map_or(true, |(b, _)| d.abs() > b) {
            best = Some((d.abs(), Src::Axis(r.code, d.signum(), base + d * 0.5)));
        }
    }
    best.map(|(_, s)| s)
}

/// Is this source still active in the given snapshot? (used to wait for release)
pub fn still_active(src: Src, now: &[Raw]) -> bool {
    match src {
        Src::Btn(code) => now.iter().any(|r| !r.is_axis && r.code == code && r.value > 0.5),
        Src::Axis(code, sign, th) => now.iter().any(|r| r.is_axis && r.code == code && sign * (r.value - th) > 0.0),
    }
}

/// Store (and save) a calibration, replacing any previous one for that controller.
pub fn set_calibration(c: Calib) {
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        p.calibs.retain(|x| x.uuid != c.uuid);
        p.calibs.push(c);
        save_calibs(&p.calibs);
    });
}

/// Forget the calibration for a controller (back to automatic).
pub fn clear_calibration(uuid: &str) {
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        p.calibs.retain(|x| x.uuid != uuid);
        save_calibs(&p.calibs);
    });
}
