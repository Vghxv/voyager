//! Drag mode's Wayland side: a live sky on the desktop layer of every monitor.
//!
//! One layer surface per output, on the "bottom" layer: below windows, above awww's
//! wallpaper, so dragging works on empty desktop and windows keep their clicks. The
//! outputs form one canvas (their logical layout); the view centre sits in the middle
//! of the monitor the cursor was on at start, so that monitor shows exactly the
//! current wallpaper and the others show the sky beside it. scene.rs holds what is
//! shown; this file only turns Wayland events into calls on it.
//!
//! Frames are paced by Wayland frame callbacks and drawn only while something
//! changes: a drag, a fade, or the slow flicker of the constellation lines.

use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData},
    delegate_dispatch2, delegate_registry,
    output::{OutputHandler, OutputState},
    reexports::{calloop::EventLoop, calloop_wayland_source::WaylandSource},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers},
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::{
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
        WaylandSurface,
    },
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, Proxy, QueueHandle,
};

use crate::data::SkyData;
use crate::render::{Gpu, Renderer, Target};
use crate::scene::{Geom, Scene};
use crate::view::View;

const BTN_LEFT: u32 = 0x110;

static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_: libc::c_int) {
    STOP.store(true, Ordering::SeqCst);
}

fn pidfile() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    dir.join("voyager-sky.pid")
}

/// The pid of a running drag mode, if any.
fn running() -> Option<i32> {
    let pid: i32 = std::fs::read_to_string(pidfile()).ok()?.trim().parse().ok()?;
    let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
    (comm.trim() == "voyager-sky" && pid != std::process::id() as i32).then_some(pid)
}

pub fn toggle(data: &Path) -> Result<(), String> {
    if let Some(pid) = running() {
        unsafe { libc::kill(pid, libc::SIGTERM) };
        return Ok(());
    }
    run(data)
}

// --- Hyprland queries ------------------------------------------------------------------

fn hyprctl(args: &[&str]) -> Option<serde_json::Value> {
    let out = std::process::Command::new("hyprctl").args(args).output().ok()?;
    serde_json::from_slice(&out.stdout).ok()
}

fn cursor_pos() -> Option<(f64, f64)> {
    let v = hyprctl(&["-j", "cursorpos"])?;
    Some((v["x"].as_f64()?, v["y"].as_f64()?))
}

/// The biggest connected monitor, in pixels, rotation included.
pub fn largest_output() -> Option<(u32, u32)> {
    let mons = hyprctl(&["-j", "monitors"])?;
    mons.as_array()?
        .iter()
        .filter_map(|m| {
            let (w, h) = (m["width"].as_u64()? as u32, m["height"].as_u64()? as u32);
            Some(if m["transform"].as_u64().unwrap_or(0) % 2 == 1 { (h, w) } else { (w, h) })
        })
        .max_by_key(|&(w, h)| w as u64 * h as u64)
}

// --- state -----------------------------------------------------------------------------

struct Screen {
    // the wgpu surface must go before the Wayland surface it draws to
    surface: wgpu::Surface<'static>,
    target: Target,
    layer: LayerSurface,
    geom: Geom,
    configured: bool,
    frame_pending: bool,
    dirty: bool,
}

struct App {
    qh: QueueHandle<App>,
    registry: RegistryState,
    seats: SeatState,
    outputs: OutputState,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    scene: Scene,
    screens: Vec<Screen>,
    format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    exit: bool,
}

impl App {
    fn mark_dirty(&mut self) {
        for s in &mut self.screens {
            s.dirty = true;
        }
    }

    fn screen_at(&self, p: (f64, f64)) -> Option<usize> {
        self.screens.iter().position(|s| s.geom.contains(p))
    }

    fn configure_surface(&self, s: &Screen) {
        let (width, height) = s.geom.pixels();
        s.surface.configure(
            &self.scene.renderer.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.format,
                width,
                height,
                present_mode: self.present_mode,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }

    fn draw(&mut self, i: usize) {
        let tex = match self.screens[i].surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost) => {
                self.configure_surface(&self.screens[i]);
                return;
            }
            Err(e) => {
                eprintln!("voyager-sky: frame skipped: {e}");
                return;
            }
        };
        let view = tex.texture.create_view(&Default::default());
        let s = &mut self.screens[i];
        self.scene.draw(&s.geom, &view, &mut s.target);
        let ws = s.layer.wl_surface();
        ws.frame(&self.qh, FrameCallbackData(ws.clone()));
        tex.present();
        s.frame_pending = true;
        s.dirty = false;
    }

    /// Draw every screen that needs it and is ready; returns how many were drawn.
    fn tick(&mut self) -> usize {
        if self.scene.exit {
            self.exit = true;
        }
        if self.scene.animating() {
            self.mark_dirty();
        }
        let ready: Vec<usize> = (0..self.screens.len())
            .filter(|&i| self.screens[i].dirty && self.screens[i].configured && !self.screens[i].frame_pending)
            .collect();
        if ready.is_empty() {
            return 0;
        }
        self.scene.update(None);
        for &i in &ready {
            self.draw(i);
        }
        self.scene.trim();
        ready.len()
    }

    /// The wallpaper: what the monitor drag mode started on shows. The view centre
    /// sits in its middle, so that is the view itself, wherever the cursor ends up.
    fn exit_view(&self) -> View {
        self.scene.view
    }
}

// --- run -------------------------------------------------------------------------------

pub fn run(data: &Path) -> Result<(), String> {
    if let Some(pid) = running() {
        return Err(format!("drag mode is already on (pid {pid}); end it with: voyager-sky toggle"));
    }
    let pf = pidfile();
    std::fs::write(&pf, std::process::id().to_string()).map_err(|e| format!("{}: {e}", pf.display()))?;
    struct Unlink(PathBuf);
    impl Drop for Unlink {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _guard = Unlink(pf);
    unsafe {
        libc::signal(libc::SIGTERM, on_signal as usize);
        libc::signal(libc::SIGINT, on_signal as usize);
    }

    let sky = SkyData::load(data)?;
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, mut queue) = registry_queue_init::<App>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let compositor = CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor: {e}"))?;
    let layer_shell = LayerShell::bind(&globals, &qh).map_err(|e| format!("layer shell: {e}"))?;
    let gpu = Gpu::new()?;
    // Vulkan on Wayland offers B8G8R8A8_UNORM; checked against each surface below
    let format = wgpu::TextureFormat::Bgra8Unorm;
    let view = crate::load_view(&sky);
    let scene = Scene::new(Renderer::new(&gpu, format, &sky), sky, view);

    let mut app = App {
        qh: qh.clone(),
        registry: RegistryState::new(&globals),
        seats: SeatState::new(&globals, &qh),
        outputs: OutputState::new(&globals, &qh),
        keyboard: None,
        pointer: None,
        scene,
        screens: Vec::new(),
        format,
        present_mode: wgpu::PresentMode::Fifo,
        exit: false,
    };

    // learn the outputs and their layout (xdg-output) before making the surfaces
    queue.roundtrip(&mut app).map_err(|e| e.to_string())?;
    queue.roundtrip(&mut app).map_err(|e| e.to_string())?;

    let display = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
        NonNull::new(conn.backend().display_ptr() as *mut _).ok_or("no Wayland display")?,
    ));
    for output in app.outputs.outputs().collect::<Vec<_>>() {
        let Some(info) = app.outputs.info(&output) else { continue };
        let (lx, ly) = info.logical_position.unwrap_or(info.location);
        let (lw, lh) = info.logical_size.unwrap_or((1920, 1080));
        let scale = info.scale_factor.max(1);
        let surface = compositor.create_surface(&qh);
        surface.set_buffer_scale(scale);
        let layer = layer_shell.create_layer_surface(&qh, surface, Layer::Bottom, Some("voyager-sky"), Some(&output));
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::OnDemand);
        layer.set_size(0, 0);
        layer.commit();
        let window = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(layer.wl_surface().id().as_ptr() as *mut _).ok_or("no surface")?,
        ));
        let surface = unsafe {
            gpu.instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: display,
                    raw_window_handle: window,
                })
                .map_err(|e| format!("GPU surface: {e}"))?
        };
        let caps = surface.get_capabilities(&gpu.adapter);
        if !caps.formats.contains(&format) {
            return Err(format!("the GPU cannot draw {format:?} to this display (it offers {:?})", caps.formats));
        }
        if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
            app.present_mode = wgpu::PresentMode::Mailbox;
        }
        let target = app.scene.renderer.target(Some(&mut app.scene.text));
        app.screens.push(Screen {
            surface,
            target,
            layer,
            geom: Geom { origin: (lx as f64, ly as f64), size: (lw.max(1) as u32, lh.max(1) as u32), scale },
            configured: false,
            frame_pending: false,
            dirty: true,
        });
    }
    if app.screens.is_empty() {
        return Err("no monitors found".into());
    }

    // the view centre goes in the middle of the monitor under the cursor
    let home = cursor_pos().and_then(|p| app.screen_at(p)).unwrap_or(0);
    app.scene.center = app.screens[home].geom.middle();

    let mut event_loop: EventLoop<App> = EventLoop::try_new().map_err(|e| e.to_string())?;
    WaylandSource::new(conn.clone(), queue).insert(event_loop.handle()).map_err(|e| e.to_string())?;
    let debug = std::env::var_os("VOYAGER_SKY_DEBUG").is_some();
    let (mut frames, mut since) = (0usize, Instant::now());
    while !app.exit && !STOP.load(Ordering::SeqCst) {
        let wait = if app.scene.animating() { Duration::from_millis(8) } else { Duration::from_millis(250) };
        event_loop.dispatch(Some(wait), &mut app).map_err(|e| e.to_string())?;
        frames += app.tick();
        if debug && since.elapsed() > Duration::from_secs(2) {
            eprintln!("voyager-sky: {:.1} frames/s (all screens together)", frames as f32 / since.elapsed().as_secs_f32());
            (frames, since) = (0, Instant::now());
        }
    }

    // make the view on the monitor we started on the wallpaper, then take the live sky away
    let v = app.exit_view();
    let size = app
        .screens
        .iter()
        .map(|s| s.geom.pixels())
        .max_by_key(|&(w, h)| w as u64 * h as u64)
        .unwrap_or((1920, 1080));
    let result = crate::write_wallpapers(&app.scene.renderer, &v, size, true);
    std::thread::sleep(Duration::from_millis(250)); // let awww put it up underneath
    app.screens.clear();
    let _ = conn.flush();
    result
}

// --- Wayland handlers ------------------------------------------------------------------

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        if let Some(s) = self.screens.iter_mut().find(|s| s.layer.wl_surface() == surface) {
            s.frame_pending = false;
        }
    }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        // its monitor went away: drop that screen, keep going on the others
        self.screens.retain(|s| &s.layer != layer);
        if self.screens.is_empty() {
            self.exit = true;
        }
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface, cfg: LayerSurfaceConfigure, _: u32) {
        let Some(i) = self.screens.iter().position(|s| &s.layer == layer) else { return };
        if cfg.new_size.0 > 0 && cfg.new_size.1 > 0 {
            self.screens[i].geom.size = cfg.new_size;
        }
        self.configure_surface(&self.screens[i]);
        let s = &mut self.screens[i];
        s.configured = true;
        s.frame_pending = false;
        s.dirty = true;
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seats.get_keyboard(qh, &seat, None).ok();
        }
        if cap == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seats.get_pointer(qh, &seat).ok();
        }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, cap: Capability) {
        if cap == Capability::Keyboard {
            if let Some(k) = self.keyboard.take() {
                k.release();
            }
        }
        if cap == Capability::Pointer {
            if let Some(p) = self.pointer.take() {
                p.release();
            }
        }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for ev in events {
            let Some(s) = self.screens.iter().find(|s| s.layer.wl_surface() == &ev.surface) else { continue };
            let g = s.geom;
            let p = (g.origin.0 + ev.position.0, g.origin.1 + ev.position.1);
            match ev.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => self.scene.pointer_moved(p),
                PointerEventKind::Leave { .. } => self.scene.pointer_left(),
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => self.scene.press(p, &g),
                PointerEventKind::Release { button, .. } if button == BTN_LEFT => self.scene.release(),
                PointerEventKind::Axis { vertical, .. } => {
                    let steps = if vertical.absolute != 0.0 { vertical.absolute / 15.0 } else { vertical.discrete as f64 };
                    if steps != 0.0 {
                        self.scene.zoom(p, steps);
                    }
                }
                _ => continue,
            }
            self.mark_dirty();
        }
    }
}

impl KeyboardHandler for App {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, ev: KeyEvent) {
        match ev.keysym {
            Keysym::c | Keysym::C => self.scene.press_chip(0),
            Keysym::l | Keysym::L => self.scene.press_chip(1),
            Keysym::Escape => self.exit = true,
            _ => return,
        }
        self.mark_dirty();
    }
    fn repeat_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn release_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: KeyEvent) {}
    fn update_modifiers(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, _: Modifiers, _: RawModifiers, _: u32) {}
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }
    registry_handlers![OutputState, SeatState];
}

delegate_registry!(App);
delegate_dispatch2!(App);
