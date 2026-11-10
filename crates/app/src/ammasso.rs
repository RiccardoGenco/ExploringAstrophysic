//! Le stelle vere che orbitano attorno al buco nero della Galassia.

use crate::formato::{decimale, grande, lunghezza};
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA};
use colossi_fisica::costanti::UA;
use colossi_fisica::osservatorio::Orbita;
use egui::{Align2, Color32, FontId, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f64::consts::TAU;

/// Nome, a in arcosecondi, e, i, nodo, periastro, passaggio, periodo.
/// Gillessen et al. 2017, tabella 3. S111 e' esclusa: la sua orbita e' aperta.
const STELLE: [(&str, f64, f64, f64, f64, f64, f64, f64); 39] = [
    ("S1", 0.595, 0.556, 119.14, 342.04, 122.3, 2001.80, 166.0),
    ("S2", 0.1255, 0.8839, 134.18, 226.94, 65.51, 2002.33, 16.00),
    ("S4", 0.3570, 0.3905, 80.33, 258.84, 290.8, 1957.4, 77.0),
    ("S6", 0.6574, 0.8400, 87.24, 85.07, 116.23, 2108.61, 192.0),
    ("S8", 0.4047, 0.8031, 74.37, 315.43, 346.70, 1983.64, 92.9),
    ("S9", 0.2724, 0.644, 82.41, 156.60, 150.6, 1976.71, 51.3),
    ("S12", 0.2987, 0.8883, 33.56, 230.1, 317.9, 1995.59, 58.9),
    ("S13", 0.2641, 0.4250, 24.70, 74.5, 245.2, 2004.86, 49.00),
    ("S14", 0.2863, 0.9761, 100.59, 226.38, 334.59, 2000.12, 55.3),
    ("S17", 0.3559, 0.397, 96.83, 191.62, 326.0, 1991.19, 76.6),
    ("S18", 0.2379, 0.471, 110.67, 49.11, 349.46, 1993.86, 41.9),
    ("S19", 0.520, 0.750, 71.96, 344.60, 155.2, 2005.39, 135.0),
    ("S21", 0.2190, 0.764, 58.8, 259.64, 166.4, 2027.40, 37.00),
    ("S22", 1.31, 0.449, 105.76, 291.7, 95.0, 1996.9, 540.0),
    ("S23", 0.253, 0.56, 48.0, 249.0, 39.0, 2024.7, 45.8),
    ("S24", 0.944, 0.8970, 103.67, 7.93, 290.0, 2024.50, 331.0),
    ("S29", 0.428, 0.728, 105.8, 161.96, 346.5, 2025.96, 101.0),
    ("S31", 0.449, 0.5497, 109.03, 137.16, 308.0, 2018.07, 108.0),
    ("S33", 0.657, 0.608, 60.5, 100.1, 303.7, 1928.0, 192.0),
    ("S38", 0.1416, 0.8201, 171.1, 101.06, 17.99, 2003.19, 19.2),
    ("S39", 0.370, 0.9236, 89.36, 159.03, 23.3, 2000.06, 81.1),
    ("S42", 0.95, 0.567, 67.16, 196.14, 35.8, 2008.24, 335.0),
    ("S54", 1.20, 0.893, 62.2, 288.35, 140.8, 2004.46, 477.0),
    ("S55", 0.1078, 0.7209, 150.1, 325.5, 331.5, 2009.34, 12.80),
    ("S60", 0.3877, 0.7179, 126.87, 170.54, 29.37, 2023.89, 87.1),
    ("S66", 1.502, 0.128, 128.5, 92.3, 134.0, 1771.0, 664.0),
    ("S67", 1.126, 0.293, 136.0, 96.5, 213.5, 1705.0, 431.0),
    ("S71", 0.973, 0.899, 74.0, 35.16, 337.8, 1695.0, 346.0),
    ("S83", 1.49, 0.365, 127.2, 87.7, 203.6, 2046.8, 656.0),
    ("S85", 4.6, 0.78, 84.78, 107.36, 156.3, 1930.2, 3580.0),
    ("S87", 2.74, 0.224, 119.54, 106.32, 336.1, 611.0, 1640.0),
    ("S89", 1.081, 0.639, 87.61, 238.99, 126.4, 1783.0, 406.0),
    ("S91", 1.917, 0.303, 114.49, 105.35, 356.4, 1108.0, 958.0),
    ("S96", 1.499, 0.174, 126.36, 115.66, 233.6, 1646.0, 662.0),
    ("S97", 2.32, 0.35, 113.0, 113.2, 28.0, 2132.0, 1270.0),
    ("S145", 1.12, 0.50, 83.7, 263.92, 185.0, 1808.0, 426.0),
    ("S175", 0.414, 0.9867, 88.53, 326.83, 68.52, 2009.51, 96.2),
    ("R34", 1.81, 0.641, 136.0, 330.0, 57.0, 1522.0, 877.0),
    ("R44", 3.9, 0.27, 131.0, 80.5, 217.0, 1963.0, 2730.0),
];

const R0_KPC: f64 = 8.33;

fn orbita(i: usize) -> Orbita {
    let s = STELLE[i];
    Orbita {
        a: s.1 * 1000.0,
        e: s.2,
        i: s.3,
        nodo: s.4,
        periastro: s.5,
        t0: s.6,
        p: s.7,
        r0: R0_KPC,
        x0: 0.0,
        y0: 0.0,
        vx: 0.0,
        vy: 0.0,
        vz: 0.0,
    }
}

pub struct Ammasso {
    pub anno: f64,
    pub in_corso: bool,
    pub ritmo: f64,
    pub scelta: usize,
    raggio_mas: f64,
    centro: [f64; 2],
}

impl Default for Ammasso {
    fn default() -> Self {
        Self { anno: 2026.0, in_corso: true, ritmo: 3.0, scelta: 1, raggio_mas: 1100.0, centro: [0.0, 0.0] }
    }
}

impl Ammasso {
    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            self.anno += self.ritmo * dt;
        }
    }
}

pub fn centrale(ui: &mut egui::Ui, area: Rect, a: &mut Ammasso) {
    let r = ui.interact(area, ui.id().with("ammasso"), Sense::click_and_drag());
    if r.hovered() {
        let rotella = ui.input(|i| i.smooth_scroll_delta.y);
        a.raggio_mas = (a.raggio_mas * f64::from(-rotella * 0.0025).exp()).clamp(40.0, 6000.0);
    }
    let px = f64::from(area.height()) * 0.5 / a.raggio_mas;
    if r.dragged_by(PointerButton::Primary) || r.dragged_by(PointerButton::Secondary) {
        let d = r.drag_delta();
        a.centro[0] += f64::from(d.x) / px;
        a.centro[1] += f64::from(d.y) / px;
    }
    // Come in cielo: est a sinistra.
    let schermo = |est: f64, nord: f64| area.center() + Vec2::new(-(((est - a.centro[0]) * px) as f32), -(((nord - a.centro[1]) * px) as f32));

    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let scala_mas = crate::banco::passo_tondo(70.0 / px);
    let o = schermo(0.0, 0.0);
    for k in 1..12 {
        let raggio = (f64::from(k) * scala_mas * px) as f32;
        p.circle_stroke(o, raggio, Stroke::new(1.0, LINEA));
    }

    for i in 0..STELLE.len() {
        let orb = orbita(i);
        let scelta = i == a.scelta;
        let n = if scelta { 480 } else { 160 };
        let punti: Vec<Pos2> = (0..=n)
            .map(|k| {
                let e_a = TAU * f64::from(k) / f64::from(n);
                let media = e_a - orb.e * e_a.sin();
                let (x, y) = orb.posizione(orb.t0 + orb.p * media / TAU);
                schermo(x, y)
            })
            .collect();
        let tratto = if scelta { Stroke::new(1.8, CIANO) } else { Stroke::new(0.8, GRIGIO.gamma_multiply(0.38)) };
        p.add(Shape::line(punti, tratto));
    }
    p.circle_filled(o, 4.0, Color32::BLACK);
    p.circle_stroke(o, 4.0, Stroke::new(1.4, GIALLO));

    let mut vicino = (16.0_f32, None);
    let cursore = r.interact_pointer_pos();
    for i in 0..STELLE.len() {
        let (x, y) = orbita(i).posizione(a.anno);
        let q = schermo(x, y);
        let scelta = i == a.scelta;
        p.circle_filled(q, if scelta { 4.5 } else { 2.5 }, if scelta { CIANO } else { CHIARO });
        if scelta || px > 0.6 {
            p.text(q + Vec2::new(5.0, -4.0), Align2::LEFT_BOTTOM, STELLE[i].0, FontId::proportional(11.0), if scelta { CIANO } else { GRIGIO });
        }
        if let Some(c) = cursore {
            let d = (q - c).length();
            if d < vicino.0 {
                vicino = (d, Some(i));
            }
        }
    }
    if r.clicked() {
        if let Some(i) = vicino.1 {
            a.scelta = i;
        }
    }

    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Stelle vere attorno a Sagittarius A*, con le orbite misurate. Rotella per lo zoom, clic per sceglierne una.", f.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, format!("Ogni cerchio: {scala_mas} millesimi di secondo d'arco. Il riquadro è largo meno di un millesimo di grado."), f, CIANO);
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.end_row();
}

pub fn colonna(ui: &mut egui::Ui, a: &mut Ammasso) {
    ui.horizontal(|ui| {
        if ui.button(if a.in_corso { "Pausa" } else { "Avvia" }).clicked() {
            a.in_corso = !a.in_corso;
        }
        if ui.button("Oggi").clicked() {
            a.anno = 2026.0;
        }
    });
    ui.add(egui::Slider::new(&mut a.anno, 1900.0..=2200.0).text("anno").fixed_decimals(1));
    ui.add(egui::Slider::new(&mut a.ritmo, 0.1..=50.0).logarithmic(true).text("anni al secondo").fixed_decimals(1));

    let orb = orbita(a.scelta);
    ui.add_space(8.0);
    ui.label(RichText::new(STELLE[a.scelta].0).size(18.0).strong().color(CIANO));
    egui::Grid::new("stella_s").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Giro completo", format!("{} anni", decimale(orb.p)));
        riga(ui, "Punto più vicino", lunghezza(orb.distanza_minima() * UA));
        riga(ui, "Velocità massima", format!("{} km/s", grande(orb.velocita_massima())));
        riga(ui, "Eccentricità", decimale(orb.e));
        riga(ui, "Massa centrale implicata", format!("{} milioni di soli", decimale(orb.massa() / 1e6)));
    });
    ui.add_space(8.0);
    ui.label(RichText::new("ALTRE STELLE").small().strong().color(GRIGIO));
    ui.horizontal_wrapped(|ui| {
        for (i, s) in STELLE.iter().enumerate() {
            if ui.selectable_label(a.scelta == i, s.0).clicked() {
                a.scelta = i;
            }
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new("Orbite: Gillessen et al. 2017, distanza 8,33 kpc. Orbite di Keplero.").small().color(GRIGIO));
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tutte_implicano_circa_la_stessa_massa() {
        for i in 0..STELLE.len() {
            let m = orbita(i).massa();
            assert!(m > 2e6 && m < 9e6, "{}: {m}", STELLE[i].0);
        }
    }
}
