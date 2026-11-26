// voyager-sky: every pass of the sky, in one shader module.
//
// Projection: stereographic around the view centre, east to the left (as the sky is
// seen from inside the sphere); e and n carry the view's roll (0 = celestial north up).
// The same maths is in view.rs.
// All work happens in gamma (sRGB-encoded) space, like the old SVG renderer, so the
// palette colours come out exactly as written.

struct U {
    e: vec4f,             // screen-left at the view centre (xyz): east when level
    n: vec4f,             // screen-up at the view centre: north when level
    f: vec4f,             // the view centre
    center: vec2f,        // projection centre, in this target's pixels
    scale: f32,           // pixels per stereographic unit
    u: f32,               // size of one 1080p pixel, in this target's pixels
    size: vec2f,          // target size in pixels
    time: f32,
    mw_strength: f32,     // Milky Way brightness
    top: vec4f,           // gradient colours
    bottom: vec4f,
    line_col: vec4f,
    voyager: vec4f,       // xyz on the sphere, w = probe opacity
    probe: vec4f,         // x = probe size in px, y = Milky Way tone curve, z = its saturation
};

@group(0) @binding(0) var<uniform> uni: U;
@group(0) @binding(1) var mw_tex: texture_2d<f32>;
@group(0) @binding(2) var lin_samp: sampler;
@group(0) @binding(3) var probe_tex: texture_2d<f32>;

const PI: f32 = 3.14159265358979;

// sky unit vector -> (pixel x, pixel y, cos of distance from centre)
fn project(p: vec3f) -> vec3f {
    let x = dot(p, uni.e.xyz);
    let y = dot(p, uni.n.xyz);
    let z = dot(p, uni.f.xyz);
    let k = 2.0 / max(1.0 + z, 1e-4);
    return vec3f(uni.center.x - k * x * uni.scale, uni.center.y - k * y * uni.scale, z);
}

fn unproject(px: vec2f) -> vec3f {
    let X = -(px.x - uni.center.x) / uni.scale;
    let Y = -(px.y - uni.center.y) / uni.scale;
    let r2 = X * X + Y * Y;
    let z = (4.0 - r2) / (4.0 + r2);
    let h = 0.5 * (1.0 + z);
    return normalize(X * h * uni.e.xyz + Y * h * uni.n.xyz + z * uni.f.xyz);
}

fn to_clip(px: vec2f) -> vec4f {
    return vec4f(px.x / uni.size.x * 2.0 - 1.0, 1.0 - px.y / uni.size.y * 2.0, 0.0, 1.0);
}

fn hash(p: vec2f) -> f32 {
    let q = fract(p * vec2f(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

// two triangles: corner i of a quad, in -1..1
fn corner(i: u32) -> vec2f {
    var c = array<vec2f, 6>(vec2f(-1.0, -1.0), vec2f(1.0, -1.0), vec2f(-1.0, 1.0),
                            vec2f(-1.0, 1.0), vec2f(1.0, -1.0), vec2f(1.0, 1.0));
    return c[i];
}

const OFFSCREEN: vec4f = vec4f(-10.0, -10.0, 0.0, 1.0);

// --- 1. sky: gradient + Milky Way ----------------------------------------------------

struct SkyOut { @builtin(position) pos: vec4f };

@vertex fn sky_vs(@builtin(vertex_index) i: u32) -> SkyOut {
    let p = vec2f(f32((i << 1u) & 2u), f32(i & 2u));       // big triangle covering the screen
    return SkyOut(vec4f(p * 2.0 - 1.0, 0.0, 1.0));
}

@fragment fn sky_fs(in: SkyOut) -> @location(0) vec4f {
    let px = in.pos.xy;
    var col = mix(uni.top.rgb, uni.bottom.rgb, clamp(px.y / uni.size.y, 0.0, 1.0));
    let p = unproject(px);
    let ra = atan2(p.y, p.x);
    let dec = asin(clamp(p.z, -1.0, 1.0));
    // equirectangular, RA 0 in the middle increasing to the left, Dec +90 at the top
    let uv = vec2f(fract(0.5 - ra / (2.0 * PI)), 0.5 - dec / PI);
    let enc = textureSampleLevel(mw_tex, lin_samp, uv, 0.0).rgb;
    // stored as sqrt(linear); probe.y = tone curve, probe.z = how much colour is kept
    var mw = pow(enc, vec3f(uni.probe.y)) * uni.mw_strength;
    mw = mix(vec3f(dot(mw, vec3f(0.299, 0.587, 0.114))), mw, uni.probe.z);
    col = 1.0 - (1.0 - col) * (1.0 - clamp(mw, vec3f(0.0), vec3f(1.0)));   // screen
    col += (hash(px) - 0.5) / 255.0;                         // dither: no banding
    return vec4f(col, 1.0);
}

// --- 2. faint background stars -------------------------------------------------------

struct FieldIn {
    @location(0) pos: vec3f,
    @location(1) size: f32,
    @location(2) col: vec4f,   // rgb + opacity
};
struct DotOut {
    @builtin(position) pos: vec4f,
    @location(0) off: vec2f,   // pixels from the centre
    @location(1) col: vec4f,
    @location(2) sigma: f32,
};

@vertex fn field_vs(@builtin(vertex_index) i: u32, s: FieldIn) -> DotOut {
    let q = project(s.pos);
    let sigma = max(s.size, 0.5) * 0.7 * uni.u;
    let r = max(3.0 * sigma, 1.5);
    let c = corner(i) * r;
    var out: DotOut;
    // flux of the old SVG dot (pi r^2 a) spread over a gaussian of this sigma
    let peak = clamp(s.size * s.size * uni.u * uni.u * s.col.a / (2.0 * sigma * sigma), 0.0, 1.0);
    out.col = vec4f(s.col.rgb, peak);
    out.off = c;
    out.sigma = sigma;
    out.pos = select(OFFSCREEN, to_clip(q.xy + c), q.z > -0.9);
    return out;
}

@fragment fn field_fs(in: DotOut) -> @location(0) vec4f {
    let a = in.col.a * exp(-dot(in.off, in.off) / (2.0 * in.sigma * in.sigma));
    return vec4f(in.col.rgb, a);
}

// --- 3. constellation stars ------------------------------------------------------------

struct StarIn {
    @location(0) pos: vec3f,
    @location(1) mag: f32,
    @location(2) col: vec4f,
};
struct StarOut {
    @builtin(position) pos: vec4f,
    @location(0) off: vec2f,
    @location(1) col: vec3f,
    @location(2) radii: vec2f, // core, halo
};

@vertex fn star_vs(@builtin(vertex_index) i: u32, s: StarIn) -> StarOut {
    let q = project(s.pos);
    var f = pow(10.0, -0.2 * (s.mag - 1.8));                // 1 at mag 1.8
    if (f > 1.0) { f = 1.0 + 0.5 * log(f); }                 // Sirius & co. don't take over
    let core = (0.7 + 1.9 * f) * uni.u;
    let halo = (3.5 + 15.0 * f) * uni.u;
    let c = corner(i) * (halo + 1.0);
    var out: StarOut;
    out.off = c;
    out.col = s.col.rgb;
    out.radii = vec2f(core, halo);
    out.pos = select(OFFSCREEN, to_clip(q.xy + c), q.z > -0.9);
    return out;
}

@fragment fn star_fs(in: StarOut) -> @location(0) vec4f {
    let d = length(in.off);
    let t = d / in.radii.y;
    // the old halo gradient: white 0.95 -> colour 0.75 at 0.18 -> 0.18 at 0.45 -> 0 at 1
    var hc = mix(vec3f(1.0), in.col, smoothstep(0.0, 0.18, t));
    var ha = 0.0;
    if (t < 0.18) { ha = mix(0.95, 0.75, t / 0.18); }
    else if (t < 0.45) { ha = mix(0.75, 0.18, (t - 0.18) / 0.27); }
    else if (t < 1.0) { ha = mix(0.18, 0.0, (t - 0.45) / 0.55); }
    let ca = clamp(in.radii.x - d + 0.5, 0.0, 1.0);          // crisp, anti-aliased core
    let a = ca + ha * (1.0 - ca);
    let rgb = (in.col * ca + hc * ha * (1.0 - ca)) / max(a, 1e-4);
    return vec4f(rgb, a);
}

// --- 4. the probe ----------------------------------------------------------------------

struct ProbeOut {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
};

@vertex fn probe_vs(@builtin(vertex_index) i: u32) -> ProbeOut {
    let q = project(uni.voyager.xyz);
    let c = corner(i);
    var out: ProbeOut;
    out.uv = c * 0.5 + 0.5;
    out.uv.y = 1.0 - out.uv.y;
    let visible = q.z > -0.9 && uni.voyager.w > 0.0;
    out.pos = select(OFFSCREEN, to_clip(q.xy + vec2f(c.x, -c.y) * uni.probe.x * 0.5), visible);
    return out;
}

@fragment fn probe_fs(in: ProbeOut) -> @location(0) vec4f {
    let c = textureSample(probe_tex, lin_samp, in.uv);
    return vec4f(c.rgb, c.a * uni.voyager.w);
}

// --- 5. constellation lines (drag mode) ---------------------------------------------

struct LineIn {
    @location(0) a: vec3f,
    @location(1) b: vec3f,
    @location(2) alpha: f32,
};
struct LineOut {
    @builtin(position) pos: vec4f,
    @location(0) side: f32,    // pixels from the line's middle
    @location(1) alpha: f32,
};

@vertex fn line_vs(@builtin(vertex_index) i: u32, l: LineIn) -> LineOut {
    let pa = project(l.a);
    let pb = project(l.b);
    var out: LineOut;
    out.alpha = l.alpha;
    let d = pb.xy - pa.xy;
    let len = length(d);
    let gap = 5.0 * uni.u;                                    // leave the stars clear
    let ok = pa.z > -0.3 && pb.z > -0.3 && len > 2.0 * gap + 1.0 && l.alpha > 0.002;
    let dir = d / max(len, 1e-4);
    let nrm = vec2f(-dir.y, dir.x);
    let half = 0.6 * uni.u + 1.0;
    let c = corner(i);
    let along = select(pa.xy + dir * gap, pb.xy - dir * gap, c.x > 0.0);
    out.side = c.y * half;
    out.pos = select(OFFSCREEN, to_clip(along + nrm * c.y * half), ok);
    return out;
}

@fragment fn line_fs(in: LineOut) -> @location(0) vec4f {
    let w = 0.6 * uni.u;
    let a = clamp(w + 0.5 - abs(in.side), 0.0, 1.0) * in.alpha;
    return vec4f(uni.line_col.rgb, a);
}

// --- 6. capsules: the chips and the navigation ball (drag mode) -----------------------

struct ShapeIn {
    @location(0) seg: vec4f,   // a.xy, b.xy in pixels
    @location(1) col: vec4f,
    @location(2) border: vec4f,
    @location(3) extra: vec4f, // x = radius
};
struct ShapeOut {
    @builtin(position) pos: vec4f,
    @location(0) p: vec2f,     // this pixel
    @location(1) seg: vec4f,
    @location(2) r: f32,
    @location(3) col: vec4f,
    @location(4) border: vec4f,
};

@vertex fn shape_vs(@builtin(vertex_index) i: u32, s: ShapeIn) -> ShapeOut {
    let r = s.extra.x;
    let lo = min(s.seg.xy, s.seg.zw) - r - 1.0;
    let hi = max(s.seg.xy, s.seg.zw) + r + 1.0;
    let px = mix(lo, hi, corner(i) * 0.5 + 0.5);
    var out: ShapeOut;
    out.p = px;
    out.seg = s.seg;
    out.r = r;
    out.col = s.col;
    out.border = s.border;
    out.pos = to_clip(px);
    return out;
}

@fragment fn shape_fs(in: ShapeOut) -> @location(0) vec4f {
    let pa = in.p - in.seg.xy;
    let ba = in.seg.zw - in.seg.xy;
    let h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
    let d = length(pa - ba * h) - in.r;
    let fill = clamp(0.5 - d, 0.0, 1.0);
    let edge = clamp(0.5 - abs(d + 0.75), 0.0, 1.0);
    let a = fill * in.col.a;
    let rgb = mix(in.col.rgb, in.border.rgb, edge * in.border.a);
    return vec4f(rgb, max(a, edge * in.border.a));
}

// --- 7. a constellation's figure (drag mode, F) --------------------------------------
//
// The image is a square of the gnomonic tangent plane at the constellation's centre,
// east to the left and north up, `half` tangent units from the middle to each edge
// (see Constellation::centre in data.rs). Every pixel is taken back to the sphere and
// onto that plane, so the art stays on its stars at any view, tilt and zoom.

struct Figure {
    c: vec4f,
    e: vec4f,
    n: vec4f,
    params: vec4f,        // x = half-width (tangent units), y = opacity
};

@group(1) @binding(0) var<uniform> fig: Figure;
@group(1) @binding(1) var fig_tex: texture_2d<f32>;
@group(1) @binding(2) var fig_samp: sampler;

@fragment fn figure_fs(in: SkyOut) -> @location(0) vec4f {
    let p = unproject(in.pos.xy);
    let d = dot(p, fig.c.xyz);
    let h = fig.params.x;
    let x = dot(p, fig.e.xyz) / max(d, 1e-4);
    let y = dot(p, fig.n.xyz) / max(d, 1e-4);
    let uv = vec2f(0.5 - x / (2.0 * h), 0.5 - y / (2.0 * h));
    let c = textureSample(fig_tex, fig_samp, uv);
    let inside = d > 0.05 && all(uv >= vec2f(0.0)) && all(uv <= vec2f(1.0));
    return vec4f(c.rgb, select(0.0, c.a * fig.params.y, inside));
}
