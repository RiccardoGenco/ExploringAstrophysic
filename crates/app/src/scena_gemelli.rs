//! La scena dei gemelli: la nave al centro, le stelle che scorrono, due orologi grandi.
//! Le distanze sono quelle che vede chi viaggia, quindi accorciate dal fattore γ.

use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, VERDE};
use colossi_fisica::relativita::{gamma, stato_a_tempo, tempo_proprio, Evento};
use egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const ROSA: Color32 = Color32::from_rgb(255, 130, 190);
const ETA_PARTENZA: f64 = 20.0;

/// Numero pseudo-casuale stabile in [0, 1) per l'indice i.
fn caso(i: u32, sale: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1).wrapping_add(sale.wrapping_mul(0x85EB_CA6B));
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65536.0
}

fn numero(x: f64) -> String {
    format!("{x:.1}").replace('.', ",")
}

/// Velocita' (in c, con segno) del tratto in cui ci si trova all'istante t.
fn velocita(linea: &[Evento], t: f64) -> f64 {
    for c in linea.windows(2) {
        if t <= c[1].t {
            return (c[1].x - c[0].x) / (c[1].t - c[0].t);
        }
    }
    0.0
}

fn faccia(p: &Painter, c: Pos2, anni: f64, tinta: Color32) {
    // Capelli che sbiancano con l'eta': a 20 anni scuri, a 90 bianchi.
    let k = (((anni - 20.0) / 70.0).clamp(0.0, 1.0)) as f32;
    let capelli = Color32::from_rgb(
        (60.0 + 190.0 * k) as u8,
        (45.0 + 205.0 * k) as u8,
        (40.0 + 210.0 * k) as u8,
    );
    p.circle_filled(c, 22.0, Color32::from_rgb(240, 205, 170));
    p.add(Shape::convex_polygon(
        vec![c + Vec2::new(-22.0, -4.0), c + Vec2::new(-16.0, -20.0), c + Vec2::new(0.0, -26.0), c + Vec2::new(16.0, -20.0), c + Vec2::new(22.0, -4.0), c + Vec2::new(12.0, -12.0), c + Vec2::new(-12.0, -12.0)],
        capelli,
        Stroke::NONE,
    ));
    p.circle_filled(c + Vec2::new(-8.0, 1.0), 2.2, Color32::BLACK);
    p.circle_filled(c + Vec2::new(8.0, 1.0), 2.2, Color32::BLACK);
    // Le rughe compaiono dopo i 45 anni.
    let rughe = ((anni - 45.0) / 40.0).clamp(0.0, 1.0) as f32;
    if rughe > 0.0 {
        let s = Stroke::new(1.0, Color32::from_rgb(150, 110, 90).gamma_multiply(rughe));
        p.line_segment([c + Vec2::new(-12.0, -6.0), c + Vec2::new(-4.0, -6.0)], s);
        p.line_segment([c + Vec2::new(4.0, -6.0), c + Vec2::new(12.0, -6.0)], s);
    }
    p.line_segment([c + Vec2::new(-6.0, 12.0), c + Vec2::new(6.0, 12.0)], Stroke::new(1.6, Color32::from_rgb(120, 60, 60)));
    p.circle_stroke(c, 24.0, Stroke::new(2.0, tinta));
}

fn scheda_eta(p: &Painter, area: Rect, anni: f64, tinta: Color32, nome: &str) {
    p.rect_filled(area, 10.0, Color32::from_black_alpha(150));
    p.rect_stroke(area, 10.0, Stroke::new(1.5, tinta), egui::StrokeKind::Inside);
    faccia(p, area.left_center() + Vec2::new(38.0, 0.0), anni, tinta);
    p.text(area.left_top() + Vec2::new(76.0, 10.0), Align2::LEFT_TOP, nome, FontId::proportional(14.0), tinta);
    p.text(area.left_top() + Vec2::new(76.0, 30.0), Align2::LEFT_TOP, format!("{} anni", numero(ETA_PARTENZA + anni)), FontId::proportional(26.0), CHIARO);
}

fn nave(p: &Painter, c: Pos2, verso_destra: bool, fiamma: f32) {
    let d = if verso_destra { 1.0 } else { -1.0 };
    let punto = |x: f32, y: f32| c + Vec2::new(x * d, y);
    if fiamma > 0.01 {
        let l = 14.0 + 46.0 * fiamma;
        p.add(Shape::convex_polygon(vec![punto(-24.0, -6.0), punto(-24.0 - l, 0.0), punto(-24.0, 6.0)], GIALLO.gamma_multiply(0.85), Stroke::NONE));
    }
    p.add(Shape::convex_polygon(
        vec![punto(-24.0, -10.0), punto(10.0, -10.0), punto(30.0, 0.0), punto(10.0, 10.0), punto(-24.0, 10.0)],
        Color32::from_rgb(210, 220, 235),
        Stroke::new(1.5, ROSA),
    ));
    p.circle_filled(punto(8.0, 0.0), 5.0, CIANO);
}

/// Disegna la scena. `avanzamento` va da 0 a 1 lungo il viaggio.
pub fn disegna(p: &Painter, area: Rect, linea: &[Evento], avanzamento: f64, tempo_reale: f64) {
    p.rect_filled(area, 0.0, FONDO);
    let durata = linea.last().map_or(1.0, |e| e.t);
    let t = avanzamento * durata;
    let (x_nave, tau_viaggio) = stato_a_tempo(linea, t);
    let beta = velocita(linea, t);
    let g = gamma(beta);
    let a_riposo = avanzamento >= 1.0;
    let beta_vis = if a_riposo { 0.0 } else { beta };

    let x_max = linea.iter().map(|e| e.x).fold(0.0, f64::max);
    let x_min = linea.iter().map(|e| e.x).fold(0.0, f64::min);
    // Distanza visibile: ci stanno 8 anni luce, come li misura chi viaggia.
    let px = area.width() / 8.0;
    let centro = Pos2::new(area.center().x, area.center().y + 20.0);
    let schermo = |x: f64| centro.x + ((x - x_nave) / g) as f32 * px;

    // Stelle di fondo: ferme nel mondo di chi resta, accorciate per chi viaggia.
    let verso = if beta_vis >= 0.0 { 1.0 } else { -1.0 };
    for i in 0..140u32 {
        let x = f64::from(x_min) - 8.0 + f64::from(caso(i, 1)) * (f64::from(x_max - x_min) + 16.0);
        let y = area.top() + caso(i, 2) * area.height();
        let sx = schermo(x);
        let luce = 0.35 + 0.65 * caso(i, 3);
        let scia = beta_vis.abs() as f32 * g as f32 * 9.0 * px * 0.05;
        if sx > area.left() - 80.0 && sx < area.right() + 80.0 {
            let colore = Color32::from_white_alpha((120.0 + 120.0 * luce) as u8);
            if scia > 1.5 {
                p.line_segment([Pos2::new(sx, y), Pos2::new(sx + verso * scia, y)], Stroke::new(1.0 + luce, colore));
            } else {
                p.circle_filled(Pos2::new(sx, y), 0.8 + 1.2 * luce, colore);
            }
        }
    }

    // Terra e destinazione.
    let terra = Pos2::new(schermo(0.0), centro.y);
    let meta = Pos2::new(schermo(x_max), centro.y);
    if terra.x > area.left() - 120.0 && terra.x < area.right() + 120.0 {
        p.circle_filled(terra, 34.0, Color32::from_rgb(40, 110, 200));
        p.circle_filled(terra + Vec2::new(-10.0, -6.0), 12.0, Color32::from_rgb(60, 160, 90));
        p.circle_filled(terra + Vec2::new(12.0, 10.0), 8.0, Color32::from_rgb(60, 160, 90));
        p.circle_stroke(terra, 34.0, Stroke::new(1.5, CIANO.gamma_multiply(0.6)));
        p.text(terra + Vec2::new(0.0, 52.0), Align2::CENTER_CENTER, "Terra", FontId::proportional(14.0), CIANO);
    }
    if x_max > 0.1 && meta.x > area.left() - 120.0 && meta.x < area.right() + 120.0 {
        let alone = 46.0 + 4.0 * (tempo_reale as f32 * 2.0).sin();
        p.circle_filled(meta, alone, GIALLO.gamma_multiply(0.15));
        p.circle_filled(meta, 26.0, GIALLO);
        p.text(meta + Vec2::new(0.0, 56.0), Align2::CENTER_CENTER, format!("meta, a {} anni luce", numero(x_max)), FontId::proportional(14.0), GIALLO);
    }

    // Una riga di anni luce dalla Terra, che mostra la contrazione.
    let y_riga = centro.y + 96.0;
    for k in 0..=(x_max.floor() as i32) {
        let q = schermo(f64::from(k));
        if q > area.left() && q < area.right() {
            p.line_segment([Pos2::new(q, y_riga - 6.0), Pos2::new(q, y_riga + 6.0)], Stroke::new(1.0, GRIGIO));
            p.text(Pos2::new(q, y_riga + 16.0), Align2::CENTER_CENTER, k.to_string(), FontId::monospace(11.0), GRIGIO);
        }
    }
    p.text(Pos2::new(area.center().x, y_riga + 34.0), Align2::CENTER_CENTER, "anni luce dalla Terra, misurati da chi viaggia", FontId::proportional(12.0), GRIGIO);

    // La nave resta al centro.
    let fiamma = if a_riposo { 0.0 } else { (beta_vis.abs() as f32).clamp(0.0, 1.0) };
    nave(p, centro, beta_vis >= 0.0, fiamma);

    // Dati in alto.
    let testo = if a_riposo {
        "Viaggio finito. Confronta le due età.".to_string()
    } else if beta_vis.abs() < 1e-6 {
        "Nave ferma".to_string()
    } else {
        format!(
            "Velocità {} c.  Per chi viaggia lo spazio è {} volte più corto e il tempo scorre {} volte più piano.",
            numero(beta_vis.abs()).replace(',', ",") + "",
            numero(g),
            numero(g)
        )
    };
    p.text(Pos2::new(area.center().x, area.top() + 22.0), Align2::CENTER_CENTER, testo, FontId::proportional(15.0), CHIARO);

    // Orologi grandi.
    let tau_casa = t;
    let largo = (area.width() * 0.5 - 24.0).min(300.0);
    let basso = area.bottom() - 16.0;
    let s1 = Rect::from_min_size(Pos2::new(area.left() + 16.0, basso - 70.0), Vec2::new(largo, 70.0));
    let s2 = Rect::from_min_size(Pos2::new(area.right() - 16.0 - largo, basso - 70.0), Vec2::new(largo, 70.0));
    scheda_eta(p, s1, tau_casa, CIANO, "Chi resta");
    scheda_eta(p, s2, tau_viaggio, ROSA, "Chi viaggia");
    if a_riposo {
        let differenza = tempo_proprio(&[Evento { t: 0.0, x: 0.0 }, Evento { t: durata, x: 0.0 }]) - tempo_proprio(linea);
        p.text(
            Pos2::new(area.center().x, basso - 35.0),
            Align2::CENTER_CENTER,
            format!("Chi viaggia è più giovane di {} anni", numero(differenza)),
            FontId::proportional(20.0),
            VERDE,
        );
    }
    let _ = TAU;
}
