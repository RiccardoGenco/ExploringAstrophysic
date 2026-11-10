//! Osservatorio: si ricava un risultato da misure vere e lo si confronta con
//! quello pubblicato. Prima a mano, poi col calcolo.

use crate::ammasso::{self, Ammasso};
use crate::formato::{decimale, grande, lunghezza};
use crate::missioni::{self, Missione};
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use colossi_fisica::costanti::UA;
use colossi_fisica::osservatorio::{
    leggi_csv, s2_ghez_2008, Dati, Incertezze, Orbita, Pubblicato, GHEZ_VZ_FISSA, GHEZ_VZ_LIBERA,
    INIZIO,
};
use egui::{Align2, Color32, FontId, Frame, Margin, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f64::consts::TAU;
use std::fmt::Write as _;

const LARGHEZZA: f32 = 430.0;
const QUOTA_CIELO: f32 = 0.70;
const PRESA: f32 = 11.0;
const PUNTI_ORBITA: usize = 500;
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);
const ARANCIO: Color32 = Color32::from_rgb(255, 160, 90);
const CHI_BUONO: f64 = 2.0;

struct Voce {
    modo: &'static str,
    misure: usize,
    orbita: Orbita,
    chi2r: f64,
    errore_massa: Option<f64>,
    vz_fissa: bool,
}

/// Cosa il banco chiede al resto del programma.
pub struct SulBanco {
    pub masse: f64,
    pub a_ua: f64,
    pub e: f64,
}

/// Quale missione e' aperta.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Sezione {
    Orbita,
    Ammasso,
    Altra(usize),
}

pub struct Osservatorio {
    pub sezione: Sezione,
    pub missioni: Vec<Missione>,
    pub ammasso: Ammasso,
    galassia: usize,
    pub dati: Dati,
    nome_dati: String,
    pub orbita: Orbita,
    vz_fissa: bool,
    pub incertezze: Option<Incertezze>,
    modo: &'static str,
    confronto: bool,
    quaderno: Vec<Voce>,
    percorso: String,
    avviso: Option<(String, bool)>,
}

impl Default for Osservatorio {
    fn default() -> Self {
        Self {
            sezione: Sezione::Orbita,
            missioni: missioni::tutte(),
            ammasso: Ammasso::default(),
            galassia: 0,
            dati: s2_ghez_2008(),
            nome_dati: "Stella S2, misure Keck e VLT".to_owned(),
            orbita: INIZIO,
            vz_fissa: true,
            incertezze: None,
            modo: "a mano",
            confronto: false,
            quaderno: Vec::new(),
            percorso: String::new(),
            avviso: None,
        }
    }
}

impl Osservatorio {
    pub fn calcola(&mut self) {
        self.orbita = self.dati.adatta(&self.orbita, self.vz_fissa);
        self.incertezze = self.dati.incertezze(&self.orbita, self.vz_fissa);
        self.modo = "col calcolo";
    }

    pub fn riparti(&mut self) {
        self.orbita = INIZIO;
        self.incertezze = None;
        self.modo = "a mano";
        self.confronto = false;
    }

    fn importa(&mut self, percorso: &str) {
        let esito = std::fs::read_to_string(percorso)
            .map_err(|e| format!("non riesco a leggere il file: {e}"))
            .and_then(|testo| leggi_csv(&testo));
        match esito {
            Ok(dati) => {
                let n = dati.numero();
                self.dati = dati;
                self.nome_dati = format!("Importati da {percorso}");
                self.riparti();
                self.avviso = Some((format!("Caricate {n} misure."), true));
            }
            Err(e) => self.avviso = Some((format!("Rifiutato: {e}"), false)),
        }
    }

    /// Vero se lo scarto e' sotto `soglia` con i soli cursori.
    pub fn a_mano_vicino(&self, soglia: f64) -> bool {
        self.modo == "a mano" && self.sezione == Sezione::Orbita && self.dati.chi2_ridotto(&self.orbita) < soglia
    }

    pub fn confronto_aperto(&self) -> bool {
        self.confronto && self.incertezze.is_some()
    }

    fn pubblicato(&self) -> &'static Pubblicato {
        if self.vz_fissa {
            &GHEZ_VZ_FISSA
        } else {
            &GHEZ_VZ_LIBERA
        }
    }

    fn salva(&mut self, nome: &str, testo: &str) {
        let esito = std::env::current_dir().and_then(|c| {
            let cartella = c.join("misure");
            std::fs::create_dir_all(&cartella)?;
            let file = cartella.join(nome);
            std::fs::write(&file, testo)?;
            Ok(file)
        });
        self.avviso = Some(match esito {
            Ok(file) => (format!("Salvato in {}", file.display()), true),
            Err(e) => (format!("Salvataggio non riuscito: {e}"), false),
        });
    }

    fn csv_quaderno(&self) -> String {
        let mut s = String::from("modo,misure,massa_soli,errore_massa_soli,r0_kpc,a_mas,e,p_anni,chi2_ridotto,vz_fissa\n");
        for v in &self.quaderno {
            let o = &v.orbita;
            let _ = writeln!(
                s,
                "{},{},{},{},{},{},{},{},{},{}",
                v.modo, v.misure, o.massa(), v.errore_massa.unwrap_or(f64::NAN), o.r0, o.a, o.e, o.p, v.chi2r, v.vz_fissa
            );
        }
        s
    }

    /// Le due aree del grafico dentro lo spazio centrale.
    fn piani(area: Rect) -> (Rect, Rect) {
        let divisione = area.top() + area.height() * QUOTA_CIELO;
        let cielo = Rect::from_min_max(area.min + Vec2::new(52.0, 62.0), Pos2::new(area.right() - 22.0, divisione - 26.0));
        let velocita = Rect::from_min_max(
            Pos2::new(area.left() + 52.0, divisione + 18.0),
            area.max - Vec2::new(22.0, 30.0),
        );
        (cielo, velocita)
    }

    #[cfg(test)]
    pub fn schermo_prima_posizione(&self, area: Rect) -> Pos2 {
        let (cielo, _) = Self::piani(area);
        let v = VistaCielo::nuova(cielo, self);
        let p = self.dati.posizioni[0];
        v.a_schermo(p.est, p.nord)
    }
}

struct VistaCielo {
    centro: Pos2,
    px: f32,
    centro_est: f64,
    centro_nord: f64,
}

impl VistaCielo {
    fn nuova(piano: Rect, o: &Osservatorio) -> Self {
        let (mut est_min, mut est_max, mut nord_min, mut nord_max) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
        for p in &o.dati.posizioni {
            est_min = est_min.min(p.est);
            est_max = est_max.max(p.est);
            nord_min = nord_min.min(p.nord);
            nord_max = nord_max.max(p.nord);
        }
        est_min = est_min.min(0.0) - 25.0;
        est_max = est_max.max(0.0) + 25.0;
        nord_min = nord_min.min(0.0) - 25.0;
        nord_max = nord_max.max(0.0) + 25.0;
        let px = (f64::from(piano.width()) / (est_max - est_min))
            .min(f64::from(piano.height()) / (nord_max - nord_min)) as f32;
        Self {
            centro: piano.center(),
            px,
            centro_est: (est_min + est_max) * 0.5,
            centro_nord: (nord_min + nord_max) * 0.5,
        }
    }

    /// Come in cielo: il nord in alto e l'est a sinistra.
    fn a_schermo(&self, est: f64, nord: f64) -> Pos2 {
        self.centro + Vec2::new(-((est - self.centro_est) as f32) * self.px, -((nord - self.centro_nord) as f32) * self.px)
    }
}

/// Pulsanti per scegliere la missione, in cima alla colonna.
fn selettore(ui: &mut egui::Ui, o: &mut Osservatorio) {
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut o.sezione, Sezione::Orbita, "S2");
        ui.selectable_value(&mut o.sezione, Sezione::Ammasso, "Stelle centrali");
        for i in 0..o.missioni.len() {
            ui.selectable_value(&mut o.sezione, Sezione::Altra(i), o.missioni[i].nome);
        }
    });
    ui.separator();
}

fn altra_sezione(ui: &mut egui::Ui, o: &mut Osservatorio) {
    let sezione = o.sezione;
    missioni::pannello_colonna(ui, |ui| {
        selettore(ui, o);
        match sezione {
        Sezione::Ammasso => ammasso::colonna(ui, &mut o.ammasso),
        Sezione::Altra(i) => {
            if o.missioni[i].nome == "Galassie" {
                let mut scelta = None;
                ui.horizontal_wrapped(|ui| {
                    for (k, nome) in missioni::GALASSIE.iter().enumerate() {
                        if ui.selectable_label(o.galassia == k, *nome).clicked() {
                            scelta = Some(k);
                        }
                    }
                });
                if let Some(k) = scelta {
                    o.galassia = k;
                    o.missioni[i] = missioni::galassia(missioni::GALASSIE[k]);
                }
            }
            missioni::colonna(ui, &mut o.missioni[i]);
        }
        Sezione::Orbita => {}
        }
    });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        match sezione {
            Sezione::Ammasso => ammasso::centrale(ui, area, &mut o.ammasso),
            Sezione::Altra(i) => missioni::centrale(ui, area, &mut o.missioni[i]),
            Sezione::Orbita => {}
        }
    });
}

/// Restituisce l'orbita da mettere sul banco del buco nero, se richiesto.
pub fn mostra(ui: &mut egui::Ui, o: &mut Osservatorio) -> Option<SulBanco> {
    if o.sezione != Sezione::Orbita {
        altra_sezione(ui, o);
        return None;
    }
    let cadute = ui.input(|i| i.raw.dropped_files.clone());
    if let Some(percorso) = cadute.iter().find_map(|f| f.path.as_ref()) {
        let testo = percorso.display().to_string();
        o.importa(&testo);
        o.percorso = testo;
    }

    let mut sul_banco = None;
    egui::Panel::right("colonna_osservatorio")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| sul_banco = strumenti(ui, o));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        grafici(ui, area, o);
    });
    sul_banco
}

fn croce(p: &Painter, c: Pos2, ex: f32, ey: f32, tinta: Color32) {
    p.line_segment([c - Vec2::new(ex, 0.0), c + Vec2::new(ex, 0.0)], Stroke::new(1.2, tinta));
    p.line_segment([c - Vec2::new(0.0, ey), c + Vec2::new(0.0, ey)], Stroke::new(1.2, tinta));
}

fn grafici(ui: &mut egui::Ui, area: Rect, o: &mut Osservatorio) {
    let risposta = ui.interact(area, ui.id().with("osservatorio"), Sense::click());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let (cielo, piano_velocita) = Osservatorio::piani(area);

    let vista = VistaCielo::nuova(cielo, o);
    disegna_cielo(&p, cielo, &vista, o);
    let scala_v = disegna_velocita(&p, piano_velocita, o);

    if risposta.clicked() {
        if let Some(q) = risposta.interact_pointer_pos() {
            togli_punto(q, &vista, piano_velocita, scala_v, o);
        }
    }

    let testo = FontId::proportional(13.0);
    p.text(
        area.left_top() + Vec2::new(14.0, 12.0),
        Align2::LEFT_TOP,
        "Muovi i cursori finché la curva passa per le misure. Clic su una misura per toglierla o rimetterla.",
        testo.clone(),
        GRIGIO,
    );
    p.text(
        area.left_top() + Vec2::new(14.0, 32.0),
        Align2::LEFT_TOP,
        "Ogni croce è una misura vera, con il suo errore. La massa che regge quest'orbita è quella del buco nero al centro.",
        testo,
        CIANO,
    );
}

fn disegna_cielo(p: &Painter, piano: Rect, v: &VistaCielo, o: &Osservatorio) {
    p.rect_stroke(piano, 2.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let f = FontId::monospace(10.0);
    // Griglia ogni 50 mas.
    let mut q = -300;
    while q <= 300 {
        let c = v.a_schermo(f64::from(q), f64::from(q));
        if c.x > piano.left() && c.x < piano.right() {
            p.line_segment([Pos2::new(c.x, piano.top()), Pos2::new(c.x, piano.bottom())], Stroke::new(1.0, LINEA));
            p.text(Pos2::new(c.x, piano.bottom() + 3.0), Align2::CENTER_TOP, format!("{q}"), f.clone(), GRIGIO);
        }
        if c.y > piano.top() && c.y < piano.bottom() {
            p.line_segment([Pos2::new(piano.left(), c.y), Pos2::new(piano.right(), c.y)], Stroke::new(1.0, LINEA));
            p.text(Pos2::new(piano.left() - 4.0, c.y), Align2::RIGHT_CENTER, format!("{q}"), f.clone(), GRIGIO);
        }
        q += 50;
    }
    p.text(piano.left_top() + Vec2::new(6.0, 4.0), Align2::LEFT_TOP, "nord in alto, est a sinistra, millesimi di secondo d'arco", f, GRIGIO);

    // L'orbita.
    // Passi uguali in anomalia eccentrica: fitti dove la stella corre.
    let punti: Vec<Pos2> = (0..=PUNTI_ORBITA)
        .map(|i| {
            let angolo = TAU * i as f64 / PUNTI_ORBITA as f64;
            let media = angolo - o.orbita.e * angolo.sin();
            let (e, n) = o.orbita.posizione(o.orbita.t0 + o.orbita.p * media / TAU);
            v.a_schermo(e, n)
        })
        .collect();
    p.add(Shape::line(punti, Stroke::new(2.2, ARANCIO)));

    // Il centro attorno a cui gira.
    let centro = v.a_schermo(o.orbita.x0, o.orbita.y0);
    p.circle_filled(centro, 5.0, Color32::BLACK);
    p.circle_stroke(centro, 5.0, Stroke::new(1.5, CIANO));

    for m in &o.dati.posizioni {
        let c = v.a_schermo(m.est, m.nord);
        if m.attiva {
            let (e, n) = o.orbita.posizione(m.t);
            p.line_segment([c, v.a_schermo(e, n)], Stroke::new(0.8, ROSSO.gamma_multiply(0.75)));
            croce(p, c, (m.err_est as f32 * v.px).max(2.5), (m.err_nord as f32 * v.px).max(2.5), CHIARO);
        } else {
            p.circle_stroke(c, 4.0, Stroke::new(1.0, GRIGIO));
        }
    }
}

const T_INIZIO: f64 = 1995.0;
const T_FINE: f64 = 2011.0;

#[derive(Clone, Copy)]
struct ScalaVelocita {
    piano: Rect,
    basso: f64,
    alto: f64,
}

impl ScalaVelocita {
    fn a_schermo(&self, t: f64, v: f64) -> Pos2 {
        Pos2::new(
            self.piano.left() + self.piano.width() * ((t - T_INIZIO) / (T_FINE - T_INIZIO)) as f32,
            self.piano.bottom() - self.piano.height() * ((v - self.basso) / (self.alto - self.basso)) as f32,
        )
    }
}

fn disegna_velocita(p: &Painter, piano: Rect, o: &Osservatorio) -> ScalaVelocita {
    let mut misure: Vec<f64> = o.dati.velocita.iter().map(|m| m.v).collect();
    misure.extend([-1000.0, 1000.0]);
    let basso = misure.iter().copied().fold(f64::MAX, f64::min).min(-1000.0) - 400.0;
    let alto = misure.iter().copied().fold(f64::MIN, f64::max).max(1000.0) + 400.0;
    let s = ScalaVelocita { piano, basso, alto };

    p.rect_stroke(piano, 2.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let f = FontId::monospace(10.0);
    let zero = s.a_schermo(T_INIZIO, 0.0).y;
    p.line_segment([Pos2::new(piano.left(), zero), Pos2::new(piano.right(), zero)], Stroke::new(1.0, LINEA));
    p.text(Pos2::new(piano.left() - 4.0, zero), Align2::RIGHT_CENTER, "0", f.clone(), GRIGIO);
    p.text(piano.left_top() + Vec2::new(6.0, 3.0), Align2::LEFT_TOP, "velocità verso di noi o via da noi, km/s", f.clone(), GRIGIO);
    let mut anno = 1996;
    while anno <= T_FINE as i32 {
        let x = s.a_schermo(f64::from(anno), 0.0).x;
        p.text(Pos2::new(x, piano.bottom() + 3.0), Align2::CENTER_TOP, format!("{anno}"), f.clone(), GRIGIO);
        anno += 2;
    }

    let n = 1200;
    let punti: Vec<Pos2> = (0..=n)
        .map(|i| {
            let t = T_INIZIO + (T_FINE - T_INIZIO) * f64::from(i) / f64::from(n);
            s.a_schermo(t, o.orbita.velocita_radiale(t))
        })
        .collect();
    // La curva esce dal riquadro al passaggio ravvicinato: si taglia, non si schiaccia.
    p.with_clip_rect(piano).add(Shape::line(punti, Stroke::new(1.8, ARANCIO)));

    for m in &o.dati.velocita {
        let c = s.a_schermo(m.t, m.v);
        if m.attiva {
            let alto_err = s.a_schermo(m.t, m.v + m.err).y;
            croce(p, c, 2.5, (c.y - alto_err).abs().max(2.5), CHIARO);
        } else {
            p.circle_stroke(c, 4.0, Stroke::new(1.0, GRIGIO));
        }
    }
    s
}

fn togli_punto(q: Pos2, vista: &VistaCielo, _piano: Rect, s: ScalaVelocita, o: &mut Osservatorio) {
    let mut migliore = (PRESA, None);
    for (i, m) in o.dati.posizioni.iter().enumerate() {
        let d = (vista.a_schermo(m.est, m.nord) - q).length();
        if d < migliore.0 {
            migliore = (d, Some((true, i)));
        }
    }
    for (i, m) in o.dati.velocita.iter().enumerate() {
        let d = (s.a_schermo(m.t, m.v) - q).length();
        if d < migliore.0 {
            migliore = (d, Some((false, i)));
        }
    }
    match migliore.1 {
        Some((true, i)) => o.dati.posizioni[i].attiva = !o.dati.posizioni[i].attiva,
        Some((false, i)) => o.dati.velocita[i].attiva = !o.dati.velocita[i].attiva,
        None => {}
    }
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

fn con_errore(valore: f64, errore: Option<f64>, cifre: usize) -> String {
    match errore {
        Some(e) => format!("{valore:.cifre$} ± {e:.cifre$}").replace('.', ","),
        None => format!("{valore:.cifre$}").replace('.', ","),
    }
}

fn strumenti(ui: &mut egui::Ui, o: &mut Osservatorio) -> Option<SulBanco> {
    let mut sul_banco = None;
    selettore(ui, o);
    ui.label(RichText::new("Pesa il buco nero della Galassia").strong().size(16.0).color(CIANO));
    ui.label(RichText::new(&o.nome_dati).color(GRIGIO));

    let chi = o.dati.chi2_ridotto(&o.orbita);
    let tinta = if chi < CHI_BUONO { VERDE } else if chi < 30.0 { GIALLO } else { ROSSO };
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Scarto dalle misure").color(GRIGIO));
        let testo = if chi < 1000.0 { decimale(chi) } else { grande(chi) };
        ui.label(RichText::new(testo).monospace().size(22.0).strong().color(tinta));
        ui.label(RichText::new("vicino a 1 è buono").small().color(GRIGIO));
    });
    ui.label(RichText::new(format!("{} misure attive", o.dati.numero())).small().color(GRIGIO));

    titoletto(ui, "L'ORBITA");
    let mut prima = o.orbita;
    {
        let q = &mut prima;
        let cursore = ui.add(egui::Slider::new(&mut q.a, 40.0..=250.0).text("grandezza, mas").custom_formatter(|v, _| format!("{v:.1}")));
        crate::visite::segna(ui, "cursori", &cursore);
        ui.add(egui::Slider::new(&mut q.e, 0.0..=0.98).text("schiacciamento").custom_formatter(|v, _| format!("{v:.3}")));
        ui.add(egui::Slider::new(&mut q.i, 0.0..=180.0).text("inclinazione").suffix("°").custom_formatter(|v, _| format!("{v:.1}")));
        ui.add(egui::Slider::new(&mut q.nodo, 0.0..=360.0).text("orientamento").suffix("°").custom_formatter(|v, _| format!("{v:.1}")));
        ui.add(egui::Slider::new(&mut q.periastro, 0.0..=360.0).text("punto più vicino").suffix("°").custom_formatter(|v, _| format!("{v:.1}")));
        ui.add(egui::Slider::new(&mut q.t0, 1995.0..=2010.0).text("quando passa").custom_formatter(|v, _| format!("{v:.2}")));
        ui.add(egui::Slider::new(&mut q.p, 5.0..=40.0).text("periodo, anni").custom_formatter(|v, _| format!("{v:.2}")));
        ui.add(egui::Slider::new(&mut q.r0, 4.0..=13.0).text("distanza, kpc").custom_formatter(|v, _| format!("{v:.2}")));
    }
    if prima != o.orbita {
        o.orbita = prima;
        o.modo = "a mano";
        o.incertezze = None;
    }
    ui.horizontal(|ui| {
        if ui.button(RichText::new("Calcola la migliore").strong()).clicked() {
            o.calcola();
        }
        if ui.small_button("Riparti").clicked() {
            o.riparti();
        }
    });
    ui.checkbox(&mut o.vz_fissa, "Il buco nero è fermo lungo la linea di vista");

    titoletto(ui, "IL RISULTATO");
    let massa_errore = o.incertezze.map(|e| e.massa);
    let orbita = o.orbita;
    egui::Grid::new("risultato").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(
            ui,
            "Massa, milioni di soli",
            con_errore(orbita.massa() / 1e6, massa_errore.map(|e| e / 1e6), 2),
        );
        riga(ui, "Distanza, kpc", con_errore(orbita.r0, o.incertezze.map(|e| e.r0), 2));
        riga(ui, "Periodo, anni", con_errore(orbita.p, o.incertezze.map(|e| e.p), 2));
        riga(ui, "Distanza minima dal centro", lunghezza(orbita.distanza_minima() * UA));
        riga(ui, "Velocità massima", format!("{} km/s", grande(orbita.velocita_massima())));
    });
    ui.label(RichText::new(format!("Metodo: {}.", o.modo)).color(GRIGIO));
    if !o.dati.velocita.iter().any(|v| v.attiva) {
        ui.label(RichText::new("Senza velocità la distanza non si può misurare, e con lei la massa.").color(GIALLO));
    }

    ui.horizontal(|ui| {
        if ui.small_button("Annota").clicked() {
            o.quaderno.push(Voce {
                modo: o.modo,
                misure: o.dati.numero(),
                orbita: o.orbita,
                chi2r: chi,
                errore_massa: massa_errore,
                vz_fissa: o.vz_fissa,
            });
        }
        ui.toggle_value(&mut o.confronto, "Confronta con l'articolo");
        if ui.small_button("Mettila sul banco").clicked() {
            sul_banco = Some(SulBanco { masse: o.orbita.massa(), a_ua: o.orbita.a_ua(), e: o.orbita.e });
        }
    });

    if o.confronto {
        let p = o.pubblicato();
        ui.label(RichText::new(p.descrizione).strong().color(CHIARO));
        ui.label(format!(
            "Massa {} ± {} milioni di soli, distanza {} ± {} kpc.",
            decimale(p.massa / 1e6),
            decimale(p.massa_errore / 1e6),
            decimale(p.r0),
            decimale(p.r0_errore)
        ));
        if let Some(e) = o.incertezze {
            let scarto = (orbita.massa() - p.massa).abs() / e.massa.hypot(p.massa_errore);
            let (giudizio, tinta) = if scarto < 2.0 { ("compatibile", VERDE) } else { ("in tensione", GIALLO) };
            ui.label(RichText::new(format!("Differenza: {} deviazioni. {giudizio}.", decimale(scarto))).color(tinta));
        } else {
            ui.label(RichText::new("A mano non c'è un'incertezza: premi Calcola per averla.").color(GRIGIO));
        }
        ui.label(
            RichText::new("L'articolo usa una velocità in più, che non ha pubblicato in numeri.")
                .small()
                .color(GRIGIO),
        );
    }

    if !o.quaderno.is_empty() {
        titoletto(ui, "QUADERNO");
        egui::Grid::new("quaderno").num_columns(4).spacing([12.0, 2.0]).striped(true).show(ui, |ui| {
            for t in ["Metodo", "Misure", "Massa", "Scarto"] {
                ui.label(RichText::new(t).small().color(GRIGIO));
            }
            ui.end_row();
            for v in &o.quaderno {
                ui.label(v.modo);
                ui.label(RichText::new(v.misure.to_string()).monospace());
                ui.label(RichText::new(con_errore(v.orbita.massa() / 1e6, v.errore_massa.map(|e| e / 1e6), 2)).monospace());
                ui.label(RichText::new(decimale(v.chi2r)).monospace());
                ui.end_row();
            }
        });
        if ui.small_button("Salva il quaderno").clicked() {
            let testo = o.csv_quaderno();
            o.salva("quaderno.csv", &testo);
        }
    }

    titoletto(ui, "I TUOI DATI");
    ui.horizontal(|ui| {
        ui.add(egui::TextEdit::singleline(&mut o.percorso).hint_text("percorso di un CSV, o trascinalo qui").desired_width(250.0));
        if ui.small_button("Carica").clicked() {
            let percorso = o.percorso.clone();
            o.importa(percorso.trim().trim_matches('"'));
        }
    });
    ui.horizontal(|ui| {
        if ui.small_button("Salva queste misure in CSV").clicked() {
            let testo = o.dati.in_csv();
            o.salva("misure_orbita.csv", &testo);
        }
        if ui.small_button("Torna a S2").clicked() {
            let (dati, nome) = (s2_ghez_2008(), "Stella S2, misure Keck e VLT".to_owned());
            o.dati = dati;
            o.nome_dati = nome;
            o.riparti();
        }
    });
    if let Some((testo, buono)) = &o.avviso {
        ui.label(RichText::new(testo).color(if *buono { VERDE } else { ROSSO }));
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new("Dati: Ghez et al. 2008, Eisenhauer et al. 2003 e 2005. Orbita di Keplero. Unità e incertezze obbligatorie.")
            .small()
            .color(GRIGIO),
    );
    sul_banco
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_calcolo_da_un_incertezza_e_annotare_conserva_il_risultato() {
        let mut o = Osservatorio::default();
        o.calcola();
        assert!(o.incertezze.is_some());
        assert!(o.dati.chi2_ridotto(&o.orbita) < CHI_BUONO);
    }

    #[test]
    fn un_file_inesistente_e_rifiutato_senza_toccare_i_dati() {
        let mut o = Osservatorio::default();
        let prima = o.dati.clone();
        o.importa("non/esiste.csv");
        assert_eq!(o.dati, prima);
        assert!(o.avviso.as_ref().is_some_and(|a| !a.1));
    }
}
