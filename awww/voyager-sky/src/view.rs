//! Where the camera looks, and the projection between the sky and the screen.
//!
//! Stereographic around the view centre, celestial north up, east to the left (the
//! sky seen from inside the sphere). sky.wgsl has the same maths.

use serde::{Deserialize, Serialize};

pub type V3 = [f64; 3];

pub fn unit(ra: f64, dec: f64) -> V3 {
    let (ra, dec) = (ra.to_radians(), dec.to_radians());
    [dec.cos() * ra.cos(), dec.cos() * ra.sin(), dec.sin()]
}

pub fn radec(p: V3) -> (f64, f64) {
    let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
    (p[1].atan2(p[0]).to_degrees().rem_euclid(360.0), (p[2] / n).clamp(-1.0, 1.0).asin().to_degrees())
}

fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub const FOV_MIN: f64 = 40.0;
pub const FOV_MAX: f64 = 140.0;
/// Width of the reference screen the field of view is measured across.
pub const REF_WIDTH: f64 = 1920.0;

/// The saved view: the sky at the screen centre, and the horizontal field of view
/// across a 1920-px-wide screen (wider screens see proportionally more).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct View {
    pub ra: f64,
    pub dec: f64,
    pub fov: f64,
}

impl View {
    pub fn clamped(mut self) -> Self {
        self.ra = self.ra.rem_euclid(360.0);
        self.dec = self.dec.clamp(-90.0, 90.0);
        self.fov = self.fov.clamp(FOV_MIN, FOV_MAX);
        self
    }

    /// Pixels per stereographic unit, for logical pixels (1920 across REF_WIDTH).
    pub fn scale(&self) -> f64 {
        (REF_WIDTH / 2.0) / (2.0 * (self.fov.to_radians() / 4.0).tan())
    }

    /// The camera basis: east, north and forward at the view centre.
    pub fn basis(&self) -> (V3, V3, V3) {
        let (a, d) = (self.ra.to_radians(), self.dec.to_radians());
        let f = [d.cos() * a.cos(), d.cos() * a.sin(), d.sin()];
        let e = [-a.sin(), a.cos(), 0.0];
        let n = [-d.sin() * a.cos(), -d.sin() * a.sin(), d.cos()];
        (e, n, f)
    }
}

/// A view placed on a canvas: `center` is where the view centre sits, in canvas
/// pixels (logical), and `scale` is pixels per stereographic unit.
#[derive(Clone, Copy, Debug)]
pub struct Proj {
    pub e: V3,
    pub n: V3,
    pub f: V3,
    pub center: (f64, f64),
    pub scale: f64,
}

impl Proj {
    pub fn new(view: &View, center: (f64, f64)) -> Self {
        let (e, n, f) = view.basis();
        Proj { e, n, f, center, scale: view.scale() }
    }

    /// Sky -> canvas pixel and the cosine of the angle from the view centre.
    pub fn project(&self, p: V3) -> (f64, f64, f64) {
        let (x, y, z) = (dot(p, self.e), dot(p, self.n), dot(p, self.f));
        let k = 2.0 / (1.0 + z).max(1e-6);
        (self.center.0 - k * x * self.scale, self.center.1 - k * y * self.scale, z)
    }

    pub fn unproject(&self, px: (f64, f64)) -> V3 {
        let x = -(px.0 - self.center.0) / self.scale;
        let y = -(px.1 - self.center.1) / self.scale;
        let r2 = x * x + y * y;
        let z = (4.0 - r2) / (4.0 + r2);
        let h = 0.5 * (1.0 + z);
        let p = [
            x * h * self.e[0] + y * h * self.n[0] + z * self.f[0],
            x * h * self.e[1] + y * h * self.n[1] + z * self.f[1],
            x * h * self.e[2] + y * h * self.n[2] + z * self.f[2],
        ];
        let l = dot(p, p).sqrt();
        [p[0] / l, p[1] / l, p[2] / l]
    }
}

/// Move `view` so the sky point `p` lands at canvas pixel `px`, the view centre
/// staying at `center`. Grab-the-sky dragging and zoom-about-the-cursor both use it.
pub fn pin(view: View, center: (f64, f64), p: V3, px: (f64, f64)) -> View {
    let (ra_p, dec_p) = radec(p);
    let mut v = view;
    for _ in 0..4 {
        let (ra_q, dec_q) = radec(Proj::new(&v, center).unproject(px));
        let dra = (ra_p - ra_q + 540.0).rem_euclid(360.0) - 180.0;
        v.ra += dra;
        v.dec += dec_p - dec_q;
        v = v.clamped();
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let v = View { ra: 258.88, dec: 12.05, fov: 100.0 };
        let pr = Proj::new(&v, (960.0, 540.0));
        for &(x, y) in &[(0.0, 0.0), (960.0, 540.0), (1919.0, 1079.0), (3000.0, -200.0)] {
            let (x2, y2, _) = pr.project(pr.unproject((x, y)));
            assert!((x - x2).abs() < 1e-6 && (y - y2).abs() < 1e-6, "{x},{y} -> {x2},{y2}");
        }
    }

    #[test]
    fn centre_east_left_north_up() {
        let v = View { ra: 258.88, dec: 12.05, fov: 100.0 };
        let pr = Proj::new(&v, (960.0, 540.0));
        let (x, y, _) = pr.project(unit(258.88, 12.05));
        assert!((x - 960.0).abs() < 1e-6 && (y - 540.0).abs() < 1e-6);
        let (xe, _, _) = pr.project(unit(268.88, 12.05));
        assert!(xe < 960.0, "east should be left");
        let (_, yn, _) = pr.project(unit(258.88, 22.05));
        assert!(yn < 540.0, "north should be up");
    }

    #[test]
    fn fov_spans_the_reference_width() {
        let v = View { ra: 0.0, dec: 0.0, fov: 100.0 };
        let pr = Proj::new(&v, (960.0, 540.0));
        let (x, _, _) = pr.project(unit(310.0, 0.0)); // 50 degrees west: right edge
        assert!((x - 1920.0).abs() < 1e-6, "{x}");
    }

    #[test]
    fn pin_moves_point_under_cursor() {
        let v = View { ra: 100.0, dec: 20.0, fov: 90.0 };
        let c = (960.0, 540.0);
        let p = Proj::new(&v, c).unproject((500.0, 300.0));
        let v2 = pin(v, c, p, (700.0, 450.0));
        let (x, y, _) = Proj::new(&v2, c).project(p);
        assert!((x - 700.0).abs() < 0.5 && (y - 450.0).abs() < 0.5, "{x},{y}");
    }
}
