//! The GPU side: one set of pipelines that draws the sky into a Wayland surface (drag
//! mode) or an offscreen texture (the wallpaper PNG), so the two always match.

use std::path::Path;

use bytemuck::{Pod, Zeroable};
use glyphon::{Cache, FontSystem, SwashCache, TextArea, TextAtlas, TextRenderer, Viewport};
use wgpu::util::DeviceExt;

use crate::data::{Palette, SkyData};
use crate::view::Proj;

/// The Milky Way over the gradient: brightness, tone curve on the sqrt-encoded map,
/// share of its colour kept. VOYAGER_SKY_MW=a,b,c overrides it, for tuning.
const MILKY_WAY: [f32; 3] = [0.62, 1.35, 0.55];

fn milky_way() -> [f32; 3] {
    std::env::var("VOYAGER_SKY_MW")
        .ok()
        .and_then(|s| {
            let v: Vec<f32> = s.split(',').filter_map(|t| t.trim().parse().ok()).collect();
            v.try_into().ok()
        })
        .unwrap_or(MILKY_WAY)
}
/// The probe's width at 1080p, in pixels.
pub const PROBE_PX: f32 = 48.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    e: [f32; 4],
    n: [f32; 4],
    f: [f32; 4],
    center: [f32; 2],
    scale: f32,
    u: f32,
    size: [f32; 2],
    time: f32,
    mw_strength: f32,
    top: [f32; 4],
    bottom: [f32; 4],
    line_col: [f32; 4],
    voyager: [f32; 4],
    probe: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PointInst {
    pos: [f32; 3],
    size_or_mag: f32,
    col: [u8; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct LineInst {
    a: [f32; 3],
    b: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Rect {
    pub rect: [f32; 4],
    pub col: [f32; 4],
    pub border: [f32; 4],
}

const MAX_RECTS: usize = 8;

/// What one frame shows, in the target's own pixels.
pub struct Frame {
    pub proj: Proj,
    /// one 1080p pixel, in target pixels
    pub u: f32,
    pub size: (u32, u32),
    pub time: f32,
    pub probe_alpha: f32,
    /// draw the constellation lines and the chips (drag mode only)
    pub overlay: bool,
}

pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            ..Default::default()
        }))
        .map_err(|e| format!("no Vulkan GPU found ({e}); is vulkan-intel / vulkan-radeon installed?"))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("voyager-sky"),
            ..Default::default()
        }))
        .map_err(|e| format!("could not open the GPU: {e}"))?;
        Ok(Gpu { instance, adapter, device, queue })
    }
}

/// glyphon's shared state: fonts, glyph cache and atlas.
pub struct TextCtx {
    pub font_system: FontSystem,
    pub swash: SwashCache,
    cache: Cache,
    pub atlas: TextAtlas,
}

impl TextCtx {
    pub fn new(r: &Renderer) -> Self {
        let cache = Cache::new(&r.device);
        let atlas = TextAtlas::with_color_mode(&r.device, &r.queue, &cache, r.format, glyphon::ColorMode::Web);
        TextCtx { font_system: FontSystem::new(), swash: SwashCache::new(), cache, atlas }
    }
}

/// Per-target state: its uniforms, and its text renderer if it shows text.
pub struct Target {
    uniform: wgpu::Buffer,
    bind: wgpu::BindGroup,
    text: Option<(TextRenderer, Viewport)>,
}

pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    mw_view: wgpu::TextureView,
    probe_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    sky: wgpu::RenderPipeline,
    field: wgpu::RenderPipeline,
    stars: wgpu::RenderPipeline,
    probe: wgpu::RenderPipeline,
    lines: wgpu::RenderPipeline,
    rects: wgpu::RenderPipeline,
    field_buf: wgpu::Buffer,
    n_field: u32,
    star_buf: wgpu::Buffer,
    n_stars: u32,
    line_buf: wgpu::Buffer,
    line_alpha: wgpu::Buffer,
    n_lines: u32,
    rect_buf: wgpu::Buffer,
    n_rects: u32,
    palette: Palette,
    milky_way: [f32; 3],
    voyager: [f32; 3],
}

fn v3(p: [f64; 3]) -> [f32; 3] {
    [p[0] as f32, p[1] as f32, p[2] as f32]
}

fn v4(p: [f32; 3], w: f32) -> [f32; 4] {
    [p[0], p[1], p[2], w]
}

fn texture(gpu: &wgpu::Device, q: &wgpu::Queue, levels: &[(u32, u32, Vec<u8>)], label: &str) -> wgpu::TextureView {
    let (w, h, _) = &levels[0];
    let tex = gpu.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (level, (w, h, px)) in levels.iter().enumerate() {
        q.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &tex,
                mip_level: level as u32,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            px,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(4 * w), rows_per_image: Some(*h) },
            wgpu::Extent3d { width: *w, height: *h, depth_or_array_layers: 1 },
        );
    }
    tex.create_view(&Default::default())
}

/// The probe and its smaller copies, so it stays clean when drawn at 48 px.
fn mip_chain(img: &crate::data::Image) -> Vec<(u32, u32, Vec<u8>)> {
    let base = image::RgbaImage::from_raw(img.width, img.height, img.rgba.clone()).expect("probe size");
    let mut out = vec![(img.width, img.height, img.rgba.clone())];
    let (mut w, mut h) = (img.width, img.height);
    while w > 1 && h > 1 {
        w = (w / 2).max(1);
        h = (h / 2).max(1);
        let small = image::imageops::resize(&base, w, h, image::imageops::FilterType::Lanczos3);
        out.push((w, h, small.into_raw()));
    }
    out
}

impl Renderer {
    pub fn new(gpu: &Gpu, format: wgpu::TextureFormat, sky: &SkyData) -> Self {
        let device = gpu.device.clone();
        let queue = gpu.queue.clone();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sky.wgsl"),
            source: wgpu::ShaderSource::Wgsl(include_str!("sky.wgsl").into()),
        });

        let entry = |binding, visibility, ty| wgpu::BindGroupLayoutEntry { binding, visibility, ty, count: None };
        let tex_ty = wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let vf = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky"),
            entries: &[
                entry(0, vf, wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                }),
                entry(1, wgpu::ShaderStages::FRAGMENT, tex_ty),
                entry(2, wgpu::ShaderStages::FRAGMENT, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
                entry(3, wgpu::ShaderStages::FRAGMENT, tex_ty),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sky"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });

        let blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        let pipeline = |name: &str, vs: &str, fs: &str, buffers: &[wgpu::VertexBufferLayout], blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(name),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(vs),
                    buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState { format, blend, write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: Default::default(),
                multiview: None,
                cache: None,
            })
        };
        let inst = |stride, attrs: &'static [wgpu::VertexAttribute]| wgpu::VertexBufferLayout {
            array_stride: stride,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: attrs,
        };
        const POINT: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Unorm8x4];
        const LINE: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        const ALPHA: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![2 => Float32];
        const RECT: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let point_layout = inst(20, &POINT);

        let sky_p = pipeline("sky", "sky_vs", "sky_fs", &[], None);
        let field = pipeline("field", "field_vs", "field_fs", &[point_layout.clone()], blend);
        let stars = pipeline("stars", "star_vs", "star_fs", &[point_layout], blend);
        let probe = pipeline("probe", "probe_vs", "probe_fs", &[], blend);
        let lines = pipeline("lines", "line_vs", "line_fs", &[inst(24, &LINE), inst(4, &ALPHA)], blend);
        let rects = pipeline("rects", "rect_vs", "rect_fs", &[inst(48, &RECT)], blend);

        let field_inst: Vec<PointInst> = sky
            .field
            .iter()
            .map(|s| PointInst { pos: v3(s.pos), size_or_mag: s.size, col: s.rgba })
            .collect();
        let star_inst: Vec<PointInst> = sky
            .stars
            .iter()
            .map(|s| PointInst { pos: v3(s.pos), size_or_mag: s.mag, col: [s.rgb[0], s.rgb[1], s.rgb[2], 255] })
            .collect();
        let line_inst: Vec<LineInst> = sky
            .constellations
            .iter()
            .flat_map(|c| c.lines.iter())
            .map(|&[a, b]| LineInst { a: v3(sky.stars[a].pos), b: v3(sky.stars[b].pos) })
            .collect();
        let vbuf = |label, contents: &[u8]| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        };
        let field_buf = vbuf("field", bytemuck::cast_slice(&field_inst));
        let star_buf = vbuf("stars", bytemuck::cast_slice(&star_inst));
        let line_buf = vbuf("lines", bytemuck::cast_slice(&line_inst));
        let line_alpha = vbuf("line alpha", bytemuck::cast_slice(&vec![0f32; line_inst.len()]));
        let rect_buf = vbuf("rects", bytemuck::cast_slice(&[Rect::zeroed(); MAX_RECTS]));

        let mw = &sky.milky_way;
        let mw_view = texture(&device, &queue, &[(mw.width, mw.height, mw.rgba.clone())], "milky way");
        let probe_view = texture(&device, &queue, &mip_chain(&sky.probe), "probe");
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Renderer {
            device,
            queue,
            format,
            layout,
            mw_view,
            probe_view,
            sampler,
            sky: sky_p,
            field,
            stars,
            probe,
            lines,
            rects,
            field_buf,
            n_field: field_inst.len() as u32,
            star_buf,
            n_stars: star_inst.len() as u32,
            line_buf,
            line_alpha,
            n_lines: line_inst.len() as u32,
            rect_buf,
            n_rects: 0,
            palette: Palette::get(),
            milky_way: milky_way(),
            voyager: v3(sky.voyager),
        }
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn target(&self, text: Option<&mut TextCtx>) -> Target {
        let uniform = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.mw_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&self.probe_view) },
            ],
        });
        let text = text.map(|t| {
            let r = TextRenderer::new(&mut t.atlas, &self.device, wgpu::MultisampleState::default(), None);
            (r, Viewport::new(&self.device, &t.cache))
        });
        Target { uniform, bind, text }
    }

    /// One alpha per line segment, in constellation order (drag mode).
    pub fn set_line_alpha(&self, alpha: &[f32]) {
        debug_assert_eq!(alpha.len(), self.n_lines as usize);
        self.queue.write_buffer(&self.line_alpha, 0, bytemuck::cast_slice(alpha));
    }

    pub fn set_rects(&mut self, rects: &[Rect]) {
        let n = rects.len().min(MAX_RECTS);
        self.queue.write_buffer(&self.rect_buf, 0, bytemuck::cast_slice(&rects[..n]));
        self.n_rects = n as u32;
    }

    fn uniforms(&self, fr: &Frame) -> Uniforms {
        let p = &fr.proj;
        let pal = &self.palette;
        let [mw_strength, mw_curve, mw_colour] = self.milky_way;
        Uniforms {
            e: v4(v3(p.e), 0.0),
            n: v4(v3(p.n), 0.0),
            f: v4(v3(p.f), 0.0),
            center: [p.center.0 as f32, p.center.1 as f32],
            scale: p.scale as f32,
            u: fr.u,
            size: [fr.size.0 as f32, fr.size.1 as f32],
            time: fr.time,
            mw_strength,
            top: v4(pal.top, 1.0),
            bottom: v4(pal.bottom, 1.0),
            line_col: v4(pal.indigo, 1.0),
            voyager: v4(self.voyager, fr.probe_alpha),
            probe: [PROBE_PX * fr.u, mw_curve, mw_colour, 0.0],
        }
    }

    /// Draw a frame into `view`. `texts` are drawn last, on top (drag mode only).
    pub fn draw(
        &self,
        target: &mut Target,
        view: &wgpu::TextureView,
        fr: &Frame,
        mut text: Option<&mut TextCtx>,
        texts: Vec<TextArea<'_>>,
    ) {
        self.queue.write_buffer(&target.uniform, 0, bytemuck::bytes_of(&self.uniforms(fr)));
        let mut text_ok = false;
        if let (Some(ctx), Some((tr, vp))) = (text.as_deref_mut(), target.text.as_mut()) {
            if !texts.is_empty() {
                vp.update(&self.queue, glyphon::Resolution { width: fr.size.0, height: fr.size.1 });
                text_ok = tr
                    .prepare(&self.device, &self.queue, &mut ctx.font_system, &mut ctx.atlas, vp, texts, &mut ctx.swash)
                    .is_ok();
            }
        }
        let mut enc = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sky"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_bind_group(0, &target.bind, &[]);
            pass.set_pipeline(&self.sky);
            pass.draw(0..3, 0..1);
            pass.set_pipeline(&self.field);
            pass.set_vertex_buffer(0, self.field_buf.slice(..));
            pass.draw(0..6, 0..self.n_field);
            if fr.overlay {
                pass.set_pipeline(&self.lines);
                pass.set_vertex_buffer(0, self.line_buf.slice(..));
                pass.set_vertex_buffer(1, self.line_alpha.slice(..));
                pass.draw(0..6, 0..self.n_lines);
            }
            pass.set_pipeline(&self.stars);
            pass.set_vertex_buffer(0, self.star_buf.slice(..));
            pass.draw(0..6, 0..self.n_stars);
            if fr.probe_alpha > 0.0 {
                pass.set_pipeline(&self.probe);
                pass.draw(0..6, 0..1);
            }
            if fr.overlay && self.n_rects > 0 {
                pass.set_pipeline(&self.rects);
                pass.set_vertex_buffer(0, self.rect_buf.slice(..));
                pass.draw(0..6, 0..self.n_rects);
            }
            if text_ok {
                if let (Some((tr, vp)), Some(ctx)) = (target.text.as_ref(), text.as_deref()) {
                    let _ = tr.render(&ctx.atlas, vp, &mut pass);
                }
            }
        }
        self.queue.submit([enc.finish()]);
    }

    /// Render a frame offscreen and save it as an RGB PNG.
    pub fn export(&self, fr: &Frame, path: &Path) -> Result<(), String> {
        let mut target = self.target(None);
        self.export_with(&mut target, fr, None, Vec::new(), path)
    }

    /// Like export, with text on top (drag-mode previews).
    pub fn export_with(
        &self,
        target: &mut Target,
        fr: &Frame,
        text: Option<&mut TextCtx>,
        texts: Vec<TextArea<'_>>,
        path: &Path,
    ) -> Result<(), String> {
        let (w, h) = fr.size;
        let tex = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("export"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.draw(target, &tex.create_view(&Default::default()), fr, text, texts);

        let row = (4 * w).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: (row * h) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut enc = self.device.create_command_encoder(&Default::default());
        enc.copy_texture_to_buffer(
            tex.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        self.queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::Wait).map_err(|e| format!("GPU: {e}"))?;
        let data = slice.get_mapped_range();
        let bgra = matches!(self.format, wgpu::TextureFormat::Bgra8Unorm);
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for y in 0..h {
            let line = &data[(y * row) as usize..(y * row + 4 * w) as usize];
            for px in line.chunks_exact(4) {
                if bgra {
                    rgb.extend_from_slice(&[px[2], px[1], px[0]]);
                } else {
                    rgb.extend_from_slice(&px[..3]);
                }
            }
        }
        drop(data);
        buf.unmap();
        let tmp = path.with_extension("png.part");
        image::save_buffer_with_format(&tmp, &rgb, w, h, image::ExtendedColorType::Rgb8, image::ImageFormat::Png)
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
    }
}
