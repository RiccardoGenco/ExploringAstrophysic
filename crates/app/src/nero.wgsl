// Buco nero non rotante: per ogni pixel si segue un raggio di luce all'indietro.
// Lo schema e' lo stesso di `fisica::luce`, che lo valida con i test.
// Unita': raggi di Schwarzschild e rs/c.

struct Parametri {
    cam_pos: vec4f,      // xyz, w = sqrt(1 - 1/r) alla camera
    cam_destra: vec4f,
    cam_su: vec4f,
    cam_avanti: vec4f,
    schermo: vec4f,      // larghezza, altezza, tan(mezzo campo visivo), tempo
    disco: vec4f,        // raggio interno, raggio esterno, attivo, doppler attivo
    varie: vec4f,        // raggio di fuga, numero anelli, passi massimi, numero sorgenti
    scie: vec4f,         // attive, -, -, -
    anelli: array<vec4f, 6>, // raggio, r, g, b
    sorgenti: array<vec4f, 16>,        // xyz, raggio
    colori_sorgenti: array<vec4f, 16>, // rgb
}

@group(0) @binding(0) var<uniform> P: Parametri;
@group(0) @binding(1) var t_stelle: texture_2d<f32>;
@group(0) @binding(2) var s_stelle: sampler;
@group(0) @binding(3) var t_scie: texture_2d<f32>;

const PI: f32 = 3.14159265;
const PASSO_RELATIVO: f32 = 0.06;
const TEMPERATURA_DISCO: f32 = 6500.0;
const R_MAPPA_SCIE: f32 = 1000.0;

struct Uscita {
    @builtin(position) pos: vec4f,
}

@vertex
fn vs_schermo(@builtin(vertex_index) i: u32) -> Uscita {
    let x = f32((i << 1u) & 2u);
    let y = f32(i & 2u);
    var u: Uscita;
    u.pos = vec4f(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    return u;
}

fn acc(x: vec3f, h2: f32) -> vec3f {
    let r2 = dot(x, x);
    return -1.5 * h2 * x / (r2 * r2 * sqrt(r2));
}

fn hash3(p: vec3f) -> f32 {
    let q = fract(p * vec3f(0.1031, 0.1030, 0.0973));
    let s = q + dot(q, q.yzx + 33.33);
    return fract((s.x + s.y) * s.z);
}

fn rumore3(p: vec3f) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(mix(hash3(i), hash3(i + vec3f(1.0, 0.0, 0.0)), u.x),
                mix(hash3(i + vec3f(0.0, 1.0, 0.0)), hash3(i + vec3f(1.0, 1.0, 0.0)), u.x), u.y);
    let b = mix(mix(hash3(i + vec3f(0.0, 0.0, 1.0)), hash3(i + vec3f(1.0, 0.0, 1.0)), u.x),
                mix(hash3(i + vec3f(0.0, 1.0, 1.0)), hash3(i + vec3f(1.0, 1.0, 1.0)), u.x), u.y);
    return mix(a, b, u.z);
}

fn frattale(p: vec3f, ottave: i32) -> f32 {
    var somma = 0.0;
    var ampiezza = 0.5;
    var q = p;
    for (var i = 0; i < ottave; i++) {
        somma += ampiezza * rumore3(q);
        q = q * 2.03 + vec3f(1.7, 9.2, 3.1);
        ampiezza *= 0.5;
    }
    return somma;
}

// Colore di un corpo nero, dalla legge di Planck in tre lunghezze d'onda.
fn corpo_nero(t: f32) -> vec3f {
    let l = vec3f(610.0, 550.0, 465.0);
    let c = 1.4388e7 / (l * t);
    let v = 1.0 / (pow(l / 550.0, vec3f(5.0)) * (exp(min(c, vec3f(60.0))) - 1.0));
    return v / max(v.r, max(v.g, v.b));
}

fn trama_disco(angolo: f32, r: f32, seme: f32) -> f32 {
    let lr = log(r);
    let p = vec3f(cos(angolo) * 3.0, sin(angolo) * 3.0, lr * 9.0 + seme);
    let n = frattale(p, 4);
    return n * n;
}

// Restituisce colore emesso e opacita' del disco nel punto `p`.
fn disco(p: vec3f, r: f32, l: f32) -> vec4f {
    let r_in = P.disco.x;
    let r_out = P.disco.y;
    if (r < r_in || r > r_out) { return vec4f(0.0); }

    let bordo = smoothstep(r_in, r_in * 1.08, r) * (1.0 - smoothstep(r_out * 0.55, r_out, r));
    let omega = sqrt(0.5 / (r * r * r));
    var g = 1.0;
    if (P.disco.w > 0.5) {
        g = sqrt(1.0 - 1.5 / r) / (1.0 - omega * l) / P.cam_pos.w;
    }

    // Disco sottile: T proporzionale a r^-3/4 con la correzione al bordo interno.
    let x = r / r_in;
    let termico = pow(x, -0.75) * pow(max(1.0 - sqrt(1.0 / x), 0.0), 0.25) * 2.1;
    let t_oss = TEMPERATURA_DISCO * termico * g;

    // Due strati sfalsati evitano che la rotazione differenziale avvolga la trama.
    let periodo = 60.0;
    let fase = fract(P.schermo.w / periodo);
    let t1 = fase * periodo;
    let t2 = fract(fase + 0.5) * periodo;
    let peso = abs(2.0 * fase - 1.0);
    let a = atan2(p.z, p.x);
    let n = mix(trama_disco(a + omega * t2, r, 40.0), trama_disco(a + omega * t1, r, 0.0), peso);
    let densita = 0.25 + n * 3.2;

    let intensita = pow(termico, 3.0) * pow(g, 4.0) * densita * 0.42;
    return vec4f(corpo_nero(max(t_oss, 900.0)) * intensita * bordo, clamp(densita * 0.9, 0.0, 0.95) * bordo);
}

fn cielo(d: vec3f) -> vec3f {
    let u = atan2(d.z, d.x) / (2.0 * PI) + 0.5;
    let v = acos(clamp(d.y, -1.0, 1.0)) / PI;
    let s = textureSampleLevel(t_stelle, s_stelle, vec2f(u, v), 0.0).rgb * 5.0;

    // Banda diffusa inventata, inclinata rispetto al disco.
    let n = normalize(vec3f(0.25, 0.9, 0.35));
    let alt = dot(d, n);
    let banda = exp(-alt * alt * 14.0);
    let nubi = frattale(d * 4.0 + vec3f(5.0, 1.0, 2.0), 5);
    let polvere = smoothstep(0.42, 0.62, frattale(d * 9.0, 4));
    let colore = mix(vec3f(0.30, 0.34, 0.55), vec3f(0.62, 0.48, 0.36), banda);
    return s + colore * banda * nubi * nubi * (1.0 - 0.75 * polvere) * 0.32;
}

fn distanza_segmento(p: vec3f, a: vec3f, b: vec3f) -> f32 {
    let ab = b - a;
    let t = clamp(dot(p - a, ab) / max(dot(ab, ab), 1e-9), 0.0, 1.0);
    return length(a + ab * t - p);
}

@fragment
fn fs_nero(u: Uscita) -> @location(0) vec4f {
    let aspetto = P.schermo.x / P.schermo.y;
    let tan_campo = P.schermo.z;
    let ndc = vec2f(u.pos.x / P.schermo.x * 2.0 - 1.0, 1.0 - u.pos.y / P.schermo.y * 2.0);
    let locale = normalize(P.cam_avanti.xyz
        + P.cam_destra.xyz * ndc.x * aspetto * tan_campo
        + P.cam_su.xyz * ndc.y * tan_campo);

    // Da direzione misurata dall'osservatore fermo a direzione coordinata.
    var x = P.cam_pos.xyz;
    let r_cam = length(x);
    let radiale = x / r_cam;
    let vr = dot(locale, radiale);
    let tangenziale = locale - radiale * vr;
    let radice = P.cam_pos.w;
    var v = radiale * vr * radice + tangenziale;
    let b = r_cam * length(tangenziale) / radice;

    let h_vett = cross(x, v);
    let h2 = dot(h_vett, h_vett);
    let h_dir = h_vett / max(sqrt(h2), 1e-6);
    // Il raggio e' seguito all'indietro: il fotone vero ha momento opposto.
    let l = -h_dir.y * b;

    let angolo_pixel = 2.0 * tan_campo / P.schermo.y;
    var somma = vec3f(0.0);
    var trasmesso = 1.0;
    var bagliore: array<f32, 16>;
    let n_sorgenti = i32(P.varie.w);
    let r_fuga = P.varie.x;
    let passi = i32(P.varie.z);
    var fuggito = false;

    for (var i = 0; i < passi; i++) {
        let r = length(x);
        if (r <= 1.0) { break; }
        if (r >= r_fuga && dot(x, v) > 0.0) { fuggito = true; break; }

        let d = PASSO_RELATIVO * r / length(v);
        let k1x = v;
        let k1v = acc(x, h2);
        let k2x = v + k1v * d * 0.5;
        let k2v = acc(x + k1x * d * 0.5, h2);
        let k3x = v + k2v * d * 0.5;
        let k3v = acc(x + k2x * d * 0.5, h2);
        let k4x = v + k3v * d;
        let k4v = acc(x + k3x * d, h2);
        let nx = x + (k1x + 2.0 * (k2x + k3x) + k4x) * d / 6.0;
        let nv = v + (k1v + 2.0 * (k2v + k3v) + k4v) * d / 6.0;

        for (var k = 0; k < n_sorgenti; k++) {
            let s = P.sorgenti[k];
            let dist = distanza_segmento(s.xyz, x, nx);
            bagliore[k] = max(bagliore[k], exp(-dist * dist / (s.w * s.w)) * trasmesso);
        }

        if (x.y * nx.y < 0.0) {
            let t = x.y / (x.y - nx.y);
            let p = mix(x, nx, t);
            let rp = length(p);
            let spessore = angolo_pixel * length(p - P.cam_pos.xyz) * 1.3;
            for (var a = 0; a < i32(P.varie.y); a++) {
                let anello = P.anelli[a];
                let scarto = abs(rp - anello.x);
                if (scarto < spessore) {
                    somma += trasmesso * anello.yzw * 1.6 * (1.0 - scarto / spessore);
                }
            }
            if (P.scie.x > 0.5 && rp > 1.0 && rp < R_MAPPA_SCIE) {
                // Le scie del banco stanno nel piano: si vedono deviate come il disco.
                let uv = vec2f(atan2(p.z, p.x) / (2.0 * PI) + 0.5, log(rp) / log(R_MAPPA_SCIE));
                let scia = textureSampleLevel(t_scie, s_stelle, uv, 0.0);
                somma += trasmesso * scia.rgb * scia.rgb * 1.5;
            }
            if (P.disco.z > 0.5) {
                let e = disco(p, rp, l);
                somma += trasmesso * e.rgb;
                trasmesso *= 1.0 - e.a;
            }
        }

        x = nx;
        v = nv;
        if (trasmesso < 0.01) { break; }
    }

    if (fuggito) {
        somma += trasmesso * cielo(normalize(v));
    }
    for (var k = 0; k < n_sorgenti; k++) {
        somma += P.colori_sorgenti[k].rgb * bagliore[k];
    }
    return vec4f(somma, 1.0);
}
