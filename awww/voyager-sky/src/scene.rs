//! What drag mode shows, apart from Wayland: the view, the cursor, the constellation
//! lines and names, the chips and the probe, and how they fade. app.rs feeds it input;
//! `export --overlay` uses it headless for previews.

use std::path::Path;
use std::time::Instant;

use glyphon::{Attrs, Buffer, Color, Family, Metrics, Shaping, TextArea, TextBounds};

use crate::data::SkyData;
use crate::render::{Frame, Rect, Renderer, Target, TextCtx};
use crate::view::{pin, Proj, View, V3};

/// A constellation this close to the cursor (logical pixels) lights up.
const HOVER_PX: f64 = 120.0;
/// The probe appears when the cursor is this close to Voyager.
const PROBE_HOVER_PX: f64 = 150.0;
/// Label and chip text size, logical pixels.
const LABEL_PX: f32 = 14.0;
const CHIP_PX: f32 = 13.0;
const FONT: &str = "Noto Sans CJK TC";

/// A monitor's place in the canvas, in logical pixels, and its buffer scale.
#[derive(Clone, Copy, Debug)]
pub struct Geom {
    pub origin: (f64, f64),
    pub size: (u32, u32),
    pub scale: i32,
}

impl Geom {
    pub fn contains(&self, p: (f64, f64)) -> bool {
        p.0 >= self.origin.0
            && p.1 >= self.origin.1
            && p.0 < self.origin.0 + self.size.0 as f64
            && p.1 < self.origin.1 + self.size.1 as f64
    }

    pub fn middle(&self) -> (f64, f64) {
        (self.origin.0 + self.size.0 as f64 / 2.0, self.origin.1 + self.size.1 as f64 / 2.0)
    }

    pub fn pixels(&self) -> (u32, u32) {
        (self.size.0 * self.scale as u32, self.size.1 * self.scale as u32)
    }
}

struct Label {
    zh: Buffer,
    en: Buffer,
    zh_size: (f32, f32),
    en_size: (f32, f32),
}

/// The chips in the bottom-right corner, left to right; each has a key too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chip {
    /// C: constellation lines and names on/off
    Lines,
    /// L: names in Chinese or English
    Lang,
    /// Esc: end drag mode, making the view the wallpaper
    Done,
}

impl Chip {
    const ALL: [Chip; 3] = [Chip::Lines, Chip::Lang, Chip::Done];
}

struct ChipText {
    text: Buffer,
    size: (f32, f32),
}

/// One piece of text to draw: which buffer, where (target px) and how bright.
struct Spec {
    which: Which,
    left: f32,
    top: f32,
    colour: Color,
}

enum Which {
    Label(usize),
    Chip(usize),
}

pub struct Scene {
    pub renderer: Renderer,
    pub text: TextCtx,
    pub sky: SkyData,

    pub view: View,
    /// where the view centre sits, canvas pixels
    pub center: (f64, f64),
    pub cursor: Option<(f64, f64)>,
    grab: Option<V3>,
    pub lines_on: bool,
    pub zh: bool,
    pub exit: bool,
    hovered: Option<usize>,

    labels: Vec<Label>,
    /// in Chip::ALL order
    chips: Vec<ChipText>,
    /// per constellation: how lit its lines and name are (0 hidden, 0.5 shown, 1 hovered)
    glow: Vec<f32>,
    probe: f32,
    /// line segment -> constellation, and a flicker phase per constellation
    seg_con: Vec<usize>,
    phase: Vec<f32>,
    line_alpha: Vec<f32>,

    start: Instant,
    last: Instant,
}

fn make_text(ctx: &mut TextCtx, s: &str, px: f32) -> (Buffer, (f32, f32)) {
    let mut b = Buffer::new(&mut ctx.font_system, Metrics::new(px, px * 1.3));
    b.set_size(&mut ctx.font_system, None, None);
    b.set_text(&mut ctx.font_system, s, &Attrs::new().family(Family::Name(FONT)), Shaping::Advanced);
    b.shape_until_scroll(&mut ctx.font_system, false);
    let (mut w, mut h) = (0f32, 0f32);
    for run in b.layout_runs() {
        w = w.max(run.line_w);
        h = h.max(run.line_top + run.line_height);
    }
    (b, (w, h))
}

fn rgba(c: [f32; 3], a: f32) -> Color {
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::rgba(q(c[0]), q(c[1]), q(c[2]), q(a))
}

impl Scene {
    pub fn new(renderer: Renderer, sky: SkyData, view: View) -> Self {
        let mut text = TextCtx::new(&renderer);
        let labels = sky
            .constellations
            .iter()
            .map(|c| {
                let (zh, zh_size) = make_text(&mut text, &c.zh, LABEL_PX);
                let (en, en_size) = make_text(&mut text, &c.en, LABEL_PX);
                Label { zh, en, zh_size, en_size }
            })
            .collect();
        let seg_con: Vec<usize> =
            sky.constellations.iter().enumerate().flat_map(|(i, c)| std::iter::repeat_n(i, c.lines.len())).collect();
        let n = sky.constellations.len();
        let mut s = Scene {
            renderer,
            text,
            view,
            center: (0.0, 0.0),
            cursor: None,
            grab: None,
            lines_on: false,
            zh: true,
            exit: false,
            hovered: None,
            labels,
            chips: Vec::new(),
            glow: vec![0.0; n],
            probe: 0.0,
            line_alpha: vec![0.0; seg_con.len()],
            seg_con,
            phase: (0..n).map(|i| (i as f32 * 0.618_034).fract()).collect(),
            sky,
            start: Instant::now(),
            last: Instant::now(),
        };
        s.set_chip_text();
        s
    }

    pub fn proj(&self) -> Proj {
        Proj::new(&self.view, self.center)
    }

    // --- input ---------------------------------------------------------------------

    pub fn pointer_moved(&mut self, p: (f64, f64)) {
        self.cursor = Some(p);
        if let Some(g) = self.grab {
            self.view = pin(self.view, self.center, g, p);
        }
    }

    pub fn pointer_left(&mut self) {
        if self.grab.is_none() {
            self.cursor = None;
        }
    }

    /// Left button down at `p` on monitor `g`: a chip, or grab the sky.
    pub fn press(&mut self, p: (f64, f64), g: &Geom) {
        self.cursor = Some(p);
        let local = ((p.0 - g.origin.0) as f32, (p.1 - g.origin.1) as f32);
        let hit = self
            .chip_rects(g.size)
            .iter()
            .position(|&(x, y, w, h)| local.0 >= x && local.0 <= x + w && local.1 >= y && local.1 <= y + h);
        match hit {
            Some(i) => self.press_chip(Chip::ALL[i]),
            None => self.grab = Some(self.proj().unproject(p)),
        }
    }

    pub fn release(&mut self) {
        self.grab = None;
    }

    /// Scroll by `steps` notches at `p`: zoom, keeping the sky under the cursor still.
    pub fn zoom(&mut self, p: (f64, f64), steps: f64) {
        let before = self.proj().unproject(p);
        let mut v = self.view;
        v.fov *= (0.12 * steps).exp();
        self.view = pin(v.clamped(), self.center, before, p);
    }

    pub fn press_chip(&mut self, chip: Chip) {
        match chip {
            Chip::Lines => self.lines_on = !self.lines_on,
            Chip::Lang => self.zh = !self.zh,
            Chip::Done => self.exit = true,
        }
        self.set_chip_text();
    }

    /// Drawn brighter: lines while they are on, and the language chip always.
    fn chip_lit(&self, chip: Chip) -> bool {
        match chip {
            Chip::Lines => self.lines_on,
            Chip::Lang => true,
            Chip::Done => false,
        }
    }

    fn set_chip_text(&mut self) {
        self.chips = Chip::ALL
            .iter()
            .map(|chip| {
                let s = match chip {
                    Chip::Lines if self.lines_on => "C  ✦ lines",
                    Chip::Lines => "C  ✧ lines",
                    Chip::Lang if self.zh => "L  中文",
                    Chip::Lang => "L  English",
                    Chip::Done => "Esc  set as wallpaper",
                };
                let (text, size) = make_text(&mut self.text, s, CHIP_PX);
                ChipText { text, size }
            })
            .collect();
    }

    // --- animation -------------------------------------------------------------------

    fn probe_target(&self) -> f32 {
        let Some(c) = self.cursor else { return 0.0 };
        let (x, y, z) = self.proj().project(self.sky.voyager);
        if z > -0.9 && (x - c.0).hypot(y - c.1) < PROBE_HOVER_PX {
            1.0
        } else {
            0.0
        }
    }

    fn glow_target(&self, i: usize) -> f32 {
        match (self.lines_on, Some(i) == self.hovered) {
            (false, _) => 0.0,
            (true, true) => 1.0,
            (true, false) => 0.5,
        }
    }

    /// Something is still moving, so keep drawing frames.
    pub fn animating(&self) -> bool {
        self.lines_on
            || self.grab.is_some()
            || (0..self.glow.len()).any(|i| self.glow[i] != self.glow_target(i))
            || self.probe != self.probe_target()
    }

    /// The constellation whose figure passes closest to the cursor, if close enough.
    fn nearest(&self) -> Option<usize> {
        let c = self.cursor?;
        let pr = self.proj();
        let pts: Vec<(f64, f64, f64)> = self.sky.stars.iter().map(|s| pr.project(s.pos)).collect();
        let mut best = (HOVER_PX, None);
        for (i, con) in self.sky.constellations.iter().enumerate() {
            for &[a, b] in &con.lines {
                let (pa, pb) = (pts[a], pts[b]);
                if pa.2 < -0.3 || pb.2 < -0.3 {
                    continue;
                }
                let (dx, dy) = (pb.0 - pa.0, pb.1 - pa.1);
                let l2 = dx * dx + dy * dy;
                let t = if l2 > 0.0 { (((c.0 - pa.0) * dx + (c.1 - pa.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
                let d = (pa.0 + t * dx - c.0).hypot(pa.1 + t * dy - c.1);
                if d < best.0 {
                    best = (d, Some(i));
                }
            }
        }
        best.1
    }

    /// Advance the fades by the time since the last call (or by `dt` seconds).
    pub fn update(&mut self, dt: Option<f32>) {
        let now = Instant::now();
        let dt = dt.unwrap_or_else(|| (now - self.last).as_secs_f32().min(0.1));
        self.last = now;
        let t = (now - self.start).as_secs_f32();
        let ease = |cur: &mut f32, target: f32, rate: f32| {
            *cur += (target - *cur) * (1.0 - (-dt * rate).exp());
            if (*cur - target).abs() < 0.002 {
                *cur = target;
            }
        };

        self.hovered = if self.lines_on { self.nearest() } else { None };
        for i in 0..self.glow.len() {
            let target = self.glow_target(i);
            ease(&mut self.glow[i], target, 7.0);
        }
        let pt = self.probe_target();
        ease(&mut self.probe, pt, 5.0);

        // the slow blue flicker: each constellation breathes on its own 6-10 s cycle
        for (j, &c) in self.seg_con.iter().enumerate() {
            let (g, ph) = (self.glow[c], self.phase[c]);
            let period = 6.0 + 4.0 * ph;
            let flicker = 0.75 + 0.25 * (t * std::f32::consts::TAU / period + ph * 40.0).sin();
            self.line_alpha[j] = if Some(c) == self.hovered { g * 0.95 } else { g * 0.7 * flicker };
        }
        self.renderer.set_line_alpha(&self.line_alpha);
    }

    // --- drawing ---------------------------------------------------------------------

    /// The chips' rectangles (x, y, w, h) on a monitor of `size`, logical px, in Chip::ALL order.
    fn chip_rects(&self, size: (u32, u32)) -> Vec<(f32, f32, f32, f32)> {
        let (pad, h, gap, margin) = (12.0, 26.0, 8.0, 24.0);
        let y = size.1 as f32 - margin - h;
        let mut x = size.0 as f32 - margin;
        let mut out = vec![(0.0, 0.0, 0.0, 0.0); self.chips.len()];
        for i in (0..self.chips.len()).rev() {
            let w = self.chips[i].size.0 + 2.0 * pad;
            x -= w;
            out[i] = (x, y, w, h);
            x -= gap;
        }
        out
    }

    /// Draw monitor `g`'s slice of the canvas into `view` (drag mode: with overlay).
    pub fn draw(&mut self, g: &Geom, view: &wgpu::TextureView, target: &mut Target) {
        let frame = self.prepare(g);
        let specs = self.text_specs(g);
        let areas = areas(&self.labels, &self.chips, self.zh, &specs, g);
        self.renderer.draw(target, view, &frame, Some(&mut self.text), areas);
    }

    /// Like draw, but into a PNG (`export --overlay`).
    pub fn snapshot(&mut self, g: &Geom, path: &Path) -> Result<(), String> {
        let frame = self.prepare(g);
        let mut target = self.renderer.target(Some(&mut self.text));
        let specs = self.text_specs(g);
        let areas = areas(&self.labels, &self.chips, self.zh, &specs, g);
        self.renderer.export_with(&mut target, &frame, Some(&mut self.text), areas, path)
    }

    /// Monitor `g`'s frame, with the chip rectangles handed to the renderer.
    fn prepare(&mut self, g: &Geom) -> Frame {
        let (frame, rects) = self.frame(g);
        self.renderer.set_rects(&rects);
        frame
    }

    fn frame(&self, g: &Geom) -> (Frame, Vec<Rect>) {
        let sc = g.scale as f64;
        let pr = self.proj();
        let mut proj = pr;
        proj.center = ((pr.center.0 - g.origin.0) * sc, (pr.center.1 - g.origin.1) * sc);
        proj.scale *= sc;
        let frame = Frame {
            proj,
            u: sc as f32,
            size: g.pixels(),
            time: (Instant::now() - self.start).as_secs_f32(),
            probe_alpha: self.probe,
            overlay: true,
        };
        let pal = self.renderer.palette();
        let rects = self
            .chip_rects(g.size)
            .iter()
            .zip(Chip::ALL)
            .map(|(&(x, y, w, h), chip)| Rect {
                rect: [x * sc as f32, y * sc as f32, w * sc as f32, h * sc as f32],
                col: [pal.ink[0], pal.ink[1], pal.ink[2], 0.62],
                border: [pal.indigo[0], pal.indigo[1], pal.indigo[2], if self.chip_lit(chip) { 0.85 } else { 0.35 }],
            })
            .collect();
        (frame, rects)
    }

    /// Where every visible label and chip text goes on monitor `g`.
    fn text_specs(&self, g: &Geom) -> Vec<Spec> {
        let sc = g.scale as f32;
        let mist = self.renderer.palette().mist;
        let pr = self.proj();
        let mut out = Vec::new();
        for (c, con) in self.sky.constellations.iter().enumerate() {
            let gl = self.glow[c];
            if gl < 0.01 {
                continue;
            }
            let (x, y, z) = pr.project(con.label);
            let (x, y) = ((x - g.origin.0) as f32, (y - g.origin.1) as f32);
            if z < 0.0 || x < -200.0 || y < -50.0 || x > g.size.0 as f32 + 200.0 || y > g.size.1 as f32 + 50.0 {
                continue;
            }
            let lab = &self.labels[c];
            let (bw, bh) = if self.zh { lab.zh_size } else { lab.en_size };
            out.push(Spec {
                which: Which::Label(c),
                left: (x - bw / 2.0) * sc,
                top: (y - bh / 2.0) * sc,
                colour: if Some(c) == self.hovered { rgba([1.0, 1.0, 1.0], gl) } else { rgba(mist, gl * 0.8) },
            });
        }
        for (k, &(x, y, rw, rh)) in self.chip_rects(g.size).iter().enumerate() {
            let size = self.chips[k].size;
            out.push(Spec {
                which: Which::Chip(k),
                left: (x + (rw - size.0) / 2.0) * sc,
                top: (y + (rh - size.1) / 2.0) * sc,
                colour: rgba(mist, if self.chip_lit(Chip::ALL[k]) { 0.95 } else { 0.6 }),
            });
        }
        out
    }

    pub fn trim(&mut self) {
        self.text.atlas.trim();
    }
}

fn areas<'a>(labels: &'a [Label], chips: &'a [ChipText], zh: bool, specs: &[Spec], g: &Geom) -> Vec<TextArea<'a>> {
    let (w, h) = g.pixels();
    let bounds = TextBounds { left: 0, top: 0, right: w as i32, bottom: h as i32 };
    specs
        .iter()
        .map(|s| TextArea {
            buffer: match s.which {
                Which::Label(c) if zh => &labels[c].zh,
                Which::Label(c) => &labels[c].en,
                Which::Chip(k) => &chips[k].text,
            },
            left: s.left,
            top: s.top,
            scale: g.scale as f32,
            bounds,
            default_color: s.colour,
            custom_glyphs: &[],
        })
        .collect()
}

