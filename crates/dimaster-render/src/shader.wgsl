// Один шейдер на всё: заливка, глиф из атласа или картинка.

@group(0) @binding(0) var<uniform> screen: vec4<f32>;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var smooth_sampler: sampler;

// бит 24: цветной глиф рисуется своими цветами; бит 25: картинка целиком
const COLOR_GLYPH: u32 = 0x1000000u;
const IMAGE: u32 = 0x2000000u;

struct Quad {
    @location(0) position: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: u32,
}

struct Pixel {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: u32,
    @location(2) @interpolate(flat) solid: u32,
}

@vertex
fn vertex(@builtin(vertex_index) index: u32, quad: Quad) -> Pixel {
    // два треугольника из triangle strip
    let corner = vec2<f32>(f32(index & 1u), f32(index >> 1u));
    let point = quad.position + corner * quad.size;
    let ndc = point / screen.xy * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0);
    let solid = u32(quad.uv.x < 0.0);
    var uv = quad.uv + corner * quad.size;
    if (quad.color & IMAGE) != 0u {
        uv = corner;
    }
    return Pixel(vec4<f32>(ndc, 0.0, 1.0), uv, quad.color, solid);
}

@fragment
fn fragment(pixel: Pixel) -> @location(0) vec4<f32> {
    let rgb = vec3<f32>(vec3<u32>(pixel.color >> 16u, pixel.color >> 8u, pixel.color) & vec3<u32>(255u)) / 255.0;
    if pixel.solid == 1u {
        return vec4<f32>(rgb, 1.0);
    }
    if (pixel.color & IMAGE) != 0u {
        return textureSampleLevel(atlas, smooth_sampler, pixel.uv, 0.0);
    }
    let texel = textureLoad(atlas, vec2<i32>(pixel.uv), 0);
    if (pixel.color & COLOR_GLYPH) != 0u {
        return texel;
    }
    return vec4<f32>(rgb, texel.a);
}
