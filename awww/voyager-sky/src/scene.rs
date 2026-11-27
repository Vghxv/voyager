//! What drag mode shows, apart from Wayland: the view, the cursor, the constellation
//! lines, names and figures, the ecliptic and equator, the chips, the navigation ball and the probe, how they fade, and
//! how the view moves on its own (snaps and the idle zodiac tour). app.rs feeds it
//! input; `export --overlay` uses it headless for previews.
//!
//! Two orbit methods, as in Blender: Turntable keeps the roll (north stays up on a
//! level sky), Trackball turns the sphere freely, so the sky can tilt. Left alone for
//! IDLE in Trackball, the view zooms out, settles on the ecliptic (keeping its tilt)
//! and glides east through the twelve zodiac signs. Only taking hold of the sky (a
//! click or drag on it or on the ball) ends the tour; moving the mouse, keys and chips
//! leave it running, and scrolling zooms the tour itself.
//!
//! F shows one constellation's figure at a time: the one nearest the cursor, or,
//! while the tour runs and the cursor has not moved since it started, the zodiac sign
//! passing the middle of the view.

use std::path::Path;
use std::time::{Duration, Instant};

use glyphon::{Attrs, Buffer, Color, Family, Metrics, Shaping, TextArea, TextBounds};

use crate::data::SkyData;
use crate::figures::Figures;
use crate::render::{Frame, Renderer, Shape, Target, TextCtx, CIRCLE_SEGS, PROBE_PX};
use crate::view::{
    self as view, angle, cross, dot, ease_basis, ecliptic_lon, ecliptic_point, neg, normalize, pin, pin_free, radec,
    rotate_basis, turn_basis_to, Basis, Proj, View, V3,
};

/// A constellation this close to the cursor (logical pixels) lights up.
const HOVER_PX: f64 = 120.0;
/// During the tour, a zodiac sign's figure shows while its centre is within this
/// angle of the middle of the view.
const ZODIAC_FIGURE_DEG: f64 = 15.0;
/// The twelve signs the tour passes, by constellation abbreviation.
const ZODIAC: [&str; 12] = ["Ari", "Tau", "Gem", "Cnc", "Leo", "Vir", "Lib", "Sco", "Sgr", "Cap", "Aqr", "Psc"];
/// The probe appears when the cursor is this close to Voyager.
const PROBE_HOVER_PX: f64 = 150.0;
/// Label and chip text size, logical pixels.
const LABEL_PX: f32 = 14.0;
const CHIP_PX: f32 = 13.0;
const FONT: &str = "Noto Sans CJK TC";
/// The probe's name, in English and 繁體中文.
const PROBE_NAME: [&str; 2] = ["Voyager 1", "航海家1號"];

/// A press that moves less than this before letting go is a click.
const CLICK_SLOP_PX: f64 = 6.0;
/// Dragging the navigation ball turns the sky this much per pixel.
const ORBIT_DEG_PER_PX: f64 = 0.5;
/// How fast a snap (axis click, levelling) closes in, per second.
const SNAP_RATE: f64 = 5.0;

/// No input for this long in Trackball starts the zodiac tour.
const IDLE: Duration = Duration::from_secs(5);
/// The tour zooms out to at least this field of view.
const TOUR_FOV: f64 = 120.0;
/// Along the ecliptic: one sign every 15 s, the whole zodiac in 3 minutes.
const TOUR_DEG_PER_S: f64 = 2.0;
/// How fast the view closes in on the moving tour target, per second, reached
/// gradually over TOUR_RAMP seconds so the start is gentle.
const TOUR_RATE: f64 = 1.0;
const TOUR_RAMP: f64 = 1.5;

/// The navigation ball, bottom right of every monitor above the chips (logical px).
const BALL_R: f32 = 44.0;
const BALL_MARGIN: f32 = 24.0;
const KNOB_R: f32 = 9.0;
const NEG_KNOB_R: f32 = 6.0;
const AXIS_LEN: f32 = BALL_R - KNOB_R - 3.0;
const AXIS_TEXT_PX: f32 = 11.0;
/// X, Y, Z: red, green, blue, softened to sit in the night palette.
const AXIS_COL: [[f32; 3]; 3] = [[0.89, 0.42, 0.45], [0.55, 0.78, 0.40], [0.40, 0.60, 0.94]];

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

/// How dragging the sky turns it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    /// the roll stays as it is: on a level sky, north stays up
    Turntable,
    /// the sphere turns freely, so the sky can tilt
    Trackball,
}

/// The chips in the bottom-right corner, left to right; each has a key too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chip {
    /// T: Turntable or Trackball
    Nav,
    /// C: constellation lines and names on/off
    Lines,
    /// F: constellation figures on/off
    Figures,
    /// E: the ecliptic on/off
    Ecliptic,
    /// Q: the celestial equator on/off
    Equator,
    /// L: names in Chinese or English
    Lang,
    /// Esc: end drag mode, making the view the wallpaper
    Done,
}

impl Chip {
    const ALL: [Chip; 7] =
        [Chip::Nav, Chip::Lines, Chip::Figures, Chip::Ecliptic, Chip::Equator, Chip::Lang, Chip::Done];
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
    Probe,
    Axis(usize),
}

/// Every piece of text drag mode can show, shaped once.
struct Texts {
    labels: Vec<Label>,
    /// in Chip::ALL order
    chips: Vec<ChipText>,
    probe: Label,
    /// X, Y, Z
    axes: Vec<ChipText>,
}

/// The view moving on its own, eased every frame toward a target.
#[derive(Clone, Copy, Debug)]
enum Motion {
    /// to a fixed view: an axis of the navigation ball, or levelling
    Snap { to: Basis, fov: f64 },
    /// the zodiac tour: along the ecliptic from longitude `lon0`, started at `t0`
    Tour { lon0: f64, t0: Instant, fov: f64 },
}

/// The left button went down on the navigation ball.
#[derive(Clone, Copy, Debug)]
struct BallPress {
    /// on an axis knob (0-2 +X +Y +Z, 3-5 -X -Y -Z), or on the ball itself
    axis: Option<usize>,
    at: (f64, f64),
    last: (f64, f64),
    /// went beyond CLICK_SLOP_PX: an orbit, not a click
    moved: bool,
}

/// One axis knob on the ball, in monitor-local logical px.
struct Knob {
    axis: usize,
    pos: (f32, f32),
    /// toward the view centre (1) or away (-1): drawn back to front
    depth: f64,
}

fn axis_dir(k: usize) -> V3 {
    let mut a = [0.0; 3];
    a[k % 3] = 1.0;
    if k < 3 { a } else { neg(a) }
}

fn ball_center(size: (u32, u32)) -> (f32, f32) {
    // clear of the chip row: its 24 px margin, 26 px height, then a 12 px gap
    (size.0 as f32 - BALL_MARGIN - BALL_R, size.1 as f32 - 24.0 - 26.0 - 12.0 - BALL_R)
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
    ball: Option<BallPress>,
    pub nav: Nav,
    motion: Option<Motion>,
    last_input: Instant,
    pub lines_on: bool,
    pub figures_on: bool,
    pub ecliptic_on: bool,
    pub equator_on: bool,
    pub zh: bool,
    pub exit: bool,
    hovered: Option<usize>,
    /// the cursor has moved on the sky since the tour started: figures follow it
    tour_hover: bool,
    /// the zodiac signs' constellation indices
    zodiac: Vec<usize>,
    /// the figure images, loaded as they are wanted
    figures: Figures,

    texts: Texts,
    /// how visible the ecliptic and the celestial equator are (0 hidden)
    ecliptic: f32,
    equator: f32,
    /// per constellation: how lit its lines and name are (0 hidden, 0.5 shown, 1 hovered)
    glow: Vec<f32>,
    probe: f32,
    /// how visible the chips and the navigation ball are: they fade out for the tour
    controls: f32,
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
        let mut label = |en: &str, zh: &str| {
            let (zh, zh_size) = make_text(&mut text, zh, LABEL_PX);
            let (en, en_size) = make_text(&mut text, en, LABEL_PX);
            Label { zh, en, zh_size, en_size }
        };
        let labels = sky.constellations.iter().map(|c| label(&c.en, &c.zh)).collect();
        let probe = label(PROBE_NAME[0], PROBE_NAME[1]);
        let axes = ["X", "Y", "Z"]
            .iter()
            .map(|s| {
                let (text, size) = make_text(&mut text, s, AXIS_TEXT_PX);
                ChipText { text, size }
            })
            .collect();
        let seg_con: Vec<usize> =
            sky.constellations.iter().enumerate().flat_map(|(i, c)| std::iter::repeat_n(i, c.lines.len())).collect();
        let n = sky.constellations.len();
        let zodiac = ZODIAC.iter().filter_map(|a| sky.constellations.iter().position(|c| c.abbr == *a)).collect();
        let figures = Figures::new((0..sky.constellations.len()).map(|i| sky.figure_path(i)).collect());
        let mut s = Scene {
            renderer,
            text,
            view,
            center: (0.0, 0.0),
            cursor: None,
            grab: None,
            ball: None,
            nav: Nav::Trackball,
            motion: None,
            last_input: Instant::now(),
            lines_on: false,
            figures_on: false,
            ecliptic_on: false,
            ecliptic: 0.0,
            equator_on: false,
            equator: 0.0,
            zh: true,
            exit: false,
            hovered: None,
            tour_hover: false,
            zodiac,
            figures,
            texts: Texts { labels, chips: Vec::new(), probe, axes },
            glow: vec![0.0; n],
            probe: 0.0,
            controls: 1.0,
            line_alpha: vec![0.0; seg_con.len() + 2 * CIRCLE_SEGS],
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

    /// Any input: it restarts the idle clock (but leaves a running tour alone).
    fn touch(&mut self) {
        self.last_input = Instant::now();
    }

    fn touring(&self) -> bool {
        matches!(self.motion, Some(Motion::Tour { .. }))
    }

    /// The sky was taken hold of: end the zodiac tour, the view staying where it is.
    fn stop_tour(&mut self) {
        if self.touring() {
            self.motion = None;
        }
    }

    /// A key was pressed (app.rs calls this for every key).
    pub fn key_input(&mut self) {
        self.touch();
    }

    pub fn pointer_moved(&mut self, p: (f64, f64)) {
        // compositors can repeat the position (a redraw, a re-enter): that is not input
        if self.cursor != Some(p) {
            self.touch();
            self.tour_hover = true;
        }
        self.cursor = Some(p);
        if let Some(mut b) = self.ball {
            if !b.moved && (p.0 - b.at.0).hypot(p.1 - b.at.1) > CLICK_SLOP_PX {
                b.moved = true;
            }
            if b.moved {
                self.motion = None;
                self.orbit((p.0 - b.last.0, p.1 - b.last.1));
                b.last = p;
            }
            self.ball = Some(b);
            return;
        }
        if let Some(g) = self.grab {
            self.motion = None;
            self.view = self.pin(self.view, g, p);
        }
    }

    pub fn pointer_left(&mut self) {
        if self.grab.is_none() && self.ball.is_none() {
            self.cursor = None;
        }
    }

    /// Left button down at `p` on monitor `g`: a chip, the navigation ball, or grab the sky.
    pub fn press(&mut self, p: (f64, f64), g: &Geom) {
        self.touch();
        self.cursor = Some(p);
        // during the tour the chips and the ball are hidden: any click takes the sky
        if self.touring() {
            self.stop_tour();
            self.grab = Some(self.proj().unproject(p));
            return;
        }
        let local = ((p.0 - g.origin.0) as f32, (p.1 - g.origin.1) as f32);
        let hit = self
            .chip_rects(g.size)
            .iter()
            .position(|&(x, y, w, h)| local.0 >= x && local.0 <= x + w && local.1 >= y && local.1 <= y + h);
        if let Some(i) = hit {
            return self.press_chip(Chip::ALL[i]);
        }
        let dist = |q: (f32, f32)| (local.0 - q.0).hypot(local.1 - q.1);
        let knob = self.knobs(g.size).into_iter().rev().find(|k| dist(k.pos) <= knob_r(k.axis) + 2.0).map(|k| k.axis);
        if knob.is_some() || dist(ball_center(g.size)) <= BALL_R {
            self.stop_tour();
            self.ball = Some(BallPress { axis: knob, at: p, last: p, moved: false });
            return;
        }
        self.stop_tour();
        self.motion = None;
        self.grab = Some(self.proj().unproject(p));
    }

    /// Left button up at `p`: let go of the sky; a click on an axis knob turns to it.
    pub fn release(&mut self, p: (f64, f64)) {
        self.touch();
        self.cursor = Some(p);
        self.grab = None;
        if let Some(BallPress { axis: Some(k), moved: false, .. }) = self.ball.take() {
            self.snap_to_axis(k);
        }
    }

    /// Scroll by `steps` notches at `p`: zoom, keeping the sky under the cursor still.
    pub fn zoom(&mut self, p: (f64, f64), steps: f64) {
        self.touch();
        // during the tour, zoom the tour: it eases to the new field of view
        if let Some(Motion::Tour { fov, .. }) = &mut self.motion {
            *fov = (*fov * (0.12 * steps).exp()).clamp(view::FOV_MIN, view::FOV_MAX);
            return;
        }
        self.motion = None;
        let before = self.proj().unproject(p);
        let mut v = self.view;
        v.fov *= (0.12 * steps).exp();
        self.view = self.pin(v.clamped(), before, p);
    }

    pub fn press_chip(&mut self, chip: Chip) {
        self.touch();
        match chip {
            Chip::Nav => {
                // Turntable never tours
                self.stop_tour();
                self.nav = match self.nav {
                    Nav::Turntable => Nav::Trackball,
                    Nav::Trackball => Nav::Turntable,
                };
                self.level();
            }
            Chip::Lines => self.lines_on = !self.lines_on,
            Chip::Figures => self.figures_on = !self.figures_on,
            Chip::Ecliptic => self.ecliptic_on = !self.ecliptic_on,
            Chip::Equator => self.equator_on = !self.equator_on,
            Chip::Lang => self.zh = !self.zh,
            Chip::Done => self.exit = true,
        }
        self.set_chip_text();
    }

    /// R, and going back to Turntable: turn the sky smoothly back to north-up.
    pub fn level(&mut self) {
        if self.nav == Nav::Turntable && self.view.roll != 0.0 {
            self.motion = Some(Motion::Snap { to: View { roll: 0.0, ..self.view }.basis(), fov: self.view.fov });
        }
    }

    /// `view` moved so sky point `p` sits under canvas pixel `px`, the current way.
    fn pin(&self, view: View, p: V3, px: (f64, f64)) -> View {
        match self.nav {
            Nav::Turntable => pin(view, self.center, p, px),
            Nav::Trackball => pin_free(view, self.center, p, px),
        }
    }

    /// Dragging the navigation ball by `d` pixels: the sky turns the way the cursor goes.
    fn orbit(&mut self, d: (f64, f64)) {
        let k = ORBIT_DEG_PER_PX;
        self.view = match self.nav {
            Nav::Turntable => View { ra: self.view.ra + d.0 * k, dec: self.view.dec + d.1 * k, ..self.view }.clamped(),
            Nav::Trackball => {
                let b = self.view.basis();
                let b = rotate_basis(b, b.1, (d.0 * k).to_radians());
                let b = rotate_basis(b, b.0, (-d.1 * k).to_radians());
                View::from_basis(b, self.view.fov)
            }
        };
    }

    /// Turn to look along axis `k`; if already looking that way, to the opposite one.
    fn snap_to_axis(&mut self, k: usize) {
        let (_, n, f) = self.view.basis();
        let mut a = axis_dir(k);
        if angle(f, a) < 0.5_f64.to_radians() {
            a = neg(a);
        }
        let (ra, dec) = radec(a);
        // at a pole the RA is arbitrary: keep the current one
        let ra = if dec.abs() > 89.9 { self.view.ra } else { ra };
        let level = View { ra, dec, fov: self.view.fov, roll: 0.0 }.basis();
        let to = match self.nav {
            Nav::Turntable => level,
            Nav::Trackball => {
                // keep screen-up as close to what it is as the new forward allows
                let up = [n[0] - dot(n, a) * a[0], n[1] - dot(n, a) * a[1], n[2] - dot(n, a) * a[2]];
                if dot(up, up) < 1e-6 {
                    level
                } else {
                    let up = normalize(up);
                    (cross(up, a), up, a)
                }
            }
        };
        self.motion = Some(Motion::Snap { to, fov: self.view.fov });
    }

    /// Drawn brighter: lines while they are on, and the language chip always.
    fn chip_lit(&self, chip: Chip) -> bool {
        match chip {
            Chip::Nav => self.nav == Nav::Trackball,
            Chip::Lines => self.lines_on,
            Chip::Figures => self.figures_on,
            Chip::Ecliptic => self.ecliptic_on,
            Chip::Equator => self.equator_on,
            Chip::Lang => true,
            Chip::Done => false,
        }
    }

    /// The chips' text, in the names' language: 繁體中文 or English.
    fn set_chip_text(&mut self) {
        let on = |b: bool| if b { "✦" } else { "✧" };
        self.texts.chips = Chip::ALL
            .iter()
            .map(|chip| {
                let s = match (chip, self.zh) {
                    (Chip::Nav, true) if self.nav == Nav::Trackball => "T  軌跡球".to_string(),
                    (Chip::Nav, true) => "T  轉盤".to_string(),
                    (Chip::Nav, false) if self.nav == Nav::Trackball => "T  trackball".to_string(),
                    (Chip::Nav, false) => "T  turntable".to_string(),
                    (Chip::Lines, true) => format!("C  {} 星座", on(self.lines_on)),
                    (Chip::Lines, false) => format!("C  {} Constellations", on(self.lines_on)),
                    (Chip::Figures, true) => format!("F  {} 星座圖", on(self.figures_on)),
                    (Chip::Figures, false) => format!("F  {} figures", on(self.figures_on)),
                    (Chip::Ecliptic, true) => format!("E  {} 黃道", on(self.ecliptic_on)),
                    (Chip::Ecliptic, false) => format!("E  {} ecliptic", on(self.ecliptic_on)),
                    (Chip::Equator, true) => format!("Q  {} 天赤道", on(self.equator_on)),
                    (Chip::Equator, false) => format!("Q  {} equator", on(self.equator_on)),
                    (Chip::Lang, true) => "L  中文".to_string(),
                    (Chip::Lang, false) => "L  English".to_string(),
                    (Chip::Done, true) => "Esc  設為桌布".to_string(),
                    (Chip::Done, false) => "Esc  set as wallpaper".to_string(),
                };
                let (text, size) = make_text(&mut self.text, &s, CHIP_PX);
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

    /// Which constellation's figure to show (F): the one nearest the cursor, or during
    /// the tour, until the cursor moves, the zodiac sign passing the middle of the view.
    fn figure_target(&self) -> Option<usize> {
        if !self.figures_on {
            return None;
        }
        if self.touring() && (!self.tour_hover || self.cursor.is_none()) {
            let f = self.view.basis().2;
            return self
                .zodiac
                .iter()
                .map(|&i| (angle(f, self.sky.constellations[i].centre).to_degrees(), i))
                .filter(|&(a, _)| a < ZODIAC_FIGURE_DEG)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, i)| i);
        }
        self.hovered
    }

    /// How visible each figure slot should be: only the one holding the target shows.
    fn figure_alpha_target(&self, slot: usize) -> f32 {
        let want = self.figure_target();
        if want.is_some() && self.renderer.figure_id(slot) == want { 1.0 } else { 0.0 }
    }

    fn ecliptic_target(&self) -> f32 {
        if self.ecliptic_on { 0.6 } else { 0.0 }
    }

    fn equator_target(&self) -> f32 {
        if self.equator_on { 0.6 } else { 0.0 }
    }

    fn controls_target(&self) -> f32 {
        if self.touring() { 0.0 } else { 1.0 }
    }

    /// Trackball, left alone for IDLE, nothing else moving: time for the zodiac tour.
    fn tour_due(&self) -> bool {
        self.nav == Nav::Trackball
            && self.motion.is_none()
            && self.grab.is_none()
            && self.ball.is_none()
            && self.last_input.elapsed() >= IDLE
    }

    /// Something is still moving, so keep drawing frames.
    pub fn animating(&self) -> bool {
        self.lines_on
            || self.grab.is_some()
            || self.motion.is_some()
            || self.controls != self.controls_target()
            || self.tour_due()
            || self.ecliptic != self.ecliptic_target()
            || self.equator != self.equator_target()
            || (0..self.glow.len()).any(|i| self.glow[i] != self.glow_target(i))
            || self.probe != self.probe_target()
            || (0..2).any(|k| self.renderer.figure_alpha(k) != self.figure_alpha_target(k))
            || self.figures.loading()
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

        if self.tour_due() {
            let lon0 = ecliptic_lon(self.view.basis().2);
            self.motion = Some(Motion::Tour { lon0, t0: now, fov: self.view.fov.max(TOUR_FOV) });
            self.tour_hover = false;
        }
        if let Some(m) = self.motion {
            self.advance(m, dt as f64, now);
        }

        let ease = |cur: &mut f32, target: f32, rate: f32| {
            *cur += (target - *cur) * (1.0 - (-dt * rate).exp());
            if (*cur - target).abs() < 0.002 {
                *cur = target;
            }
        };

        self.hovered = if self.lines_on || self.figures_on { self.nearest() } else { None };
        self.load_figure();
        for k in 0..2 {
            let mut alpha = self.renderer.figure_alpha(k);
            ease(&mut alpha, self.figure_alpha_target(k), 4.0);
            self.renderer.set_figure_alpha(k, alpha);
        }
        for i in 0..self.glow.len() {
            let target = self.glow_target(i);
            ease(&mut self.glow[i], target, 7.0);
        }
        let ct = self.controls_target();
        ease(&mut self.controls, ct, 4.0);
        let pt = self.probe_target();
        ease(&mut self.probe, pt, 5.0);
        let et = self.ecliptic_target();
        ease(&mut self.ecliptic, et, 7.0);
        let qt = self.equator_target();
        ease(&mut self.equator, qt, 7.0);

        // the slow blue flicker: each constellation breathes on its own 6-10 s cycle
        for (j, &c) in self.seg_con.iter().enumerate() {
            let (g, ph) = (self.glow[c], self.phase[c]);
            let period = 6.0 + 4.0 * ph;
            let flicker = 0.75 + 0.25 * (t * std::f32::consts::TAU / period + ph * 40.0).sin();
            self.line_alpha[j] = if Some(c) == self.hovered { g * 0.95 } else { g * 0.7 * flicker };
        }
        let n = self.seg_con.len();
        self.line_alpha[n..n + CIRCLE_SEGS].fill(self.ecliptic);
        self.line_alpha[n + CIRCLE_SEGS..].fill(self.equator);
        self.renderer.set_line_alpha(&self.line_alpha);
    }

    /// Make sure the target figure is in one of the two slots: a new one goes in the
    /// slot that is less visible, so the other can finish fading out.
    /// Have the target figure in a slot: from the cache at once, or once it has loaded
    /// (drag mode loads in the background, see `load_figures_in_background`).
    fn load_figure(&mut self) {
        let Some(want) = self.figure_target() else { return };
        if (0..2).any(|k| self.renderer.figure_id(k) == Some(want)) {
            return;
        }
        let Some(mips) = self.figures.get(want) else { return };
        let slot = if self.renderer.figure_alpha(0) <= self.renderer.figure_alpha(1) { 0 } else { 1 };
        let c = &self.sky.constellations[want];
        self.renderer.set_figure(slot, want, &mips, c.centre, c.half);
    }

    /// Load figures on a thread of their own from now on (drag mode); stills load
    /// them on the spot so they come out the same every time.
    pub fn load_figures_in_background(&mut self) {
        self.figures.in_background();
    }

    /// Ease the view one step toward where `m` wants it.
    fn advance(&mut self, m: Motion, dt: f64, now: Instant) {
        let (to, fov, rate) = match m {
            Motion::Snap { to, fov } => (to, fov, SNAP_RATE),
            Motion::Tour { lon0, t0, fov } => {
                // the target glides east along the ecliptic; easing toward it zooms
                // out, centres on the ecliptic and pans, all at once, then settles into
                // following it. The sky keeps its tilt: the target is the current view
                // moved there by the shortest turn, with no turn about its centre.
                let t = (now - t0).as_secs_f64();
                let f = ecliptic_point(lon0 + TOUR_DEG_PER_S * t);
                let ramp = (t / TOUR_RAMP).min(1.0);
                (turn_basis_to(self.view.basis(), f), fov, TOUR_RATE * (ramp * ramp * (3.0 - 2.0 * ramp)).max(0.02))
            }
        };
        let k = 1.0 - (-dt * rate).exp();
        let (b, left) = ease_basis(self.view.basis(), to, k);
        let new_fov = self.view.fov + (fov - self.view.fov) * k;
        if matches!(m, Motion::Snap { .. }) && left < 0.05_f64.to_radians() && (fov - new_fov).abs() < 0.05 {
            self.view = View::from_basis(to, fov);
            self.motion = None;
        } else {
            self.view = View::from_basis(b, new_fov);
        }
    }

    // --- drawing ---------------------------------------------------------------------

    /// The chips' rectangles (x, y, w, h) on a monitor of `size`, logical px, in Chip::ALL order.
    fn chip_rects(&self, size: (u32, u32)) -> Vec<(f32, f32, f32, f32)> {
        let (pad, h, gap, margin) = (12.0, 26.0, 8.0, 24.0);
        let y = size.1 as f32 - margin - h;
        let mut x = size.0 as f32 - margin;
        let mut out = vec![(0.0, 0.0, 0.0, 0.0); self.texts.chips.len()];
        for i in (0..self.texts.chips.len()).rev() {
            let w = self.texts.chips[i].size.0 + 2.0 * pad;
            x -= w;
            out[i] = (x, y, w, h);
            x -= gap;
        }
        out
    }

    /// The navigation ball's axis knobs on a monitor of `size`, back to front.
    fn knobs(&self, size: (u32, u32)) -> Vec<Knob> {
        let (e, n, f) = self.view.basis();
        let c = ball_center(size);
        let mut out: Vec<Knob> = (0..6)
            .map(|axis| {
                let a = axis_dir(axis);
                // east is left and north up, as on the sky
                let pos = (c.0 - AXIS_LEN * dot(a, e) as f32, c.1 - AXIS_LEN * dot(a, n) as f32);
                Knob { axis, pos, depth: dot(a, f) }
            })
            .collect();
        out.sort_by(|a, b| a.depth.total_cmp(&b.depth));
        out
    }

    /// Draw monitor `g`'s slice of the canvas into `view` (drag mode: with overlay).
    pub fn draw(&mut self, g: &Geom, view: &wgpu::TextureView, target: &mut Target) {
        let frame = self.prepare(g);
        let specs = self.text_specs(g);
        let areas = areas(&self.texts, self.zh, &specs, g);
        self.renderer.draw(target, view, &frame, Some(&mut self.text), areas);
    }

    /// Like draw, but into a PNG (`export --overlay`).
    pub fn snapshot(&mut self, g: &Geom, path: &Path) -> Result<(), String> {
        let frame = self.prepare(g);
        let mut target = self.renderer.target(Some(&mut self.text));
        let specs = self.text_specs(g);
        let areas = areas(&self.texts, self.zh, &specs, g);
        self.renderer.export_with(&mut target, &frame, Some(&mut self.text), areas, path)
    }

    /// Monitor `g`'s frame, with the chips and the ball handed to the renderer.
    fn prepare(&mut self, g: &Geom) -> Frame {
        let (frame, shapes) = self.frame(g);
        self.renderer.set_shapes(&shapes);
        frame
    }

    fn frame(&self, g: &Geom) -> (Frame, Vec<Shape>) {
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
        let sc = sc as f32;
        let at = |p: (f32, f32)| (p.0 * sc, p.1 * sc);
        let mut shapes: Vec<Shape> = self
            .chip_rects(g.size)
            .iter()
            .zip(Chip::ALL)
            .map(|(&(x, y, w, h), chip)| {
                let r = h / 2.0;
                Shape::capsule(
                    at((x + r, y + r)),
                    at((x + w - r, y + r)),
                    r * sc,
                    [pal.ink[0], pal.ink[1], pal.ink[2], 0.62],
                    [pal.indigo[0], pal.indigo[1], pal.indigo[2], if self.chip_lit(chip) { 0.85 } else { 0.35 }],
                )
            })
            .collect();

        // the navigation ball: brighter under the cursor
        let c = ball_center(g.size);
        let local = self.cursor.map(|p| ((p.0 - g.origin.0) as f32, (p.1 - g.origin.1) as f32));
        let near = |q: (f32, f32), r: f32| local.is_some_and(|l| (l.0 - q.0).hypot(l.1 - q.1) <= r);
        let hot = near(c, BALL_R) || self.ball.is_some_and(|b| b.moved);
        shapes.push(Shape::capsule(
            at(c),
            at(c),
            BALL_R * sc,
            [pal.ink[0], pal.ink[1], pal.ink[2], if hot { 0.55 } else { 0.35 }],
            [pal.indigo[0], pal.indigo[1], pal.indigo[2], if hot { 0.7 } else { 0.3 }],
        ));
        for k in self.knobs(g.size) {
            let [r, gr, b] = AXIS_COL[k.axis % 3];
            let ring = if near(k.pos, knob_r(k.axis) + 2.0) { [1.0, 1.0, 1.0, 0.9] } else { [r, gr, b, 0.0] };
            if k.axis < 3 {
                shapes.push(Shape::capsule(at(c), at(k.pos), 1.1 * sc, [r, gr, b, 0.85], [0.0; 4]));
                shapes.push(Shape::capsule(at(k.pos), at(k.pos), KNOB_R * sc, [r, gr, b, 1.0], ring));
            } else {
                let ring = if ring[3] > 0.0 { ring } else { [r, gr, b, 0.8] };
                shapes.push(Shape::capsule(at(k.pos), at(k.pos), NEG_KNOB_R * sc, [r, gr, b, 0.25], ring));
            }
        }
        // faded out during the tour
        let c = self.controls;
        shapes.retain_mut(|s| {
            s.col[3] *= c;
            s.border[3] *= c;
            c > 0.01
        });
        (frame, shapes)
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
            let lab = &self.texts.labels[c];
            let (bw, bh) = if self.zh { lab.zh_size } else { lab.en_size };
            out.push(Spec {
                which: Which::Label(c),
                left: (x - bw / 2.0) * sc,
                top: (y - bh / 2.0) * sc,
                colour: if Some(c) == self.hovered { rgba([1.0, 1.0, 1.0], gl) } else { rgba(mist, gl * 0.8) },
            });
        }
        // the probe's name, under it, fading in and out with it
        if self.probe > 0.01 {
            let (x, y, z) = pr.project(self.sky.voyager);
            if z > -0.9 {
                let (x, y) = ((x - g.origin.0) as f32, (y - g.origin.1) as f32);
                let lab = &self.texts.probe;
                let (bw, _) = if self.zh { lab.zh_size } else { lab.en_size };
                out.push(Spec {
                    which: Which::Probe,
                    left: (x - bw / 2.0) * sc,
                    top: (y + PROBE_PX / 2.0 + 4.0) * sc,
                    colour: rgba(mist, self.probe),
                });
            }
        }
        let ink = self.renderer.palette().ink;
        for k in self.knobs(g.size).iter().filter(|k| k.axis < 3) {
            let (w, h) = self.texts.axes[k.axis].size;
            out.push(Spec {
                which: Which::Axis(k.axis),
                left: (k.pos.0 - w / 2.0) * sc,
                top: (k.pos.1 - h / 2.0) * sc,
                colour: rgba(ink, self.controls),
            });
        }
        for (k, &(x, y, rw, rh)) in self.chip_rects(g.size).iter().enumerate() {
            let size = self.texts.chips[k].size;
            out.push(Spec {
                which: Which::Chip(k),
                left: (x + (rw - size.0) / 2.0) * sc,
                top: (y + (rh - size.1) / 2.0) * sc,
                colour: rgba(mist, if self.chip_lit(Chip::ALL[k]) { 0.95 } else { 0.6 } * self.controls),
            });
        }
        if self.controls < 0.01 {
            out.retain(|s| !matches!(s.which, Which::Axis(_) | Which::Chip(_)));
        }
        out
    }

    pub fn trim(&mut self) {
        self.text.atlas.trim();
    }
}

fn knob_r(axis: usize) -> f32 {
    if axis < 3 { KNOB_R } else { NEG_KNOB_R }
}

fn areas<'a>(t: &'a Texts, zh: bool, specs: &[Spec], g: &Geom) -> Vec<TextArea<'a>> {
    let (w, h) = g.pixels();
    let bounds = TextBounds { left: 0, top: 0, right: w as i32, bottom: h as i32 };
    specs
        .iter()
        .map(|s| TextArea {
            buffer: match s.which {
                Which::Label(c) if zh => &t.labels[c].zh,
                Which::Label(c) => &t.labels[c].en,
                Which::Chip(k) => &t.chips[k].text,
                Which::Probe if zh => &t.probe.zh,
                Which::Probe => &t.probe.en,
                Which::Axis(k) => &t.axes[k].text,
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::Gpu;

    fn scene() -> Option<Scene> {
        let gpu = Gpu::new().ok()?;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sky");
        let sky = SkyData::load(&dir).ok()?;
        let r = Renderer::new(&gpu, wgpu::TextureFormat::Bgra8Unorm, &sky);
        let view = crate::wallpaper::default_view(&sky);
        let mut s = Scene::new(r, sky, view);
        s.center = (960.0, 540.0);
        Some(s)
    }

    #[test]
    fn only_taking_hold_of_the_sky_stops_the_tour() {
        let Some(mut s) = scene() else { return eprintln!("no GPU: skipped") };
        let g = Geom { origin: (0.0, 0.0), size: (1920, 1080), scale: 1 };
        assert_eq!(s.nav, Nav::Trackball, "drag mode opens in Trackball");
        s.last_input = Instant::now() - IDLE - Duration::from_millis(10);
        s.update(Some(0.016));
        assert!(s.touring(), "tour should start");

        // moving the mouse, keys and chips leave it running
        s.pointer_moved((100.0, 300.0));
        s.pointer_moved((140.0, 320.0));
        for chip in [Chip::Lines, Chip::Figures, Chip::Ecliptic, Chip::Equator, Chip::Lang] {
            s.key_input();
            s.press_chip(chip);
        }
        s.update(Some(0.016));
        assert!(s.touring(), "tour should keep going");

        // scrolling zooms the tour, without stopping it
        let Some(Motion::Tour { fov: before, .. }) = s.motion else { unreachable!() };
        s.zoom((500.0, 500.0), -2.0);
        let Some(Motion::Tour { fov: after, .. }) = s.motion else { panic!("scroll stopped the tour") };
        assert!(after < before);

        // the chips and the ball fade out
        for _ in 0..30 {
            s.update(Some(0.1));
        }
        assert_eq!(s.controls, 0.0);

        // a click on the sky ends it (even where a chip was), and the view stays put
        s.press((1800.0, 1040.0), &g);
        assert!(s.grab.is_some(), "a click during the tour takes the sky");
        s.release((800.0, 400.0));
        let held = s.view;
        s.update(Some(0.016));
        assert!(!s.touring(), "a click should stop the tour");
        assert_eq!(s.view, held);
        s.update(Some(0.1));
        assert!(s.controls > 0.0, "the chips come back");
    }

    #[test]
    fn figures_follow_the_cursor_or_the_zodiac() {
        let Some(mut s) = scene() else { return eprintln!("no GPU: skipped") };
        let ori = s.sky.constellations.iter().position(|c| c.abbr == "Ori").unwrap();
        let leo = s.sky.constellations.iter().position(|c| c.abbr == "Leo").unwrap();
        let (ra, dec) = radec(s.sky.constellations[ori].centre);
        s.view = View { ra, dec, fov: 90.0, roll: 0.0 };
        s.pointer_moved(s.center);
        s.update(Some(0.016));
        assert_eq!(s.figure_target(), None, "F is off");

        // case 1: the constellation under the cursor, and only that one
        s.press_chip(Chip::Figures);
        s.update(Some(0.016));
        assert_eq!(s.figure_target(), Some(ori));
        for _ in 0..30 {
            s.update(Some(0.1));
        }
        let shown: Vec<f32> = (0..2).map(|k| s.renderer.figure_alpha(k)).filter(|&a| a > 0.0).collect();
        assert_eq!(shown, vec![1.0], "one figure, fully shown");

        // case 2: the tour starts with the cursor resting: the zodiac sign in the middle
        s.last_input = Instant::now() - IDLE - Duration::from_millis(10);
        s.update(Some(0.016));
        assert!(s.touring());
        let (ra, dec) = radec(s.sky.constellations[leo].centre);
        s.view = View { ra, dec, ..s.view };
        assert_eq!(s.figure_target(), Some(leo));
        // far from every sign (the ecliptic pole): none
        let (ra, dec) = radec(crate::view::ecliptic_pole());
        s.view = View { ra, dec, ..s.view };
        assert_eq!(s.figure_target(), None);

        // the cursor moves: the tour goes on, and the figures follow the cursor again
        s.pointer_moved((s.center.0 + 40.0, s.center.1));
        assert!(s.touring());
        assert_eq!(s.figure_target(), s.hovered);
        // it leaves the sky (onto a window): back to the zodiac
        s.pointer_left();
        let (ra, dec) = radec(s.sky.constellations[leo].centre);
        s.view = View { ra, dec, ..s.view };
        assert_eq!(s.figure_target(), Some(leo));
    }

    #[test]
    fn switching_to_turntable_stops_the_tour() {
        let Some(mut s) = scene() else { return eprintln!("no GPU: skipped") };
        s.last_input = Instant::now() - IDLE - Duration::from_millis(10);
        s.update(Some(0.016));
        assert!(s.touring());
        s.press_chip(Chip::Nav);
        assert_eq!(s.nav, Nav::Turntable);
        assert!(!s.touring());
    }
}
