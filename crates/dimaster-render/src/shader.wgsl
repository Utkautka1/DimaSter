// Один шейдер на всё: заливка, глиф из атласа или картинка.

@group(0) @binding(0) var<uniform> screen: vec4<f32>;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var smooth_sampler: sampler;

// бит 24: цветной глиф рисуется своими цветами; бит 25: картинка целиком
const COLOR_GLYPH: u32 = 0x1000000u;
const IMAGE: u32 = 0x2000000u;
// бит 27: отрезок по диагонали, бит 28: снизу вверх
const LINE: u32 = 0x8000000u;
const RISING: u32 = 0x10000000u;

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
    // позиция внутри прямоугольника и его размер — для скругления
    @location(3) local: vec2<f32>,
    @location(4) @interpolate(flat) size: vec2<f32>,
    @location(5) @interpolate(flat) shape: vec2<f32>,
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
    return Pixel(vec4<f32>(ndc, 0.0, 1.0), uv, quad.color, solid, corner * quad.size, quad.size, quad.uv);
}

// сглаженный край прямоугольника со скруглёнными углами
fn coverage(local: vec2<f32>, size: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(size.x, size.y) * 0.5);
    if r <= 0.0 {
        return 1.0;
    }
    let q = abs(local - size * 0.5) - (size * 0.5 - vec2<f32>(r));
    let distance = length(max(q, vec2<f32>(0.0))) - r;
    return clamp(0.5 - distance, 0.0, 1.0);
}

// сглаженный отрезок шириной `width` от угла к углу с отступом в полуширину
fn stroke(local: vec2<f32>, size: vec2<f32>, width: f32, rising: bool) -> f32 {
    var point = local;
    if rising {
        point.y = size.y - point.y;
    }
    let half = vec2<f32>(width * 0.5);
    let along = size - half * 2.0;
    let from = point - half;
    let t = clamp(dot(from, along) / max(dot(along, along), 0.0001), 0.0, 1.0);
    return clamp(width * 0.5 - length(from - along * t) + 0.5, 0.0, 1.0);
}

@fragment
fn fragment(pixel: Pixel) -> @location(0) vec4<f32> {
    let rgb = vec3<f32>(vec3<u32>(pixel.color >> 16u, pixel.color >> 8u, pixel.color) & vec3<u32>(255u)) / 255.0;
    if pixel.solid == 1u {
        // shape.x = -непрозрачность, shape.y = радиус
        let alpha = select(1.0, 0.35, (pixel.color & 0x4000000u) != 0u) * -pixel.shape.x;
        if (pixel.color & LINE) != 0u {
            let rising = (pixel.color & RISING) != 0u;
            return vec4<f32>(rgb, alpha * stroke(pixel.local, pixel.size, pixel.shape.y, rising));
        }
        return vec4<f32>(rgb, alpha * coverage(pixel.local, pixel.size, pixel.shape.y));
    }
    if (pixel.color & IMAGE) != 0u {
        // младший байт цвета — непрозрачность картинки
        let texel = textureSampleLevel(atlas, smooth_sampler, pixel.uv, 0.0);
        return vec4<f32>(texel.rgb, texel.a * f32(pixel.color & 255u) / 255.0);
    }
    let texel = textureLoad(atlas, vec2<i32>(pixel.uv), 0);
    if (pixel.color & COLOR_GLYPH) != 0u {
        return texel;
    }
    return vec4<f32>(rgb, texel.a);
}
