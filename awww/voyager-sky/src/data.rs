//! The sky data src/build-sky-data.py and src/query-horizons.sh wrote, and the palette.

use std::path::Path;

use serde::Deserialize;

use crate::view::{unit, V3};

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
    #[allow(dead_code)]
    pub abbr: String,
    pub en: String,
    pub zh: String,
    /// star index pairs
    pub lines: Vec<[usize; 2]>,
    pub label: V3,
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
    pub stars: Vec<Star>,
    pub constellations: Vec<Constellation>,
    pub field: Vec<FieldStar>,
    pub milky_way: Image,
    pub probe: Image,
    pub voyager: V3,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn png(path: &Path) -> Result<Image, String> {
    let img = image::load_from_memory(&read(path)?).map_err(|e| format!("{}: {e}", path.display()))?;
    let rgba = img.to_rgba8();
    Ok(Image { width: rgba.width(), height: rgba.height(), rgba: rgba.into_raw() })
}

impl SkyData {
    pub fn load(dir: &Path) -> Result<Self, String> {
        let raw: ConstellationsFile = serde_json::from_slice(&read(&dir.join("constellations.json"))?)
            .map_err(|e| format!("constellations.json: {e}"))?;
        let stars = raw
            .stars
            .iter()
            .map(|s| Star { pos: unit(s[0], s[1]), mag: s[2] as f32, rgb: [s[3] as u8, s[4] as u8, s[5] as u8] })
            .collect();
        let constellations = raw
            .constellations
            .into_iter()
            .map(|c| Constellation {
                abbr: c.abbr,
                en: c.en,
                zh: c.zh,
                lines: c.lines,
                label: unit(c.label[0], c.label[1]),
            })
            .collect();

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

        let v: VoyagerFile = serde_json::from_slice(&read(&dir.join("voyager1.json"))?)
            .map_err(|e| format!("voyager1.json: {e}"))?;

        Ok(SkyData {
            stars,
            constellations,
            field,
            milky_way: png(&dir.join("milkyway.png"))?,
            probe: png(&dir.join("probe.png"))?,
            voyager: unit(v.ra, v.dec),
        })
    }
}

/// lib/palette, compiled in: navy, void, indigo, mist as 0..1 RGB.
pub struct Palette {
    pub top: [f32; 3],
    pub bottom: [f32; 3],
    pub indigo: [f32; 3],
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
            mist: colour("mist"),
            ink: colour("ink"),
        }
    }
}
