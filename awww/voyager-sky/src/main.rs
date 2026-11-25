//! voyager-sky: the voyager wallpaper as a real celestial sphere.
//!
//!   voyager-sky toggle      start drag mode, or end it (and make the view the wallpaper)
//!   voyager-sky run         start drag mode in the foreground
//!   voyager-sky export [--view RA,DEC,FOV] [--size WxH] [--out FILE] [--no-probe]
//!   voyager-sky apply       render wallpaper.png + lock.png from the saved view, show them
//!
//! Drag mode puts the live sky on the desktop layer of every monitor, behind your
//! windows, as one canvas across them: drag to turn the sky, scroll to zoom. C toggles
//! the constellation lines and names, L switches their language (中/EN), Esc ends it.
//! On exit, the view on the monitor under the cursor becomes wallpaper.png (with the
//! probe if Voyager is in it) and lock.png (without), and awww shows it.
//!
//! Docs: awww/awww.md in the voyager repo.

mod app;
mod data;
mod render;
mod scene;
mod view;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use data::SkyData;
use render::{Frame, Gpu, Renderer};
use view::{radec, Proj, View};

pub const DEFAULT_FOV: f64 = 100.0;

/// ~/.config/hypr/voyager: where the wallpaper, lock image and view live.
pub fn out_dir() -> PathBuf {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".config/hypr/voyager")
}

/// The sky data: --data, else `sky/` next to the binary (installed), else the repo's
/// awww/sky (a cargo build in awww/voyager-sky).
fn data_dir(arg: Option<&str>) -> PathBuf {
    if let Some(a) = arg {
        return PathBuf::from(a);
    }
    if let Ok(exe) = std::env::current_exe() {
        let dir = exe.parent().unwrap_or(Path::new("."));
        for cand in [dir.join("sky"), dir.join("../../../sky")] {
            if cand.join("constellations.json").exists() {
                return cand;
            }
        }
    }
    out_dir().join("sky")
}

pub fn default_view(sky: &SkyData) -> View {
    let (ra, dec) = radec(sky.voyager);
    View { ra, dec, fov: DEFAULT_FOV }
}

pub fn load_view(sky: &SkyData) -> View {
    std::fs::read(out_dir().join("view.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<View>(&b).ok())
        .map(View::clamped)
        .unwrap_or_else(|| default_view(sky))
}

pub fn save_view(v: &View) -> Result<(), String> {
    let path = out_dir().join("view.json");
    std::fs::create_dir_all(out_dir()).map_err(|e| e.to_string())?;
    std::fs::write(&path, serde_json::to_string_pretty(v).unwrap() + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

/// The frame for a still image of `view` at w x h: the view centre in the middle,
/// sized like a 1920x1080 screen scaled up (a 16:9 box fitted in w x h).
pub fn still_frame(view: &View, w: u32, h: u32, probe: bool) -> Frame {
    let u = (w as f64).min(h as f64 * 16.0 / 9.0) / view::REF_WIDTH;
    let mut proj = Proj::new(view, (w as f64 / 2.0, h as f64 / 2.0));
    proj.scale *= u;
    Frame { proj, u: u as f32, size: (w, h), time: 0.0, probe_alpha: if probe { 1.0 } else { 0.0 }, overlay: false }
}

/// wallpaper.png (probe included) and lock.png (without) for `view`, then show it.
pub fn write_wallpapers(r: &Renderer, view: &View, size: (u32, u32), show: bool) -> Result<(), String> {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    r.export(&still_frame(view, size.0, size.1, true), &dir.join("wallpaper.png"))?;
    r.export(&still_frame(view, size.0, size.1, false), &dir.join("lock.png"))?;
    save_view(view)?;
    if show {
        awww_img(&dir.join("wallpaper.png"), "none");
    }
    Ok(())
}

pub fn awww_img(path: &Path, transition: &str) {
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

fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let (w, h) = s.split_once('x').ok_or_else(|| format!("--size {s}: expected WxH"))?;
    let w: u32 = w.parse().map_err(|_| format!("--size {s}: expected WxH"))?;
    let h: u32 = h.parse().map_err(|_| format!("--size {s}: expected WxH"))?;
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return Err(format!("--size {s}: out of range"));
    }
    Ok((w, h))
}

fn parse_view(s: &str) -> Result<View, String> {
    let v: Vec<f64> = s.split(',').map(|t| t.trim().parse::<f64>()).collect::<Result<_, _>>()
        .map_err(|_| format!("--view {s}: expected RA,DEC[,FOV] in degrees"))?;
    match v.as_slice() {
        [ra, dec] => Ok(View { ra: *ra, dec: *dec, fov: DEFAULT_FOV }.clamped()),
        [ra, dec, fov] => Ok(View { ra: *ra, dec: *dec, fov: *fov }.clamped()),
        _ => Err(format!("--view {s}: expected RA,DEC[,FOV] in degrees")),
    }
}

struct Args {
    cmd: String,
    data: Option<String>,
    view: Option<String>,
    size: Option<String>,
    out: Option<String>,
    no_probe: bool,
    overlay: bool,
    cursor: Option<String>,
    english: bool,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        cmd: String::new(),
        data: None,
        view: None,
        size: None,
        out: None,
        no_probe: false,
        overlay: false,
        cursor: None,
        english: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut val = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match arg.as_str() {
            "--data" => a.data = Some(val("--data")?),
            "--view" => a.view = Some(val("--view")?),
            "--size" => a.size = Some(val("--size")?),
            "--out" => a.out = Some(val("--out")?),
            "--no-probe" => a.no_probe = true,
            "--overlay" => a.overlay = true,
            "--cursor" => a.cursor = Some(val("--cursor")?),
            "--english" => a.english = true,
            "-V" | "--version" => a.cmd = "version".into(),
            "-h" | "--help" => a.cmd = "help".into(),
            c if !c.starts_with('-') && a.cmd.is_empty() => a.cmd = c.into(),
            c => return Err(format!("unknown argument: {c}")),
        }
    }
    if a.cmd.is_empty() {
        a.cmd = "help".into();
    }
    Ok(a)
}

fn usage() -> &'static str {
    "usage: voyager-sky toggle | run | apply | export [--view RA,DEC[,FOV]] [--size WxH] [--out FILE] [--no-probe]\n\
     \x20      export --overlay [--cursor X,Y] [--english]   preview drag mode: lines and names on\n\
     \x20      [--data DIR]   sky data folder (default: sky/ next to the binary)\n\
     drag mode: drag to turn the sky, scroll to zoom, C lines + names, L 中/EN, Esc to end"
}

fn run() -> Result<(), String> {
    let a = args()?;
    match a.cmd.as_str() {
        "help" => {
            println!("{}", usage());
            Ok(())
        }
        "version" => {
            println!("voyager-sky {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "toggle" => app::toggle(&data_dir(a.data.as_deref())),
        "run" => app::run(&data_dir(a.data.as_deref())),
        "export" | "apply" => {
            let sky = SkyData::load(&data_dir(a.data.as_deref()))?;
            let view = match &a.view {
                Some(v) => parse_view(v)?,
                None if a.cmd == "apply" => load_view(&sky),
                None => default_view(&sky),
            };
            let size = match &a.size {
                Some(s) => parse_size(s)?,
                None => app::largest_output().unwrap_or((1920, 1080)),
            };
            let gpu = Gpu::new()?;
            let r = Renderer::new(&gpu, wgpu::TextureFormat::Bgra8Unorm, &sky);
            if a.cmd == "apply" {
                write_wallpapers(&r, &view, size, true)?;
                println!("wrote {}/wallpaper.png and lock.png ({}x{})", out_dir().display(), size.0, size.1);
            } else if a.overlay {
                let out = a.out.unwrap_or_else(|| "voyager-sky.png".into());
                let g = scene::Geom { origin: (0.0, 0.0), size, scale: 1 };
                let mut sc = scene::Scene::new(r, sky, view);
                sc.center = g.middle();
                sc.press_chip(0);
                if a.english {
                    sc.press_chip(1);
                }
                sc.cursor = Some(match &a.cursor {
                    Some(c) => {
                        let (x, y) = c.split_once(',').ok_or("--cursor X,Y")?;
                        (x.trim().parse().map_err(|_| "--cursor X,Y")?, y.trim().parse().map_err(|_| "--cursor X,Y")?)
                    }
                    None => g.middle(),
                });
                for _ in 0..40 {
                    sc.update(Some(0.1));
                }
                sc.snapshot(&g, Path::new(&out))?;
                println!("wrote {out} (drag-mode preview)");
            } else {
                let out = a.out.unwrap_or_else(|| "voyager-sky.png".into());
                r.export(&still_frame(&view, size.0, size.1, !a.no_probe), Path::new(&out))?;
                println!("wrote {out} ({}x{}, RA {:.2} Dec {:.2} FOV {:.0})", size.0, size.1, view.ra, view.dec, view.fov);
            }
            Ok(())
        }
        c => Err(format!("unknown command: {c}\n{}", usage())),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("voyager-sky: {e}");
            ExitCode::FAILURE
        }
    }
}
