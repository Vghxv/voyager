//! Where the camera looks, and the projection between the sky and the screen.
//!
//! Stereographic around the view centre, east to the left (the sky seen from inside
//! the sphere). With no roll celestial north is up; a roll turns the picture about the
//! view centre (Trackball drags leave one, and the wallpaper keeps it). sky.wgsl has
//! the same maths.

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

// --- vectors ------------------------------------------------------------------------------

pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

pub fn neg(a: V3) -> V3 {
    mul(a, -1.0)
}

pub fn normalize(a: V3) -> V3 {
    mul(a, 1.0 / dot(a, a).sqrt().max(1e-300))
}

/// Angle between two unit vectors, radians.
pub fn angle(a: V3, b: V3) -> f64 {
    dot(cross(a, b), cross(a, b)).sqrt().atan2(dot(a, b))
}

/// `v` turned by `angle` (radians) about the unit `axis` (Rodrigues).
pub fn rotate(v: V3, axis: V3, angle: f64) -> V3 {
    let (s, c) = angle.sin_cos();
    add(add(mul(v, c), mul(cross(axis, v), s)), mul(axis, dot(axis, v) * (1.0 - c)))
}

/// `v` turned by the shortest rotation that takes unit `a` to unit `b`, times `t`.
fn rotate_toward(v: V3, a: V3, b: V3, t: f64) -> V3 {
    let ax = cross(a, b);
    let s = dot(ax, ax).sqrt();
    if s < 1e-12 {
        return v;
    }
    rotate(v, mul(ax, 1.0 / s), s.atan2(dot(a, b)) * t)
}

/// A camera basis: east (screen left), north (screen up), forward (the view centre).
pub type Basis = (V3, V3, V3);

/// `b` turned about the unit `axis` by `angle` radians.
pub fn rotate_basis(b: Basis, axis: V3, angle: f64) -> Basis {
    (rotate(b.0, axis, angle), rotate(b.1, axis, angle), rotate(b.2, axis, angle))
}

/// `b` turned by the shortest rotation that brings its forward to unit `f`: the view
/// moves there without turning about its own centre.
pub fn turn_basis_to(b: Basis, f: V3) -> Basis {
    (rotate_toward(b.0, b.2, f, 1.0), rotate_toward(b.1, b.2, f, 1.0), rotate_toward(b.2, b.2, f, 1.0))
}

/// Move `cur` a fraction `t` of the way to `target` (forward first, then the turn
/// about it), staying orthonormal. Also returns how far off it still is, radians.
pub fn ease_basis(cur: Basis, target: Basis, t: f64) -> (Basis, f64) {
    let (e, n, f) = cur;
    let (e, n, f) = (rotate_toward(e, f, target.2, t), rotate_toward(n, f, target.2, t), rotate_toward(f, f, target.2, t));
    // the turn left: the target's north seen around the new forward
    let nt = normalize(add(target.1, mul(f, -dot(target.1, f))));
    let roll = dot(cross(n, nt), f).atan2(dot(n, nt));
    let b = rotate_basis((e, n, f), f, roll * t);
    let left = angle(b.2, target.2) + (roll * (1.0 - t)).abs();
    (b, left)
}

// --- the view -----------------------------------------------------------------------------

pub const FOV_MIN: f64 = 40.0;
pub const FOV_MAX: f64 = 140.0;
/// Width of the reference screen the field of view is measured across.
pub const REF_WIDTH: f64 = 1920.0;

/// The saved view: the sky at the screen centre, the horizontal field of view across a
/// 1920-px-wide screen (wider screens see proportionally more), and the roll: how far
/// the picture is turned from celestial-north-up, degrees (older view.json files have
/// none, which reads as 0).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct View {
    pub ra: f64,
    pub dec: f64,
    pub fov: f64,
    #[serde(default)]
    pub roll: f64,
}

impl View {
    pub fn clamped(mut self) -> Self {
        self.ra = self.ra.rem_euclid(360.0);
        self.dec = self.dec.clamp(-90.0, 90.0);
        self.fov = self.fov.clamp(FOV_MIN, FOV_MAX);
        self.roll = (self.roll + 180.0).rem_euclid(360.0) - 180.0;
        self
    }

    /// Pixels per stereographic unit, for logical pixels (1920 across REF_WIDTH).
    pub fn scale(&self) -> f64 {
        (REF_WIDTH / 2.0) / (2.0 * (self.fov.to_radians() / 4.0).tan())
    }

    /// The camera basis without the roll: east and north at the view centre.
    fn level_basis(&self) -> Basis {
        let (a, d) = (self.ra.to_radians(), self.dec.to_radians());
        let f = [d.cos() * a.cos(), d.cos() * a.sin(), d.sin()];
        let e = [-a.sin(), a.cos(), 0.0];
        let n = [-d.sin() * a.cos(), -d.sin() * a.sin(), d.cos()];
        (e, n, f)
    }

    /// The camera basis: screen-left, screen-up and forward at the view centre.
    pub fn basis(&self) -> Basis {
        let (e0, n0, f) = self.level_basis();
        let (s, c) = self.roll.to_radians().sin_cos();
        (add(mul(e0, c), mul(n0, s)), add(mul(e0, -s), mul(n0, c)), f)
    }

    /// The view with camera basis `b` (orthonormal) and field of view `fov`.
    pub fn from_basis(b: Basis, fov: f64) -> Self {
        let (ra, dec) = radec(b.2);
        let level = View { ra, dec, fov, roll: 0.0 };
        let (e0, n0, _) = level.level_basis();
        View { roll: dot(b.0, n0).atan2(dot(b.0, e0)).to_degrees(), ..level }.clamped()
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
        normalize(p)
    }
}

/// Turntable: move `view` so the sky point `p` lands at canvas pixel `px`, the view
/// centre staying at `center` and the roll unchanged. Grab-the-sky dragging and
/// zoom-about-the-cursor both use it.
pub fn pin(view: View, center: (f64, f64), p: V3, px: (f64, f64)) -> View {
    let (ra_p, dec_p) = radec(p);
    let mut v = view;
    for _ in 0..6 {
        let (ra_q, dec_q) = radec(Proj::new(&v, center).unproject(px));
        let dra = (ra_p - ra_q + 540.0).rem_euclid(360.0) - 180.0;
        v.ra += dra;
        v.dec += dec_p - dec_q;
        v = v.clamped();
    }
    v
}

/// Trackball: like `pin`, but by the shortest turn of the whole sphere, so the roll
/// follows the path of the drag.
pub fn pin_free(view: View, center: (f64, f64), p: V3, px: (f64, f64)) -> View {
    let q = Proj::new(&view, center).unproject(px);
    let ax = cross(q, p);
    let s = dot(ax, ax).sqrt();
    if s < 1e-12 {
        return view;
    }
    let b = rotate_basis(view.basis(), mul(ax, 1.0 / s), s.atan2(dot(q, p)));
    View::from_basis(b, view.fov)
}

// --- the ecliptic -------------------------------------------------------------------------

/// The tilt of the ecliptic to the equator (J2000), degrees.
pub const OBLIQUITY: f64 = 23.4393;

/// The point on the ecliptic at ecliptic longitude `lon` (degrees), equatorial.
pub fn ecliptic_point(lon: f64) -> V3 {
    let (s, c) = lon.to_radians().sin_cos();
    let (se, ce) = OBLIQUITY.to_radians().sin_cos();
    [c, s * ce, s * se]
}

/// The point on the celestial equator at right ascension `ra` (degrees).
pub fn equator_point(ra: f64) -> V3 {
    let (s, c) = ra.to_radians().sin_cos();
    [c, s, 0.0]
}

/// The ecliptic north pole (in Draco), equatorial.
#[cfg(test)]
pub fn ecliptic_pole() -> V3 {
    let (se, ce) = OBLIQUITY.to_radians().sin_cos();
    [0.0, -se, ce]
}

/// Ecliptic longitude of `p`: where on the ecliptic the point nearest to it is.
pub fn ecliptic_lon(p: V3) -> f64 {
    let (se, ce) = OBLIQUITY.to_radians().sin_cos();
    (p[1] * ce + p[2] * se).atan2(p[0]).to_degrees().rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(ra: f64, dec: f64, fov: f64) -> View {
        View { ra, dec, fov, roll: 0.0 }
    }

    #[test]
    fn round_trip() {
        for roll in [0.0, 37.0] {
            let pr = Proj::new(&View { roll, ..v(258.88, 12.05, 100.0) }, (960.0, 540.0));
            for &(x, y) in &[(0.0, 0.0), (960.0, 540.0), (1919.0, 1079.0), (3000.0, -200.0)] {
                let (x2, y2, _) = pr.project(pr.unproject((x, y)));
                assert!((x - x2).abs() < 1e-6 && (y - y2).abs() < 1e-6, "{x},{y} -> {x2},{y2}");
            }
        }
    }

    #[test]
    fn centre_east_left_north_up() {
        let pr = Proj::new(&v(258.88, 12.05, 100.0), (960.0, 540.0));
        let (x, y, _) = pr.project(unit(258.88, 12.05));
        assert!((x - 960.0).abs() < 1e-6 && (y - 540.0).abs() < 1e-6);
        let (xe, _, _) = pr.project(unit(268.88, 12.05));
        assert!(xe < 960.0, "east should be left");
        let (_, yn, _) = pr.project(unit(258.88, 22.05));
        assert!(yn < 540.0, "north should be up");
    }

    #[test]
    fn roll_turns_north_sideways() {
        let pr = Proj::new(&View { roll: 90.0, ..v(100.0, 0.0, 100.0) }, (960.0, 540.0));
        let (x, y, _) = pr.project(unit(100.0, 10.0));
        assert!((y - 540.0).abs() < 1e-6 && (x - 960.0).abs() > 50.0, "{x},{y}");
    }

    #[test]
    fn fov_spans_the_reference_width() {
        let pr = Proj::new(&v(0.0, 0.0, 100.0), (960.0, 540.0));
        let (x, _, _) = pr.project(unit(310.0, 0.0)); // 50 degrees west: right edge
        assert!((x - 1920.0).abs() < 1e-6, "{x}");
    }

    #[test]
    fn basis_round_trip() {
        for &(ra, dec, roll) in &[(10.0, 20.0, 0.0), (200.0, -45.0, 70.0), (300.0, 89.9, -120.0), (5.0, -89.9, 179.0)] {
            let a = View { ra, dec, fov: 90.0, roll };
            let b = View::from_basis(a.basis(), 90.0);
            let (ea, na, fa) = a.basis();
            let (eb, nb, fb) = b.basis();
            for (p, q) in [(ea, eb), (na, nb), (fa, fb)] {
                assert!(angle(p, q) < 1e-9, "{a:?} -> {b:?}");
            }
        }
    }

    #[test]
    fn pin_moves_point_under_cursor() {
        for roll in [0.0, 40.0] {
            let v = View { roll, ..v(100.0, 20.0, 90.0) };
            let c = (960.0, 540.0);
            let p = Proj::new(&v, c).unproject((500.0, 300.0));
            let v2 = pin(v, c, p, (700.0, 450.0));
            let (x, y, _) = Proj::new(&v2, c).project(p);
            assert!((x - 700.0).abs() < 0.5 && (y - 450.0).abs() < 0.5, "roll {roll}: {x},{y}");
            assert!((v2.roll - roll).abs() < 1e-9);
        }
    }

    #[test]
    fn pin_free_moves_point_under_cursor() {
        let v = View { roll: 15.0, ..v(100.0, 70.0, 90.0) };
        let c = (960.0, 540.0);
        let p = Proj::new(&v, c).unproject((200.0, 900.0));
        let v2 = pin_free(v, c, p, (1500.0, 200.0));
        let (x, y, _) = Proj::new(&v2, c).project(p);
        assert!((x - 1500.0).abs() < 1e-6 && (y - 200.0).abs() < 1e-6, "{x},{y}");
    }

    #[test]
    fn ease_basis_arrives() {
        let a = View { roll: 30.0, ..v(10.0, 10.0, 90.0) }.basis();
        let t = View { roll: -50.0, ..v(120.0, -40.0, 90.0) }.basis();
        let (mid, left) = ease_basis(a, t, 0.5);
        assert!(left > 0.1);
        assert!((dot(mid.0, mid.1)).abs() < 1e-9 && (dot(mid.1, mid.2)).abs() < 1e-9);
        let (end, left) = ease_basis(a, t, 1.0);
        assert!(left < 1e-9);
        assert!(angle(end.0, t.0) < 1e-9 && angle(end.1, t.1) < 1e-9 && angle(end.2, t.2) < 1e-9);
    }

    #[test]
    fn turn_basis_keeps_roll_at_the_centre() {
        let b = View { roll: 25.0, ..v(40.0, 10.0, 90.0) }.basis();
        let f = unit(50.0, 12.0);
        let t = turn_basis_to(b, f);
        assert!(angle(t.2, f) < 1e-12);
        assert!(dot(t.0, t.1).abs() < 1e-12 && dot(t.1, t.2).abs() < 1e-12);
        // already there: nothing turns, so easing toward it changes nothing
        let (same, left) = ease_basis(t, turn_basis_to(t, f), 0.5);
        assert!(left < 1e-12 && angle(same.1, t.1) < 1e-12);
    }

    #[test]
    fn equator() {
        for ra in [0.0, 83.0, 200.0, 359.0] {
            let (r, d) = radec(equator_point(ra));
            assert!((r - ra).abs() < 1e-9 && d.abs() < 1e-9, "{ra}: {r} {d}");
        }
    }

    #[test]
    fn ecliptic() {
        for lon in [0.0, 45.0, 90.0, 200.0, 359.0] {
            let p = ecliptic_point(lon);
            assert!(dot(p, ecliptic_pole()).abs() < 1e-12);
            assert!((ecliptic_lon(p) - lon).abs() < 1e-9);
        }
        // the summer solstice point: RA 6h, Dec +23.44
        let (ra, dec) = radec(ecliptic_point(90.0));
        assert!((ra - 90.0).abs() < 1e-9 && (dec - OBLIQUITY).abs() < 1e-9);
        // a point off the ecliptic maps to the ecliptic point under it
        let p = normalize(add(ecliptic_point(120.0), mul(ecliptic_pole(), 0.4)));
        assert!((ecliptic_lon(p) - 120.0).abs() < 1e-9);
    }

    #[test]
    fn old_view_json_has_no_roll() {
        let v: View = serde_json::from_str(r#"{"ra": 1.0, "dec": 2.0, "fov": 90.0}"#).unwrap();
        assert_eq!(v.roll, 0.0);
    }
}
