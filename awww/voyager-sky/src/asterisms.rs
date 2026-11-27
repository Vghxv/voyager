//! The seasonal asterisms (S) and the bright stars named on hover (with C).
//!
//! Positions are J2000 RA/Dec in degrees. Over a wallpaper's lifetime these stars move
//! far less than a pixel, so they are compiled in rather than read from awww/sky.

use crate::view::{angle, cross, normalize, rotate, unit, V3};

pub struct NamedStar {
    pub en: &'static str,
    pub zh: &'static str,
    pub ra: f64,
    pub dec: f64,
}

const fn star(en: &'static str, zh: &'static str, ra: f64, dec: f64) -> NamedStar {
    NamedStar { en, zh, ra, dec }
}

/// The asterisms' stars, and Polaris.
pub const STARS: [NamedStar; 14] = [
    star("Denebola", "五帝座一", 177.265, 14.572),
    star("Arcturus", "大角星", 213.915, 19.182),
    star("Spica", "角宿一", 201.298, -11.161),
    star("Vega", "織女星", 279.235, 38.784),
    star("Altair", "牛郎星", 297.696, 8.868),
    star("Deneb", "天津四", 310.358, 45.280),
    star("Markab", "室宿一", 346.190, 15.205),
    star("Scheat", "室宿二", 345.944, 28.083),
    star("Alpheratz", "壁宿二", 2.097, 29.091),
    star("Algenib", "壁宿一", 3.309, 15.184),
    star("Betelgeuse", "參宿四", 88.793, 7.407),
    star("Sirius", "天狼星", 101.287, -16.716),
    star("Procyon", "南河三", 114.825, 5.225),
    star("Polaris", "北極星", 37.955, 89.264),
];

pub struct Asterism {
    pub en: &'static str,
    pub zh: &'static str,
    /// indices into STARS, in order around the outline (it closes back to the first)
    pub stars: &'static [usize],
}

pub const ASTERISMS: [Asterism; 4] = [
    Asterism { en: "Spring Triangle", zh: "春季大三角", stars: &[0, 1, 2] },
    Asterism { en: "Summer Triangle", zh: "夏季大三角", stars: &[3, 4, 5] },
    Asterism { en: "Great Square of Pegasus", zh: "秋季四邊形", stars: &[6, 7, 8, 9] },
    Asterism { en: "Winter Triangle", zh: "冬季大三角", stars: &[10, 11, 12] },
];

/// Each outline edge is drawn as pieces of at most this many degrees along the great
/// circle, so it curves as the projection does; the gaps the line shader leaves at
/// each piece's ends make it dashed, like the ecliptic.
const PIECE_DEG: f64 = 2.0;

pub fn star_pos(i: usize) -> V3 {
    unit(STARS[i].ra, STARS[i].dec)
}

/// Where an asterism's name goes: the middle of its stars.
pub fn label_pos(a: &Asterism) -> V3 {
    let s = a.stars.iter().map(|&i| star_pos(i)).fold([0.0; 3], |m, p| [m[0] + p[0], m[1] + p[1], m[2] + p[2]]);
    normalize(s)
}

/// Every asterism's outline as short great-circle pieces (a, b, asterism index), in
/// ASTERISMS order. The renderer and the scene both use this, so their line counts agree.
pub fn segments() -> Vec<(V3, V3, usize)> {
    let mut out = Vec::new();
    for (k, a) in ASTERISMS.iter().enumerate() {
        let n = a.stars.len();
        for e in 0..n {
            let (p, q) = (star_pos(a.stars[e]), star_pos(a.stars[(e + 1) % n]));
            let total = angle(p, q);
            let axis = normalize(cross(p, q));
            let pieces = (total.to_degrees() / PIECE_DEG).ceil().max(1.0) as usize;
            let at = |j: usize| rotate(p, axis, total * j as f64 / pieces as f64);
            out.extend((0..pieces).map(|j| (at(j), at(j + 1), k)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{angle, dot};

    #[test]
    fn outlines_join_their_stars() {
        let segs = segments();
        for (k, a) in ASTERISMS.iter().enumerate() {
            let mine: Vec<_> = segs.iter().filter(|s| s.2 == k).collect();
            // consecutive pieces meet, and the outline closes
            for w in mine.windows(2) {
                assert!(angle(w[0].1, w[1].0) < 1e-9, "{}", a.en);
            }
            assert!(angle(mine[0].0, mine.last().unwrap().1) < 1e-9, "{} is not closed", a.en);
            for s in &mine {
                assert!(angle(s.0, s.1).to_degrees() <= PIECE_DEG + 1e-9);
                assert!((dot(s.0, s.0) - 1.0).abs() < 1e-9);
            }
        }
    }
}
