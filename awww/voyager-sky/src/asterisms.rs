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

// Indices into STARS, so the outlines below read as names.
const DENEBOLA: usize = 0;
const ARCTURUS: usize = 1;
const SPICA: usize = 2;
const VEGA: usize = 3;
const ALTAIR: usize = 4;
const DENEB: usize = 5;
const MARKAB: usize = 6;
const SCHEAT: usize = 7;
const ALPHERATZ: usize = 8;
const ALGENIB: usize = 9;
const BETELGEUSE: usize = 10;
const SIRIUS: usize = 11;
const PROCYON: usize = 12;
const DUBHE: usize = 14;
const MERAK: usize = 15;
const PHECDA: usize = 16;
const MEGREZ: usize = 17;
const ALIOTH: usize = 18;
const MIZAR: usize = 19;
const ALKAID: usize = 20;
const RIGEL: usize = 21;
const ALDEBARAN: usize = 22;
const CAPELLA: usize = 23;
const POLLUX: usize = 24;
const ALNASL: usize = 25;
const KAUS_MEDIA: usize = 26;
const KAUS_AUSTRALIS: usize = 27;
const KAUS_BOREALIS: usize = 28;
const PHI_SGR: usize = 29;
const NUNKI: usize = 30;
const TAU_SGR: usize = 31;
const ASCELLA: usize = 32;

/// The asterisms' stars, and Polaris (13).
pub const STARS: [NamedStar; 33] = [
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
    star("Dubhe", "天樞", 165.932, 61.751),
    star("Merak", "天璇", 165.460, 56.382),
    star("Phecda", "天璣", 178.458, 53.695),
    star("Megrez", "天權", 183.857, 57.033),
    star("Alioth", "玉衡", 193.507, 55.960),
    star("Mizar", "開陽", 200.981, 54.925),
    star("Alkaid", "搖光", 206.885, 49.313),
    star("Rigel", "參宿七", 78.634, -8.202),
    star("Aldebaran", "畢宿五", 68.980, 16.509),
    star("Capella", "五車二", 79.172, 45.998),
    star("Pollux", "北河三", 116.329, 28.026),
    star("Alnasl", "箕宿一", 271.452, -30.424),
    star("Kaus Media", "箕宿二", 275.249, -29.828),
    star("Kaus Australis", "箕宿三", 276.043, -34.385),
    star("Kaus Borealis", "斗宿二", 276.993, -25.422),
    star("Phi Sagittarii", "斗宿三", 281.414, -26.991),
    star("Nunki", "斗宿四", 283.816, -26.297),
    star("Tau Sagittarii", "斗宿五", 286.735, -27.671),
    star("Ascella", "斗宿六", 285.653, -29.880),
];

pub struct Asterism {
    pub en: &'static str,
    pub zh: &'static str,
    /// polylines through STARS indices; a closed outline repeats its first star at the end
    pub paths: &'static [&'static [usize]],
}

pub const ASTERISMS: [Asterism; 8] = [
    Asterism {
        en: "Big Dipper",
        zh: "北斗七星",
        paths: &[&[ALKAID, MIZAR, ALIOTH, MEGREZ, PHECDA, MERAK, DUBHE, MEGREZ]],
    },
    // the handle's arc carried on to Arcturus and Spica (the handle itself is the Dipper's)
    Asterism {
        en: "Spring Arc",
        zh: "春季大曲線",
        paths: &[&[ALKAID, ARCTURUS, SPICA]],
    },
    Asterism {
        en: "Spring Triangle",
        zh: "春季大三角",
        paths: &[&[DENEBOLA, ARCTURUS, SPICA, DENEBOLA]],
    },
    Asterism {
        en: "Summer Triangle",
        zh: "夏季大三角",
        paths: &[&[VEGA, ALTAIR, DENEB, VEGA]],
    },
    Asterism {
        en: "Teapot",
        zh: "茶壺",
        paths: &[
            // spout and body
            &[
                KAUS_AUSTRALIS,
                ALNASL,
                KAUS_MEDIA,
                KAUS_AUSTRALIS,
                ASCELLA,
                PHI_SGR,
                KAUS_MEDIA,
            ],
            // lid
            &[KAUS_MEDIA, KAUS_BOREALIS, PHI_SGR],
            // handle
            &[PHI_SGR, NUNKI, TAU_SGR, ASCELLA],
        ],
    },
    Asterism {
        en: "Great Square of Pegasus",
        zh: "秋季四邊形",
        paths: &[&[MARKAB, SCHEAT, ALPHERATZ, ALGENIB, MARKAB]],
    },
    Asterism {
        en: "Winter Triangle",
        zh: "冬季大三角",
        paths: &[&[BETELGEUSE, SIRIUS, PROCYON, BETELGEUSE]],
    },
    Asterism {
        en: "Winter Hexagon",
        zh: "冬季大橢圓",
        paths: &[&[RIGEL, SIRIUS, PROCYON, POLLUX, CAPELLA, ALDEBARAN, RIGEL]],
    },
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
    let mut ids: Vec<usize> = a.paths.iter().flat_map(|p| p.iter().copied()).collect();
    ids.sort_unstable();
    ids.dedup();
    let s = ids
        .iter()
        .map(|&i| star_pos(i))
        .fold([0.0; 3], |m, p| [m[0] + p[0], m[1] + p[1], m[2] + p[2]]);
    normalize(s)
}

/// Every asterism's outline as short great-circle pieces (a, b, asterism index), in
/// ASTERISMS order. The renderer and the scene both use this, so their line counts agree.
pub fn segments() -> Vec<(V3, V3, usize)> {
    let mut out = Vec::new();
    for (k, a) in ASTERISMS.iter().enumerate() {
        for path in a.paths {
            for e in path.windows(2) {
                let (p, q) = (star_pos(e[0]), star_pos(e[1]));
                let total = angle(p, q);
                let axis = normalize(cross(p, q));
                let pieces = (total.to_degrees() / PIECE_DEG).ceil().max(1.0) as usize;
                let at = |j: usize| rotate(p, axis, total * j as f64 / pieces as f64);
                out.extend((0..pieces).map(|j| (at(j), at(j + 1), k)));
            }
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
        let mut at = 0;
        for (k, a) in ASTERISMS.iter().enumerate() {
            for path in a.paths {
                assert!(path.len() >= 2, "{}", a.en);
                // each path's pieces run from its first star to its last without a break
                let n: usize = path
                    .windows(2)
                    .map(|e| {
                        (angle(star_pos(e[0]), star_pos(e[1])).to_degrees() / PIECE_DEG)
                            .ceil()
                            .max(1.0) as usize
                    })
                    .sum();
                let mine = &segs[at..at + n];
                at += n;
                assert!(mine.iter().all(|s| s.2 == k));
                for w in mine.windows(2) {
                    assert!(angle(w[0].1, w[1].0) < 1e-9, "{}", a.en);
                }
                assert!(angle(mine[0].0, star_pos(path[0])) < 1e-9, "{}", a.en);
                assert!(
                    angle(mine[n - 1].1, star_pos(*path.last().unwrap())) < 1e-9,
                    "{}",
                    a.en
                );
                for s in mine {
                    assert!(angle(s.0, s.1).to_degrees() <= PIECE_DEG + 1e-9);
                    assert!((dot(s.0, s.0) - 1.0).abs() < 1e-9);
                }
            }
        }
        assert_eq!(at, segs.len());
    }

    #[test]
    fn no_edge_is_drawn_twice() {
        for a in &ASTERISMS {
            let mut edges: Vec<(usize, usize)> = a
                .paths
                .iter()
                .flat_map(|p| p.windows(2).map(|e| (e[0].min(e[1]), e[0].max(e[1]))))
                .collect();
            assert!(
                edges.iter().all(|&(i, j)| i != j && j < STARS.len()),
                "{}",
                a.en
            );
            let n = edges.len();
            edges.sort_unstable();
            edges.dedup();
            assert_eq!(edges.len(), n, "{} draws an edge twice", a.en);
        }
    }

    #[test]
    fn index_names_match() {
        for (i, en) in [
            (DUBHE, "Dubhe"),
            (ALKAID, "Alkaid"),
            (RIGEL, "Rigel"),
            (POLLUX, "Pollux"),
            (ALNASL, "Alnasl"),
            (ASCELLA, "Ascella"),
        ] {
            assert_eq!(STARS[i].en, en);
        }
        assert_eq!(STARS[13].en, "Polaris");
    }
}
