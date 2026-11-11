// Terrain is multisampled before entities, translucency and shader-pack post effects.
// HDR's alpha is a light code, not coverage: choose it from the nearest sample instead
// of averaging codes into a different light tier. Integer surface metadata follows the
// same reverse-Z sample as the resolved depth.

// The frame holds linear light (`FrameConfiguration::linear_light`): its samples average
// tone-weighted (docs/hdr-passes.md, rule 3), or a sample far above paper white would own the
// edge. Gamma light averages as stored.
override LINEAR: bool = false;

@vertex fn vertex(@builtin(vertex_index) vertex: u32) -> @builtin(position) vec4<f32> {
    let p = vec2(f32((vertex << 1u) & 2u), f32(vertex & 2u));
    return vec4(p * 2.0 - 1.0, 0.0, 1.0);
}

@group(0) @binding(0) var seed_color: texture_2d<f32>;
@fragment fn seed(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(seed_color, vec2<i32>(p.xy), 0);
}

@group(1) @binding(0) var samples: texture_multisampled_2d<f32>;
@group(1) @binding(1) var depths: texture_multisampled_2d<f32>;
@group(1) @binding(2) var surfaces: texture_multisampled_2d<u32>;

fn largest(c: vec3<f32>) -> f32 {
    return max(c.r, max(c.g, c.b));
}

struct Resolved {
    color: vec4<f32>,
    depth: f32,
    surface: vec4<u32>,
}
fn resolve(p: vec2<i32>) -> Resolved {
    var rgb = vec3(0.0);
    var nearest = 0;
    var depth = 0.0;
    for (var i = 0; i < 4; i++) {
        let c = textureLoad(samples, p, i).rgb;
        rgb += select(c, c / (1.0 + largest(c)), LINEAR);
        let d = textureLoad(depths, p, i).r;
        if d > depth {
            depth = d;
            nearest = i;
        }
    }
    rgb *= 0.25;
    if LINEAR {
        // Each weighted sample stays below 1, so their mean does too.
        rgb /= max(1.0 - largest(rgb), 1e-5);
    }
    var surface = textureLoad(surfaces, p, nearest);
    if surface.w != 0u {
        surface.w = bitcast<u32>(depth);
    }
    return Resolved(vec4(rgb, textureLoad(samples, p, nearest).a), depth, surface);
}
struct ColorDepth {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) depth: f32,
}
@fragment fn resolve_color(@builtin(position) p: vec4<f32>) -> ColorDepth {
    let result = resolve(vec2<i32>(p.xy));
    return ColorDepth(result.color, result.depth);
}
struct ColorDepthSurface {
    @location(0) color: vec4<f32>,
    @location(1) surface: vec4<u32>,
    @builtin(frag_depth) depth: f32,
}
@fragment fn resolve_surface(@builtin(position) p: vec4<f32>) -> ColorDepthSurface {
    let result = resolve(vec2<i32>(p.xy));
    return ColorDepthSurface(result.color, result.surface, result.depth);
}
