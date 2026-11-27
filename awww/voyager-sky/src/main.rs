//! voyager-sky: the voyager wallpaper as a real celestial sphere.
//!
//!   voyager-sky listen      stay running; hold the left button on empty desktop to start drag mode
//!   voyager-sky toggle      start drag mode, or end it (and make the view the wallpaper)
//!   voyager-sky run         start drag mode in the foreground
//!   voyager-sky export [--view RA,DEC[,FOV[,ROLL]]] [--size WxH] [--out FILE] [--no-probe]
//!   voyager-sky export --overlay [--cursor X,Y] [--english] [--ecliptic] [--equator] [--figures]   a drag-mode screenshot
//!   voyager-sky apply       render wallpaper.png + lock.png from the saved view, show them
//!
//! listen keeps an invisible surface under your windows on every monitor. Holding the
//! left button on bare desktop (about 0.4 s, without moving) opens drag mode with the
//! sky already in hand; Esc saves the view and goes back to listening.
//!
//! Drag mode puts the live sky on the desktop layer of every monitor, behind your
//! windows, as one canvas across them: drag to turn the sky, scroll to zoom, or use the
//! XYZ ball bottom right. T switches Turntable/Trackball, R levels the sky, C toggles the
//! constellation lines and names, F the figure of the constellation under the cursor,
//! E the ecliptic, Q the celestial equator, L switches their language (中/EN),
//! Esc ends it. Left alone 5 s in Trackball, it tours the zodiac along the ecliptic.
//! On exit, the view on the monitor drag mode started on becomes wallpaper.png (with the
//! probe if Voyager is in it) and lock.png (without), and awww shows it.
//!
//! Docs: awww/awww.md in the voyager repo.

mod app;
mod data;
mod figures;
mod hypr;
mod instance;
mod render;
mod scene;
mod view;
mod wallpaper;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use data::SkyData;
use render::{Gpu, Renderer};
use scene::{Chip, Geom, Scene};
use view::View;

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
    wallpaper::out_dir().join("sky")
}

// --- arguments ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Cmd {
    Help,
    Version,
    Listen,
    Toggle,
    Run,
    Export,
    Apply,
}

impl Cmd {
    fn parse(s: &str) -> Result<Self, String> {
        Ok(match s {
            "listen" => Cmd::Listen,
            "toggle" => Cmd::Toggle,
            "run" => Cmd::Run,
            "export" => Cmd::Export,
            "apply" => Cmd::Apply,
            c => return Err(format!("unknown command: {c}\n{}", usage())),
        })
    }
}

#[derive(Default)]
struct Args {
    cmd: Option<Cmd>,
    data: Option<String>,
    view: Option<String>,
    size: Option<String>,
    out: Option<String>,
    no_probe: bool,
    overlay: bool,
    cursor: Option<String>,
    english: bool,
    ecliptic: bool,
    equator: bool,
    figures: bool,
}

fn args() -> Result<Args, String> {
    let mut a = Args::default();
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
            "--ecliptic" => a.ecliptic = true,
            "--equator" => a.equator = true,
            "--figures" => a.figures = true,
            "-V" | "--version" => a.cmd = Some(Cmd::Version),
            "-h" | "--help" => a.cmd = Some(Cmd::Help),
            c if !c.starts_with('-') && a.cmd.is_none() => a.cmd = Some(Cmd::parse(c)?),
            c => return Err(format!("unknown argument: {c}")),
        }
    }
    Ok(a)
}

fn usage() -> &'static str {
    "usage: voyager-sky listen | toggle | run | apply | export [--view RA,DEC[,FOV[,ROLL]]] [--size WxH] [--out FILE] [--no-probe]\n\
     \x20      export --overlay [--cursor X,Y] [--english] [--ecliptic] [--equator] [--figures]   preview drag mode: lines and names on\n\
     \x20      [--data DIR]   sky data folder (default: sky/ next to the binary)\n\
     listen: hold the left button on empty desktop to start drag mode\n\
     drag mode: drag to turn the sky, scroll to zoom, XYZ ball to orbit or snap to an axis,\n\
     \x20          T turntable/trackball, R level, C lines + names, F figures, E ecliptic, Q equator, L 中/EN, Esc to end"
}

fn parse_size(s: &str) -> Result<(u32, u32), String> {
    let err = || format!("--size {s}: expected WxH");
    let (w, h) = s.split_once('x').ok_or_else(err)?;
    let (w, h): (u32, u32) = (w.parse().map_err(|_| err())?, h.parse().map_err(|_| err())?);
    if w == 0 || h == 0 || w > 16384 || h > 16384 {
        return Err(format!("--size {s}: out of range"));
    }
    Ok((w, h))
}

fn parse_view(s: &str) -> Result<View, String> {
    let err = || format!("--view {s}: expected RA,DEC[,FOV[,ROLL]] in degrees");
    let v: Vec<f64> = s.split(',').map(|t| t.trim().parse::<f64>()).collect::<Result<_, _>>().map_err(|_| err())?;
    let (ra, dec) = match v.as_slice() {
        [ra, dec, ..] if v.len() <= 4 => (*ra, *dec),
        _ => return Err(err()),
    };
    let fov = v.get(2).copied().unwrap_or(wallpaper::DEFAULT_FOV);
    let roll = v.get(3).copied().unwrap_or(0.0);
    Ok(View { ra, dec, fov, roll }.clamped())
}

fn parse_cursor(s: &str) -> Result<(f64, f64), String> {
    let err = || "--cursor X,Y".to_string();
    let (x, y) = s.split_once(',').ok_or_else(err)?;
    Ok((x.trim().parse().map_err(|_| err())?, y.trim().parse().map_err(|_| err())?))
}

// --- commands ----------------------------------------------------------------------------

/// `export` and `apply`: render without Wayland.
fn render_still(cmd: Cmd, a: Args) -> Result<(), String> {
    let sky = SkyData::load(&data_dir(a.data.as_deref()))?;
    let view = match &a.view {
        Some(v) => parse_view(v)?,
        None if cmd == Cmd::Apply => wallpaper::load_view(&sky),
        None => wallpaper::default_view(&sky),
    };
    let size = match &a.size {
        Some(s) => parse_size(s)?,
        None => hypr::largest_output().unwrap_or((1920, 1080)),
    };
    let gpu = Gpu::new()?;
    let r = Renderer::new(&gpu, wgpu::TextureFormat::Bgra8Unorm, &sky);
    let out = a.out.clone().unwrap_or_else(|| "voyager-sky.png".into());

    if cmd == Cmd::Apply {
        wallpaper::write(&r, &view, size, true)?;
        println!("wrote {}/wallpaper.png and lock.png ({}x{})", wallpaper::out_dir().display(), size.0, size.1);
    } else if a.overlay {
        let cursor = a.cursor.as_deref().map(parse_cursor).transpose()?;
        let mut sc = Scene::new(r, sky, view);
        for (on, chip) in [(a.figures, Chip::Figures), (a.ecliptic, Chip::Ecliptic), (a.equator, Chip::Equator)] {
            if on {
                sc.press_chip(chip);
            }
        }
        export_overlay(sc, size, cursor, a.english, Path::new(&out))?;
        println!("wrote {out} (drag-mode preview)");
    } else {
        r.export(&wallpaper::still_frame(&view, size, !a.no_probe), Path::new(&out))?;
        println!(
            "wrote {out} ({}x{}, RA {:.2} Dec {:.2} FOV {:.0} roll {:.0})",
            size.0, size.1, view.ra, view.dec, view.fov, view.roll
        );
    }
    Ok(())
}

/// A drag-mode screenshot: lines and names on, the cursor at `cursor` (default: the
/// middle), and the fades run to the end.
fn export_overlay(
    mut sc: Scene,
    size: (u32, u32),
    cursor: Option<(f64, f64)>,
    english: bool,
    out: &Path,
) -> Result<(), String> {
    let g = Geom { origin: (0.0, 0.0), size, scale: 1 };
    sc.center = g.middle();
    sc.press_chip(Chip::Lines);
    if english {
        sc.press_chip(Chip::Lang);
    }
    sc.cursor = Some(cursor.unwrap_or_else(|| g.middle()));
    for _ in 0..40 {
        sc.update(Some(0.1));
    }
    sc.snapshot(&g, out)
}

fn run() -> Result<(), String> {
    let a = args()?;
    let data = || data_dir(a.data.as_deref());
    match a.cmd.unwrap_or(Cmd::Help) {
        Cmd::Help => println!("{}", usage()),
        Cmd::Version => println!("voyager-sky {}", env!("CARGO_PKG_VERSION")),
        Cmd::Listen => app::listen(&data())?,
        Cmd::Toggle => app::toggle(&data())?,
        Cmd::Run => app::run(&data())?,
        cmd @ (Cmd::Export | Cmd::Apply) => render_still(cmd, a)?,
    }
    Ok(())
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
