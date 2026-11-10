//! Banco dei gemelli: chi resta, chi viaggia, disegnati nello spazio-tempo.

use crate::banco::passo_tondo;
use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::relativita::{
    arrivo_a_casa, arrivo_sulla_linea, boost, evento_a_tempo_proprio, gamma, stato_a_tempo,
    tempo_proprio, vincola, Evento, VELOCITA_MASSIMA,
};
use egui::{Align2, Color32, FontId, Frame, Margin, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const VIAGGIATORE: Color32 = Color32::from_rgb(255, 130, 190);
const LARGHEZZA: f32 = 400.0;
const PRESA: f32 = 13.0;

const VIAGGI: [(&str, &[(f64, f64)]); 4] = [
    ("Andata e ritorno a 0,8 c", &[(0.0, 0.0), (5.0, 4.0), (10.0, 0.0)]),
    ("Alpha Centauri a 0,99 c", &[(0.0, 0.0), (4.414, 4.37), (8.828, 0.0)]),
    ("Avanti e indietro", &[(0.0, 0.0), (3.0, 2.0), (6.0, -1.0), (9.0, 2.0), (12.0, 0.0)]),
    ("Senza fretta", &[(0.0, 0.0), (10.0, 0.5), (20.0, 0.0)]),
];

pub struct Gemelli {
    pub linea: Vec<Evento>,
    pub avanzamento: f64,
    pub in_corso: bool,
    pub ritmo: f64,
    pub beta_vista: f64,
    pub segnali: bool,
    trascinato: Option<usize>,
    vista_ferma: Option<Vista>,
}

impl Default for Gemelli {
    fn default() -> Self {
        let mut g = Self {
            linea: Vec::new(),
            avanzamento: 0.0,
            in_corso: true,
            ritmo: 1.5,
            beta_vista: 0.0,
            segnali: false,
            trascinato: None,
            vista_ferma: None,
        };
        g.carica(0);
        g
    }
}

impl Gemelli {
    pub fn carica(&mut self, i: usize) {
        self.linea = VIAGGI[i].1.iter().map(|&(t, x)| Evento { t, x }).collect();
        self.avanzamento = 0.0;
        self.in_corso = true;
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            let durata = self.linea.last().map_or(1.0, |e| e.t);
            self.avanzamento = (self.avanzamento + self.ritmo * dt / durata).min(1.0);
            if self.avanzamento >= 1.0 {
                self.in_corso = false;
            }
        }
    }

    fn casa(&self) -> [Evento; 2] {
        [Evento { t: 0.0, x: 0.0 }, Evento { t: self.linea.last().map_or(1.0, |e| e.t), x: 0.0 }]
    }

    fn visti(&self, linea: &[Evento]) -> Vec<Evento> {
        linea.iter().map(|e| boost(*e, self.beta_vista)).collect()
    }

    /// Sposta un vertice, restando sotto la velocita' della luce.
    fn sposta(&mut self, i: usize, voluto: Evento) {
        let prima = self.linea[i - 1];
        let ultimo = i + 1 == self.linea.len();
        let dopo = self.linea.get(i + 1).copied();
        let voluto = if ultimo {
            // Il ritorno deve avvenire a casa.
            Evento { t: voluto.t.max(prima.t + prima.x.abs() / VELOCITA_MASSIMA + 0.02), x: 0.0 }
        } else {
            voluto
        };
        if let Some(e) = vincola(prima, dopo, voluto) {
            self.linea[i] = e;
        }
    }

    #[cfg(test)]
    pub fn posizione_vertice(&self, area: Rect, i: usize) -> Pos2 {
        calcola_vista(self, area).schermo(boost(self.linea[i], self.beta_vista))
    }

    /// Inserisce un vertice sul tratto piu' vicino al punto.
    fn aggiungi(&mut self, punto: Evento) {
        let migliore = self
            .linea
            .windows(2)
            .enumerate()
            .filter(|(_, c)| punto.t > c[0].t + 0.05 && punto.t < c[1].t - 0.05)
            .map(|(i, _)| i)
            .next();
        if let Some(i) = migliore {
            let c = (self.linea[i], self.linea[i + 1]);
            let k = (punto.t - c.0.t) / (c.1.t - c.0.t);
            let e = Evento { t: punto.t, x: c.0.x + (c.1.x - c.0.x) * k };
            self.linea.insert(i + 1, e);
        }
    }
}

#[derive(Clone, Copy)]
struct Vista {
    origine: Pos2,
    px: f32,
}

impl Vista {
    fn schermo(&self, e: Evento) -> Pos2 {
        self.origine + Vec2::new(e.x as f32 * self.px, -e.t as f32 * self.px)
    }

    fn mondo(&self, p: Pos2) -> Evento {
        let d = p - self.origine;
        Evento { t: f64::from(-d.y / self.px), x: f64::from(d.x / self.px) }
    }
}

fn numero(x: f64) -> String {
    format!("{x:.2}").replace('.', ",")
}

pub fn mostra(ui: &mut egui::Ui, g: &mut Gemelli) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        g.in_corso = !g.in_corso;
    }
    egui::Panel::bottom("tempo_gemelli")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            let ricomincia = pannello::barra_tempo(ui, &mut g.in_corso, &mut g.ritmo, 0.1..=20.0, "anni al secondo");
            if ricomincia {
                g.avanzamento = 0.0;
                g.in_corso = true;
            }
        });
    egui::Panel::right("colonna_gemelli")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, g));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        diagramma(ui, area, g);
    });
}

fn strumenti(ui: &mut egui::Ui, g: &mut Gemelli) {
    let linea = g.linea.clone();
    let casa = g.casa();
    let totale_casa = casa[1].t;
    let totale_viaggio = tempo_proprio(&linea);

    // Orologi sulla stessa fetta di "adesso" di chi guarda.
    let (vista_casa, vista_linea) = (g.visti(&casa), g.visti(&linea));
    let adesso = g.avanzamento * vista_casa[1].t;
    let (_, tau_casa) = stato_a_tempo(&vista_casa, adesso);
    let (_, tau_viaggio) = stato_a_tempo(&vista_linea, adesso);

    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 138.0), Sense::hover());
    let p = ui.painter_at(area);
    let y = area.top() + 56.0;
    quadrante(&p, Pos2::new(area.center().x - 92.0, y), tau_casa, CIANO, "chi resta");
    quadrante(&p, Pos2::new(area.center().x + 92.0, y), tau_viaggio, VIAGGIATORE, "chi viaggia");

    ui.separator();
    titoletto(ui, "A FINE VIAGGIO");
    egui::Grid::new("esiti").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
        riga(ui, "Chi resta invecchia", format!("{} anni", numero(totale_casa)));
        riga(ui, "Chi viaggia invecchia", format!("{} anni", numero(totale_viaggio)));
        riga(ui, "Differenza", format!("{} anni", numero(totale_casa - totale_viaggio)));
    });

    titoletto(ui, "TRATTI DEL VIAGGIO");
    egui::Grid::new("tratti").num_columns(4).spacing([16.0, 3.0]).striped(true).show(ui, |ui| {
        for t in ["Velocità", "γ", "Per chi resta", "Per chi viaggia"] {
            ui.label(RichText::new(t).small().color(GRIGIO));
        }
        ui.end_row();
        for c in linea.windows(2) {
            let durata = c[1].t - c[0].t;
            let beta = (c[1].x - c[0].x) / durata;
            let g_tratto = gamma(beta);
            ui.label(RichText::new(format!("{} c", numero(beta.abs()))).monospace());
            ui.label(RichText::new(numero(g_tratto)).monospace());
            ui.label(RichText::new(format!("{} a", numero(durata))).monospace());
            ui.label(RichText::new(format!("{} a", numero(durata / g_tratto))).monospace());
            ui.end_row();
        }
    });

    titoletto(ui, "CHI GUARDA");
    ui.add(
        egui::Slider::new(&mut g.beta_vista, -0.9..=0.9)
            .text("velocità osservatore")
            .fixed_decimals(2),
    );
    ui.label(
        RichText::new("Le coordinate cambiano. I due totali qui sopra no.").color(GRIGIO),
    );
    if ui.small_button("Torna a chi resta").clicked() {
        g.beta_vista = 0.0;
    }

    titoletto(ui, "LAMPI DI LUCE");
    ui.checkbox(&mut g.segnali, "Ognuno manda un lampo all'anno");
    if g.segnali {
        ui.label(format!(
            "Chi resta ne riceve {}, chi viaggia {}.",
            totale_viaggio.floor(),
            totale_casa.floor()
        ));
    }

    titoletto(ui, "VIAGGI PRONTI");
    let mut carica = None;
    ui.horizontal_wrapped(|ui| {
        for (i, (nome, _)) in VIAGGI.iter().enumerate() {
            if ui.small_button(*nome).clicked() {
                carica = Some(i);
            }
        }
    });
    if let Some(i) = carica {
        g.carica(i);
    }
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().strong().color(CHIARO));
    ui.end_row();
}

fn quadrante(p: &Painter, c: Pos2, anni: f64, tinta: Color32, nome: &str) {
    const RAGGIO: f32 = 42.0;
    p.circle_stroke(c, RAGGIO, Stroke::new(2.0, tinta));
    for i in 0..12 {
        let a = i as f32 / 12.0 * TAU;
        let d = Vec2::new(a.sin(), -a.cos());
        p.line_segment([c + d * (RAGGIO - 5.0), c + d * (RAGGIO - 1.0)], Stroke::new(1.0, tinta));
    }
    let a = (anni.fract() as f32) * TAU;
    p.line_segment([c, c + Vec2::new(a.sin(), -a.cos()) * (RAGGIO - 8.0)], Stroke::new(2.5, tinta));
    p.circle_filled(c, 3.0, tinta);
    p.text(c + Vec2::new(0.0, RAGGIO + 12.0), Align2::CENTER_CENTER, nome, FontId::proportional(13.0), tinta);
    p.text(
        c + Vec2::new(0.0, RAGGIO + 30.0),
        Align2::CENTER_CENTER,
        format!("{} anni", numero(anni)),
        FontId::monospace(14.0),
        CHIARO,
    );
}

fn calcola_vista(g: &Gemelli, area: Rect) -> Vista {
    let (casa, linea) = (g.visti(&g.casa()), g.visti(&g.linea));
    let tutti = || casa.iter().chain(linea.iter());
    let x_min = tutti().map(|e| e.x).fold(f64::INFINITY, f64::min) - 1.5;
    let x_max = tutti().map(|e| e.x).fold(f64::NEG_INFINITY, f64::max) + 1.5;
    let t_max = tutti().map(|e| e.t).fold(f64::NEG_INFINITY, f64::max) + 1.0;
    let t_min = tutti().map(|e| e.t).fold(f64::INFINITY, f64::min).min(0.0) - 0.9;
    let px = (f64::from(area.width()) / (x_max - x_min)).min(f64::from(area.height()) / (t_max - t_min)) as f32;
    let centro_x = ((x_min + x_max) * 0.5) as f32;
    let centro_t = ((t_min + t_max) * 0.5) as f32;
    Vista { origine: area.center() + Vec2::new(-centro_x * px, centro_t * px), px }
}

fn griglia(p: &Painter, area: Rect, v: Vista) {
    let passo = passo_tondo(f64::from(46.0 / v.px));
    let etichette = FontId::monospace(10.0);
    let tratto = Stroke::new(1.0, LINEA);
    let angolo_a = v.mondo(area.left_bottom());
    let angolo_b = v.mondo(area.right_top());
    let mut t = (angolo_a.t / passo).ceil() * passo;
    while t < angolo_b.t {
        let y = v.schermo(Evento { t, x: 0.0 }).y;
        p.line_segment([Pos2::new(area.left(), y), Pos2::new(area.right(), y)], tratto);
        p.text(Pos2::new(area.left() + 6.0, y - 1.0), Align2::LEFT_BOTTOM, format!("{}", t + 0.0), etichette.clone(), GRIGIO);
        t += passo;
    }
    let mut x = (angolo_a.x / passo).ceil() * passo;
    while x < angolo_b.x {
        let q = v.schermo(Evento { t: 0.0, x }).x;
        p.line_segment([Pos2::new(q, area.top()), Pos2::new(q, area.bottom())], tratto);
        p.text(Pos2::new(q + 3.0, area.bottom() - 4.0), Align2::LEFT_BOTTOM, format!("{x}"), etichette.clone(), GRIGIO);
        x += passo;
    }
}

fn linea_con_anni(p: &Painter, g: &Gemelli, v: Vista, linea: &[Evento], tinta: Color32) {
    let punti: Vec<Pos2> = g.visti(linea).iter().map(|e| v.schermo(*e)).collect();
    p.add(Shape::line(punti, Stroke::new(2.4, tinta)));
    let totale = tempo_proprio(linea);
    let passo = if totale > 60.0 { 5 } else { 1 };
    let mut anno = passo;
    while f64::from(anno) <= totale {
        if let Some(e) = evento_a_tempo_proprio(linea, f64::from(anno)) {
            let q = v.schermo(boost(e, g.beta_vista));
            p.circle_filled(q, 3.2, tinta);
            p.text(q + Vec2::new(11.0, 0.0), Align2::LEFT_CENTER, anno.to_string(), FontId::monospace(10.0), tinta);
        }
        anno += passo;
    }
}

fn lampi(p: &Painter, g: &Gemelli, v: Vista) {
    let casa = g.casa();
    let tratto_v = Stroke::new(0.9, VIAGGIATORE.gamma_multiply(0.5));
    let tratto_c = Stroke::new(0.9, CIANO.gamma_multiply(0.5));
    let disegna = |da: Evento, a: Evento, t: Stroke| {
        p.line_segment([v.schermo(boost(da, g.beta_vista)), v.schermo(boost(a, g.beta_vista))], t);
    };
    let mut anno = 1.0;
    while anno < tempo_proprio(&g.linea) {
        if let Some(e) = evento_a_tempo_proprio(&g.linea, anno) {
            let arrivo = arrivo_a_casa(e);
            if arrivo <= casa[1].t {
                disegna(e, Evento { t: arrivo, x: 0.0 }, tratto_v);
            }
        }
        anno += 1.0;
    }
    let mut anno = 1.0;
    while anno < casa[1].t {
        if let Some(e) = arrivo_sulla_linea(&g.linea, anno) {
            disegna(Evento { t: anno, x: 0.0 }, e, tratto_c);
        }
        anno += 1.0;
    }
}

fn diagramma(ui: &mut egui::Ui, area: Rect, g: &mut Gemelli) {
    let risposta = ui.interact(area, ui.id().with("gemelli"), Sense::click_and_drag());
    let v = g.vista_ferma.unwrap_or_else(|| calcola_vista(g, area));
    let beta = g.beta_vista;
    let vicino = |g: &Gemelli, punto: Pos2| {
        (1..g.linea.len()).find(|&i| (v.schermo(boost(g.linea[i], beta)) - punto).length() < PRESA)
    };

    if risposta.drag_started_by(PointerButton::Primary) {
        let premuto = ui.input(|i| i.pointer.press_origin()).unwrap_or(area.center());
        g.trascinato = vicino(g, premuto);
        g.vista_ferma = g.trascinato.map(|_| v);
    }
    if let (Some(i), true) = (g.trascinato, risposta.dragged_by(PointerButton::Primary)) {
        if let Some(q) = ui.input(|i| i.pointer.latest_pos()) {
            g.sposta(i, boost(v.mondo(q), -beta));
        }
    }
    if risposta.drag_stopped_by(PointerButton::Primary) {
        g.trascinato = None;
        g.vista_ferma = None;
    }
    if risposta.clicked() {
        if let Some(q) = risposta.interact_pointer_pos() {
            if vicino(g, q).is_none() {
                g.aggiungi(boost(v.mondo(q), -beta));
            }
        }
    }
    if risposta.secondary_clicked() {
        if let Some(q) = risposta.interact_pointer_pos() {
            if let Some(i) = vicino(g, q) {
                if i + 1 < g.linea.len() {
                    g.linea.remove(i);
                }
            }
        }
    }

    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    griglia(&p, area, v);

    // Cono di luce dall'evento di partenza.
    let o = v.schermo(Evento { t: 0.0, x: 0.0 });
    let lungo = area.width() + area.height();
    for segno in [-1.0_f32, 1.0] {
        let fine = o + Vec2::new(segno * lungo, -lungo);
        p.line_segment([o, fine], Stroke::new(1.2, GIALLO.gamma_multiply(0.55)));
    }

    if g.segnali {
        lampi(&p, g, v);
    }
    let casa = g.casa();
    linea_con_anni(&p, g, v, &casa, CIANO);
    linea_con_anni(&p, g, v, &g.linea, VIAGGIATORE);

    for e in g.linea.iter().skip(1) {
        let q = v.schermo(boost(*e, beta));
        p.circle_filled(q, 7.0, VIAGGIATORE);
        p.circle_stroke(q, 7.0, Stroke::new(1.5, CHIARO));
    }

    // L'adesso di chi guarda.
    let (vista_casa, vista_linea) = (g.visti(&casa), g.visti(&g.linea));
    let adesso = g.avanzamento * vista_casa[1].t;
    let y = v.schermo(Evento { t: adesso, x: 0.0 }).y;
    p.line_segment(
        [Pos2::new(area.left(), y), Pos2::new(area.right(), y)],
        Stroke::new(1.0, CHIARO.gamma_multiply(0.55)),
    );
    for (linea, tinta) in [(&vista_casa[..], CIANO), (&vista_linea[..], VIAGGIATORE)] {
        let (x, _) = stato_a_tempo(linea, adesso);
        p.circle_filled(v.schermo(Evento { t: adesso, x }), 8.0, tinta);
    }

    let testo = FontId::proportional(13.0);
    p.text(
        area.left_top() + Vec2::new(30.0, 12.0),
        Align2::LEFT_TOP,
        "Trascina i punti rosa. Clic su uno spazio vuoto all'altezza giusta: nuova tappa. Tasto destro su un punto: via.",
        testo.clone(),
        GRIGIO,
    );
    p.text(
        area.left_top() + Vec2::new(30.0, 32.0),
        Align2::LEFT_TOP,
        "In verticale il tempo in anni, in orizzontale lo spazio in anni luce. La luce viaggia sulle diagonali gialle.",
        testo,
        CIANO,
    );
}
