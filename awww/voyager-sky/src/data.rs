//! The sky data in awww/sky (written by src/sky/build-sky-data.py, src/sky/query-horizons.sh and,
//! for figures/, src/sky/install-stellarium-figures.py), and the palette.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::view::{cross, normalize, unit, V3};

#[derive(Deserialize)]
struct ConstellationsFile {
    stars: Vec<[f64; 6]>,
    constellations: Vec<ConstellationRaw>,
}

#[derive(Deserialize)]
struct ConstellationRaw {
    abbr: String,
    en: String,
    zh: String,
    lines: Vec<[usize; 2]>,
    label: [f64; 2],
}

/// A figure's placement in figures/frames.json (src/sky/install-stellarium-figures.py).
#[derive(Deserialize)]
struct FrameRaw {
    file: String,
    centre: [f64; 3],
    half: f64,
}

#[derive(Deserialize)]
struct VoyagerFile {
    ra: f64,
    dec: f64,
}

pub struct Star {
    pub pos: V3,
    pub mag: f32,
    pub rgb: [u8; 3],
}

pub struct Constellation {
    pub abbr: String,
    pub en: String,
    pub zh: String,
    /// star index pairs
    pub lines: Vec<[usize; 2]>,
    pub label: V3,
    /// Its figure: figures/<figure>, a square of the gnomonic tangent plane at `centre`
    /// (the normalised mean of its line stars), east to the left and north up, reaching
    /// `half` tangent units from the middle to each edge. All three come from
    /// figures/frames.json, written with the images by src/sky/install-stellarium-figures.py.
    pub figure: String,
    pub centre: V3,
    pub half: f64,
}

/// East and north at sky point `c`, for a tangent plane there.
pub fn tangent_basis(c: V3) -> (V3, V3) {
    let e = normalize([-c[1], c[0], 0.0]);
    (e, cross(c, e))
}

pub struct FieldStar {
    pub pos: V3,
    pub size: f32,
    pub rgba: [u8; 4],
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub struct SkyData {
    /// where it was loaded from (the figures are read from here when needed)
    pub dir: PathBuf,
    pub stars: Vec<Star>,
    pub constellations: Vec<Constellation>,
    pub field: Vec<FieldStar>,
    pub milky_way: Image,
    pub probe: Image,
    /// Voyager 1 and Voyager 2, in that order
    pub voyagers: [V3; 2],
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn png(path: &Path) -> Result<Image, String> {
    let img = image::load_from_memory(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))?;
    let rgba = img.to_rgba8();
    Ok(Image { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() })
}

impl SkyData {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let raw: ConstellationsFile = serde_json::from_slice(&read(&dir.join("constellations.json"))?)
            .map_err(|e| format!("constellations.json: {e}"))?;
        let mut frames: std::collections::HashMap<String, FrameRaw> =
            serde_json::from_slice(&read(&dir.join("figures").join("frames.json"))?)
                .map_err(|e| format!("figures/frames.json: {e}"))?;
        let stars: Vec<Star> = raw
            .stars
            .iter()
            .map(|s| Star { pos: unit(s[0], s[1]), mag: s[2] as f32, rgb: [s[3] as u8, s[4] as u8, s[5] as u8] })
            .collect();
        let constellations = raw
            .constellations
            .into_iter()
            .map(|c| {
                let f = frames.remove(&c.abbr).ok_or_else(|| format!("figures/frames.json: no {}", c.abbr))?;
                Ok(Constellation {
                    abbr: c.abbr,
                    en: c.en,
                    zh: c.zh,
                    lines: c.lines,
                    label: unit(c.label[0], c.label[1]),
                    figure: f.file,
                    centre: normalize(f.centre),
                    half: f.half,
                })
            })
            .collect::<Result<_, String>>()?;

        let bytes = read(&dir.join("field.bin"))?;
        if bytes.len() % 16 != 0 {
            return Err("field.bin: size is not a multiple of 16 bytes".into());
        }
        let f32_at = |b: &[u8], i: usize| f32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
        let field = bytes
            .chunks_exact(16)
            .map(|b| FieldStar {
                pos: unit(f32_at(b, 0) as f64, f32_at(b, 4) as f64),
                rgba: [b[8], b[9], b[10], b[11]],
                size: f32_at(b, 12),
            })
            .collect();

        let voyager = |n: u32| -> Result<V3, String> {
            let file = format!("voyager{n}.json");
            let v: VoyagerFile =
                serde_json::from_slice(&read(&dir.join(&file))?).map_err(|e| format!("{file}: {e}"))?;
            Ok(unit(v.ra, v.dec))
        };

        Ok(SkyData {
            dir: dir.to_path_buf(),
            stars,
            constellations,
            field,
            milky_way: png(&dir.join("milkyway.png"))?,
            probe: png(&dir.join("probe.png"))?,
            voyagers: [voyager(1)?, voyager(2)?],
        })
    }
}

impl SkyData {
    /// Constellation `i`'s figure image (see `crate::figures`).
    pub fn figure_path(&self, i: usize) -> PathBuf {
        self.dir.join("figures").join(&self.constellations[i].figure)
    }
}

/// lib/palette, compiled in: navy, void, indigo, teal, mist, ink as 0..1 RGB.
pub struct Palette {
    pub top: [f32; 3],
    pub bottom: [f32; 3],
    pub indigo: [f32; 3],
    pub teal: [f32; 3],
    pub mist: [f32; 3],
    pub ink: [f32; 3],
}

impl Palette {
    pub fn get() -> Self {
        let text = include_str!("../../../lib/palette");
        let colour = |name: &str| -> [f32; 3] {
            let line = text
                .lines()
                .find(|l| l.split_whitespace().next() == Some(name))
                .unwrap_or_else(|| panic!("lib/palette has no {name}"));
            let hex = line.split_whitespace().nth(1).unwrap().trim_start_matches('#');
            let c = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap() as f32 / 255.0;
            [c(0), c(2), c(4)]
        };
        Palette {
            top: colour("navy"),
            bottom: colour("void"),
            indigo: colour("indigo"),
            teal: colour("teal"),
            mist: colour("mist"),
            ink: colour("ink"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_star_is_in_one_constellation() {
        // `members` (in the data, not read by the renderer) lists each constellation's
        // stars by IAU boundary, its own stick-figure stars first. No star is in two: a
        // figure may borrow a neighbour's star (Alpheratz in the Square of Pegasus), but
        // that star stays a member of the one constellation that holds it.
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sky");
        let raw: serde_json::Value = serde_json::from_slice(&read(&dir.join("constellations.json")).unwrap()).unwrap();
        let n = raw["stars"].as_array().unwrap().len();
        assert!(n > 2500, "only {n} stars");
        let mut owner: Vec<Option<&str>> = vec![None; n];
        for c in raw["constellations"].as_array().unwrap() {
            let abbr = c["abbr"].as_str().unwrap();
            for v in c["members"].as_array().unwrap() {
                let i = v.as_u64().unwrap() as usize;
                assert!(i < n, "{abbr}");
                assert!(owner[i].is_none(), "star {i} in both {} and {abbr}", owner[i].unwrap());
                owner[i] = Some(abbr);
            }
        }
        assert!(owner.iter().all(Option::is_some), "a star in no constellation");
        let borrowed = raw["constellations"].as_array().unwrap().iter().map(|c| {
            let abbr = c["abbr"].as_str().unwrap();
            let mut ends: Vec<usize> = c["lines"].as_array().unwrap().iter()
                .flat_map(|l| l.as_array().unwrap().iter().map(|i| i.as_u64().unwrap() as usize)).collect();
            ends.sort_unstable();
            ends.dedup();
            ends.iter().filter(|&&i| owner[i] != Some(abbr)).count()
        }).sum::<usize>();
        assert!(borrowed < 12, "{borrowed} borrowed figure stars");
    }

    #[test]
    fn every_constellation_has_its_figure() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sky");
        let sky = SkyData::load(&dir).unwrap();
        assert_eq!(sky.constellations.len(), 88);
        for (i, c) in sky.constellations.iter().enumerate() {
            assert!(c.figure.is_ascii(), "{}", c.en);
            let img = png(&sky.figure_path(i)).unwrap_or_else(|e| panic!("{}: {e}", c.en));
            assert_eq!((img.width, img.height), (1024, 1024), "{}", c.en);
            assert!(c.half > 0.0 && c.half < 5.0, "{}: half {}", c.abbr, c.half);
        }
    }
}
