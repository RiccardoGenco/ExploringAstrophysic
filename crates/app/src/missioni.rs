//! Missioni dell'Osservatorio su dati pubblicati: stessa forma, dati diversi.

use crate::formato::decimale;
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use colossi_fisica::fondo::{Cosmologia, PLANCK};
use colossi_fisica::missioni::{
    adatta, cerca_ligo, chi2, hulse_taylor, incertezze, ligo, parabola, rotazione, sequenza, supernova, Misura, Parametro,
    RigaRotazione, ANNO_INIZIO_HT, DP_HT, DP_HT_ERRORE, MASSA_C_HT, PERIODO_HT,
};
use egui::{Align2, Color32, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const LARGHEZZA: f32 = 430.0;
const PRESA: f32 = 11.0;
const PUNTI_CURVA: usize = 500;
const ARANCIO: Color32 = Color32::from_rgb(255, 160, 90);
const CHI_BUONO: f64 = 2.5;

type Modello = Box<dyn Fn(&[f64], f64) -> f64>;
type Risultato = fn(&[f64], Option<&[f64]>) -> Vec<(String, String)>;
type Ricerca = fn(&[Misura], &mut [Parametro]);

pub struct Missione {
    pub nome: &'static str,
    titolo: String,
    nota: &'static str,
    fonte: &'static str,
    confronto: &'static str,
    pub dati: Vec<Misura>,
    iniziali: Vec<Parametro>,
    pub par: Vec<Parametro>,
    modello: Modello,
    asse_x: &'static str,
    asse_y: &'static str,
    x_log: bool,
    y_invertito: bool,
    solo_punti: bool,
    seconda: Option<(Vec<[f64; 2]>, &'static str)>,
    risultato: Risultato,
    ricerca: Option<Ricerca>,
    sigma: Option<Vec<f64>>,
    modo: &'static str,
    mostra_confronto: bool,
    appunti: Vec<String>,
}

fn p(nome: &'static str, valore: f64, min: f64, max: f64) -> Parametro {
    Parametro { nome, min, max, valore, libero: true }
}

fn fissa(mut q: Parametro) -> Parametro {
    q.libero = false;
    q
}

fn numeri(riga: &str) -> Vec<f64> {
    riga.split(',').filter_map(|c| c.trim().parse().ok()).collect()
}

fn con_errore(v: f64, s: Option<f64>, cifre: usize) -> String {
    match s {
        Some(e) if e > 0.0 => format!("{v:.cifre$} ± {e:.cifre$}").replace('.', ","),
        _ => format!("{v:.cifre$}").replace('.', ","),
    }
}

fn voce(s: Option<&[f64]>, i: usize) -> Option<f64> {
    s.map(|q| q[i])
}

fn missione_ligo() -> Missione {
    let mut dati = Vec::new();
    let mut teorica = Vec::new();
    for riga in include_str!("../dati/ligo_h1.csv").lines() {
        let v = numeri(riga);
        dati.push(Misura { x: v[0], y: v[1], err: 0.15, attiva: v[0] < 0.405 && v[0] > 0.30 });
        teorica.push([v[0], v[2]]);
    }
    let par = vec![
        p("massa chirp, soli", 20.0, 10.0, 60.0),
        p("istante della fusione, s", 0.41, 0.40, 0.46),
        p("ampiezza, parte 1", 0.3, -3.0, 3.0),
        p("ampiezza, parte 2", 0.3, -3.0, 3.0),
    ];
    Missione {
        nome: "Onde LIGO",
        titolo: "Il primo segnale, 14 settembre 2015".to_owned(),
        nota: "La curva bianca è il segnale registrato a Hanford. Regola la massa finché la spirale si sovrappone alle oscillazioni.",
        fonte: "Dati: LIGO, GW150914, rivelatore H1, filtrati. Modello all'ordine più basso e incertezza stimata: non è l'analisi di LIGO.",
        confronto: "LIGO: massa chirp di circa 28 soli alla sorgente, circa 30 osservata da qui per via dell'espansione.",
        dati,
        iniziali: par.clone(),
        par,
        modello: Box::new(ligo),
        asse_x: "secondi",
        asse_y: "deformazione, 10^-21",
        x_log: false,
        y_invertito: false,
        solo_punti: true,
        seconda: Some((teorica, "calcolo di relatività numerica")),
        risultato: |v, s| {
            vec![
                ("Massa chirp".to_owned(), format!("{} soli", con_errore(v[0], voce(s, 0), 1))),
                ("Fusione".to_owned(), format!("{} s", con_errore(v[1], voce(s, 1), 4))),
            ]
        },
        ricerca: Some(cerca_ligo),
        sigma: None,
        modo: "a mano",
        mostra_confronto: false,
        appunti: Vec::new(),
    }
}

fn missione_pulsar() -> Missione {
    // Un solo punto: lo scarto accumulato al 2016 che corrisponde al decadimento misurato.
    let t = (2016.0 - ANNO_INIZIO_HT) * 3.155_76e7;
    let y = 0.5 * DP_HT / PERIODO_HT * t * t;
    let err = (y * DP_HT_ERRORE / DP_HT).abs();
    let par = vec![p("massa della pulsar, soli", 0.9, 0.5, 3.0)];
    Missione {
        nome: "Pulsar binaria",
        titolo: "L'orbita che perde energia".to_owned(),
        nota: "Due stelle di neutroni perdono energia in onde gravitazionali. Il punto è il ritardo accumulato: quale massa lo spiega?",
        fonte: "Periodo, eccentricità e decadimento: Weisberg e Huang 2016. Il punto è derivato dal decadimento pubblicato.",
        confronto: "Massa della pulsar 1,438 soli. Il decadimento misurato è 0,9983 ± 0,0016 volte la previsione di Einstein.",
        dati: vec![Misura { x: 2016.0, y, err, attiva: true }],
        iniziali: par.clone(),
        par,
        modello: Box::new(hulse_taylor),
        asse_x: "anno",
        asse_y: "ritardo accumulato, secondi",
        x_log: false,
        y_invertito: false,
        solo_punti: false,
        seconda: None,
        risultato: |v, s| {
            let dp = colossi_fisica::missioni::dp_dt_relativita(v[0], MASSA_C_HT, PERIODO_HT, colossi_fisica::missioni::ECCENTRICITA_HT);
            vec![
                ("Massa della pulsar".to_owned(), format!("{} soli", con_errore(v[0], voce(s, 0), 3))),
                ("Il periodo si accorcia di".to_owned(), format!("{} microsecondi all'anno", decimale(dp.abs() * 3.155_76e7 * 1e6))),
            ]
        },
        ricerca: None,
        sigma: None,
        modo: "a mano",
        mostra_confronto: false,
        appunti: Vec::new(),
    }
}

pub const GALASSIE: [&str; 6] = ["NGC3198", "NGC2403", "NGC6503", "DDO154", "UGC02885", "NGC7814"];

pub fn galassia(nome: &str) -> Missione {
    let mut dati = Vec::new();
    let mut righe = Vec::new();
    for riga in include_str!("../dati/sparc.csv").lines() {
        let Some((etichetta, resto)) = riga.split_once(',') else { continue };
        if etichetta != nome {
            continue;
        }
        let v = numeri(resto);
        dati.push(Misura { x: v[0], y: v[1], err: v[2].max(1.0), attiva: true });
        righe.push(RigaRotazione { r: v[0], gas: v[3], disco: v[4], bulbo: v[5] });
    }
    let par = vec![
        p("massa per luce, disco", 0.5, 0.05, 3.0),
        p("massa per luce, bulbo", 0.7, 0.05, 3.0),
        p("alone, velocità km/s", 30.0, 0.0, 400.0),
        p("alone, raggio kpc", 2.0, 0.2, 40.0),
    ];
    Missione {
        nome: "Galassie",
        titolo: format!("La massa che manca: {nome}"),
        nota: "Le croci sono le velocità misurate. Abbassa l'alone a zero: con la sola materia visibile la curva non basta.",
        fonte: "Dati: SPARC, Lelli, McGaugh e Schombert 2016. Alone a densità con nucleo.",
        confronto: "Per i dischi ci si aspetta una massa per luce attorno a 0,5. Con quella, la materia visibile non spiega le parti esterne.",
        dati,
        iniziali: par.clone(),
        par,
        modello: Box::new(move |v, r| rotazione(&righe, v, r)),
        asse_x: "distanza dal centro, kpc",
        asse_y: "velocità, km/s",
        x_log: false,
        y_invertito: false,
        solo_punti: false,
        seconda: None,
        risultato: |v, s| {
            vec![
                ("Massa per luce del disco".to_owned(), con_errore(v[0], voce(s, 0), 2)),
                ("Velocità data dall'alone".to_owned(), format!("{} km/s", con_errore(v[2], voce(s, 2), 0))),
                ("Raggio dell'alone".to_owned(), format!("{} kpc", con_errore(v[3], voce(s, 3), 1))),
            ]
        },
        ricerca: None,
        sigma: None,
        modo: "a mano",
        mostra_confronto: false,
        appunti: Vec::new(),
    }
}

fn missione_supernove() -> Missione {
    let dati = include_str!("../dati/pantheon.csv")
        .lines()
        .map(|riga| {
            let v = numeri(riga);
            Misura { x: v[0], y: v[1], err: v[2], attiva: true }
        })
        .collect();
    let par = vec![p("materia", 1.0, 0.0, 1.5), fissa(p("energia oscura", 0.0, 0.0, 1.5)), p("costante di Hubble", 60.0, 50.0, 90.0)];
    Missione {
        nome: "Supernovae",
        titolo: "L'espansione che accelera".to_owned(),
        nota: "Parti senza energia oscura: le supernovae lontane non tornano. Sblocca il parametro e guarda quanta ne serve.",
        fonte: "Dati: Pantheon+ e SH0ES, Scolnic e Brout 2022. Qui solo gli errori sulla diagonale.",
        confronto: "Pantheon+, universo piatto: materia 0,334 ± 0,018, il resto energia oscura. Hubble circa 73 con la calibrazione SH0ES.",
        dati,
        iniziali: par.clone(),
        par,
        modello: Box::new(supernova),
        asse_x: "redshift",
        asse_y: "modulo di distanza",
        x_log: true,
        y_invertito: false,
        solo_punti: true,
        seconda: None,
        risultato: |v, s| {
            vec![
                ("Materia".to_owned(), con_errore(v[0], voce(s, 0), 3)),
                ("Energia oscura".to_owned(), con_errore(v[1], voce(s, 1), 3)),
                ("Hubble, km/s/Mpc".to_owned(), con_errore(v[2], voce(s, 2), 1)),
            ]
        },
        ricerca: None,
        sigma: None,
        modo: "a mano",
        mostra_confronto: false,
        appunti: Vec::new(),
    }
}

fn missione_gaia() -> Missione {
    let dati = include_str!("../dati/gaia.csv")
        .lines()
        .map(|riga| {
            let v = numeri(riga);
            let (c, m) = (v[0], v[1]);
            let vicina = (m - (1.9 + 3.3 * c)).abs() < 1.6 && c > 0.5 && c < 3.0;
            Misura { x: c, y: m, err: 0.35, attiva: vicina }
        })
        .collect();
    let par = vec![p("punto di partenza", 3.0, -3.0, 8.0), p("pendenza", 1.5, 0.0, 8.0), p("curvatura", 0.0, -2.0, 2.0)];
    Missione {
        nome: "Gaia",
        titolo: "Il diagramma colore-luminosità".to_owned(),
        nota: "Ogni punto è una stella vicina. La striscia diagonale sono le stelle che bruciano idrogeno; in alto le giganti, in basso le nane bianche.",
        fonte: "Dati: Gaia DR3, ESA, stelle entro 50 parsec. I punti grigi sono esclusi dalla curva.",
        confronto: "Il Sole ha colore BP-RP 0,82 e magnitudine assoluta G 4,67: dovrebbe stare sulla curva.",
        dati,
        iniziali: par.clone(),
        par,
        modello: Box::new(sequenza),
        asse_x: "colore BP-RP: blu a sinistra, rosso a destra",
        asse_y: "magnitudine assoluta G: luminose in alto",
        x_log: false,
        y_invertito: true,
        solo_punti: true,
        seconda: None,
        risultato: |v, _| {
            vec![("Una stella come il Sole (0,82)".to_owned(), format!("magnitudine {}", decimale(sequenza(v, 0.82).abs())))]
        },
        ricerca: None,
        sigma: None,
        modo: "a mano",
        mostra_confronto: false,
        appunti: Vec::new(),
    }
}

/// Dati Planck binned: multipolo, D_l, errore, curva migliore.
pub fn righe_planck() -> Vec<[f64; 4]> {
    include_str!("../dati/planck_tt.csv")
        .lines()
        .map(|riga| {
            let v = numeri(riga);
            [v[0], v[1], v[2], v[3]]
        })
        .collect()
}

fn curvatura_da_picco(picco: f64) -> f64 {
    let (mut a, mut b) = (-0.3, 0.3);
    let f = |k: f64| Cosmologia { curvatura: k, ..PLANCK }.picco(1) - picco;
    if f(a) * f(b) > 0.0 {
        return f64::NAN;
    }
    for _ in 0..50 {
        let m = 0.5 * (a + b);
        if f(a) * f(m) <= 0.0 {
            b = m;
        } else {
            a = m;
        }
    }
    0.5 * (a + b)
}

fn missione_planck() -> Missione {
    let mut dati = Vec::new();
    let mut teorica = Vec::new();
    for r in righe_planck() {
        dati.push(Misura { x: r[0], y: r[1], err: r[2], attiva: r[0] > 160.0 && r[0] < 290.0 });
        teorica.push([r[0], r[3]]);
    }
    let par = vec![p("altezza", 5000.0, 2000.0, 7000.0), p("posizione del primo picco", 300.0, 100.0, 400.0), p("larghezza", 150.0, 50.0, 300.0)];
    Missione {
        nome: "Planck",
        titolo: "Di che forma è lo spazio".to_owned(),
        nota: "Queste sono le macchie del cielo misurate da Planck. La posizione del primo picco dice se lo spazio è curvo.",
        fonte: "Dati: Planck 2018, spettro TT a intervalli.",
        confronto: "Planck: primo picco vicino al multipolo 220, spazio piatto entro pochi millesimi.",
        dati,
        iniziali: par.clone(),
        par,
        modello: Box::new(parabola),
        asse_x: "multipolo: macchie più piccole verso destra",
        asse_y: "contrasto, microkelvin quadri",
        x_log: false,
        y_invertito: false,
        solo_punti: false,
        seconda: Some((teorica, "miglior modello di Planck")),
        risultato: |v, s| {
            let k = curvatura_da_picco(v[1]);
            vec![
                ("Primo picco".to_owned(), con_errore(v[1], voce(s, 1), 1)),
                ("Curvatura che servirebbe".to_owned(), if k.is_nan() { "fuori scala".to_owned() } else { format!("{k:+.3}").replace('.', ",") }),
            ]
        },
        ricerca: None,
        sigma: None,
        modo: "a mano",
        mostra_confronto: false,
        appunti: Vec::new(),
    }
}

pub fn tutte() -> Vec<Missione> {
    vec![missione_ligo(), missione_pulsar(), galassia(GALASSIE[0]), missione_supernove(), missione_gaia(), missione_planck()]
}

impl Missione {
    fn valori(&self) -> Vec<f64> {
        self.par.iter().map(|q| q.valore).collect()
    }

    pub fn chi_ridotto(&self) -> f64 {
        let liberi = self.par.iter().filter(|q| q.libero).count();
        let n = self.dati.iter().filter(|d| d.attiva).count();
        chi2(&self.modello, &self.dati, &self.valori()) / (n as f64 - liberi as f64).max(1.0)
    }

    pub fn calcola(&mut self) {
        if let Some(r) = self.ricerca {
            r(&self.dati, &mut self.par);
        }
        adatta(&self.modello, &self.dati, &mut self.par);
        self.sigma = incertezze(&self.modello, &self.dati, &self.par);
        self.modo = "col calcolo";
    }

    /// Vero dopo che si e' premuto "Calcola la migliore".
    pub fn calcolata(&self) -> bool {
        self.modo == "col calcolo"
    }

    /// Torna ai valori iniziali e ai soli cursori.
    pub fn riparti(&mut self) {
        self.par = self.iniziali.clone();
        self.sigma = None;
        self.modo = "a mano";
        self.mostra_confronto = false;
    }
}

#[derive(Clone, Copy)]
struct Scala {
    piano: Rect,
    x: (f64, f64),
    y: (f64, f64),
    log: bool,
    invertito: bool,
}

impl Scala {
    fn nuova(m: &Missione, piano: Rect) -> Self {
        let tx = |v: f64| if m.x_log { v.log10() } else { v };
        let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for d in &m.dati {
            x0 = x0.min(tx(d.x));
            x1 = x1.max(tx(d.x));
            y0 = y0.min(d.y - d.err.min((d.y).abs() * 0.5));
            y1 = y1.max(d.y + d.err.min((d.y).abs() * 0.5));
        }
        if m.dati.len() == 1 {
            // Un punto solo: l'asse copre la storia, dall'inizio al punto.
            x0 = ANNO_INIZIO_HT;
            x1 = 2020.0;
            y0 *= 1.6;
            y1 = 0.0;
        }
        let (mx, my) = ((x1 - x0) * 0.04, (y1 - y0).max(1e-9) * 0.08);
        Self { piano, x: (x0 - mx, x1 + mx), y: (y0 - my, y1 + my), log: m.x_log, invertito: m.y_invertito }
    }

    fn a_schermo(&self, x: f64, y: f64) -> Pos2 {
        let vx = if self.log { x.max(1e-9).log10() } else { x };
        let u = ((vx - self.x.0) / (self.x.1 - self.x.0)) as f32;
        let mut v = ((y - self.y.0) / (self.y.1 - self.y.0)) as f32;
        if self.invertito {
            v = 1.0 - v;
        }
        Pos2::new(self.piano.left() + self.piano.width() * u, self.piano.bottom() - self.piano.height() * v)
    }

    fn da_x(&self, k: f64) -> f64 {
        let v = self.x.0 + (self.x.1 - self.x.0) * k;
        if self.log {
            10f64.powf(v)
        } else {
            v
        }
    }
}

pub fn area_centrale(area: Rect) -> Rect {
    Rect::from_min_max(area.min + Vec2::new(62.0, 70.0), area.max - Vec2::new(22.0, 40.0))
}

pub fn centrale(ui: &mut egui::Ui, area: Rect, m: &mut Missione) {
    let r = ui.interact(area, ui.id().with("missione"), Sense::click());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let piano = area_centrale(area);
    p.rect_stroke(piano, 2.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let s = Scala::nuova(m, piano);

    let f = FontId::monospace(10.0);
    for k in 0..=6 {
        let u = f64::from(k) / 6.0;
        let x = piano.left() + piano.width() * u as f32;
        p.line_segment([Pos2::new(x, piano.top()), Pos2::new(x, piano.bottom())], Stroke::new(1.0, LINEA));
        let valore = s.da_x(u);
        let testo = if valore.abs() >= 100.0 { format!("{valore:.0}") } else { format!("{valore:.3}").replace('.', ",") };
        p.text(Pos2::new(x, piano.bottom() + 3.0), Align2::CENTER_TOP, testo, f.clone(), GRIGIO);
        let vy = s.y.0 + (s.y.1 - s.y.0) * u;
        let y = s.a_schermo(s.da_x(0.0), vy).y;
        p.line_segment([Pos2::new(piano.left(), y), Pos2::new(piano.right(), y)], Stroke::new(1.0, LINEA));
        let etichetta = if vy.abs() >= 100.0 { format!("{vy:.0}") } else { format!("{vy:.2}").replace('.', ",") };
        p.text(Pos2::new(piano.left() - 4.0, y), Align2::RIGHT_CENTER, etichetta, f.clone(), GRIGIO);
    }
    p.text(piano.right_bottom() + Vec2::new(0.0, 16.0), Align2::RIGHT_TOP, m.asse_x, f.clone(), GRIGIO);
    p.text(piano.left_top() + Vec2::new(6.0, 4.0), Align2::LEFT_TOP, m.asse_y, f.clone(), GRIGIO);

    if let Some((linea, nome)) = &m.seconda {
        let passo = (linea.len() / 1500).max(1);
        let punti: Vec<Pos2> = linea.iter().step_by(passo).map(|q| s.a_schermo(q[0], q[1])).collect();
        p.with_clip_rect(piano).extend(Shape::dashed_line(&punti, Stroke::new(1.2, VERDE), 5.0, 3.0));
        p.text(piano.right_top() + Vec2::new(-6.0, 4.0), Align2::RIGHT_TOP, format!("- - {nome}"), f.clone(), VERDE);
    }

    let denso = m.dati.len() > 600;
    let unita_linea = m.nome == "Onde LIGO";
    if unita_linea {
        let punti: Vec<Pos2> = m.dati.iter().map(|d| s.a_schermo(d.x, d.y)).collect();
        p.add(Shape::line(punti, Stroke::new(1.2, CHIARO)));
    } else {
        for d in &m.dati {
            let q = s.a_schermo(d.x, d.y);
            let tinta = if d.attiva { CHIARO } else { GRIGIO.gamma_multiply(0.5) };
            if m.solo_punti && denso {
                p.rect_filled(Rect::from_center_size(q, Vec2::splat(2.0)), 0.0, tinta);
            } else {
                let alto = s.a_schermo(d.x, d.y + d.err).y;
                let e = (q.y - alto).abs().max(2.0);
                p.line_segment([q - Vec2::new(0.0, e), q + Vec2::new(0.0, e)], Stroke::new(1.2, tinta));
                p.line_segment([q - Vec2::new(3.0, 0.0), q + Vec2::new(3.0, 0.0)], Stroke::new(1.2, tinta));
            }
        }
    }

    let v = m.valori();
    let punti: Vec<Pos2> = (0..=PUNTI_CURVA)
        .map(|i| {
            let x = s.da_x(i as f64 / PUNTI_CURVA as f64);
            s.a_schermo(x, (m.modello)(&v, x))
        })
        .collect();
    p.with_clip_rect(piano).add(Shape::line(punti, Stroke::new(2.2, ARANCIO)));

    if r.clicked() && !unita_linea {
        if let Some(q) = r.interact_pointer_pos() {
            let vicino = m
                .dati
                .iter()
                .enumerate()
                .map(|(i, d)| (i, (s.a_schermo(d.x, d.y) - q).length()))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((i, d)) = vicino.filter(|c| c.1 < PRESA) {
                let _ = d;
                m.dati[i].attiva = !m.dati[i].attiva;
            }
        }
    }

    let t = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, m.titolo.as_str(), FontId::proportional(14.0), CHIARO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, m.nota, t, CIANO);
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.end_row();
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

pub fn colonna(ui: &mut egui::Ui, m: &mut Missione) {
    let chi = m.chi_ridotto();
    let tinta = if chi < CHI_BUONO { VERDE } else if chi < 30.0 { GIALLO } else { Color32::from_rgb(255, 105, 95) };
    ui.horizontal(|ui| {
        ui.label(RichText::new("Scarto dalle misure").color(GRIGIO));
        let testo = if chi < 1000.0 { decimale(chi) } else { format!("{chi:.1e}") };
        ui.label(RichText::new(testo).monospace().size(22.0).strong().color(tinta));
        ui.label(RichText::new("vicino a 1 è buono").small().color(GRIGIO));
    });

    titoletto(ui, "IL MODELLO");
    let mut cambiato = false;
    for q in &mut m.par {
        ui.horizontal(|ui| {
            let (mn, mx) = (q.min, q.max);
            cambiato |= ui.add(egui::Slider::new(&mut q.valore, mn..=mx).text(q.nome).custom_formatter(|v, _| {
                if v.abs() >= 100.0 { format!("{v:.1}") } else { format!("{v:.3}") }
            })).changed();
        });
        if m.nome == "Supernovae" && q.nome == "energia oscura" {
            cambiato |= ui.checkbox(&mut q.libero, "Può variare nel calcolo").changed();
        }
    }
    if cambiato {
        m.sigma = None;
        m.modo = "a mano";
    }
    ui.horizontal(|ui| {
        let calcola = ui.button(RichText::new("Calcola la migliore").strong());
        crate::visite::segna(ui, "calcola_missione", &calcola);
        if calcola.clicked() {
            m.calcola();
        }
        if ui.small_button("Riparti").clicked() {
            m.riparti();
        }
    });

    titoletto(ui, "IL RISULTATO");
    let v = m.valori();
    let voci = (m.risultato)(&v, m.sigma.as_deref());
    egui::Grid::new("risultato_missione").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        for (a, b) in &voci {
            riga(ui, a, b.clone());
        }
    });
    ui.label(RichText::new(format!("Metodo: {}. Misure attive: {}.", m.modo, m.dati.iter().filter(|d| d.attiva).count())).color(GRIGIO));
    ui.horizontal(|ui| {
        ui.toggle_value(&mut m.mostra_confronto, "Confronta con la pubblicazione");
        if ui.small_button("Annota").clicked() {
            let testo = voci.iter().map(|(a, b)| format!("{a}: {b}")).collect::<Vec<_>>().join("; ");
            m.appunti.push(format!("{}. {testo}", m.modo));
        }
    });
    if m.mostra_confronto {
        ui.label(RichText::new(m.confronto).color(CIANO));
    }
    if !m.appunti.is_empty() {
        titoletto(ui, "QUADERNO");
        for a in &m.appunti {
            ui.label(RichText::new(a).small().color(CHIARO));
        }
    }
    ui.add_space(6.0);
    ui.label(RichText::new(m.fonte).small().color(GRIGIO));
}

pub fn pannello_colonna(ui: &mut egui::Ui, corpo: impl FnOnce(&mut egui::Ui)) {
    egui::Panel::right("colonna_missione")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, corpo);
        });
}

#[cfg(test)]
mod test {
    use super::*;

    fn trova(nome: &str) -> Missione {
        tutte().into_iter().find(|m| m.nome == nome).unwrap()
    }

    #[test]
    fn ligo_ritrova_la_massa_del_segnale() {
        let mut m = trova("Onde LIGO");
        m.calcola();
        assert!(m.par[0].valore > 26.0 && m.par[0].valore < 39.0, "{:?}", m.par[0]);
    }

    #[test]
    fn la_pulsar_ritrova_la_sua_massa() {
        let mut m = trova("Pulsar binaria");
        m.calcola();
        assert!((m.par[0].valore - 1.438).abs() < 0.03, "{:?}", m.par[0]);
    }

    #[test]
    fn senza_alone_la_galassia_non_torna() {
        let mut m = trova("Galassie");
        m.calcola();
        let con = m.chi_ridotto();
        m.par[2].valore = 0.0;
        m.par[2].libero = false;
        m.par[0].valore = 0.5;
        m.par[0].libero = false;
        assert!(m.chi_ridotto() > 3.0 * con, "{} {con}", m.chi_ridotto());
    }

    #[test]
    fn le_supernovae_chiedono_energia_oscura() {
        let mut m = trova("Supernovae");
        m.calcola();
        let senza = m.chi_ridotto();
        m.par[1].libero = true;
        m.calcola();
        let con = m.chi_ridotto();
        println!("senza {senza} con {con} {:?}", m.par);
        assert!(con < senza);
        assert!(m.par[1].valore > 0.3, "{:?}", m.par[1]);
        assert!((m.par[2].valore - 73.0).abs() < 2.0, "{:?}", m.par[2]);
    }

    #[test]
    fn il_sole_sta_sulla_sequenza_di_gaia() {
        let mut m = trova("Gaia");
        m.calcola();
        let mg = sequenza(&m.valori(), 0.82);
        assert!((mg - 4.67).abs() < 0.8, "{mg}");
    }

    #[test]
    fn il_primo_picco_di_planck() {
        let mut m = trova("Planck");
        m.calcola();
        assert!((m.par[1].valore - 220.0).abs() < 12.0, "{:?}", m.par[1]);
        let k = curvatura_da_picco(m.par[1].valore);
        assert!(k.abs() < 0.15, "{k}");
    }

    #[test]
    fn ogni_galassia_ha_i_suoi_dati() {
        for nome in GALASSIE {
            assert!(galassia(nome).dati.len() > 5, "{nome}");
        }
    }
}
