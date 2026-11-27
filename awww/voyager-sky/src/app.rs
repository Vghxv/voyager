//! Drag mode's Wayland side: a live sky on the desktop layer of every monitor.
//!
//! One layer surface per output, on the "bottom" layer: below windows, above awww's
//! wallpaper, so dragging works on empty desktop and windows keep their clicks. The
//! outputs form one canvas (their logical layout); the view centre sits in the middle
//! of the monitor drag mode started on, so that monitor shows exactly the current
//! wallpaper and the others show the sky beside it. scene.rs holds what is shown;
//! this file only turns Wayland events into calls on it.
//!
//! Two ways in:
//!   run     drag mode right away, on the monitor under the cursor; the process ends with it
//!   listen  stays running with an invisible surface on the same layer of every monitor.
//!           It only gets clicks where no window covers the desktop, so holding the left
//!           button there (LONG_PRESS, without moving) opens drag mode, and that same
//!           press already holds the sky. Esc saves the view and goes back to listening.
//!
//! Drag mode takes the keyboard only while the cursor is on the sky: on a window,
//! typing goes to the window as usual.
//!
//! Frames are paced by Wayland frame callbacks and drawn only while something
//! changes: a drag, a fade (names, figures, the probe), a snap, the zodiac tour, or the
//! slow flicker of the constellation lines, which on its own is drawn at 12 fps.

use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::{Duration, Instant};

use raw_window_handle::{RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState, FrameCallbackData, Region},
    delegate_dispatch2, delegate_registry,
    dispatch2::Dispatch2,
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
    shm::{
        slot::{Buffer, SlotPool},
        Shm, ShmHandler,
    },
};
use wayland_client::{
    globals::{registry_queue_init, GlobalList},
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, Proxy, QueueHandle,
};
use wayland_protocols::wp::viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter};

use crate::data::SkyData;
use crate::{hypr, instance, wallpaper};
use crate::render::{Gpu, Renderer, Target};
use crate::scene::{Chip, Geom, Scene};

const BTN_LEFT: u32 = 0x110;

/// How long the left button must be held on empty desktop before drag mode opens.
const LONG_PRESS: Duration = Duration::from_millis(400);
/// Moving further than this while holding cancels it: that was a drag, not a long-press.
const PRESS_SLOP_PX: f64 = 6.0;

// --- entry points ------------------------------------------------------------------------

/// Start drag mode, or end the running one (which makes its view the wallpaper).
pub fn toggle(data: &Path) -> Result<(), String> {
    if instance::stop(instance::RUN) {
        return Ok(());
    }
    run(data)
}

/// Drag mode now, on the monitor under the cursor; returns when it ends.
pub fn run(data: &Path) -> Result<(), String> {
    if let Some(pid) = instance::running(instance::RUN) {
        return Err(format!("drag mode is already on (pid {pid}); end it with: voyager-sky toggle"));
    }
    let _pid = instance::PidFile::claim(instance::RUN)?;
    instance::catch_stop_signals();

    let (mut app, mut event_loop) = App::connect(data, false)?;
    let home = hypr::cursor_pos().and_then(|p| app.output_at(p));
    app.open_drag(home, None)?;
    while app.drag.as_ref().is_some_and(|d| !d.scene.exit) && !instance::stopping() {
        app.step(&mut event_loop)?;
    }
    app.close_drag()
}

/// Wait for a long-press on empty desktop, open drag mode, repeat; until SIGTERM/SIGINT.
pub fn listen(data: &Path) -> Result<(), String> {
    if let Some(pid) = instance::running(instance::LISTEN) {
        return Err(format!("already listening (pid {pid})"));
    }
    let _pid = instance::PidFile::claim(instance::LISTEN)?;
    instance::catch_stop_signals();

    let (mut app, mut event_loop) = App::connect(data, true)?;
    if app.viewporter.is_none() {
        return Err("the compositor has no wp_viewporter, which listen needs".into());
    }
    for output in app.outputs.outputs().collect::<Vec<_>>() {
        app.add_listener(output);
    }
    while !instance::stopping() {
        app.step(&mut event_loop)?;
    }
    app.close_drag()
}

// --- state -----------------------------------------------------------------------------

/// One monitor's live sky while drag mode is on.
struct Screen {
    // the wgpu surface must go before the Wayland surface it draws to
    surface: wgpu::Surface<'static>,
    target: Target,
    layer: LayerSurface,
    output: wl_output::WlOutput,
    geom: Geom,
    configured: bool,
    frame_pending: bool,
    dirty: bool,
}

/// Everything drag mode needs; dropped when it ends, so listening holds no GPU memory.
struct Drag {
    // field order is drop order: the screens' wgpu surfaces, then the renderer, then the GPU
    screens: Vec<Screen>,
    scene: Scene,
    format: wgpu::TextureFormat,
    present_mode: wgpu::PresentMode,
    gpu: Gpu,
}

/// One monitor's invisible click-catcher while listening: a 1x1 transparent buffer
/// stretched over the monitor.
struct Listener {
    layer: LayerSurface,
    output: wl_output::WlOutput,
    viewport: WpViewport,
    buffer: Option<Buffer>,
    geom: Geom,
}

/// A left button held on a listener, not yet long enough to count.
struct Press {
    since: Instant,
    at: (f64, f64),
    output: wl_output::WlOutput,
}

struct App {
    qh: QueueHandle<App>,
    conn: Connection,
    registry: RegistryState,
    seats: SeatState,
    outputs: OutputState,
    compositor: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    pool: Option<SlotPool>,
    viewporter: Option<WpViewporter>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    data: PathBuf,
    /// listen mode: keep the listeners and return to them after each drag
    listening: bool,
    listeners: Vec<Listener>,
    press: Option<Press>,
    drag: Option<Drag>,
}

impl Drag {
    fn mark_dirty(&mut self) {
        for s in &mut self.screens {
            s.dirty = true;
        }
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

    /// Take the keyboard while the cursor is on this sky surface, give it back when it
    /// leaves (onto a window), so drag mode's keys never steal typing from windows.
    fn grab_keyboard(&self, surface: &wl_surface::WlSurface, on: bool) {
        if let Some(s) = self.screens.iter().find(|s| s.layer.wl_surface() == surface) {
            s.layer.set_keyboard_interactivity(if on {
                KeyboardInteractivity::Exclusive
            } else {
                KeyboardInteractivity::None
            });
            s.layer.commit();
        }
    }

    fn draw(&mut self, i: usize, qh: &QueueHandle<App>) {
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
        ws.frame(qh, FrameCallbackData(ws.clone()));
        tex.present();
        s.frame_pending = true;
        s.dirty = false;
    }

    /// Draw every screen that needs it and is ready.
    fn tick(&mut self, qh: &QueueHandle<App>) {
        if self.scene.animating() || self.scene.flicker_due_in() == Some(Duration::ZERO) {
            self.mark_dirty();
        }
        let ready: Vec<usize> = (0..self.screens.len())
            .filter(|&i| self.screens[i].dirty && self.screens[i].configured && !self.screens[i].frame_pending)
            .collect();
        if ready.is_empty() {
            return;
        }
        self.scene.update(None);
        for &i in &ready {
            self.draw(i, qh);
        }
        self.scene.trim();
    }
}

impl App {
    fn connect(data: &Path, listening: bool) -> Result<(App, EventLoop<'static, App>), String> {
        let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
        let (globals, mut queue) = registry_queue_init::<App>(&conn).map_err(|e| e.to_string())?;
        let qh = queue.handle();
        let mut app = App {
            qh: qh.clone(),
            conn: conn.clone(),
            registry: RegistryState::new(&globals),
            seats: SeatState::new(&globals, &qh),
            outputs: OutputState::new(&globals, &qh),
            compositor: CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor: {e}"))?,
            layer_shell: LayerShell::bind(&globals, &qh).map_err(|e| format!("layer shell: {e}"))?,
            shm: Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm: {e}"))?,
            pool: None,
            viewporter: bind_viewporter(&globals, &qh),
            keyboard: None,
            pointer: None,
            data: data.to_path_buf(),
            listening,
            listeners: Vec::new(),
            press: None,
            drag: None,
        };
        // learn the outputs and their layout (xdg-output) before making any surfaces
        queue.roundtrip(&mut app).map_err(|e| e.to_string())?;
        queue.roundtrip(&mut app).map_err(|e| e.to_string())?;

        let event_loop: EventLoop<App> = EventLoop::try_new().map_err(|e| e.to_string())?;
        WaylandSource::new(conn, queue).insert(event_loop.handle()).map_err(|e| e.to_string())?;
        Ok((app, event_loop))
    }

    /// Wait for events (no longer than the next thing due), then act on them.
    fn step(&mut self, event_loop: &mut EventLoop<'static, App>) -> Result<(), String> {
        let wait = match (&self.drag, &self.press) {
            (Some(d), _) if d.scene.animating() => Duration::from_millis(8),
            (Some(d), _) => d.scene.flicker_due_in().map_or(Duration::from_millis(250), |t| t.min(Duration::from_millis(250))),
            (None, Some(p)) => LONG_PRESS.saturating_sub(p.since.elapsed()),
            (None, None) => Duration::from_secs(1),
        };
        if let Err(e) = event_loop.dispatch(Some(wait), self) {
            // a stop signal can interrupt the wait; that is not an error
            if !instance::stopping() {
                return Err(e.to_string());
            }
        }

        if self.press.as_ref().is_some_and(|p| p.since.elapsed() >= LONG_PRESS) {
            let p = self.press.take().unwrap();
            if let Err(e) = self.open_drag(Some(p.output), Some(p.at)) {
                eprintln!("voyager-sky: could not start drag mode: {e}");
            }
        }
        let Some(d) = &mut self.drag else { return Ok(()) };
        d.tick(&self.qh);
        if d.scene.exit && self.listening {
            if let Err(e) = self.close_drag() {
                eprintln!("voyager-sky: {e}");
            }
        }
        Ok(())
    }

    /// The output whose area holds global point `p`.
    fn output_at(&self, p: (f64, f64)) -> Option<wl_output::WlOutput> {
        self.outputs.outputs().find(|o| output_geom(&self.outputs, o, None).is_some_and(|g| g.contains(p)))
    }

    /// The monitor a surface of ours sits on, for turning its local coordinates global.
    fn geom_of(&self, surface: &wl_surface::WlSurface) -> Option<Geom> {
        let screens = self.drag.iter().flat_map(|d| &d.screens);
        screens
            .map(|s| (&s.layer, s.geom))
            .chain(self.listeners.iter().map(|l| (&l.layer, l.geom)))
            .find(|(layer, _)| layer.wl_surface() == surface)
            .map(|(_, g)| g)
    }

    // --- listening ----------------------------------------------------------------------

    fn add_listener(&mut self, output: wl_output::WlOutput) {
        let Some(viewporter) = &self.viewporter else { return };
        let Some(geom) = output_geom(&self.outputs, &output, None) else { return };
        let surface = self.compositor.create_surface(&self.qh);
        let viewport = viewporter.get_viewport(&surface, &self.qh, Noop);
        let layer =
            self.layer_shell.create_layer_surface(&self.qh, surface, Layer::Bottom, Some("voyager-sky-listen"), Some(&output));
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(0, 0);
        layer.commit();
        self.listeners.push(Listener { layer, output, viewport, buffer: None, geom });
    }

    fn configure_listener(&mut self, i: usize, size: (u32, u32)) {
        let l = &mut self.listeners[i];
        if size.0 > 0 && size.1 > 0 {
            l.geom.size = size;
        }
        if let Some(g) = output_geom(&self.outputs, &l.output, Some(l.geom.size)) {
            l.geom = g;
        }
        let surface = l.layer.wl_surface();
        l.viewport.set_destination(l.geom.size.0 as i32, l.geom.size.1 as i32);
        if l.buffer.is_none() {
            let pool = match &mut self.pool {
                Some(p) => p,
                None => match SlotPool::new(64, &self.shm) {
                    Ok(p) => self.pool.insert(p),
                    Err(e) => return eprintln!("voyager-sky: shm pool: {e}"),
                },
            };
            match pool.create_buffer(1, 1, 4, wl_shm::Format::Argb8888) {
                Ok((buffer, canvas)) => {
                    canvas.fill(0); // fully transparent
                    if buffer.attach_to(surface).is_ok() {
                        l.buffer = Some(buffer);
                    }
                }
                Err(e) => return eprintln!("voyager-sky: shm buffer: {e}"),
            }
        }
        surface.commit();
    }

    /// While drag mode is on the listeners stop taking input, so it goes to the sky.
    fn set_listeners_accept_input(&self, accept: bool) {
        let empty = if accept { None } else { Region::new(&self.compositor).ok() };
        for l in &self.listeners {
            let surface = l.layer.wl_surface();
            surface.set_input_region(empty.as_ref().map(|r| r.wl_region()));
            surface.commit();
        }
    }

    fn listener_pointer(&mut self, kind: &PointerEventKind, p: (f64, f64), surface: &wl_surface::WlSurface) {
        match kind {
            PointerEventKind::Press { button, .. } if *button == BTN_LEFT => {
                let output = self.listeners.iter().find(|l| l.layer.wl_surface() == surface).map(|l| l.output.clone());
                self.press = output.map(|output| Press { since: Instant::now(), at: p, output });
            }
            PointerEventKind::Motion { .. } => {
                if let Some(pr) = &self.press {
                    if (p.0 - pr.at.0).hypot(p.1 - pr.at.1) > PRESS_SLOP_PX {
                        self.press = None;
                    }
                }
            }
            // let go too soon, left the desktop, another button, a scroll: not a long-press
            PointerEventKind::Release { .. }
            | PointerEventKind::Leave { .. }
            | PointerEventKind::Press { .. }
            | PointerEventKind::Axis { .. } => self.press = None,
            _ => {}
        }
    }

    // --- drag mode ----------------------------------------------------------------------

    /// Put the live sky on every monitor, centred on `home`. With `held_at`, the left
    /// button is already down there (a long-press), so the sky is grabbed at once.
    fn open_drag(&mut self, home: Option<wl_output::WlOutput>, held_at: Option<(f64, f64)>) -> Result<(), String> {
        let sky = SkyData::load(&self.data)?;
        let gpu = Gpu::new()?;
        // Vulkan on Wayland offers B8G8R8A8_UNORM; checked against each surface below
        let format = wgpu::TextureFormat::Bgra8Unorm;
        let view = wallpaper::load_view(&sky);
        let mut scene = Scene::new(Renderer::new(&gpu, format, &sky), sky, view);
        scene.load_figures_in_background();
        let mut d = Drag { screens: Vec::new(), scene, format, present_mode: wgpu::PresentMode::Fifo, gpu };

        let display = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(self.conn.backend().display_ptr() as *mut _).ok_or("no Wayland display")?,
        ));
        for output in self.outputs.outputs().collect::<Vec<_>>() {
            let Some(info) = self.outputs.info(&output) else { continue };
            let Some(geom) = output_geom(&self.outputs, &output, None) else { continue };
            let surface = self.compositor.create_surface(&self.qh);
            surface.set_buffer_scale(info.scale_factor.max(1));
            let layer =
                self.layer_shell.create_layer_surface(&self.qh, surface, Layer::Bottom, Some("voyager-sky"), Some(&output));
            layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
            layer.set_exclusive_zone(-1);
            // the keyboard is ours only while the cursor is on the sky (see grab_keyboard);
            // a long-press starts there, so its monitor takes it at once
            let held_here = held_at.is_some() && home.as_ref() == Some(&output);
            layer.set_keyboard_interactivity(if held_here {
                KeyboardInteractivity::Exclusive
            } else {
                KeyboardInteractivity::None
            });
            layer.set_size(0, 0);
            layer.commit();
            let window = RawWindowHandle::Wayland(WaylandWindowHandle::new(
                NonNull::new(layer.wl_surface().id().as_ptr() as *mut _).ok_or("no surface")?,
            ));
            let surface = unsafe {
                d.gpu
                    .instance
                    .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                        raw_display_handle: display,
                        raw_window_handle: window,
                    })
                    .map_err(|e| format!("GPU surface: {e}"))?
            };
            let caps = surface.get_capabilities(&d.gpu.adapter);
            if !caps.formats.contains(&format) {
                return Err(format!("the GPU cannot draw {format:?} to this display (it offers {:?})", caps.formats));
            }
            if caps.present_modes.contains(&wgpu::PresentMode::Mailbox) {
                d.present_mode = wgpu::PresentMode::Mailbox;
            }
            let target = d.scene.renderer.target(Some(&mut d.scene.text));
            d.screens.push(Screen {
                surface,
                target,
                layer,
                output,
                geom,
                configured: false,
                frame_pending: false,
                dirty: true,
            });
        }
        if d.screens.is_empty() {
            return Err("no monitors found".into());
        }

        // the view centre goes in the middle of the home monitor
        let home = home.and_then(|o| d.screens.iter().position(|s| s.output == o)).unwrap_or(0);
        let home_geom = d.screens[home].geom;
        d.scene.center = home_geom.middle();
        if let Some(p) = held_at {
            d.scene.pointer_moved(p);
            d.scene.press(p, &home_geom);
        }

        self.set_listeners_accept_input(false);
        self.drag = Some(d);
        Ok(())
    }

    /// Make the view on the home monitor the wallpaper, then take the live sky away.
    fn close_drag(&mut self) -> Result<(), String> {
        let Some(d) = self.drag.take() else { return Ok(()) };
        let v = d.scene.view;
        let size = d.screens.iter().map(|s| s.geom.pixels()).max_by_key(|&(w, h)| w as u64 * h as u64).unwrap_or((1920, 1080));
        let result = wallpaper::write(&d.scene.renderer, &v, size, true);
        std::thread::sleep(Duration::from_millis(250)); // let awww put it up underneath
        drop(d);
        if self.listening {
            self.set_listeners_accept_input(true);
        }
        let _ = self.conn.flush();
        result
    }
}

/// An output's place on the canvas (logical px), with `size` overriding its own.
fn output_geom(outputs: &OutputState, output: &wl_output::WlOutput, size: Option<(u32, u32)>) -> Option<Geom> {
    let info = outputs.info(output)?;
    let (x, y) = info.logical_position.unwrap_or(info.location);
    let (w, h) = size.unwrap_or_else(|| {
        let (w, h) = info.logical_size.unwrap_or((1920, 1080));
        (w.max(1) as u32, h.max(1) as u32)
    });
    Some(Geom { origin: (x as f64, y as f64), size: (w, h), scale: info.scale_factor.max(1) })
}

fn bind_viewporter(globals: &GlobalList, qh: &QueueHandle<App>) -> Option<WpViewporter> {
    globals.bind::<WpViewporter, _, _>(qh, 1..=1, Noop).ok()
}

/// User data for protocol objects with no events we care about (the viewporter's).
struct Noop;

impl Dispatch2<WpViewporter, App> for Noop {
    fn event(&self, _: &mut App, _: &WpViewporter, _: <WpViewporter as Proxy>::Event, _: &Connection, _: &QueueHandle<App>) {}
}

impl Dispatch2<WpViewport, App> for Noop {
    fn event(&self, _: &mut App, _: &WpViewport, _: <WpViewport as Proxy>::Event, _: &Connection, _: &QueueHandle<App>) {}
}

// --- Wayland handlers ------------------------------------------------------------------

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        let Some(d) = &mut self.drag else { return };
        if let Some(s) = d.screens.iter_mut().find(|s| s.layer.wl_surface() == surface) {
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
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        // a monitor plugged in while listening gets its own listener
        if self.listening && !self.listeners.is_empty() {
            self.add_listener(output);
        }
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        // moved or rescaled: keep the listener's idea of where it is current
        if let Some(l) = self.listeners.iter_mut().find(|l| l.output == output) {
            if let Some(g) = output_geom(&self.outputs, &output, Some(l.geom.size)) {
                l.geom = g;
            }
        }
    }
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.listeners.retain(|l| l.output != output);
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        self.listeners.retain(|l| &l.layer != layer);
        // a monitor went away during drag mode: drop that screen, keep going on the others
        if let Some(d) = &mut self.drag {
            d.screens.retain(|s| &s.layer != layer);
            if d.screens.is_empty() {
                d.scene.exit = true;
            }
        }
    }

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface, cfg: LayerSurfaceConfigure, _: u32) {
        if let Some(i) = self.listeners.iter().position(|l| &l.layer == layer) {
            return self.configure_listener(i, cfg.new_size);
        }
        let Some(d) = &mut self.drag else { return };
        let Some(i) = d.screens.iter().position(|s| &s.layer == layer) else { return };
        if cfg.new_size.0 > 0 && cfg.new_size.1 > 0 {
            d.screens[i].geom.size = cfg.new_size;
        }
        d.configure_surface(&d.screens[i]);
        let s = &mut d.screens[i];
        s.configured = true;
        s.frame_pending = false;
        s.dirty = true;
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
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
            let Some(g) = self.geom_of(&ev.surface) else { continue };
            let p = (g.origin.0 + ev.position.0, g.origin.1 + ev.position.1);
            if self.drag.is_none() {
                self.listener_pointer(&ev.kind, p, &ev.surface);
                continue;
            }
            let d = self.drag.as_mut().unwrap();
            // in drag mode, events may still come from the listener the long-press
            // started on (the compositor keeps a held button on its surface); they are
            // in global coordinates all the same
            match ev.kind {
                PointerEventKind::Enter { .. } => d.grab_keyboard(&ev.surface, true),
                PointerEventKind::Leave { .. } => d.grab_keyboard(&ev.surface, false),
                _ => {}
            }
            match ev.kind {
                PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => d.scene.pointer_moved(p),
                PointerEventKind::Leave { .. } => d.scene.pointer_left(),
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => d.scene.press(p, &g),
                PointerEventKind::Release { button, .. } if button == BTN_LEFT => d.scene.release(p),
                PointerEventKind::Axis { vertical, .. } => {
                    let steps = if vertical.absolute != 0.0 { vertical.absolute / 15.0 } else { vertical.discrete as f64 };
                    if steps != 0.0 {
                        d.scene.zoom(p, steps);
                    }
                }
                _ => continue,
            }
            d.mark_dirty();
        }
    }
}

impl KeyboardHandler for App {
    fn enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32, _: &[u32], _: &[Keysym]) {}
    fn leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: &wl_surface::WlSurface, _: u32) {}
    fn press_key(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_keyboard::WlKeyboard, _: u32, ev: KeyEvent) {
        let Some(d) = &mut self.drag else { return };
        d.scene.key_input();
        match ev.keysym {
            Keysym::t | Keysym::T => d.scene.press_chip(Chip::Nav),
            Keysym::r | Keysym::R => d.scene.level(),
            Keysym::c | Keysym::C => d.scene.press_chip(Chip::Lines),
            Keysym::f | Keysym::F => d.scene.press_chip(Chip::Figures),
            Keysym::e | Keysym::E => d.scene.press_chip(Chip::Ecliptic),
            Keysym::q | Keysym::Q => d.scene.press_chip(Chip::Equator),
            Keysym::l | Keysym::L => d.scene.press_chip(Chip::Lang),
            Keysym::Escape => d.scene.exit = true,
            _ => return,
        }
        d.mark_dirty();
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
