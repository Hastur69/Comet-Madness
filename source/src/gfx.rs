//! Rendering infrastructure: shaders, procedural textures, text and neon helpers.

use crate::util::*;
use macroquad::miniquad::{BlendFactor, BlendState, BlendValue, Equation};
use macroquad::prelude::*;

const VERTEX: &str = r#"#version 100
precision mediump float;
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying vec2 uv;
varying vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1.0);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

const ADD_FRAG: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
void main() {
    gl_FragColor = texture2D(Texture, uv) * color;
}
"#;

/// Renders a lit, rotating sphere from an equirectangular map onto a quad.
const PLANET_FRAG: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform float rot;
uniform float flash;
uniform float fog;
uniform vec4 tint;
uniform vec4 rim;
// Polynomial atan2/asin and no pow(): some GL stacks (seen with Mesa under a core
// profile) crash on pow() in GLSL ES 1.00 shaders, so the planet shader avoids them.
float atan2_approx(float y, float x) {
    float ax = abs(x);
    float ay = abs(y);
    float a = min(ax, ay) / (max(ax, ay) + 1e-6);
    float s = a * a;
    float r = ((-0.0464964749 * s + 0.15931422) * s - 0.327622764) * s * a + a;
    if (ay > ax) r = 1.57079637 - r;
    if (x < 0.0) r = 3.14159274 - r;
    if (y < 0.0) r = -r;
    return r;
}
float asin_approx(float x) {
    // asin(x) ~= pi/2 - sqrt(1-x)*(1.5707288 - 0.2121144x + 0.0742610x^2 - 0.0187293x^3), x>=0
    float ax = abs(x);
    float r = 1.5707288 + ax * (-0.2121144 + ax * (0.0742610 - 0.0187293 * ax));
    r = 1.57079637 - sqrt(max(1.0 - ax, 0.0)) * r;
    return x < 0.0 ? -r : r;
}
void main() {
    vec2 p = uv * 2.0 - 1.0;
    float r2 = dot(p, p);
    // no `discard`: pixels outside the disc simply get zero alpha
    float edge = clamp((1.0 - r2) / 0.06, 0.0, 1.0);
    float z = sqrt(max(1.0 - r2, 0.0001));
    vec3 n = normalize(vec3(p.x, p.y, z));
    float lon = atan2_approx(n.x, n.z) / 6.2831853 + rot;
    float lat = asin_approx(clamp(n.y, -0.999, 0.999)) / 3.14159265 + 0.5;
    vec3 tex = texture2D(Texture, vec2(fract(lon), clamp(lat, 0.002, 0.998))).rgb;
    vec3 L = normalize(vec3(-0.55, -0.55, 0.65));
    float diff = clamp(dot(n, L), 0.0, 1.0);
    vec3 col = tex * tint.rgb * (0.16 + diff * 1.15);
    float sd = clamp(dot(reflect(-L, n), vec3(0.0, 0.0, 1.0)), 0.0, 1.0);
    float sd2 = sd * sd; float sd4 = sd2 * sd2; float sd8 = sd4 * sd4;
    float spec = sd8 * sd8 * sd8 * 0.35;
    col += vec3(spec);
    float fz = clamp(1.0 - z, 0.0, 1.0);
    float fres = fz * fz * sqrt(sqrt(fz));
    col += rim.rgb * fres * 1.6;
    col = mix(col, vec3(1.0, 1.0, 1.0), flash);
    col = mix(col, tint.rgb * 0.3 + rim.rgb * 0.15, fog);
    gl_FragColor = vec4(col, edge * color.a);
}
"#;

/// Bright-pass + blur used while downsampling the scene for bloom.
const BLOOM_EXTRACT_FRAG: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 texel;
uniform float threshold;
void main() {
    vec3 acc = vec3(0.0);
    float wsum = 0.0;
    for (int x = -2; x <= 2; x++) {
        for (int y = -2; y <= 2; y++) {
            vec2 o = vec2(float(x), float(y));
            float w = exp(-dot(o, o) * 0.25);
            vec3 c = texture2D(Texture, uv + o * texel).rgb;
            float l = max(c.r, max(c.g, c.b));
            acc += c * smoothstep(threshold, threshold + 0.45, l) * w;
            wsum += w;
        }
    }
    gl_FragColor = vec4(acc / wsum, 1.0);
}
"#;

const BLUR_FRAG: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform vec2 texel;
void main() {
    vec3 acc = vec3(0.0);
    float wsum = 0.0;
    for (int x = -3; x <= 3; x++) {
        for (int y = -3; y <= 3; y++) {
            vec2 o = vec2(float(x), float(y));
            float w = exp(-dot(o, o) * 0.18);
            acc += texture2D(Texture, uv + o * texel).rgb * w;
            wsum += w;
        }
    }
    gl_FragColor = vec4(acc / wsum, 1.0);
}
"#;

const COMPOSITE_FRAG: &str = r#"#version 100
precision mediump float;
varying vec2 uv;
varying vec4 color;
uniform sampler2D Texture;
uniform sampler2D BloomA;
uniform sampler2D BloomB;
uniform float crt;
uniform float aberr;
uniform float pulse;
uniform float flash_amt;
uniform vec4 flash_col;
uniform float time;
void main() {
    vec2 q = uv;
    vec2 d = q - 0.5;
    // branch-free CRT: barrel warp, scanlines and flicker are blended in by `crt`
    q = q + d * dot(d, d) * 0.07 * crt;
    vec2 off = d * (aberr + crt * 0.0025);
    vec3 c;
    c.r = texture2D(Texture, q + off).r;
    c.g = texture2D(Texture, q).g;
    c.b = texture2D(Texture, q - off).b;
    vec3 bloom = texture2D(BloomA, q).rgb * 0.85 + texture2D(BloomB, q).rgb * 1.35;
    c += bloom * (1.0 + pulse * 0.4);
    float tri = abs(fract(q.y * 540.0) * 2.0 - 1.0);
    float sl = 0.9 + 0.1 * tri;
    c *= mix(1.0, sl * time, crt);
    float vig = smoothstep(0.92, 0.3, length(d * vec2(1.0, 0.85)));
    c *= mix(0.45, 1.0, vig);
    c = mix(c, flash_col.rgb, flash_amt);
    if (q.x < 0.0 || q.x > 1.0 || q.y < 0.0 || q.y > 1.0) c = vec3(0.0);
    // gentle filmic tonemap
    c = c / (1.0 + c * 0.12);
    gl_FragColor = vec4(c, 1.0);
}
"#;

fn additive_blend() -> PipelineParams {
    PipelineParams {
        color_blend: Some(BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One)),
        alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
        ..Default::default()
    }
}
fn alpha_blend() -> PipelineParams {
    PipelineParams {
        color_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::Value(BlendValue::SourceAlpha),
            BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
        )),
        alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
        ..Default::default()
    }
}

pub struct Gfx {
    pub additive: Material,
    pub planet: Material,
    pub bloom_extract: Material,
    pub blur: Material,
    pub composite: Material,
    pub glow: Texture2D,
    pub spark: Texture2D,
    pub planet_maps: Vec<Texture2D>,
    pub nebula: Texture2D,
    pub scene: RenderTarget,
    pub bloom_a: RenderTarget,
    pub bloom_b: RenderTarget,
    pub font: Font,
    pub font_black: Font,
}

fn mat(frag: &str, params: PipelineParams, uniforms: Vec<UniformDesc>, textures: Vec<String>) -> Material {
    load_material(
        ShaderSource::Glsl { vertex: VERTEX, fragment: frag },
        MaterialParams { pipeline_params: params, uniforms, textures },
    )
    .unwrap_or_else(|e| panic!("shader compile failed: {e:?}"))
}

impl Gfx {
    pub fn new() -> Self {
        let additive = mat(ADD_FRAG, additive_blend(), vec![], vec![]);
        let planet = mat(
            PLANET_FRAG,
            alpha_blend(),
            vec![
                UniformDesc::new("rot", UniformType::Float1),
                UniformDesc::new("flash", UniformType::Float1),
                UniformDesc::new("fog", UniformType::Float1),
                UniformDesc::new("tint", UniformType::Float4),
                UniformDesc::new("rim", UniformType::Float4),
            ],
            vec![],
        );
        let bloom_extract = mat(
            BLOOM_EXTRACT_FRAG,
            PipelineParams::default(),
            vec![UniformDesc::new("texel", UniformType::Float2), UniformDesc::new("threshold", UniformType::Float1)],
            vec![],
        );
        let blur = mat(BLUR_FRAG, PipelineParams::default(), vec![UniformDesc::new("texel", UniformType::Float2)], vec![]);
        let composite = mat(
            COMPOSITE_FRAG,
            PipelineParams::default(),
            vec![
                UniformDesc::new("crt", UniformType::Float1),
                UniformDesc::new("aberr", UniformType::Float1),
                UniformDesc::new("pulse", UniformType::Float1),
                UniformDesc::new("flash_amt", UniformType::Float1),
                UniformDesc::new("flash_col", UniformType::Float4),
                UniformDesc::new("time", UniformType::Float1),
            ],
            vec!["BloomA".to_string(), "BloomB".to_string()],
        );

        let scene = render_target(RW, RH);
        scene.texture.set_filter(FilterMode::Linear);
        let bloom_a = render_target(RW / 4, RH / 4);
        bloom_a.texture.set_filter(FilterMode::Linear);
        let bloom_b = render_target(RW / 8, RH / 8);
        bloom_b.texture.set_filter(FilterMode::Linear);

        let font = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Orbitron-700.ttf")).expect("font");
        let font_black = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/Orbitron-900.ttf")).expect("font");
        // Pre-build every glyph size we will ever use so the font atlas never has to
        // grow mid-frame (growing it while draws are batched corrupts glyph UVs).
        let chars = Font::ascii_character_list();
        for &px in FONT_SIZES.iter() {
            font_black.populate_font_cache(&chars, px);
            if px <= REGULAR_MAX {
                font.populate_font_cache(&chars, px);
            }
        }

        Gfx {
            additive,
            planet,
            bloom_extract,
            blur,
            composite,
            glow: gen_glow(128, 2.2),
            spark: gen_glow(32, 4.0),
            planet_maps: (0..6).map(gen_planet_map).collect(),
            nebula: gen_nebula(),
            scene,
            bloom_a,
            bloom_b,
            font,
            font_black,
        }
    }

    /// Camera mapping the virtual 1280x720 field onto a render target.
    pub fn rt_camera(rt: &RenderTarget, offset: Vec2) -> Camera2D {
        let mut c = Camera2D::from_display_rect(Rect::new(offset.x, offset.y, VW, VH));
        c.render_target = Some(rt.clone());
        // Render targets are stored bottom-up; flip so the final blit is upright.
        c.zoom.y = -c.zoom.y;
        c
    }

    pub fn begin_scene(&self, shake: Vec2) {
        set_camera(&Self::rt_camera(&self.scene, shake));
        clear_background(Color::new(0.01, 0.005, 0.03, 1.0));
    }

    /// Runs bloom passes and composites the scene to the window with letterboxing.
    pub fn present(&self, crt: bool, aberr: f32, pulse: f32, flash: f32, flash_col: Color) {
        // bloom extract
        set_camera(&Self::rt_camera(&self.bloom_a, Vec2::ZERO));
        clear_background(BLACK);
        self.bloom_extract.set_uniform("texel", vec2(1.0 / RW as f32 * 2.0, 1.0 / RH as f32 * 2.0));
        self.bloom_extract.set_uniform("threshold", 0.42f32);
        gl_use_material(&self.bloom_extract);
        draw_texture_ex(&self.scene.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(VW, VH)), ..Default::default() });
        gl_use_default_material();
        // blur
        set_camera(&Self::rt_camera(&self.bloom_b, Vec2::ZERO));
        clear_background(BLACK);
        self.blur.set_uniform("texel", vec2(4.0 / RW as f32 * 1.5, 4.0 / RH as f32 * 1.5));
        gl_use_material(&self.blur);
        draw_texture_ex(&self.bloom_a.texture, 0.0, 0.0, WHITE, DrawTextureParams { dest_size: Some(vec2(VW, VH)), ..Default::default() });
        gl_use_default_material();

        // composite to screen
        set_default_camera();
        clear_background(BLACK);
        let sw = screen_width();
        let sh = screen_height();
        let s = (sw / VW).min(sh / VH);
        let (w, h) = (VW * s, VH * s);
        let (x, y) = ((sw - w) * 0.5, (sh - h) * 0.5);
        self.composite.set_texture("BloomA", self.bloom_a.texture.clone());
        self.composite.set_texture("BloomB", self.bloom_b.texture.clone());
        self.composite.set_uniform("crt", if crt { 1.0f32 } else { 0.0 });
        self.composite.set_uniform("aberr", aberr);
        self.composite.set_uniform("pulse", pulse);
        self.composite.set_uniform("flash_amt", flash.clamp(0.0, 1.0));
        self.composite.set_uniform("flash_col", flash_col.to_vec());
        // `time` carries the CRT flicker factor (computed on the CPU)
        let flicker = 0.97 + 0.03 * ((get_time() * 60.0).sin() as f32);
        self.composite.set_uniform("time", flicker);
        gl_use_material(&self.composite);
        draw_texture_ex(&self.scene.texture, x, y, WHITE, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
        gl_use_default_material();
    }

    // ------------------------------------------------------------- drawing helpers

    pub fn glow_at(&self, p: Vec2, radius: f32, c: Color) {
        let d = radius * 2.0;
        draw_texture_ex(&self.glow, p.x - radius, p.y - radius, c, DrawTextureParams { dest_size: Some(vec2(d, d)), ..Default::default() });
    }

    /// Stretched glow aligned to a direction (for bullets, streaks).
    pub fn streak(&self, p: Vec2, dir_angle: f32, length: f32, width: f32, c: Color) {
        draw_texture_ex(
            &self.spark,
            p.x - length * 0.5,
            p.y - width * 0.5,
            c,
            DrawTextureParams { dest_size: Some(vec2(length, width)), rotation: dir_angle, ..Default::default() },
        );
    }

    pub fn planet(&self, map: usize, p: Vec2, r: f32, rot: f32, tint: Color, rim: Color, flash: f32, fog: f32, alpha: f32) {
        self.planet.set_uniform("rot", rot);
        self.planet.set_uniform("flash", flash);
        self.planet.set_uniform("fog", fog);
        self.planet.set_uniform("tint", tint.to_vec());
        self.planet.set_uniform("rim", rim.to_vec());
        gl_use_material(&self.planet);
        draw_texture_ex(
            &self.planet_maps[map % self.planet_maps.len()],
            p.x - r,
            p.y - r,
            Color::new(1.0, 1.0, 1.0, alpha),
            DrawTextureParams { dest_size: Some(vec2(r * 2.0, r * 2.0)), ..Default::default() },
        );
        gl_use_default_material();
    }

    pub fn text_size(&self, s: &str, size: f32, black: bool) -> TextDimensions {
        let f = if black { &self.font_black } else { &self.font };
        let px = quant_px(size, black);
        measure_text(s, Some(f), px, size / px as f32)
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, c: Color, black: bool) {
        let f = if black { &self.font_black } else { &self.font };
        let px = quant_px(size, black);
        draw_text_ex(s, x, y, TextParams { font: Some(f), font_size: px, font_scale: size / px as f32, color: c, ..Default::default() });
    }

    pub fn text_center(&self, s: &str, cx: f32, y: f32, size: f32, c: Color, black: bool) {
        let d = self.text_size(s, size, black);
        self.text(s, cx - d.width * 0.5, y, size, c, black);
    }

    pub fn text_right(&self, s: &str, rx: f32, y: f32, size: f32, c: Color, black: bool) {
        let d = self.text_size(s, size, black);
        self.text(s, rx - d.width, y, size, c, black);
    }

    /// Neon text: soft additive halo behind crisp text.
    pub fn neon_text_center(&self, s: &str, cx: f32, y: f32, size: f32, c: Color, black: bool) {
        let d = self.text_size(s, size, black);
        gl_use_material(&self.additive);
        let pad = size * 0.6;
        draw_texture_ex(
            &self.glow,
            cx - d.width * 0.5 - pad,
            y - d.offset_y - pad * 0.7,
            with_alpha(c, 0.22),
            DrawTextureParams { dest_size: Some(vec2(d.width + pad * 2.0, d.height + pad * 1.4)), ..Default::default() },
        );
        gl_use_default_material();
        self.text(s, cx - d.width * 0.5, y, size, c, black);
    }
}

/// Glyph sizes (in render-target pixels) baked into the font atlases.
pub const FONT_SIZES: [u16; 14] = [14, 18, 22, 26, 30, 36, 42, 50, 60, 72, 84, 100, 120, 140];
const REGULAR_MAX: u16 = 50;

/// Pick the baked glyph size closest to (and preferably above) the wanted size.
fn quant_px(size: f32, black: bool) -> u16 {
    let want = size * RSCALE;
    let max = if black { 140 } else { REGULAR_MAX };
    for &px in FONT_SIZES.iter() {
        if px > max {
            break;
        }
        if px as f32 >= want * 0.92 {
            return px;
        }
    }
    max
}

// ------------------------------------------------------------------ procedural textures

fn gen_glow(size: u16, power: f32) -> Texture2D {
    let mut img = Image::gen_image_color(size, size, Color::new(1.0, 1.0, 1.0, 0.0));
    let c = (size as f32 - 1.0) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 - c) / c;
            let dy = (y as f32 - c) / c;
            let d = (dx * dx + dy * dy).sqrt();
            let a = (1.0 - d).max(0.0).powf(power);
            img.set_pixel(x as u32, y as u32, Color::new(1.0, 1.0, 1.0, a));
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Linear);
    t
}

fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374761393)
        ^ (y as u32).wrapping_mul(668265263)
        ^ (z as u32).wrapping_mul(2147483647)
        ^ seed.wrapping_mul(1274126177);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^= h >> 16;
    (h & 0xffffff) as f32 / 16777215.0
}

fn vnoise(p: Vec3, seed: u32) -> f32 {
    let i = p.floor();
    let f = p - i;
    let u = f * f * (Vec3::splat(3.0) - 2.0 * f);
    let (ix, iy, iz) = (i.x as i32, i.y as i32, i.z as i32);
    let mut v = [0.0f32; 8];
    for k in 0..8 {
        v[k] = hash3(ix + (k & 1) as i32, iy + ((k >> 1) & 1) as i32, iz + ((k >> 2) & 1) as i32, seed);
    }
    let x00 = lerp(v[0], v[1], u.x);
    let x10 = lerp(v[2], v[3], u.x);
    let x01 = lerp(v[4], v[5], u.x);
    let x11 = lerp(v[6], v[7], u.x);
    lerp(lerp(x00, x10, u.y), lerp(x01, x11, u.y), u.z)
}

fn fbm(p: Vec3, oct: u32, seed: u32) -> f32 {
    let mut a = 0.5;
    let mut f = 1.0;
    let mut s = 0.0;
    let mut n = 0.0;
    for o in 0..oct {
        s += a * vnoise(p * f, seed + o * 17);
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    s / n
}

fn ramp(stops: &[(f32, [f32; 3])], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        let (a, ca) = w[0];
        let (b, cb) = w[1];
        if t <= b {
            let k = ((t - a) / (b - a)).clamp(0.0, 1.0);
            return [lerp(ca[0], cb[0], k), lerp(ca[1], cb[1], k), lerp(ca[2], cb[2], k)];
        }
    }
    stops.last().unwrap().1
}

/// Equirectangular planet surfaces. Style 0 is an Earth-like world — a nod to
/// Zzap!64 remarking that the original comets resembled the Earth's surface.
fn gen_planet_map(style: usize) -> Texture2D {
    let (w, h) = (256u16, 128u16);
    let mut img = Image::gen_image_color(w, h, BLACK);
    let seed = 1000 + style as u32 * 7919;
    for y in 0..h {
        let lat = (y as f32 / (h - 1) as f32 - 0.5) * std::f32::consts::PI;
        for x in 0..w {
            let lon = x as f32 / w as f32 * std::f32::consts::TAU;
            let p = vec3(lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin());
            let rgb = match style {
                0 => {
                    let e = fbm(p * 2.2, 6, seed);
                    let cl = fbm(p * 3.5 + vec3(9.0, 0.0, 0.0), 5, seed + 99);
                    let mut c = if e < 0.5 {
                        ramp(&[(0.0, [0.02, 0.06, 0.25]), (0.5, [0.1, 0.35, 0.75])], e / 0.5)
                    } else {
                        ramp(&[(0.5, [0.85, 0.8, 0.5]), (0.56, [0.2, 0.6, 0.2]), (0.7, [0.35, 0.45, 0.15]), (0.85, [0.5, 0.4, 0.3]), (1.0, [1.0, 1.0, 1.0])], e)
                    };
                    let ice = (lat.abs() - 1.2).max(0.0) * 4.0;
                    let cw = ((cl - 0.55) * 3.0).clamp(0.0, 0.85) + ice.min(1.0);
                    for k in 0..3 {
                        c[k] = lerp(c[k], 1.0, cw.min(1.0));
                    }
                    c
                }
                1 => {
                    let e = fbm(p * 3.0, 6, seed);
                    let cr = (1.0 - (fbm(p * 6.0, 4, seed + 5) - 0.5).abs() * 9.0).max(0.0);
                    let mut c = ramp(&[(0.0, [0.05, 0.02, 0.02]), (0.6, [0.25, 0.08, 0.05]), (1.0, [0.4, 0.15, 0.1])], e);
                    c[0] += cr * 1.0;
                    c[1] += cr * 0.45;
                    c[2] += cr * 0.05;
                    c
                }
                2 => {
                    let e = fbm(p * 2.8, 6, seed);
                    ramp(&[(0.0, [0.3, 0.55, 0.8]), (0.5, [0.75, 0.9, 1.0]), (0.7, [1.0, 1.0, 1.0]), (1.0, [0.6, 0.85, 1.0])], e)
                }
                3 => {
                    let warp = fbm(p * 2.0, 4, seed) * 2.5;
                    let b = ((lat * 7.0 + warp).sin() * 0.5 + 0.5) * 0.8 + fbm(p * 8.0, 3, seed + 3) * 0.2;
                    ramp(&[(0.0, [0.55, 0.3, 0.15]), (0.35, [0.9, 0.7, 0.45]), (0.6, [0.98, 0.92, 0.8]), (0.8, [0.7, 0.4, 0.25]), (1.0, [0.95, 0.85, 0.6])], b)
                }
                4 => {
                    let e = fbm(p * 3.2, 6, seed);
                    let s = ((fbm(p * 1.5, 3, seed + 8) * 12.0).sin() * 0.5 + 0.5) * 0.3;
                    ramp(&[(0.0, [0.15, 0.02, 0.25]), (0.45, [0.3, 0.8, 0.2]), (0.7, [0.7, 1.0, 0.3]), (1.0, [0.2, 0.1, 0.4])], e + s - 0.15)
                }
                _ => {
                    let e = fbm(p * 4.0, 5, seed);
                    let f = ((e * 14.0).fract() - 0.5).abs() * 2.0;
                    ramp(&[(0.0, [0.9, 0.2, 0.8]), (0.5, [0.3, 0.1, 0.6]), (1.0, [0.2, 0.9, 1.0])], f * 0.7 + e * 0.3)
                }
            };
            img.set_pixel(x as u32, y as u32, Color::new(rgb[0].min(1.0), rgb[1].min(1.0), rgb[2].min(1.0), 1.0));
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Linear);
    t
}

fn gen_nebula() -> Texture2D {
    let (w, h) = (480u16, 270u16);
    let mut img = Image::gen_image_color(w, h, BLACK);
    for y in 0..h {
        for x in 0..w {
            let p = vec3(x as f32 / 120.0, y as f32 / 120.0, 0.5);
            let q = vec3(fbm(p, 4, 7), fbm(p + vec3(5.2, 1.3, 0.0), 4, 8), 0.0);
            let n = fbm(p + q * 2.0, 6, 9);
            let n2 = fbm(p * 1.7 + q, 5, 21);
            let a = ((n - 0.42) * 2.2).clamp(0.0, 1.0).powf(1.6);
            let b = ((n2 - 0.5) * 2.5).clamp(0.0, 1.0).powf(2.0);
            // two-tone nebula (tinted again at draw time)
            let r = a * 0.9 + b * 0.2;
            let g = a * 0.25 + b * 0.5;
            let bl = a * 1.0 + b * 0.9;
            img.set_pixel(x as u32, y as u32, Color::new(r.min(1.0), g.min(1.0), bl.min(1.0), 1.0));
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Linear);
    t
}
