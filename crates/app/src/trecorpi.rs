//! Banco dei tre corpi: lanci liberi, gemello caotico, errore numerico, mappa del caos.

use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use colossi_fisica::trecorpi::{
    corpi_mappa, elementi, esito_mappa, punti_lagrange, Corpo, EsitoMappa, Sistema, GALLERIA, MAX_CORPI, TEMPO_MAPPA,
};
use egui::{Align2, Color32, ColorImage, FontId, Frame, Margin, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, TextureHandle, TextureOptions, Vec2};
use std::f64::consts::TAU;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

const LARGHEZZA: f32 = 430.0;
const BUDGET: usize = 60_000;
const MAX_CAMPIONI: usize = 12_000;
const MAX_PUNTI: usize = 1800;
const MAX_STORIA: usize = 3000;
const PRESA: f32 = 12.0;
const LATO_MAPPA: usize = 256;
const ALTEZZA_GRAFICO: f32 = 110.0;
const ALTEZZA_CIELO: f32 = 56.0;
const COLORI: [Color32; MAX_CORPI] = [
    Color32::from_rgb(90, 215, 255),
    Color32::from_rgb(255, 160, 90),
    Color32::from_rgb(255, 130, 190),
    Color32::from_rgb(120, 235, 150),
    Color32::from_rgb(245, 200, 90),
    Color32::from_rgb(175, 150, 255),
];
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);

#[derive(Clone, Copy)]
struct Campione {
    p: [f32; 2],
    r: [f32; 2],
    t: f32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pagina {
    Banco,
    Mappa,
}

/// Casella della mappa: esito e colore, condivisi con i calcoli in sottofondo.
struct Griglia {
    esiti: Vec<Option<EsitoMappa>>,
    pixel: Vec<u8>,
}

pub struct Mappa {
    massa: f64,
    griglia: Arc<Mutex<Griglia>>,
    fatti: Arc<AtomicUsize>,
    ferma: Arc<AtomicBool>,
    totale: usize,
    avviata: bool,
    texture: Option<TextureHandle>,
    disegnati: usize,
    #[cfg(target_arch = "wasm32")]
    coda: Vec<(usize, usize, usize)>,
    #[cfg(target_arch = "wasm32")]
    prossimo: usize,
}

/// Calcola una casella e la scrive nella griglia.
fn calcola_casella(lavoro: (usize, usize, usize), massa: f64, griglia: &Mutex<Griglia>) {
    let (x, y, lato) = lavoro;
    let (alfa, beta) = coordinate_mappa(x, y);
    let esito = esito_mappa(alfa, beta, massa);
    let colore = colore_esito(&esito);
    if let Ok(mut g) = griglia.lock() {
        for yy in y..(y + lato).min(LATO_MAPPA) {
            for xx in x..(x + lato).min(LATO_MAPPA) {
                let i = yy * LATO_MAPPA + xx;
                g.esiti[i] = Some(esito);
                g.pixel[i * 4..i * 4 + 4].copy_from_slice(&colore);
            }
        }
    }
}

impl Mappa {
    fn nuova(massa: f64) -> Self {
        Self {
            massa,
            griglia: Arc::new(Mutex::new(Griglia {
                esiti: vec![None; LATO_MAPPA * LATO_MAPPA],
                // Le caselle non ancora calcolate non devono sembrare un esito.
                pixel: [36u8, 40, 58, 255].repeat(LATO_MAPPA * LATO_MAPPA),
            })),
            fatti: Arc::new(AtomicUsize::new(0)),
            ferma: Arc::new(AtomicBool::new(false)),
            totale: 0,
            avviata: false,
            texture: None,
            disegnati: usize::MAX,
            #[cfg(target_arch = "wasm32")]
            coda: Vec::new(),
            #[cfg(target_arch = "wasm32")]
            prossimo: 0,
        }
    }

    /// Dal grossolano al fine: si vede subito la forma e poi i dettagli.
    fn lavori() -> Vec<(usize, usize, usize)> {
        let mut lista = Vec::new();
        // Nel browser c'e' un solo processore a disposizione: ci si ferma prima.
        #[cfg(target_arch = "wasm32")]
        const LATI: [usize; 3] = [16, 8, 4];
        #[cfg(not(target_arch = "wasm32"))]
        const LATI: [usize; 5] = [16, 8, 4, 2, 1];
        for lato in LATI {
            for y in (0..LATO_MAPPA).step_by(lato) {
                for x in (0..LATO_MAPPA).step_by(lato) {
                    let gia_fatto = lato < 16 && x % (2 * lato) == 0 && y % (2 * lato) == 0;
                    if !gia_fatto {
                        lista.push((x, y, lato));
                    }
                }
            }
        }
        lista
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn avvia(&mut self) {
        if self.avviata {
            return;
        }
        self.avviata = true;
        let lavori = Arc::new(Self::lavori());
        self.totale = lavori.len();
        let prossimo = Arc::new(AtomicUsize::new(0));
        let quanti = std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(1).max(1));
        for _ in 0..quanti {
            let (lavori, prossimo) = (lavori.clone(), prossimo.clone());
            let (griglia, fatti, ferma) = (self.griglia.clone(), self.fatti.clone(), self.ferma.clone());
            let massa = self.massa;
            std::thread::spawn(move || loop {
                if ferma.load(Ordering::Relaxed) {
                    return;
                }
                let k = prossimo.fetch_add(1, Ordering::Relaxed);
                let Some(&lavoro) = lavori.get(k) else { return };
                calcola_casella(lavoro, massa, &griglia);
                fatti.fetch_add(1, Ordering::Relaxed);
            });
        }
    }

    /// Nel browser si lavora a piccole dosi, dentro il fotogramma.
    #[cfg(target_arch = "wasm32")]
    fn avvia(&mut self) {
        if !self.avviata {
            self.avviata = true;
            self.coda = Self::lavori();
            self.totale = self.coda.len();
        }
        let inizio = web_time::Instant::now();
        while inizio.elapsed().as_secs_f64() < 0.012 {
            let Some(&lavoro) = self.coda.get(self.prossimo) else { return };
            self.prossimo += 1;
            calcola_casella(lavoro, self.massa, &self.griglia);
            self.fatti.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn avanzamento(&self) -> f32 {
        if self.totale == 0 {
            0.0
        } else {
            self.fatti.load(Ordering::Relaxed) as f32 / self.totale as f32
        }
    }

    fn esito(&self, x: usize, y: usize) -> Option<EsitoMappa> {
        self.griglia.lock().ok()?.esiti[y * LATO_MAPPA + x]
    }

    fn aggiorna_texture(&mut self, ctx: &egui::Context) -> Option<TextureHandle> {
        let fatti = self.fatti.load(Ordering::Relaxed);
        if self.texture.is_none() || fatti != self.disegnati {
            self.disegnati = fatti;
            let immagine = {
                let g = self.griglia.lock().ok()?;
                ColorImage::from_rgba_unmultiplied([LATO_MAPPA, LATO_MAPPA], &g.pixel)
            };
            match &mut self.texture {
                Some(t) => t.set(immagine, TextureOptions::NEAREST),
                None => self.texture = Some(ctx.load_texture("mappa_caos", immagine, TextureOptions::NEAREST)),
            }
        }
        self.texture.clone()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn attendi(&self, secondi: f64) {
        let inizio = std::time::Instant::now();
        while self.fatti.load(Ordering::Relaxed) < self.totale && inizio.elapsed().as_secs_f64() < secondi {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
}

impl Drop for Mappa {
    fn drop(&mut self) {
        self.ferma.store(true, Ordering::Relaxed);
    }
}

fn coordinate_mappa(x: usize, y: usize) -> (f64, f64) {
    let lato = LATO_MAPPA as f64;
    ((x as f64 + 0.5) / lato, 1.0 - (y as f64 + 0.5) / lato)
}

fn colore_esito(e: &EsitoMappa) -> [u8; 4] {
    let luce = (1.0 - ((1.0 + e.tempo) as f64).ln() / (1.0 + TEMPO_MAPPA).ln()).clamp(0.08, 1.0) as f32 * 1.25;
    let base = match (e.scontro, e.espulso) {
        (true, _) => [255.0, 255.0, 255.0],
        (_, Some(0)) => [90.0, 215.0, 255.0],
        (_, Some(1)) => [255.0, 160.0, 90.0],
        (_, Some(_)) => [255.0, 110.0, 180.0],
        _ => return [10, 12, 22, 255],
    };
    [
        (base[0] * luce).min(255.0) as u8,
        (base[1] * luce).min(255.0) as u8,
        (base[2] * luce).min(255.0) as u8,
        255,
    ]
}

#[derive(Clone, Copy)]
struct VistaPiano {
    centro: [f64; 2],
    raggio: f64,
}

pub struct TreCorpi {
    pub iniziale: Vec<Corpo>,
    nome: String,
    nota: &'static str,
    reali: bool,
    pub sistema: Sistema,
    gemello: Option<Sistema>,
    pub caos: bool,
    log_soffio: f64,
    log_tolleranza: f64,
    scie: Vec<Vec<Campione>>,
    scia_gemello: Vec<Vec<[f32; 2]>>,
    errore: Vec<[f32; 2]>,
    divergenza: Vec<[f32; 2]>,
    temperatura: Vec<[f32; 2]>,
    pub in_corso: bool,
    pub ritmo: f64,
    t: f64,
    pub scelto: usize,
    vista: VistaPiano,
    pub rotante: bool,
    pub pagina: Pagina,
    coda: f64,
    previsione: Vec<Vec<[f32; 2]>>,
    gesto: Option<(usize, bool)>,
    pub mappa: Mappa,
    massa_mappa: f64,
}

impl Default for TreCorpi {
    fn default() -> Self {
        let mut s = Self {
            iniziale: Vec::new(),
            nome: String::new(),
            nota: "",
            reali: false,
            sistema: Sistema::nuovo(&[], 1e-10),
            gemello: None,
            caos: false,
            log_soffio: -6.0,
            log_tolleranza: -11.0,
            scie: Vec::new(),
            scia_gemello: Vec::new(),
            errore: Vec::new(),
            divergenza: Vec::new(),
            temperatura: Vec::new(),
            in_corso: true,
            ritmo: 1.0,
            t: 0.0,
            scelto: 0,
            vista: VistaPiano { centro: [0.0, 0.0], raggio: 3.0 },
            rotante: false,
            pagina: Pagina::Banco,
            coda: 1e9,
            previsione: Vec::new(),
            gesto: None,
            mappa: Mappa::nuova(1.0),
            massa_mappa: 1.0,
        };
        s.carica(1);
        s
    }
}

/// Le due masse maggiori: la vista che ruota segue loro.
fn coppia_maggiore(corpi: &[Corpo]) -> (usize, usize) {
    let mut indici: Vec<usize> = (0..corpi.len()).collect();
    indici.sort_by(|a, b| corpi[*b].massa.total_cmp(&corpi[*a].massa));
    (indici[0], *indici.get(1).unwrap_or(&indici[0]))
}

/// Coordinate nel sistema che ruota con la coppia maggiore.
fn ruota(s: &Sistema, coppia: (usize, usize), p: [f64; 2]) -> [f64; 2] {
    let (a, b) = (s.posizione(coppia.0), s.posizione(coppia.1));
    let (ma, mb) = (s.massa(coppia.0), s.massa(coppia.1));
    let m = (ma + mb).max(1e-30);
    let (cx, cy) = ((a[0] * ma + b[0] * mb) / m, (a[1] * ma + b[1] * mb) / m);
    let (sa, ca) = (b[1] - a[1]).atan2(b[0] - a[0]).sin_cos();
    let (dx, dy) = (p[0] - cx, p[1] - cy);
    [ca * dx + sa * dy, -sa * dx + ca * dy]
}

fn registra(s: &Sistema, scie: &mut [Vec<Campione>], coppia: (usize, usize)) {
    for (i, scia) in scie.iter_mut().enumerate() {
        let p = s.posizione(i);
        if let Some(u) = scia.last() {
            let (dx, dy) = (f64::from(u.p[0]) - p[0], f64::from(u.p[1]) - p[1]);
            let soglia = 0.002 * (0.2 + p[0].hypot(p[1]));
            if dx * dx + dy * dy < soglia * soglia {
                continue;
            }
        }
        let r = ruota(s, coppia, p);
        scia.push(Campione { p: [p[0] as f32, p[1] as f32], r: [r[0] as f32, r[1] as f32], t: s.t as f32 });
        if scia.len() > MAX_CAMPIONI {
            let mut k = 0;
            scia.retain(|_| {
                k += 1;
                k % 2 == 1
            });
        }
    }
}

fn accoda(serie: &mut Vec<[f32; 2]>, punto: [f32; 2]) {
    serie.push(punto);
    if serie.len() > MAX_STORIA {
        let mut k = 0;
        serie.retain(|_| {
            k += 1;
            k % 2 == 1
        });
    }
}

impl TreCorpi {
    fn tolleranza(&self) -> f64 {
        10f64.powf(self.log_tolleranza)
    }

    pub fn carica(&mut self, i: usize) {
        let v = &GALLERIA[i];
        self.imposta((v.corpi)(), v.nome.to_owned(), v.nota, v.unita_reali);
        self.in_corso = true;
    }

    fn imposta(&mut self, corpi: Vec<Corpo>, nome: String, nota: &'static str, reali: bool) {
        self.iniziale = corpi;
        self.nome = nome;
        self.nota = nota;
        self.reali = reali;
        self.scelto = 0;
        self.rotante = false;
        let massimo = self.iniziale.iter().map(|c| c.x[0].hypot(c.x[1])).fold(0.5, f64::max);
        self.vista = VistaPiano { centro: [0.0, 0.0], raggio: massimo * 1.6 };
        self.ritmo = if reali { 30.0 } else { 1.0 };
        self.ricomincia();
    }

    pub fn ricomincia(&mut self) {
        let tol = self.tolleranza();
        self.sistema = Sistema::nuovo(&self.iniziale, tol);
        self.gemello = self.caos.then(|| {
            let mut altri = self.iniziale.clone();
            altri[0].x[0] += 10f64.powf(self.log_soffio);
            Sistema::nuovo(&altri, tol)
        });
        self.t = 0.0;
        self.scie = vec![Vec::new(); self.iniziale.len()];
        self.scia_gemello = vec![Vec::new(); self.iniziale.len()];
        self.errore.clear();
        self.divergenza.clear();
        self.temperatura.clear();
        self.previsione.clear();
        let coppia = coppia_maggiore(&self.iniziale);
        registra(&self.sistema, &mut self.scie, coppia);
    }

    /// Dopo una modifica: si riparte fermi e si mostra dove porterebbe.
    fn modificato(&mut self) {
        self.ricomincia();
        self.in_corso = false;
        let mut prova = Sistema::nuovo(&self.iniziale, 1e-8);
        let n = self.iniziale.len();
        let mut tracce: Vec<Vec<[f32; 2]>> = vec![Vec::new(); n];
        let durata = if self.reali { 60.0 } else { 15.0 };
        prova.avanza(durata, 3000, |s| {
            for (i, traccia) in tracce.iter_mut().enumerate() {
                let p = s.posizione(i);
                traccia.push([p[0] as f32, p[1] as f32]);
            }
        });
        self.previsione = tracce;
    }

    fn pianeta(&self) -> Option<usize> {
        if self.reali {
            self.iniziale.iter().position(|c| c.massa == 0.0)
        } else {
            None
        }
    }

    /// Temperatura di equilibrio sul pianeta: 278 K a 1 UA da un Sole.
    fn clima(&self, pianeta: usize) -> f64 {
        let p = self.sistema.posizione(pianeta);
        let flusso: f64 = (0..self.sistema.numero())
            .filter(|&j| j != pianeta)
            .map(|j| {
                let q = self.sistema.posizione(j);
                let r2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
                self.sistema.massa(j).powf(3.5) / r2
            })
            .sum();
        278.0 * flusso.powf(0.25)
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            self.t += self.ritmo * dt;
        }
        let coppia = coppia_maggiore(&self.iniziale);
        let scie = &mut self.scie;
        self.sistema.avanza(self.t, BUDGET, |s| registra(s, scie, coppia));
        if let Some(g) = &mut self.gemello {
            let scie = &mut self.scia_gemello;
            g.avanza(self.t, BUDGET, |s| {
                for (i, scia) in scie.iter_mut().enumerate() {
                    let p = s.posizione(i);
                    scia.push([p[0] as f32, p[1] as f32]);
                    if scia.len() > MAX_CAMPIONI {
                        let mut k = 0;
                        scia.retain(|_| {
                            k += 1;
                            k % 2 == 1
                        });
                    }
                }
            });
        }
        if self.in_corso {
            let t = self.sistema.t as f32;
            accoda(&mut self.errore, [t, (self.sistema.errore_energia().max(1e-17)).log10() as f32]);
            if let Some(g) = &self.gemello {
                accoda(&mut self.divergenza, [t, self.sistema.distanza_da(g).max(1e-17).log10() as f32]);
            }
            if let Some(p) = self.pianeta() {
                let clima = self.clima(p) as f32;
                accoda(&mut self.temperatura, [t, clima]);
            }
        }
    }

    /// Stima del ritmo a cui due gemelli si separano, prima che la distanza saturi.
    fn lyapunov(&self) -> Option<f64> {
        let primo = self.divergenza.first()?;
        let ultimo = self
            .divergenza
            .iter()
            .take_while(|p| p[1] < -1.0)
            .last()?;
        (ultimo[0] - primo[0] > 2.0).then(|| f64::from(ultimo[1] - primo[1]) * std::f64::consts::LN_10 / f64::from(ultimo[0] - primo[0]))
    }

    #[cfg(test)]
    pub fn schermo_partenza(&self, area: Rect, i: usize) -> Pos2 {
        Vista::nuova(area, self.vista).a_schermo(self.iniziale[i].x)
    }

    fn apri_dalla_mappa(&mut self, alfa: f64, beta: f64) {
        self.imposta(corpi_mappa(alfa, beta, self.massa_mappa), "Dalla mappa".to_owned(), "Tre corpi fermi, presi dalla casella che hai cliccato.", false);
        self.in_corso = true;
        self.pagina = Pagina::Banco;
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn mappa_pronta(&mut self, secondi: f64) {
        self.pagina = Pagina::Mappa;
        self.mappa.avvia();
        self.mappa.attendi(secondi);
    }

    fn punto(&self, q: [f32; 2], r: [f32; 2]) -> [f64; 2] {
        if self.rotante {
            [f64::from(r[0]), f64::from(r[1])]
        } else {
            [f64::from(q[0]), f64::from(q[1])]
        }
    }
}

#[derive(Clone, Copy)]
struct Vista {
    schermo: Pos2,
    centro: [f64; 2],
    px: f64,
}

impl Vista {
    fn nuova(area: Rect, v: VistaPiano) -> Self {
        Self { schermo: area.center(), centro: v.centro, px: f64::from(area.height()) * 0.5 / v.raggio }
    }

    fn a_schermo(&self, p: [f64; 2]) -> Pos2 {
        self.schermo + Vec2::new(((p[0] - self.centro[0]) * self.px) as f32, -(((p[1] - self.centro[1]) * self.px) as f32))
    }

    fn a_mondo(&self, q: Pos2) -> [f64; 2] {
        let d = q - self.schermo;
        [self.centro[0] + f64::from(d.x) / self.px, self.centro[1] - f64::from(d.y) / self.px]
    }
}

pub fn mostra(ui: &mut egui::Ui, c: &mut TreCorpi) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        c.in_corso = !c.in_corso;
    }
    egui::Panel::bottom("tempo_tre")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            if pannello::barra_tempo(ui, &mut c.in_corso, &mut c.ritmo, 0.05..=2000.0, "unità al secondo") {
                c.ricomincia();
                c.in_corso = true;
            }
        });
    egui::Panel::right("colonna_tre")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, c));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        match c.pagina {
            Pagina::Banco => banco(ui, area, c),
            Pagina::Mappa => mappa(ui, area, c),
        }
    });
}

fn punta(c: &TreCorpi, i: usize, v: &Vista) -> Pos2 {
    let corpo = c.iniziale[i];
    let k = c.vista.raggio * 0.35;
    v.a_schermo([corpo.x[0] + corpo.v[0] * k, corpo.x[1] + corpo.v[1] * k])
}

fn gesti(ui: &egui::Ui, r: &egui::Response, area: Rect, c: &mut TreCorpi, v: Vista) {
    if r.hovered() {
        let rotella = ui.input(|i| i.smooth_scroll_delta.y);
        if rotella != 0.0 {
            let cursore = ui.input(|i| i.pointer.hover_pos()).unwrap_or(area.center());
            let sotto = v.a_mondo(cursore);
            c.vista.raggio = (c.vista.raggio * f64::from(-rotella * 0.0025).exp()).clamp(0.05, 5e4);
            let n = Vista::nuova(area, c.vista);
            let d = cursore - area.center();
            c.vista.centro = [sotto[0] - f64::from(d.x) / n.px, sotto[1] + f64::from(d.y) / n.px];
        }
    }
    if r.dragged_by(PointerButton::Secondary) || r.dragged_by(PointerButton::Middle) {
        let d = r.drag_delta();
        c.vista.centro[0] -= f64::from(d.x) / v.px;
        c.vista.centro[1] += f64::from(d.y) / v.px;
    }

    let presa_su = |c: &TreCorpi, q: Pos2| -> Option<(usize, bool)> {
        if c.rotante {
            return None;
        }
        (0..c.iniziale.len()).find_map(|i| {
            if (punta(c, i, &v) - q).length() < PRESA {
                Some((i, true))
            } else if (v.a_schermo(c.iniziale[i].x) - q).length() < PRESA {
                Some((i, false))
            } else {
                None
            }
        })
    };

    if r.drag_started_by(PointerButton::Primary) {
        let premuto = ui.input(|i| i.pointer.press_origin()).unwrap_or(area.center());
        c.gesto = presa_su(c, premuto);
        if let Some((i, _)) = c.gesto {
            c.scelto = i;
        }
    }
    if r.dragged_by(PointerButton::Primary) {
        match c.gesto {
            Some((i, punta_v)) => {
                if let Some(q) = ui.input(|p| p.pointer.latest_pos()) {
                    let m = v.a_mondo(q);
                    if punta_v {
                        let k = c.vista.raggio * 0.35;
                        c.iniziale[i].v = [(m[0] - c.iniziale[i].x[0]) / k, (m[1] - c.iniziale[i].x[1]) / k];
                    } else {
                        c.iniziale[i].x = m;
                    }
                    c.modificato();
                }
            }
            None => {
                let d = r.drag_delta();
                c.vista.centro[0] -= f64::from(d.x) / v.px;
                c.vista.centro[1] += f64::from(d.y) / v.px;
            }
        }
    }
    if r.drag_stopped_by(PointerButton::Primary) {
        c.gesto = None;
    }
    if r.clicked() {
        if let Some(q) = r.interact_pointer_pos() {
            let vicino = (0..c.sistema.numero()).min_by(|&a, &b| {
                let d = |i: usize| (v.a_schermo(posizione_vista(c, i)) - q).length_sq();
                d(a).total_cmp(&d(b))
            });
            if let Some(i) = vicino.filter(|&i| (v.a_schermo(posizione_vista(c, i)) - q).length() < 24.0) {
                c.scelto = i;
            }
        }
    }
}

fn posizione_vista(c: &TreCorpi, i: usize) -> [f64; 2] {
    let p = c.sistema.posizione(i);
    if c.rotante {
        ruota(&c.sistema, coppia_maggiore(&c.iniziale), p)
    } else {
        p
    }
}

fn raggio_corpo(massa: f64) -> f32 {
    if massa <= 0.0 {
        2.5
    } else {
        (massa.cbrt() as f32 * 7.0).clamp(3.0, 15.0)
    }
}

fn banco(ui: &mut egui::Ui, area: Rect, c: &mut TreCorpi) {
    let risposta = ui.interact(area, ui.id().with("tre"), Sense::click_and_drag());
    let v = Vista::nuova(area, c.vista);
    gesti(ui, &risposta, area, c, v);
    let v = Vista::nuova(area, c.vista);
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    griglia(&p, area, v);

    if c.rotante {
        punti_l(&p, c, v);
    }

    let limite = c.sistema.t as f32 - c.coda as f32;
    for (i, traccia) in c.previsione.iter().enumerate() {
        let passo = (traccia.len() / MAX_PUNTI).max(1);
        let punti: Vec<Pos2> = traccia
            .iter()
            .step_by(passo)
            .map(|q| v.a_schermo([f64::from(q[0]), f64::from(q[1])]))
            .collect();
        p.extend(Shape::dashed_line(&punti, Stroke::new(1.2, COLORI[i].gamma_multiply(0.7)), 5.0, 5.0));
    }
    for (i, scia) in c.scia_gemello.iter().enumerate() {
        if scia.len() > 1 && !c.rotante {
            let passo = (scia.len() / MAX_PUNTI).max(1);
            let punti: Vec<Pos2> = scia.iter().step_by(passo).map(|q| v.a_schermo([f64::from(q[0]), f64::from(q[1])])).collect();
            p.add(Shape::line(punti, Stroke::new(1.0, COLORI[i].gamma_multiply(0.35))));
        }
    }
    for (i, scia) in c.scie.iter().enumerate() {
        let inizio = scia.partition_point(|q| q.t < limite);
        let visibili = &scia[inizio..];
        if visibili.len() > 1 {
            let passo = (visibili.len() / MAX_PUNTI).max(1);
            let punti: Vec<Pos2> = visibili.iter().step_by(passo).map(|q| v.a_schermo(c.punto(q.p, q.r))).collect();
            p.add(Shape::line(punti, Stroke::new(if i == c.scelto { 2.2 } else { 1.5 }, COLORI[i])));
        }
    }

    for i in 0..c.sistema.numero() {
        let q = v.a_schermo(posizione_vista(c, i));
        let massa = c.sistema.massa(i);
        p.circle_filled(q, raggio_corpo(massa), COLORI[i]);
        if i == c.scelto {
            p.circle_stroke(q, raggio_corpo(massa) + 5.0, Stroke::new(1.3, CHIARO));
        }
        if let Some(g) = &c.gemello {
            if !c.rotante {
                let a = g.posizione(i);
                p.circle_stroke(v.a_schermo(a), raggio_corpo(massa), Stroke::new(1.0, COLORI[i].gamma_multiply(0.6)));
            }
        }
    }

    if !c.rotante {
        for i in 0..c.iniziale.len() {
            let inizio = v.a_schermo(c.iniziale[i].x);
            let cima = punta(c, i, &v);
            p.circle_stroke(inizio, 5.0, Stroke::new(1.1, COLORI[i].gamma_multiply(0.85)));
            p.line_segment([inizio, cima], Stroke::new(1.1, COLORI[i].gamma_multiply(0.6)));
            p.circle_filled(cima, 3.8, COLORI[i].gamma_multiply(0.9));
        }
    }

    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Trascina un cerchietto per spostare la partenza, il punto pieno per la spinta. Rotella per lo zoom.", f.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, c.nota, f, CIANO);
}

fn passo_tondo(minimo: f64) -> f64 {
    crate::banco::passo_tondo(minimo)
}

fn griglia(p: &Painter, area: Rect, v: Vista) {
    let passo = passo_tondo(70.0 / v.px);
    let a = v.a_mondo(area.left_bottom());
    let b = v.a_mondo(area.right_top());
    let tratto = Stroke::new(1.0, LINEA);
    let f = FontId::monospace(10.0);
    let mut x = (a[0] / passo).ceil() * passo;
    while x < b[0] {
        let q = v.a_schermo([x, 0.0]).x;
        p.line_segment([Pos2::new(q, area.top()), Pos2::new(q, area.bottom())], tratto);
        p.text(Pos2::new(q + 3.0, area.bottom() - 4.0), Align2::LEFT_BOTTOM, format!("{}", x + 0.0).replace('.', ","), f.clone(), GRIGIO);
        x += passo;
    }
    let mut y = (a[1] / passo).ceil() * passo;
    while y < b[1] {
        let q = v.a_schermo([0.0, y]).y;
        p.line_segment([Pos2::new(area.left(), q), Pos2::new(area.right(), q)], tratto);
        y += passo;
    }
}

/// I cinque punti di Lagrange della coppia maggiore, nella vista che ruota.
fn punti_l(p: &Painter, c: &TreCorpi, v: Vista) {
    let coppia = coppia_maggiore(&c.iniziale);
    let (ma, mb) = (c.iniziale[coppia.0].massa, c.iniziale[coppia.1].massa);
    let mu = mb / (ma + mb).max(1e-30);
    let (a, b) = (c.sistema.posizione(coppia.0), c.sistema.posizione(coppia.1));
    let separazione = (a[0] - b[0]).hypot(a[1] - b[1]);
    for (i, l) in punti_lagrange(mu).iter().enumerate() {
        let q = v.a_schermo([l[0] * separazione, l[1] * separazione]);
        p.line_segment([q - Vec2::new(5.0, 0.0), q + Vec2::new(5.0, 0.0)], Stroke::new(1.2, GIALLO));
        p.line_segment([q - Vec2::new(0.0, 5.0), q + Vec2::new(0.0, 5.0)], Stroke::new(1.2, GIALLO));
        p.text(q + Vec2::new(7.0, -7.0), Align2::LEFT_BOTTOM, format!("L{}", i + 1), FontId::monospace(10.5), GIALLO);
    }
}

fn mappa(ui: &mut egui::Ui, area: Rect, c: &mut TreCorpi) {
    c.mappa.avvia();
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let lato = (area.height() - 96.0).min(area.width() - 120.0);
    let piano = Rect::from_center_size(area.center() + Vec2::new(0.0, 14.0), Vec2::splat(lato));
    if let Some(texture) = c.mappa.aggiorna_texture(ui.ctx()) {
        p.image(texture.id(), piano, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
    let risposta = ui.interact(piano, ui.id().with("mappa"), Sense::click());

    let casella = |q: Pos2| {
        let x = ((q.x - piano.left()) / piano.width() * LATO_MAPPA as f32) as usize;
        let y = ((q.y - piano.top()) / piano.height() * LATO_MAPPA as f32) as usize;
        (x.min(LATO_MAPPA - 1), y.min(LATO_MAPPA - 1))
    };
    if let Some(q) = risposta.hover_pos() {
        let (x, y) = casella(q);
        if let Some(e) = c.mappa.esito(x, y) {
            let testo = match (e.scontro, e.espulso) {
                (true, _) => format!("scontro dopo {:.1}", e.tempo),
                (_, Some(0)) => format!("se ne va il corpo azzurro dopo {:.1}", e.tempo),
                (_, Some(1)) => format!("se ne va il corpo arancione dopo {:.1}", e.tempo),
                (_, Some(_)) => format!("se ne va il corpo rosa dopo {:.1}", e.tempo),
                _ => format!("ancora insieme dopo {:.0}", e.tempo),
            }
            .replace('.', ",");
            p.text(q + Vec2::new(14.0, -8.0), Align2::LEFT_BOTTOM, testo, FontId::proportional(13.0), Color32::WHITE);
        }
    }
    if risposta.clicked() {
        if let Some(q) = risposta.interact_pointer_pos() {
            let (x, y) = casella(q);
            let (alfa, beta) = coordinate_mappa(x, y);
            c.apri_dalla_mappa(alfa, beta);
        }
    }

    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Tre corpi fermi: due alla base, il terzo parte da questa casella. Il colore dice chi se ne va, la luce quanto presto.", f.clone(), GRIGIO);
    let avanzamento = c.mappa.avanzamento();
    let testo = if avanzamento < 1.0 {
        format!("Calcolo in corso: {:.0} %. Clic su una casella per guardarla da vicino.", avanzamento * 100.0)
    } else {
        "Pronta. Clic su una casella per guardarla da vicino: dove i colori si mescolano, un millimetro cambia tutto.".to_owned()
    };
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, testo, f, CIANO);
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

fn numero(x: f64) -> String {
    format!("{x:.3}").replace('.', ",")
}

fn grafico(ui: &mut egui::Ui, nome: &str, serie: &[[f32; 2]], tinta: Color32, banda: Option<(f32, f32)>) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_GRAFICO), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let dentro = area.shrink2(Vec2::new(8.0, 14.0));
    let f = FontId::monospace(10.0);
    p.text(area.left_top() + Vec2::new(5.0, 3.0), Align2::LEFT_TOP, nome, f.clone(), GRIGIO);
    if serie.len() < 2 {
        return;
    }
    let (mut basso, mut alto) = (f32::MAX, f32::MIN);
    for q in serie {
        basso = basso.min(q[1]);
        alto = alto.max(q[1]);
    }
    if let Some((a, b)) = banda {
        basso = basso.min(a);
        alto = alto.max(b);
    }
    if alto - basso < 1e-3 {
        alto += 0.5;
        basso -= 0.5;
    }
    let (t0, t1) = (serie[0][0], serie[serie.len() - 1][0].max(serie[0][0] + 1e-3));
    let a_schermo = |t: f32, v: f32| {
        Pos2::new(
            dentro.left() + dentro.width() * (t - t0) / (t1 - t0),
            dentro.bottom() - dentro.height() * (v - basso) / (alto - basso),
        )
    };
    if let Some((a, b)) = banda {
        p.rect_filled(
            Rect::from_two_pos(a_schermo(t0, a), a_schermo(t1, b)),
            0.0,
            VERDE.gamma_multiply(0.14),
        );
    }
    let passo = (serie.len() / 900).max(1);
    let punti: Vec<Pos2> = serie.iter().step_by(passo).map(|q| a_schermo(q[0], q[1])).collect();
    p.add(Shape::line(punti, Stroke::new(1.5, tinta)));
    p.text(area.right_top() + Vec2::new(-5.0, 3.0), Align2::RIGHT_TOP, format!("{alto:.1}  /  {basso:.1}").replace('.', ","), f, GRIGIO);
}

/// Il cielo del pianeta: dove sono i soli e quanto sono grandi visti da li'.
fn cielo(ui: &mut egui::Ui, c: &TreCorpi, pianeta: usize) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_CIELO), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, Color32::from_rgb(14, 24, 44));
    p.text(area.left_top() + Vec2::new(5.0, 3.0), Align2::LEFT_TOP, "IL SUO CIELO, ORIZZONTE COMPLETO", FontId::monospace(10.0), GRIGIO);
    let qui = c.sistema.posizione(pianeta);
    for j in (0..c.sistema.numero()).filter(|&j| j != pianeta) {
        let q = c.sistema.posizione(j);
        let (dx, dy) = (q[0] - qui[0], q[1] - qui[1]);
        let angolo = dy.atan2(dx).rem_euclid(TAU);
        let distanza = dx.hypot(dy);
        let r = ((12.0 / distanza) as f32).clamp(1.5, 24.0);
        let x = area.left() + area.width() * (angolo / TAU) as f32;
        p.circle_filled(Pos2::new(x, area.center().y + 6.0), r, COLORI[j]);
    }
}

fn strumenti(ui: &mut egui::Ui, c: &mut TreCorpi) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut c.pagina, Pagina::Banco, RichText::new("Banco").strong());
        let scheda_mappa = ui.selectable_value(&mut c.pagina, Pagina::Mappa, RichText::new("Mappa del caos").strong());
        crate::visite::segna(ui, "mappa", &scheda_mappa);
    });
    ui.label(RichText::new(&c.nome).size(16.0).strong().color(CIANO));

    let mut carica = None;
    titoletto(ui, "SCENE");
    let mut gruppi: Vec<&str> = Vec::new();
    for v in &GALLERIA {
        if !gruppi.contains(&v.gruppo) {
            gruppi.push(v.gruppo);
        }
    }
    for g in gruppi {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(g).small().color(GRIGIO));
            for (i, v) in GALLERIA.iter().enumerate().filter(|(_, v)| v.gruppo == g) {
                if ui.small_button(v.nome).clicked() {
                    carica = Some(i);
                }
            }
        });
    }
    if let Some(i) = carica {
        c.carica(i);
    }

    if c.pagina == Pagina::Mappa {
        titoletto(ui, "LA MAPPA");
        ui.add(egui::Slider::new(&mut c.massa_mappa, 0.2..=3.0).text("massa del terzo corpo").fixed_decimals(2));
        // Si ricalcola quando il cursore viene lasciato, non a ogni spostamento.
        if c.mappa.massa != c.massa_mappa && ui.input(|i| !i.pointer.any_down()) {
            c.mappa = Mappa::nuova(c.massa_mappa);
        }
        ui.add_space(4.0);
        for (tinta, testo) in [
            (COLORI[0], "se ne va il corpo in basso a sinistra"),
            (COLORI[1], "se ne va il corpo in basso a destra"),
            (Color32::from_rgb(255, 110, 180), "se ne va quello che parte dalla casella"),
            (Color32::WHITE, "scontro, o incontro troppo stretto da calcolare"),
            (Color32::from_rgb(40, 44, 66), "ancora tutti insieme alla fine del calcolo"),
        ] {
            ui.horizontal(|ui| {
                let (q, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                ui.painter().rect_filled(q, 2.0, tinta);
                ui.label(RichText::new(testo).color(CHIARO));
            });
        }
        ui.label(RichText::new(format!("Ogni casella è un'intera simulazione, fino al tempo {TEMPO_MAPPA:.0}.")).color(GRIGIO));
        return;
    }

    titoletto(ui, "CORPO SCELTO");
    let n = c.iniziale.len();
    if c.scelto >= n {
        c.scelto = 0;
    }
    ui.horizontal(|ui| {
        for i in 0..n {
            let etichetta = RichText::new(format!("{}", i + 1)).strong().color(COLORI[i]);
            if ui.selectable_label(c.scelto == i, etichetta).clicked() {
                c.scelto = i;
            }
        }
        if n < MAX_CORPI && ui.small_button("Aggiungi").clicked() {
            let r = c.vista.raggio * 0.5;
            c.iniziale.push(Corpo { massa: 0.1, x: [r, r], v: [0.0, 0.0] });
            c.modificato();
        }
        if n > 2 && ui.small_button("Togli").clicked() {
            c.iniziale.remove(c.scelto);
            c.scelto = 0;
            c.modificato();
        }
    });
    let scelto = c.scelto;
    if c.iniziale[scelto].massa > 0.0 {
        let mut massa = c.iniziale[scelto].massa;
        if ui.add(egui::Slider::new(&mut massa, 1e-7..=2000.0).logarithmic(true).text("massa")).changed() {
            c.iniziale[scelto].massa = massa;
            c.modificato();
        }
    } else {
        ui.label(RichText::new("Senza massa: subisce la gravità, non la esercita.").color(GRIGIO));
    }

    titoletto(ui, "CAOS");
    let prima = (c.caos, c.log_soffio);
    let gemello = ui.checkbox(&mut c.caos, "Un gemello che parte quasi uguale");
    crate::visite::segna(ui, "gemello", &gemello);
    if c.caos {
        ui.add(egui::Slider::new(&mut c.log_soffio, -12.0..=-1.0).text("differenza, 10 alla").fixed_decimals(0));
    }
    if prima != (c.caos, c.log_soffio) {
        c.ricomincia();
    }
    if c.gemello.is_some() {
        if let Some(g) = &c.gemello {
            ui.label(format!("Distanza dal gemello: {:.2e}", c.sistema.distanza_da(g)).replace('.', ","));
        }
        if let Some(l) = c.lyapunov() {
            ui.label(RichText::new(format!("L'errore si moltiplica per e ogni {} unità di tempo.", numero(1.0 / l.max(1e-9)))).color(CIANO));
        }
        grafico(ui, "distanza dal gemello, 10 alla", &c.divergenza, ROSSO, None);
    }

    ui.add_space(8.0);
    egui::CollapsingHeader::new(RichText::new("Avanzate").strong()).default_open(false).show(ui, |ui| {
    titoletto(ui, "PRECISIONE DEL CALCOLO");
    let prima = c.log_tolleranza;
    ui.add(egui::Slider::new(&mut c.log_tolleranza, -13.0..=-3.0).text("errore per passo, 10 alla").fixed_decimals(0));
    if prima != c.log_tolleranza {
        c.ricomincia();
    }
    ui.horizontal(|ui| {
        if ui.small_button("Inverti il tempo").on_hover_text("Tutte le velocità al contrario: dovrebbe tornare all'inizio").clicked() {
            c.sistema.inverti();
            if let Some(g) = &mut c.gemello {
                g.inverti();
            }
        }
        ui.checkbox(&mut c.rotante, "Vista che ruota");
    });
    ui.add(egui::Slider::new(&mut c.coda, 2.0..=1e9).logarithmic(true).text("durata della scia").show_value(false));

    titoletto(ui, "LETTURE");
    let tempo = if c.reali { format!("{} anni", numero(c.sistema.t / TAU)) } else { numero(c.sistema.t) };
    let finale = match c.sistema.scontro {
        Some((a, b)) => format!("scontro tra {} e {}", a + 1, b + 1),
        None => match c.sistema.espulso(c.vista.raggio.max(10.0)) {
            Some(i) => format!("il corpo {} se ne va", i + 1),
            None => "ancora insieme".to_owned(),
        },
    };
    egui::Grid::new("letture_tre").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Tempo", tempo);
        riga(ui, "Esito", finale);
        riga(ui, "Errore sull'energia", format!("{:.1e}", c.sistema.errore_energia()));
        riga(ui, "Errore sul momento", format!("{:.1e}", c.sistema.errore_momento()));
        riga(ui, "Passi di calcolo", c.sistema.passi.to_string());
        let coppia = coppia_maggiore(&c.iniziale);
        let centro = if scelto == coppia.0 { coppia.1 } else { coppia.0 };
        if scelto != centro {
            let (a, e) = elementi(&c.sistema, scelto, centro);
            if a > 0.0 && e < 1.0 {
                riga(ui, "Orbita: semiasse", numero(a));
                riga(ui, "Orbita: eccentricità", numero(e));
            } else {
                riga(ui, "Orbita", "aperta".to_owned());
            }
        }
    });
    grafico(ui, "errore sull'energia, 10 alla", &c.errore, GIALLO, None);
    });

    if let Some(p) = c.pianeta() {
        titoletto(ui, "IL PIANETA");
        cielo(ui, c, p);
        ui.add_space(4.0);
        ui.label(format!("Temperatura stimata: {:.0} K, {:.0} °C", c.clima(p), c.clima(p) - 273.15).replace('.', ","));
        grafico(ui, "temperatura, kelvin", &c.temperatura, ROSSO, Some((273.0, 373.0)));
    }

    ui.add_space(4.0);
    ui.label(RichText::new("Masse puntiformi nel piano, G = 1. Temperatura e luminosità dei soli sono stime.").small().color(GRIGIO));
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn caricare_e_avanzare_ogni_scena() {
        for i in 0..GALLERIA.len() {
            let mut c = TreCorpi::default();
            c.carica(i);
            c.caos = true;
            c.ricomincia();
            for _ in 0..20 {
                c.avanza(1.0 / 30.0);
            }
            assert!(c.sistema.t > 0.0, "{}", GALLERIA[i].nome);
        }
    }

    #[test]
    fn il_pianeta_ha_un_clima_che_si_puo_misurare() {
        let mut c = TreCorpi::default();
        let i = GALLERIA.iter().position(|v| v.nome == "Pianeta fra tre soli").unwrap();
        c.carica(i);
        let p = c.pianeta().unwrap();
        assert!(c.clima(p) > 100.0 && c.clima(p).is_finite());
    }

    #[test]
    fn la_mappa_copre_ogni_casella_una_volta() {
        let lavori = Mappa::lavori();
        let mut coperte = vec![false; LATO_MAPPA * LATO_MAPPA];
        for &(x, y, lato) in &lavori {
            for yy in y..(y + lato).min(LATO_MAPPA) {
                for xx in x..(x + lato).min(LATO_MAPPA) {
                    coperte[yy * LATO_MAPPA + xx] = true;
                }
            }
        }
        assert!(coperte.iter().all(|c| *c));
        let unici: std::collections::HashSet<(usize, usize, usize)> = lavori.iter().copied().collect();
        assert_eq!(unici.len(), lavori.len());
        assert_eq!(lavori.iter().filter(|l| l.2 == 1).count(), LATO_MAPPA * LATO_MAPPA * 3 / 4);
    }

    #[test]
    fn il_gemello_si_allontana_nel_caos() {
        let mut c = TreCorpi::default();
        let i = GALLERIA.iter().position(|v| v.nome == "Problema pitagorico").unwrap();
        c.carica(i);
        c.caos = true;
        c.ricomincia();
        c.ritmo = 1.0;
        for _ in 0..300 {
            c.avanza(0.2);
        }
        let d = c.sistema.distanza_da(c.gemello.as_ref().unwrap());
        assert!(d > 1e-3, "{d}");
    }
}
