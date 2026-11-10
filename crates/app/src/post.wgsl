// Bagliore e conversione dell'immagine ad alta dinamica verso lo schermo.

@group(0) @binding(0) var t_a: texture_2d<f32>;
@group(0) @binding(1) var t_b: texture_2d<f32>;
@group(0) @binding(2) var s: sampler;

struct Uscita {
    @builtin(position) pos: vec4f,
    @location(0) uv: vec2f,
}

@vertex
fn vs_schermo(@builtin(vertex_index) i: u32) -> Uscita {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var u: Uscita;
    u.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    u.uv = vec2f(x, y);
    return u;
}

fn passo_texel(t: texture_2d<f32>) -> vec2f {
    return 1.0 / vec2f(textureDimensions(t));
}

@fragment
fn fs_giu(u: Uscita) -> @location(0) vec4f {
    let d = passo_texel(t_a);
    var c = textureSample(t_a, s, u.uv).rgb * 4.0;
    c += textureSample(t_a, s, u.uv + d * vec2f(-1.0, -1.0)).rgb;
    c += textureSample(t_a, s, u.uv + d * vec2f(1.0, -1.0)).rgb;
    c += textureSample(t_a, s, u.uv + d * vec2f(-1.0, 1.0)).rgb;
    c += textureSample(t_a, s, u.uv + d * vec2f(1.0, 1.0)).rgb;
    // Limita i pixel estremi, che altrimenti lampeggerebbero nel bagliore.
    return vec4f(min(c / 8.0, vec3f(60.0)), 1.0);
}

@fragment
fn fs_su(u: Uscita) -> @location(0) vec4f {
    let d = passo_texel(t_a) * 1.5;
    var c = textureSample(t_a, s, u.uv).rgb * 4.0;
    c += (textureSample(t_a, s, u.uv + vec2f(d.x, 0.0)).rgb
        + textureSample(t_a, s, u.uv - vec2f(d.x, 0.0)).rgb
        + textureSample(t_a, s, u.uv + vec2f(0.0, d.y)).rgb
        + textureSample(t_a, s, u.uv - vec2f(0.0, d.y)).rgb) * 2.0;
    c += textureSample(t_a, s, u.uv + d).rgb
        + textureSample(t_a, s, u.uv - d).rgb
        + textureSample(t_a, s, u.uv + vec2f(d.x, -d.y)).rgb
        + textureSample(t_a, s, u.uv + vec2f(-d.x, d.y)).rgb;
    return vec4f(c / 16.0 * 0.75, 1.0);
}

fn aces(x: vec3f) -> vec3f {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), vec3f(0.0), vec3f(1.0));
}

fn srgb(c: vec3f) -> vec3f {
    let basso = c * 12.92;
    let alto = 1.055 * pow(c, vec3f(1.0 / 2.4)) - 0.055;
    return select(alto, basso, c <= vec3f(0.0031308));
}

@fragment
fn fs_finale(u: Uscita) -> @location(0) vec4f {
    let scena = textureSample(t_a, s, u.uv).rgb;
    let bagliore = textureSample(t_b, s, u.uv).rgb;
    let colore = srgb(aces((scena + bagliore * 0.28) * 0.75));
    let rumore = fract(52.9829189 * fract(dot(u.pos.xy, vec2f(0.06711056, 0.00583715))));
    return vec4f(colore + (rumore - 0.5) / 255.0, 1.0);
}
