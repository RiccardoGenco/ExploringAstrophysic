//! Cosa curva lo spazio-tempo: una palla di particelle lasciata cadere, fuori e
//! dentro la materia.

use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::relativita::{Palla, Sfera};
use egui::{Align2, Color32, FontId, Frame, Margin, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const LARGHEZZA: f32 = 400.0;
const RAGGIO_RELATIVO_PALLA: f64 = 0.22;
const ALTEZZA_ZOOM: f32 = 230.0;
const CAMPO: f64 = 4.2;
const INGRANDIMENTO: f32 = 4.0;
const MATERIA: Color32 = Color32::from_rgb(255, 170, 90);

pub struct Curva {
    pub sfera: Sfera,
    pub palla: Palla,
    pub partenza: [f64; 2],
    pub in_corso: bool,
    pub ritmo: f64,
    scia: Vec<[f32; 2]>,
}

impl Default for Curva {
    fn default() -> Self {
        let sfera = Sfera { massa: 1.0, raggio: 1.0 };
        let partenza = [2.6, -0.6];
        Self {
            sfera,
            palla: Palla::nuova(sfera, partenza, RAGGIO_RELATIVO_PALLA),
            partenza,
            in_corso: true,
            ritmo: 0.4,
            scia: Vec::new(),
        }
    }
}

impl Curva {
    pub fn lascia(&mut self, partenza: [f64; 2]) {
        self.partenza = partenza;
        self.palla = Palla::nuova(self.sfera, partenza, RAGGIO_RELATIVO_PALLA * self.sfera.raggio);
        self.scia.clear();
        self.in_corso = true;
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            self.palla.avanza(self.ritmo * dt);
            let c = self.palla.centro();
            self.scia.push([c[0] as f32, c[2] as f32]);
            if self.scia.len() > 4000 {
                self.scia.remove(0);
            }
        }
    }
}

pub fn mostra(ui: &mut egui::Ui, c: &mut Curva) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        c.in_corso = !c.in_corso;
    }
    egui::Panel::bottom("tempo_curva")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            if pannello::barra_tempo(ui, &mut c.in_corso, &mut c.ritmo, 0.05..=3.0, "volte") {
                let p = c.partenza;
                c.lascia(p);
            }
        });
    egui::Panel::right("colonna_curva")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, c));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        crate::visite::segna_rect(ui.ctx(), "scena", area);
        mappa(ui, area, c);
    });
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

fn zoom(ui: &mut egui::Ui, c: &Curva) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_ZOOM), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let centro = area.center();
    let scala = area.height() * 0.28;
    p.circle_stroke(centro, scala, Stroke::new(1.0, GRIGIO.gamma_multiply(0.6)));

    let [cx, _, cz] = c.palla.centro();
    let raggio = c.palla.raggio;
    let mut punti: Vec<Pos2> = c
        .palla
        .anello()
        .map(|q| {
            centro
                + Vec2::new(
                    ((q[0] - cx) / raggio) as f32,
                    ((q[1] - cz) / raggio) as f32,
                ) * scala
        })
        .collect();
    if let Some(primo) = punti.first().copied() {
        punti.push(primo);
    }
    p.add(Shape::line(punti, Stroke::new(2.5, CIANO)));

    // Direzione di caduta iniziale.
    let [ax, az] = c.palla.asse_radiale;
    let verso = Vec2::new(ax as f32, az as f32);
    p.line_segment(
        [centro - verso * (scala * 1.6), centro + verso * (scala * 1.6)],
        Stroke::new(1.0, GIALLO.gamma_multiply(0.5)),
    );
    p.text(
        area.left_top() + Vec2::new(8.0, 6.0),
        Align2::LEFT_TOP,
        "VISTA DA CHI CADE CON LA PALLA",
        FontId::proportional(10.5),
        CHIARO,
    );
}

fn barra(ui: &mut egui::Ui, nome: &str, rapporto: f64, tinta: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(nome).color(GRIGIO));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let testo = format!("{:+.1} %", (rapporto - 1.0) * 100.0).replace('.', ",");
            ui.label(RichText::new(testo).monospace().color(CHIARO));
        });
    });
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 7.0), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 2.0, LINEA);
    let met = area.center().x;
    let d = ((rapporto - 1.0) as f32).clamp(-1.0, 1.0) * area.width() * 0.5;
    let barra = Rect::from_min_max(
        Pos2::new(met.min(met + d), area.top()),
        Pos2::new(met.max(met + d), area.bottom()),
    );
    p.rect_filled(barra, 1.0, tinta);
    p.line_segment([Pos2::new(met, area.top() - 2.0), Pos2::new(met, area.bottom() + 2.0)], Stroke::new(1.0, CHIARO));
    ui.add_space(3.0);
}

fn numero(x: f64) -> String {
    format!("{x:.3}").replace('.', ",")
}

fn strumenti(ui: &mut egui::Ui, c: &mut Curva) {
    ui.label(RichText::new("IL GIOCO").small().strong().color(GRIGIO));
    ui.add(egui::Label::new(RichText::new("Lascia cadere una palla di particelle e guarda cosa le succede.").size(17.0).strong().color(CHIARO)).wrap());
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new("Clicca in un punto qualsiasi della mappa: la palla parte ferma e cade verso la materia.").color(GRIGIO)).wrap());
    ui.add_space(8.0);

    let r = c.sfera.raggio;
    for (nome, punto) in [("Lasciala nel vuoto", [r * 2.6, -r * 0.6]), ("Lasciala dentro la materia", [r * 0.55, r * 0.2]), ("Lasciala al centro", [0.0, 0.0])] {
        if ui.add_sized([ui.available_width(), 34.0], egui::Button::new(RichText::new(nome).size(15.0))).clicked() {
            c.lascia(punto);
        }
    }

    titoletto(ui, "COSA SUCCEDE ALLA PALLA");
    let v = c.palla.volume_relativo();
    let (barra_area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 34.0), Sense::hover());
    let pp = ui.painter_at(barra_area);
    pp.rect_filled(barra_area, 8.0, LINEA);
    let k = (v / 1.2).clamp(0.0, 1.0) as f32;
    let stretta = (1.0 - v as f32).clamp(0.0, 1.0);
    let tinta = Color32::from_rgb((90.0 + 165.0 * stretta) as u8, (215.0 - 120.0 * stretta) as u8, (255.0 - 175.0 * stretta) as u8);
    pp.rect_filled(Rect::from_min_size(barra_area.min, Vec2::new(barra_area.width() * k, barra_area.height())), 8.0, tinta.gamma_multiply(0.65));
    pp.text(barra_area.center(), Align2::CENTER_CENTER, format!("Volume della palla: {:.0}%", v * 100.0), FontId::proportional(17.0), Color32::WHITE);
    let teorica = c.sfera.focalizzazione_teorica(c.partenza);
    let esito = if teorica < 0.0 {
        "Qui c'è materia: la palla si schiaccia da subito. È la materia che curva lo spazio-tempo."
    } else {
        "Nel vuoto la palla si stira in una direzione e si stringe nell'altra, ma il volume resta lo stesso."
    };
    ui.add_space(4.0);
    ui.add(egui::Label::new(RichText::new(esito).size(15.0).color(CIANO)).wrap());

    ui.add_space(10.0);
    egui::CollapsingHeader::new(RichText::new("Avanzate").strong()).default_open(false).show(ui, |ui| {
        zoom(ui, c);
        titoletto(ui, "FORMA DELLA PALLA");
        let [lungo, largo, fuori] = c.palla.semiassi().map(|a| a / c.palla.raggio);
        barra(ui, "Lungo la caduta", lungo, GIALLO);
        barra(ui, "Di lato", largo, CIANO);
        barra(ui, "Fuori dal piano", fuori, CIANO);
        titoletto(ui, "ALL'INIZIO DELLA CADUTA");
        let misurata = c.sfera.focalizzazione_misurata(c.partenza);
        egui::Grid::new("focalizzazione").num_columns(2).spacing([16.0, 4.0]).show(ui, |ui| {
            ui.label(RichText::new("Il volume accelera (misura)").color(GRIGIO));
            ui.label(RichText::new(numero(misurata)).monospace().strong());
            ui.end_row();
            ui.label(RichText::new("Einstein: -4π G densità").color(GRIGIO));
            ui.label(RichText::new(numero(teorica)).monospace().strong());
            ui.end_row();
        });
        titoletto(ui, "LA MATERIA");
        let prima = c.sfera;
        ui.add(egui::Slider::new(&mut c.sfera.massa, 0.2..=4.0).text("massa").fixed_decimals(2));
        ui.add(egui::Slider::new(&mut c.sfera.raggio, 0.5..=2.0).text("raggio").fixed_decimals(2));
        if prima != c.sfera {
            let p = c.partenza;
            c.lascia(p);
        }
        ui.label(RichText::new("Unità con G = 1. Campo debole e materia lenta.").small().color(GRIGIO));
    });
}

fn mappa(ui: &mut egui::Ui, area: Rect, c: &mut Curva) {
    let risposta = ui.interact(area, ui.id().with("curva"), Sense::click());
    let px = area.height().min(area.width()) * 0.5 / (CAMPO * f64::from(c.sfera.raggio as f32)) as f32;
    let centro = area.center();
    let a_schermo = |q: [f64; 2]| centro + Vec2::new(q[0] as f32, q[1] as f32) * px;

    if risposta.clicked() {
        if let Some(q) = risposta.interact_pointer_pos() {
            let d = q - centro;
            c.lascia([f64::from(d.x / px), f64::from(d.y / px)]);
        }
    }

    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    for i in 0..90u32 {
        let x = (i.wrapping_mul(2654435761) >> 8) as f32 % 1000.0 / 1000.0;
        let y = (i.wrapping_mul(40503).wrapping_add(11) >> 3) as f32 % 1000.0 / 1000.0;
        p.circle_filled(area.min + Vec2::new(x * area.width(), y * area.height()), 0.7 + (i % 3) as f32 * 0.4, Color32::from_white_alpha(110));
    }
    disegna_campo(&p, centro, px, c.sfera.raggio as f32, area);
    let raggio_px = c.sfera.raggio as f32 * px;
    // Alone e corpo del pianeta: piu' denso al centro.
    for k in (1..=6).rev() {
        let f = k as f32 / 6.0;
        p.circle_filled(centro, raggio_px * (1.0 + 0.35 * f), MATERIA.gamma_multiply(0.035));
    }
    for k in 0..12 {
        let f = 1.0 - k as f32 / 12.0;
        p.circle_filled(centro, raggio_px * f, MATERIA.gamma_multiply(0.05 + 0.05 * (1.0 - f)));
    }
    p.circle_stroke(centro, raggio_px, Stroke::new(1.5, MATERIA));
    p.text(centro, Align2::CENTER_CENTER, "materia", FontId::proportional(12.0), MATERIA.gamma_multiply(0.8));

    if c.scia.len() > 1 {
        let punti: Vec<Pos2> = c.scia.iter().map(|q| a_schermo([f64::from(q[0]), f64::from(q[1])])).collect();
        p.add(Shape::line(punti, Stroke::new(1.0, GRIGIO.gamma_multiply(0.6))));
    }
    let partenza = a_schermo(c.partenza);
    p.circle_stroke(partenza, 5.0, Stroke::new(1.0, GRIGIO));

    // La palla e' piccola: la si disegna ingrandita attorno al suo centro, cosi' si vede la forma.
    let [cx0, _, cz0] = c.palla.centro();
    let centro_palla = a_schermo([cx0, cz0]);
    let ingrandisci = |q: Pos2| centro_palla + (q - centro_palla) * INGRANDIMENTO;
    let mut anello: Vec<Pos2> = c.palla.anello().map(|q| ingrandisci(a_schermo(q))).collect();
    if let Some(primo) = anello.first().copied() {
        anello.push(primo);
    }
    // Piu' il volume si restringe, piu' la palla vira dal ciano al rosso.
    let v = c.palla.volume_relativo().clamp(0.0, 1.5) as f32;
    let stretta = (1.0 - v).clamp(0.0, 1.0);
    let tinta = Color32::from_rgb((90.0 + 165.0 * stretta) as u8, (215.0 - 120.0 * stretta) as u8, (255.0 - 175.0 * stretta) as u8);
    if anello.len() > 2 {
        p.add(Shape::convex_polygon(anello.clone(), tinta.gamma_multiply(0.22), Stroke::NONE));
        for q in anello.iter().step_by(6) {
            p.circle_filled(*q, 1.8, tinta);
        }
    }
    p.add(Shape::line(anello, Stroke::new(2.0, tinta)));
    let [x, _, z] = c.palla.centro();
    p.circle_filled(a_schermo([x, z]), 2.5, CIANO);
    p.text(centro_palla + Vec2::new(0.0, -(RAGGIO_RELATIVO_PALLA as f32) * INGRANDIMENTO * px * c.sfera.raggio as f32 - 18.0), Align2::CENTER_BOTTOM, "la palla, ingrandita 4 volte", FontId::proportional(12.0), CIANO);

    p.text(
        area.left_top() + Vec2::new(14.0, 12.0),
        Align2::LEFT_TOP,
        "Clic in un punto qualsiasi per lasciare cadere una palla di particelle, tutte ferme.",
        FontId::proportional(13.0),
        GRIGIO,
    );
    p.text(
        area.left_top() + Vec2::new(14.0, 32.0),
        Align2::LEFT_TOP,
        "Nel vuoto la gravità deforma. Solo dove c'è materia il volume si restringe: è ciò che la curvatura ha come sorgente.",
        FontId::proportional(13.0),
        CIANO,
    );
}

/// Frecce leggere che mostrano da che parte si cade.
fn disegna_campo(p: &Painter, centro: Pos2, px: f32, raggio: f32, area: Rect) {
    let passo = raggio * px * 0.9;
    let n = (area.width().max(area.height()) / passo) as i32 + 1;
    let tratto = Stroke::new(1.0, LINEA);
    for i in -n..=n {
        for j in -n..=n {
            let q = centro + Vec2::new(i as f32, j as f32) * passo;
            if !area.contains(q) || (i == 0 && j == 0) {
                continue;
            }
            let d = q - centro;
            let verso = d.normalized();
            p.line_segment([q, q - verso * 9.0], tratto);
            let punta = q - verso * 9.0;
            p.line_segment([punta, punta + verso.rot90() * 1.4 + verso * 3.0], tratto);
            p.line_segment([punta, punta - verso.rot90() * 1.4 + verso * 3.0], tratto);
        }
    }
}
