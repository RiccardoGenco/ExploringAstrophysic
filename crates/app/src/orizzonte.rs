//! Dentro l'orizzonte: i coni di luce che si piegano, lo spazio che viene trascinato.

use crate::formato::decimale;
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::orizzonte::{pendenze_cono, solo_verso_il_centro, Kerr};
use egui::{Align2, Color32, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const LARGHEZZA: f32 = 340.0;
const R_MASSIMO: f64 = 4.0;
const T_MASSIMO: f64 = 6.0;
const N_GRANELLI: usize = 26;
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);

pub struct BancoOrizzonte {
    pub spin: f64,
    pub tempo: f64,
    pub r_cono: f64,
}

impl Default for BancoOrizzonte {
    fn default() -> Self {
        Self { spin: 0.9, tempo: 0.0, r_cono: 2.2 }
    }
}

impl BancoOrizzonte {
    pub fn avanza(&mut self, dt: f64) {
        self.tempo += dt;
    }
}

pub fn mostra(ui: &mut egui::Ui, b: &mut BancoOrizzonte) {
    egui::Panel::right("colonna_orizzonte")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| colonna(ui, b));
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        let meta = area.left() + area.width() * 0.56;
        coni(ui, Rect::from_min_max(area.min, Pos2::new(meta, area.bottom())), b);
        trascinamento(ui, Rect::from_min_max(Pos2::new(meta, area.top()), area.max), b);
    });
}

fn coni(ui: &mut egui::Ui, area: Rect, b: &mut BancoOrizzonte) {
    let r = ui.interact(area, ui.id().with("coni"), Sense::click_and_drag());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let piano = Rect::from_min_max(area.min + Vec2::new(36.0, 70.0), area.max - Vec2::new(14.0, 36.0));
    let a_schermo = |x: f64, t: f64| {
        Pos2::new(
            piano.left() + piano.width() * (x / R_MASSIMO) as f32,
            piano.bottom() - piano.height() * (t / T_MASSIMO) as f32,
        )
    };
    if r.is_pointer_button_down_on() {
        if let Some(q) = ui.input(|i| i.pointer.latest_pos()) {
            b.r_cono = (f64::from((q.x - piano.left()) / piano.width()) * R_MASSIMO).clamp(0.08, R_MASSIMO - 0.05);
        }
    }

    // La zona oltre l'orizzonte.
    p.rect_filled(Rect::from_min_max(piano.min, a_schermo(1.0, 0.0)), 0.0, Color32::from_rgba_unmultiplied(255, 100, 90, 18));
    let oriz = a_schermo(1.0, 0.0).x;
    p.line_segment([Pos2::new(oriz, piano.top()), Pos2::new(oriz, piano.bottom())], Stroke::new(1.8, CIANO));
    p.text(Pos2::new(oriz + 4.0, piano.top() + 2.0), Align2::LEFT_TOP, "orizzonte", FontId::proportional(12.0), CIANO);
    p.text(a_schermo(0.0, 0.0) + Vec2::new(0.0, 4.0), Align2::CENTER_TOP, "centro", FontId::monospace(10.0), GRIGIO);
    p.text(a_schermo(R_MASSIMO, 0.0) + Vec2::new(-4.0, 4.0), Align2::RIGHT_TOP, "fuori", FontId::monospace(10.0), GRIGIO);

    let cono = |x: f64, t: f64, lato: f64, tinta: Color32, spessore: f32| {
        let (dentro, fuori) = pendenze_cono(x);
        let q = a_schermo(x, t);
        // dt/dr: verso il futuro (t crescente) si cammina di `lato` in tempo.
        let dr_entrante = lato / dentro;
        let dr_uscente = if fuori.is_finite() { lato / fuori } else { 0.0 };
        let (a, c) = (a_schermo(x + dr_entrante, t + lato), a_schermo(x + dr_uscente, t + lato));
        p.add(Shape::convex_polygon(vec![q, a, c], tinta.gamma_multiply(0.2), Stroke::NONE));
        p.line_segment([q, a], Stroke::new(spessore, tinta));
        p.line_segment([q, c], Stroke::new(spessore, tinta));
    };

    for i in 0..9 {
        let t = 0.4 + f64::from(i) * 0.62;
        for j in 0..8 {
            let x = 0.28 + f64::from(j) * 0.5;
            if (x - 1.0).abs() < 0.05 {
                continue;
            }
            cono(x, t, 0.26, if x < 1.0 { ROSSO } else { GRIGIO }, 1.0);
        }
    }
    let t_scelto = 2.2;
    let tinta = if solo_verso_il_centro(b.r_cono) { ROSSO } else { GIALLO };
    cono(b.r_cono, t_scelto, 0.8, tinta, 2.4);
    p.circle_filled(a_schermo(b.r_cono, t_scelto), 4.5, CHIARO);

    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Il tempo sale, il centro è a sinistra. Ogni cono è tutto il futuro che puoi raggiungere.", f.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, "Trascina in orizzontale: oltre l'orizzonte anche la luce va solo verso il centro.", f, CIANO);
}

fn trascinamento(ui: &mut egui::Ui, area: Rect, b: &BancoOrizzonte) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    p.line_segment([area.left_top(), area.left_bottom()], Stroke::new(1.0, LINEA));
    let centro = area.center() + Vec2::new(0.0, 22.0);
    let scala = area.width().min(area.height() - 110.0) * 0.5 / 3.1;
    let k = Kerr { a: b.spin };

    let raggio_erg = k.ergosfera() as f32 * scala;
    p.circle_filled(centro, raggio_erg, Color32::from_rgba_unmultiplied(120, 150, 255, 26));
    p.circle_stroke(centro, raggio_erg, Stroke::new(1.0, GIALLO.gamma_multiply(0.7)));
    let oriz = k.orizzonte() as f32 * scala;
    p.circle_filled(centro, oriz, Color32::BLACK);
    p.circle_stroke(centro, oriz, Stroke::new(1.8, CIANO));

    // Granelli lasciati cadere senza rotazione: lo spazio li porta con se'.
    for i in 0..N_GRANELLI {
        let inizio = i as f64 / N_GRANELLI as f64 * 6.0;
        let fase = (b.tempo + inizio) % 6.0;
        let a0 = i as f32 / N_GRANELLI as f32 * TAU;
        let (mut r, mut ang, h) = (3.0_f64, f64::from(a0), 0.02);
        let mut punti = Vec::new();
        let mut t = 0.0;
        while t < fase && r > k.orizzonte() * 1.005 {
            ang += k.trascinamento(r) * h * 6.0;
            r -= (r - k.orizzonte()).max(0.02).sqrt() * h * 0.9;
            t += h;
            let d = Vec2::new(ang.cos() as f32, ang.sin() as f32) * (r as f32 * scala);
            punti.push(centro + d);
        }
        if punti.len() > 1 {
            p.add(Shape::line(punti.clone(), Stroke::new(1.0, CHIARO.gamma_multiply(0.5))));
            if let Some(q) = punti.last() {
                p.circle_filled(*q, 2.4, CHIARO);
            }
        }
    }
    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Un buco nero che ruota, visto dall'alto.", f.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, "Questi granelli cadono dritti: è lo spazio a girare.", f, CIANO);
    p.text(area.left_bottom() + Vec2::new(14.0, -10.0), Align2::LEFT_BOTTOM, "Il moto di caduta è schematico, la rotazione impressa è calcolata.", FontId::proportional(11.5), GRIGIO);
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.end_row();
}

fn colonna(ui: &mut egui::Ui, b: &mut BancoOrizzonte) {
    ui.label(RichText::new("Il cono di luce").strong().size(15.0).color(CIANO));
    let (_, fuori) = pendenze_cono(b.r_cono);
    let esito = if solo_verso_il_centro(b.r_cono) {
        "Dentro: il centro non è un luogo, è il tuo futuro. Qualsiasi direzione ci porta."
    } else {
        "Fuori: la luce che punta in fuori si allontana, sempre più a fatica vicino all'orizzonte."
    };
    egui::Grid::new("cono").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Distanza", format!("{} raggi", decimale(b.r_cono)));
        riga(ui, "Pendenza del raggio uscente", format!("{fuori:.2}").replace('.', ","));
    });
    ui.label(RichText::new(esito).color(CHIARO));
    ui.add_space(14.0);
    ui.separator();
    ui.label(RichText::new("La rotazione").strong().size(15.0).color(CIANO));
    ui.add(egui::Slider::new(&mut b.spin, 0.0..=0.998).text("rotazione").fixed_decimals(3));
    let k = Kerr { a: b.spin };
    egui::Grid::new("kerr").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Orizzonte", format!("{} raggi", decimale(k.orizzonte())));
        riga(ui, "Ergosfera", format!("{} raggi", decimale(k.ergosfera())));
        riga(ui, "L'orizzonte gira a", format!("{} c/raggio", decimale(k.omega_orizzonte())));
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Dentro la zona blu nessuno può restare fermo: lo spazio trascina tutto con sé, nel verso della rotazione.").color(CHIARO));
    ui.add_space(8.0);
    ui.label(RichText::new("Coni: coordinate di Eddington-Finkelstein. Rotazione: metrica di Kerr nel piano equatoriale.").small().color(GRIGIO));
}
