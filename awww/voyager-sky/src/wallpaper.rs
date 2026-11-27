//! The files voyager-sky keeps in ~/.config/hypr/voyager: the saved view, and the
//! wallpaper.png / lock.png rendered from it, which awww is then told to show.

use std::path::{Path, PathBuf};

use crate::data::SkyData;
use crate::render::{Frame, Renderer};
use crate::view::{self, radec, Proj, View};

pub const DEFAULT_FOV: f64 = 100.0;

/// ~/.config/hypr/voyager: where the wallpaper, lock image and view live.
pub fn out_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".config/hypr/voyager")
}

/// Centred on Voyager 1.
pub fn default_view(sky: &SkyData) -> View {
    let (ra, dec) = radec(sky.voyagers[0]);
    View { ra, dec, fov: DEFAULT_FOV, roll: 0.0 }
}

/// The saved view, or the default if there is none (or it is unreadable).
pub fn load_view(sky: &SkyData) -> View {
    std::fs::read(out_dir().join("view.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<View>(&b).ok())
        .map(View::clamped)
        .unwrap_or_else(|| default_view(sky))
}

fn save_view(v: &View) -> Result<(), String> {
    let path = out_dir().join("view.json");
    let json = serde_json::to_string_pretty(v).expect("a View always serialises") + "\n";
    std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))
}

/// The frame for a still image of `view` at w x h: the view centre in the middle,
/// sized like a 1920x1080 screen scaled up (a 16:9 box fitted in w x h).
pub fn still_frame(view: &View, (w, h): (u32, u32), probe: bool) -> Frame {
    let u = (w as f64).min(h as f64 * 16.0 / 9.0) / view::REF_WIDTH;
    let mut proj = Proj::new(view, (w as f64 / 2.0, h as f64 / 2.0));
    proj.scale *= u;
    Frame { proj, u: u as f32, size: (w, h), time: 0.0, probe_alpha: [if probe { 1.0 } else { 0.0 }; 2], overlay: false }
}

/// wallpaper.png (probe included) and lock.png (without) for `view`, save the view,
/// and with `show`, have awww put the wallpaper up.
pub fn write(r: &Renderer, view: &View, size: (u32, u32), show: bool) -> Result<(), String> {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    r.export(&still_frame(view, size, true), &dir.join("wallpaper.png"))?;
    r.export(&still_frame(view, size, false), &dir.join("lock.png"))?;
    save_view(view)?;
    if show {
        awww_img(&dir.join("wallpaper.png"), "none");
    }
    Ok(())
}

fn awww_img(path: &Path, transition: &str) {
    // awww caches by path: clear it so the same path with new pixels is re-read
    let _ = std::process::Command::new("awww").arg("clear-cache").status();
    match std::process::Command::new("awww")
        .args(["img", &path.to_string_lossy(), "--transition-type", transition])
        .status()
    {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("voyager-sky: awww img exited with {s}"),
        Err(e) => eprintln!("voyager-sky: could not run awww: {e}"),
    }
}
