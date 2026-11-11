//! Banco della radiazione di fondo: nebbia, cielo sbucciato, macchie.

use crate::audio::Audio;
use crate::formato::{decimale, grande, tempo};
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::costanti::ANNO;
use colossi_fisica::fondo::{
    fotoni_per_cm3, lunghezza_picco_um, planck_relativa, temperatura, Cosmologia, PLANCK, T_OGGI, Z_ULTIMO_URTO,
};
use egui::{Align2, Color32, ColorImage, FontId, Frame, Margin, Mesh, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, TextureHandle, TextureOptions, Vec2};
use std::f64::consts::{FRAC_PI_2, PI, TAU};
use std::sync::{Arc, Mutex};

const LARGHEZZA: f32 = 430.0;
const LATO_X: usize = 720;
const LATO_Y: usize = 360;
const N_FOTONI: usize = 420;
pub const Z_MASSIMO: f64 = 6000.0;
const DIPOLO_UK: f64 = 3362.0;
const DIPOLO_L: f64 = 264.0;
const DIPOLO_B: f64 = 48.25;
const GALASSIA_UK: f64 = 1400.0;
const L_MASSIMO_MAPPA: f64 = 400.0;
const BIN_MAPPA: usize = 60;
const ONDE_PER_BIN: usize = 42;
const FREQUENZA_AUDIO: u32 = 44_100;
const ALTEZZA_SPETTRO: f32 = 150.0;
const MERIDIANI: usize = 72;
const PARALLELI: usize = 36;
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pagina {
    Nebbia,
    Cielo,
    Macchie,
}

#[derive(Clone, Copy)]
struct Fotone {
    x: f32,
    y: f32,
    angolo: f32,
}

struct Casuale(u64);

impl Casuale {
    fn valore(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f64 + 0.5) / (1u64 << 31) as f64
    }
}

type Onda = ([f32; 3], f32, f32, f32);

/// Le onde che compongono il cielo simulato, con lo spettro delle macchie.
fn onde_del_cielo(cosmo: Cosmologia, seme: u64) -> Vec<Onda> {
    let mut caso = Casuale(seme);
    let mut onde = Vec::with_capacity(BIN_MAPPA * ONDE_PER_BIN);
    let (l_min, l_max) = (2.0_f64, L_MASSIMO_MAPPA);
    let passo = (l_max / l_min).ln() / BIN_MAPPA as f64;
    for b in 0..BIN_MAPPA {
        let l = l_min * ((b as f64 + 0.5) * passo).exp();
        let dl = l * passo;
        let varianza = dl * cosmo.spettro(l).max(0.0) / l;
        let ampiezza = (2.0 * varianza / ONDE_PER_BIN as f64).sqrt() as f32;
        for _ in 0..ONDE_PER_BIN {
            let z = 2.0 * caso.valore() - 1.0;
            let phi = TAU * caso.valore();
            let s = (1.0 - z * z).sqrt();
            onde.push(([(s * phi.cos()) as f32, (s * phi.sin()) as f32, z as f32], l as f32, ampiezza, (TAU * caso.valore()) as f32));
        }
    }
    onde
}

fn calcola_riga(y: usize, onde: &[Onda], riga: &mut [f32]) {
    let lat = FRAC_PI_2 - PI * (y as f64 + 0.5) / LATO_Y as f64;
    let (sl, cl) = lat.sin_cos();
    for (x, v) in riga.iter_mut().enumerate() {
        let lon = PI - TAU * (x as f64 + 0.5) / LATO_X as f64;
        let n = [(cl * lon.cos()) as f32, (cl * lon.sin()) as f32, sl as f32];
        let mut somma = 0.0_f32;
        for (d, l, a, fase) in onde {
            somma += a * (l * (n[0] * d[0] + n[1] * d[1] + n[2] * d[2]) + fase).cos();
        }
        *v = somma;
    }
}

/// Cielo simulato per intero, usando piu' processori.
#[cfg(not(target_arch = "wasm32"))]
fn sintetizza(cosmo: Cosmologia, seme: u64) -> Vec<f32> {
    let onde = onde_del_cielo(cosmo, seme);
    let mut mappa = vec![0.0_f32; LATO_X * LATO_Y];
    let quanti = std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1).max(1));
    let righe_per_blocco = LATO_Y.div_ceil(quanti);
    std::thread::scope(|ambito| {
        for (b, blocco) in mappa.chunks_mut(LATO_X * righe_per_blocco).enumerate() {
            let onde = &onde;
            ambito.spawn(move || {
                for (r, riga) in blocco.chunks_mut(LATO_X).enumerate() {
                    calcola_riga(b * righe_per_blocco + r, onde, riga);
                }
            });
        }
    });
    mappa
}

/// Il cielo misurato da WMAP in nove anni, in microkelvin.
fn leggi_mappa_reale() -> Vec<f32> {
    include_bytes!("../dati/cielo_wmap.bin")
        .chunks_exact(2)
        .map(|c| f32::from(i16::from_le_bytes([c[0], c[1]])))
        .collect()
}

fn dipolo(lon: f64, lat: f64) -> f64 {
    let (lb, ll) = (DIPOLO_B.to_radians(), DIPOLO_L.to_radians());
    let c = lat.sin() * lb.sin() + lat.cos() * lb.cos() * (lon - ll).cos();
    DIPOLO_UK * c
}

fn galassia(lon: f64, lat: f64) -> f64 {
    let b = lat.to_degrees();
    let strato = (-(b / 5.0).powi(2)).exp();
    let centro = 0.45 + 0.55 * (-(lon.rem_euclid(TAU).min(TAU - lon.rem_euclid(TAU)) / 1.2).powi(2)).exp();
    GALASSIA_UK * strato * centro * (0.75 + 0.25 * (lon * 13.0 + lat * 40.0).sin() * (lon * 7.0).cos())
}

fn colore_diverging(v: f64) -> [u8; 3] {
    let a = v.clamp(-1.0, 1.0) as f32;
    let chiaro = [245.0, 240.0, 225.0];
    let estremo = if a > 0.0 { [190.0, 30.0, 25.0] } else { [25.0, 50.0, 170.0] };
    let k = a.abs().powf(0.8);
    [
        (chiaro[0] + (estremo[0] - chiaro[0]) * k) as u8,
        (chiaro[1] + (estremo[1] - chiaro[1]) * k) as u8,
        (chiaro[2] + (estremo[2] - chiaro[2]) * k) as u8,
    ]
}

fn colore_corpo_nero(t: f64) -> [f32; 3] {
    let v = [0.610_f64, 0.550, 0.465].map(|l| planck_relativa(l, t));
    let m = v[0].max(v[1]).max(v[2]).max(1e-30);
    [(v[0] / m) as f32, (v[1] / m) as f32, (v[2] / m) as f32]
}

/// Quanto il colore e' visibile a occhio: spento sotto gli 800 K circa.
fn visibilita(t: f64) -> f32 {
    ((t - 700.0) / 1200.0).clamp(0.0, 1.0) as f32
}

pub struct BancoFondo {
    pub pagina: Pagina,
    pub z: f64,
    pub in_corso: bool,
    fotoni: Vec<Fotone>,
    caso: Casuale,
    pub cosmo: Cosmologia,
    pronta: Arc<Mutex<Option<Vec<f32>>>>,
    in_calcolo: bool,
    mappa: Option<Vec<f32>>,
    cosmo_mappa: Cosmologia,
    pub con_dipolo: bool,
    pub con_galassia: bool,
    pub con_macchie: bool,
    pub reale: bool,
    mappa_reale: Vec<f32>,
    planck: Vec<[f64; 4]>,
    pub dall_interno: bool,
    yaw: f64,
    pitch: f64,
    texture: Option<TextureHandle>,
    stato_texture: Option<(bool, bool, bool, bool, bool)>,
    audio: Audio,
    avviso: Option<String>,
    #[cfg(target_arch = "wasm32")]
    in_costruzione: Option<(Vec<Onda>, Vec<f32>, usize)>,
}

impl Default for BancoFondo {
    fn default() -> Self {
        let mut caso = Casuale(5);
        let fotoni = (0..N_FOTONI)
            .map(|_| Fotone { x: caso.valore() as f32, y: caso.valore() as f32, angolo: (caso.valore() * TAU) as f32 })
            .collect();
        Self {
            pagina: Pagina::Nebbia,
            z: Z_MASSIMO,
            in_corso: true,
            fotoni,
            caso,
            cosmo: PLANCK,
            pronta: Arc::new(Mutex::new(None)),
            in_calcolo: false,
            mappa: None,
            cosmo_mappa: PLANCK,
            con_dipolo: true,
            con_galassia: true,
            con_macchie: true,
            reale: true,
            mappa_reale: leggi_mappa_reale(),
            planck: crate::missioni::righe_planck(),
            dall_interno: false,
            yaw: 0.0,
            pitch: 0.0,
            texture: None,
            stato_texture: None,
            audio: Audio::default(),
            avviso: None,
            #[cfg(target_arch = "wasm32")]
            in_costruzione: None,
        }
    }
}

impl BancoFondo {
    fn calcola_mappa(&mut self) {
        if self.in_calcolo {
            return;
        }
        self.in_calcolo = true;
        self.cosmo_mappa = self.cosmo;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (uscita, cosmo) = (self.pronta.clone(), self.cosmo);
            std::thread::spawn(move || {
                let m = sintetizza(cosmo, 41);
                if let Ok(mut u) = uscita.lock() {
                    *u = Some(m);
                }
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.in_costruzione = Some((onde_del_cielo(self.cosmo, 41), vec![0.0; LATO_X * LATO_Y], 0));
        }
    }

    /// Nel browser il cielo si compone una riga alla volta.
    #[cfg(target_arch = "wasm32")]
    fn lavora(&mut self) {
        let Some((onde, mappa, y)) = &mut self.in_costruzione else { return };
        let inizio = web_time::Instant::now();
        while *y < LATO_Y && inizio.elapsed().as_secs_f64() < 0.012 {
            let (da, a) = (*y * LATO_X, (*y + 1) * LATO_X);
            calcola_riga(*y, onde, &mut mappa[da..a]);
            *y += 1;
        }
        if *y >= LATO_Y {
            let finita = std::mem::take(mappa);
            self.in_costruzione = None;
            if let Ok(mut u) = self.pronta.lock() {
                *u = Some(finita);
            }
        }
    }

    fn raccogli(&mut self) {
        #[cfg(target_arch = "wasm32")]
        self.lavora();
        if let Ok(mut u) = self.pronta.try_lock() {
            if let Some(m) = u.take() {
                self.mappa = Some(m);
                self.in_calcolo = false;
                self.stato_texture = None;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn attendi_mappa(&mut self, secondi: f64) {
        self.calcola_mappa();
        let inizio = std::time::Instant::now();
        while self.mappa.is_none() && inizio.elapsed().as_secs_f64() < secondi {
            std::thread::sleep(std::time::Duration::from_millis(20));
            self.raccogli();
        }
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.pagina != Pagina::Nebbia {
            return;
        }
        if self.in_corso {
            // La discesa del redshift e' logaritmica, per vedere tutte le epoche.
            self.z = (1.0 + self.z) * (-0.28 * dt).exp() - 1.0;
            if self.z <= 0.0 {
                self.z = 0.0;
                self.in_corso = false;
            }
        }
        let urti = self.cosmo.ionizzazione(self.z).sqrt() as f32;
        let passo = (dt as f32) * 0.16;
        for f in &mut self.fotoni {
            if (self.caso.valore() as f32) < urti * 0.6 {
                f.angolo = (self.caso.valore() * TAU) as f32;
            }
            f.x = (f.x + f.angolo.cos() * passo).rem_euclid(1.0);
            f.y = (f.y + f.angolo.sin() * passo).rem_euclid(1.0);
        }
    }

    fn scala_colori(&self) -> f64 {
        if self.con_dipolo {
            DIPOLO_UK
        } else if self.con_galassia && !self.reale {
            900.0
        } else {
            300.0
        }
    }

    fn aggiorna_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let macchie_pronte = self.mappa.is_some() || self.reale;
        let stato = (self.con_dipolo, self.con_galassia && !self.reale, self.con_macchie, macchie_pronte, self.reale);
        if self.stato_texture != Some(stato) || self.texture.is_none() {
            self.stato_texture = Some(stato);
            let scala = self.scala_colori();
            let mut pixel = vec![255u8; LATO_X * LATO_Y * 4];
            for y in 0..LATO_Y {
                let lat = FRAC_PI_2 - PI * (y as f64 + 0.5) / LATO_Y as f64;
                for x in 0..LATO_X {
                    let lon = PI - TAU * (x as f64 + 0.5) / LATO_X as f64;
                    let mut v = 0.0;
                    if self.con_dipolo {
                        v += dipolo(lon, lat);
                    }
                    if self.con_galassia && !self.reale {
                        v += galassia(lon, lat);
                    }
                    if self.con_macchie {
                        if self.reale {
                            v += f64::from(self.mappa_reale[y * LATO_X + x]);
                        } else if let Some(m) = &self.mappa {
                            v += f64::from(m[y * LATO_X + x]);
                        }
                    }
                    let c = colore_diverging(v / scala);
                    let i = (y * LATO_X + x) * 4;
                    pixel[i..i + 3].copy_from_slice(&c);
                }
            }
            let immagine = ColorImage::from_rgba_unmultiplied([LATO_X, LATO_Y], &pixel);
            match &mut self.texture {
                Some(t) => t.set(immagine, TextureOptions::LINEAR),
                None => self.texture = Some(ctx.load_texture("cielo_fondo", immagine, TextureOptions::LINEAR)),
            }
        }
        self.texture.clone()
    }

    fn suona(&mut self) {
        let picchi: Vec<(f64, f64)> = (1..=5)
            .map(|n| {
                let l = self.cosmo.picco(n);
                (l, self.cosmo.spettro(l).max(0.0).sqrt())
            })
            .collect();
        let (l1, a1) = picchi[0];
        let n = FREQUENZA_AUDIO as usize * 5;
        let totale: f64 = picchi.iter().map(|p| p.1 / a1).sum();
        let campioni: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f64 / f64::from(FREQUENZA_AUDIO);
                let inviluppo = (t / 0.3).min(1.0) * ((5.0 - t) / 1.0).min(1.0);
                let s: f64 = picchi.iter().map(|(l, a)| a / a1 * (TAU * 110.0 * l / l1 * t).sin()).sum();
                (s / totale * inviluppo * 0.7) as f32
            })
            .collect();
        self.avviso = self.audio.suona(campioni, FREQUENZA_AUDIO).err();
    }
}

pub fn mostra(ui: &mut egui::Ui, b: &mut BancoFondo) {
    b.raccogli();
    egui::Panel::right("colonna_fondo")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, b));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        match b.pagina {
            Pagina::Nebbia => nebbia(ui, area, b),
            Pagina::Cielo => {
                if !b.reale {
                    b.calcola_se_serve();
                }
                cielo(ui, area, b);
            }
            Pagina::Macchie => macchie(ui, area, b),
        }
    });
}

impl BancoFondo {
    fn calcola_se_serve(&mut self) {
        if self.mappa.is_none() && !self.in_calcolo {
            self.calcola_mappa();
        }
    }
}

fn nebbia(ui: &mut egui::Ui, area: Rect, b: &BancoFondo) {
    let p = ui.painter_at(area);
    let t = temperatura(b.z);
    let colore = colore_corpo_nero(t);
    let lume = visibilita(t);
    let ion = b.cosmo.ionizzazione(b.z) as f32;
    let tinta = Color32::from_rgb(
        (8.0 + colore[0] * 245.0 * lume) as u8,
        (10.0 + colore[1] * 230.0 * lume) as u8,
        (16.0 + colore[2] * 225.0 * lume) as u8,
    );
    p.rect_filled(area, 0.0, tinta);

    // I fotoni: nella nebbia rimbalzano, poi vanno dritti.
    let linea = if lume > 0.5 { Color32::from_black_alpha(150) } else { Color32::from_rgb(200, 215, 255) };
    for f in &b.fotoni {
        let q = area.min + Vec2::new(f.x * area.width(), f.y * area.height());
        let coda = Vec2::new(f.angolo.cos(), f.angolo.sin()) * if ion > 0.1 { 6.0 } else { 20.0 };
        p.line_segment([q - coda, q], Stroke::new(1.3, linea));
    }
    if ion > 0.01 {
        p.rect_filled(area, 0.0, tinta.gamma_multiply(ion * 0.55));
    }

    let scritta = if lume > 0.5 { Color32::from_rgb(20, 22, 30) } else { CHIARO };
    let f = FontId::proportional(14.0);
    let frase = if b.z > 1500.0 {
        "Plasma: la luce rimbalza di continuo sugli elettroni. L'universo è una nebbia luminosa."
    } else if b.z > Z_ULTIMO_URTO * 0.85 {
        "Si formano gli atomi. Gli elettroni spariscono e la nebbia si dirada."
    } else if b.z > 1.0 {
        "La luce ora viaggia libera, e l'espansione la stira: dal visibile all'infrarosso."
    } else {
        "Oggi: la stessa luce, stirata in microonde, arriva da ogni direzione."
    };
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, frase, f, scritta);
}

/// Mollweide: da latitudine ad angolo ausiliario.
fn theta_mollweide(lat: f64) -> f64 {
    let mut t = lat;
    for _ in 0..30 {
        let f = 2.0 * t + (2.0 * t).sin() - PI * lat.sin();
        let d = 2.0 + 2.0 * (2.0 * t).cos();
        if d.abs() < 1e-9 {
            break;
        }
        t -= f / d;
    }
    t
}

fn cielo(ui: &mut egui::Ui, area: Rect, b: &mut BancoFondo) {
    let Some(texture) = b.aggiorna_texture(ui.ctx()) else { return };
    let r = ui.interact(area, ui.id().with("cielo_fondo"), Sense::click_and_drag());
    if b.dall_interno && (r.dragged_by(PointerButton::Primary) || r.dragged_by(PointerButton::Secondary)) {
        let d = r.drag_delta();
        b.yaw += f64::from(d.x) * 0.004;
        b.pitch = (b.pitch + f64::from(d.y) * 0.004).clamp(-1.4, 1.4);
    }
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);

    let centro = area.center() + Vec2::new(0.0, 16.0);
    let mut mesh = Mesh::with_texture(texture.id());
    let griglia = (MERIDIANI + 1) * (PARALLELI + 1);
    mesh.reserve_vertices(griglia);
    let mut validi = vec![true; griglia];
    let semilato = (area.width() * 0.46).min((area.height() - 90.0) * 0.5 * 2.0);
    let (sy, cy) = b.yaw.sin_cos();
    let (sp, cp) = b.pitch.sin_cos();
    for j in 0..=PARALLELI {
        let v = j as f64 / PARALLELI as f64;
        let lat = FRAC_PI_2 - PI * v;
        for i in 0..=MERIDIANI {
            let u = i as f64 / MERIDIANI as f64;
            let lon = PI - TAU * u;
            let q = if b.dall_interno {
                let (x, y, z) = (lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin());
                let (x1, y1) = (cy * x - sy * y, sy * x + cy * y);
                let (x2, z2) = (cp * x1 + sp * z, -sp * x1 + cp * z);
                if x2 < 0.12 {
                    validi[j * (MERIDIANI + 1) + i] = false;
                }
                let scala = area.height() as f64 * 0.8;
                centro + Vec2::new((-y1 / x2.max(0.05) * scala) as f32, (-z2 / x2.max(0.05) * scala) as f32)
            } else {
                let t = theta_mollweide(lat);
                let x = -lon / PI * t.cos();
                centro + Vec2::new(x as f32 * semilato, -(t.sin() as f32) * semilato * 0.5)
            };
            mesh.vertices.push(egui::epaint::Vertex { pos: q, uv: Pos2::new(u as f32, v as f32), color: Color32::WHITE });
        }
    }
    let w = (MERIDIANI + 1) as u32;
    for j in 0..PARALLELI as u32 {
        for i in 0..MERIDIANI as u32 {
            let k = j * w + i;
            let tutti = [k, k + 1, k + w, k + w + 1].iter().all(|&q| validi[q as usize]);
            if tutti {
                mesh.add_triangle(k, k + 1, k + w);
                mesh.add_triangle(k + 1, k + w + 1, k + w);
            }
        }
    }
    p.add(mesh);

    if b.dall_interno {
        // Da dentro il cielo riempie tutto: le scritte hanno bisogno di un fondo.
        let velo = Color32::from_black_alpha(185);
        p.rect_filled(Rect::from_min_size(area.min, Vec2::new(area.width(), 54.0)), 0.0, velo);
        p.rect_filled(Rect::from_min_max(area.left_bottom() - Vec2::new(0.0, 30.0), area.right_bottom()), 0.0, velo);
    }
    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Togli uno strato alla volta: il nostro moto, poi la Via Lattea. Sotto restano da cui nasceranno le galassie.", f.clone(), GRIGIO);
    let tutti_spenti = !b.con_dipolo && !b.con_galassia && !b.con_macchie;
    let frase = if tutti_spenti {
        format!("Il cielo a occhio chiuso sui dettagli: {} K in ogni direzione.", decimale(T_OGGI))
    } else {
        format!("Bianco = {} K. Rosso e blu: più o meno {} microkelvin.", decimale(T_OGGI), grande(b.scala_colori()))
    };
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, frase, f, CIANO);
    p.text(
        area.left_bottom() + Vec2::new(14.0, -10.0),
        Align2::LEFT_BOTTOM,
        if b.reale {
            "Mappa reale: NASA/WMAP, nove anni, combinazione lineare interna. Il moto è aggiunto dal valore misurato."
        } else {
            "Cielo simulato dallo spettro misurato: stesse statistiche, non la mappa reale. La fascia della Via Lattea è illustrativa."
        },
        FontId::proportional(12.0),
        GIALLO,
    );
}

fn macchie(ui: &mut egui::Ui, area: Rect, b: &BancoFondo) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let piano = Rect::from_min_max(area.min + Vec2::new(64.0, 70.0), area.max - Vec2::new(24.0, 46.0));
    p.rect_stroke(piano, 2.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let (lg_min, lg_max) = (2.0_f64.log10(), 1800.0_f64.log10());
    let massimo = 7000.0;
    let a_schermo = |l: f64, d: f64| {
        Pos2::new(
            piano.left() + piano.width() * ((l.log10() - lg_min) / (lg_max - lg_min)) as f32,
            piano.bottom() - piano.height() * (d / massimo) as f32,
        )
    };
    let f = FontId::monospace(10.0);
    for l in [2.0, 10.0, 30.0, 100.0, 300.0, 1000.0] {
        let q = a_schermo(l, 0.0);
        p.line_segment([q, Pos2::new(q.x, piano.top())], Stroke::new(1.0, LINEA));
        p.text(q + Vec2::new(0.0, 4.0), Align2::CENTER_TOP, format!("{l}"), f.clone(), GRIGIO);
        let gradi = 180.0 / l;
        p.text(Pos2::new(q.x, piano.top() - 3.0), Align2::CENTER_BOTTOM, format!("{gradi:.1}°").replace('.', ","), f.clone(), GRIGIO);
    }
    for k in 0..=6 {
        let d = f64::from(k) * 1000.0;
        let y = a_schermo(2.0, d).y;
        p.line_segment([Pos2::new(piano.left(), y), Pos2::new(piano.right(), y)], Stroke::new(1.0, LINEA));
        p.text(Pos2::new(piano.left() - 6.0, y), Align2::RIGHT_CENTER, format!("{}", k * 1000), f.clone(), GRIGIO);
    }
    let n = 600;
    let curva = |c: &Cosmologia| -> Vec<Pos2> {
        (0..=n)
            .map(|i| {
                let l = 10f64.powf(lg_min + (lg_max - lg_min) * f64::from(i) / f64::from(n));
                let q = a_schermo(l, c.spettro(l).min(massimo));
                Pos2::new(q.x, q.y.clamp(piano.top(), piano.bottom()))
            })
            .collect()
    };
    if b.cosmo != PLANCK {
        p.add(Shape::line(curva(&PLANCK), Stroke::new(1.2, GRIGIO)));
    }
    p.add(Shape::line(curva(&b.cosmo), Stroke::new(2.4, CIANO)));
    // Le misure vere di Planck.
    for r in &b.planck {
        let q = a_schermo(r[0], r[1].min(massimo));
        let alto = a_schermo(r[0], (r[1] + r[2]).min(massimo)).y;
        let e = (q.y - alto).abs().max(1.5);
        p.line_segment([q - Vec2::new(0.0, e), q + Vec2::new(0.0, e)], Stroke::new(1.1, CHIARO));
        p.circle_filled(q, 1.8, CHIARO);
    }
    for n in 1..=3 {
        let l = b.cosmo.picco(n);
        let q = a_schermo(l, b.cosmo.spettro(l).min(massimo));
        p.circle_filled(q, 4.0, GIALLO);
    }

    let t = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Quanto contrasto c'è a ogni grandezza di macchia. In alto la grandezza nel cielo, in basso il numero di macchie intorno al cielo.", t.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, "I punti sono le misure di Planck. I picchi sono gli armonici del suono nel plasma: posizione calcolata, altezze della curva indicative.", t, CIANO);
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.end_row();
}

fn lunghezza_onda(um: f64) -> String {
    if um < 1.0 {
        format!("{:.0} nm", um * 1e3)
    } else if um < 1000.0 {
        format!("{} µm", decimale(um))
    } else {
        format!("{} mm", decimale(um / 1e3))
    }
}

fn spettro(p: &Painter, area: Rect, t: f64) {
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let (lg_a, lg_b) = (-1.0_f64, 4.3_f64);
    let x_di = |um: f64| area.left() + area.width() * ((um.log10() - lg_a) / (lg_b - lg_a)) as f32;
    // La fascia che vede l'occhio.
    let banda = Rect::from_min_max(Pos2::new(x_di(0.38), area.top()), Pos2::new(x_di(0.75), area.bottom()));
    p.rect_filled(banda, 0.0, Color32::from_rgba_unmultiplied(255, 240, 200, 44));
    p.text(banda.center_top() + Vec2::new(0.0, 3.0), Align2::CENTER_TOP, "occhio", FontId::proportional(10.0), CHIARO);
    let punti: Vec<Pos2> = (0..=220)
        .map(|i| {
            let lg = lg_a + (lg_b - lg_a) * f64::from(i) / 220.0;
            Pos2::new(
                area.left() + area.width() * i as f32 / 220.0,
                area.bottom() - 6.0 - (area.height() - 22.0) * planck_relativa(10f64.powf(lg), t) as f32,
            )
        })
        .collect();
    p.add(Shape::line(punti, Stroke::new(2.0, ROSSO)));
}

fn strumenti(ui: &mut egui::Ui, b: &mut BancoFondo) {
    ui.horizontal(|ui| {
        for (pagina, nome) in [(Pagina::Nebbia, "Nebbia"), (Pagina::Cielo, "Il cielo"), (Pagina::Macchie, "Macchie e suono")] {
            ui.selectable_value(&mut b.pagina, pagina, RichText::new(nome).strong());
        }
    });
    match b.pagina {
        Pagina::Nebbia => col_nebbia(ui, b),
        Pagina::Cielo => col_cielo(ui, b),
        Pagina::Macchie => col_macchie(ui, b),
    }
    ui.add_space(6.0);
    ui.label(RichText::new("Cosmologia Planck 2018. Saha: stima d'equilibrio della ricombinazione.").small().color(GRIGIO));
}

fn col_nebbia(ui: &mut egui::Ui, b: &mut BancoFondo) {
    titoletto(ui, "QUANDO");
    ui.horizontal(|ui| {
        let scorri = ui.button(if b.in_corso { "Pausa" } else { "Scorri" });
        crate::visite::segna(ui, "scorri", &scorri);
        if scorri.clicked() {
            b.in_corso = !b.in_corso;
        }
        if ui.button("Dall'inizio").clicked() {
            b.z = Z_MASSIMO;
            b.in_corso = true;
        }
    });
    let mut a = (1.0 + b.z).ln();
    if ui.add(egui::Slider::new(&mut a, Z_MASSIMO.ln_1p()..=0.0).text("indietro nel tempo").show_value(false)).changed() {
        b.z = (a.exp() - 1.0).max(0.0);
        b.in_corso = false;
    }

    let (t, eta) = (temperatura(b.z), b.cosmo.eta(b.z));
    titoletto(ui, "IN QUEL MOMENTO");
    egui::Grid::new("letture_nebbia").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Età dell'universo", tempo(eta * ANNO));
        riga(ui, "Spostamento z", decimale(b.z));
        riga(ui, "Temperatura", format!("{} K", decimale(t)));
        riga(ui, "Picco della luce", lunghezza_onda(lunghezza_picco_um(b.z)));
        riga(ui, "Fotoni per cm³", grande(fotoni_per_cm3(b.z)));
        riga(ui, "Idrogeno ionizzato", format!("{:.2} %", b.cosmo.ionizzazione(b.z) * 100.0).replace('.', ","));
        riga(ui, "Tra un urto e l'altro", format!("{} anni luce", grande(b.cosmo.cammino_libero_anni_luce(b.z))));
    });
    ui.add_space(4.0);
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_SPETTRO), Sense::hover());
    let p = ui.painter_at(area);
    spettro(&p, area, t);
    p.text(area.right_top() + Vec2::new(-6.0, 3.0), Align2::RIGHT_TOP, "colore della luce", FontId::monospace(10.0), GRIGIO);
    if b.z < 1.0 {
        ui.label(RichText::new(format!("In ogni centimetro cubo attorno a te: circa {:.0} fotoni di quell'epoca.", fotoni_per_cm3(0.0))).color(CIANO));
    }
}

fn col_cielo(ui: &mut egui::Ui, b: &mut BancoFondo) {
    titoletto(ui, "GLI STRATI");
    ui.checkbox(&mut b.con_dipolo, "Il nostro moto (più caldo davanti)");
    ui.checkbox(&mut b.reale, "Mappa reale (WMAP)");
    ui.add_enabled(!b.reale, egui::Checkbox::new(&mut b.con_galassia, "La Via Lattea, solo nel cielo simulato"));
    ui.checkbox(&mut b.con_macchie, "Le macchie: da qui nasceranno le galassie");
    titoletto(ui, "VISTA");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut b.dall_interno, false, "Tutto il cielo");
        ui.selectable_value(&mut b.dall_interno, true, "Da dentro, trascina");
    });
    if b.in_calcolo {
        ui.label(RichText::new("Sto generando il cielo...").color(GIALLO));
    }
    if b.cosmo != b.cosmo_mappa && !b.in_calcolo && ui.button("Rigenera con la cosmologia scelta").clicked() {
        b.calcola_mappa();
    }
}

fn col_macchie(ui: &mut egui::Ui, b: &mut BancoFondo) {
    titoletto(ui, "DI COSA È FATTO L'UNIVERSO");
    ui.add(egui::Slider::new(&mut b.cosmo.barioni_h2, 0.005..=0.06).text("materia normale").fixed_decimals(4));
    ui.add(egui::Slider::new(&mut b.cosmo.oscura_h2, 0.03..=0.3).text("materia oscura").fixed_decimals(3));
    let curvatura = ui.add(egui::Slider::new(&mut b.cosmo.curvatura, -0.3..=0.3).text("curvatura dello spazio").fixed_decimals(3));
    crate::visite::segna(ui, "curvatura", &curvatura);
    if ui.small_button("Torna ai valori misurati").clicked() {
        b.cosmo = PLANCK;
    }
    titoletto(ui, "LETTURE");
    let c = b.cosmo;
    egui::Grid::new("letture_macchie").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Primo picco", decimale(c.picco(1)));
        riga(ui, "Grandezza delle macchie", format!("{}°", decimale(180.0 / c.picco(1))));
        riga(ui, "Fin dove arrivò il suono", format!("{} Mpc", decimale(c.orizzonte_suono(Z_ULTIMO_URTO))));
        riga(ui, "Età dell'universo", format!("{} miliardi di anni", decimale(c.eta(0.0) / 1e9)));
    });
    ui.add_space(4.0);
    ui.label(RichText::new("La Luna piena copre mezzo grado: la macchia tipica è grande circa due Lune.").color(CIANO));
    ui.add_space(6.0);
    if ui.button(RichText::new("Ascolta il plasma").strong()).clicked() {
        b.suona();
    }
    ui.label(RichText::new("Armonici veri, trasposti di circa 50 ottave per poterli sentire.").small().color(GRIGIO));
    if let Some(e) = &b.avviso {
        ui.label(RichText::new(format!("Audio non disponibile: {e}")).color(GIALLO));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn la_mappa_ha_circa_le_fluttuazioni_giuste() {
        let m = sintetizza(PLANCK, 3);
        let media = m.iter().sum::<f32>() / m.len() as f32;
        let rms = (m.iter().map(|v| (v - media).powi(2)).sum::<f32>() / m.len() as f32).sqrt();
        assert!(rms > 40.0 && rms < 220.0, "{rms}");
    }

    #[test]
    fn la_mappa_reale_ha_la_misura_e_le_fluttuazioni_giuste() {
        let m = leggi_mappa_reale();
        assert_eq!(m.len(), LATO_X * LATO_Y);
        let rms = (m.iter().map(|v| v * v).sum::<f32>() / m.len() as f32).sqrt();
        assert!(rms > 50.0 && rms < 120.0, "{rms}");
    }

    #[test]
    fn il_dipolo_e_massimo_nella_sua_direzione() {
        let (l, b) = (DIPOLO_L.to_radians(), DIPOLO_B.to_radians());
        assert!((dipolo(l, b) - DIPOLO_UK).abs() < 1e-6);
        assert!((dipolo(l + PI, -b) + DIPOLO_UK).abs() < 1e-6);
    }

    #[test]
    fn la_nebbia_scorre_fino_a_oggi() {
        let mut b = BancoFondo::default();
        for _ in 0..4000 {
            b.avanza(0.05);
        }
        assert_eq!(b.z, 0.0);
    }

    #[test]
    fn mollweide_ai_poli_e_all_equatore() {
        assert!(theta_mollweide(0.0).abs() < 1e-9);
        assert!((theta_mollweide(1.5) - 1.5).abs() < 1.0);
    }
}
