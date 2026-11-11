//! La scena dei gemelli: la nave al centro, le stelle che scorrono, due orologi grandi.
//! Le distanze sono quelle che vede chi viaggia, quindi accorciate dal fattore γ.

use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, VERDE};
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
    let punto = |x: f32, y: f32| c + Vec2::new(x * d * 1.5, y * 1.5);
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

fn pianeta(p: &Painter, c: Pos2, r: f32) {
    p.circle_filled(c, r, Color32::from_rgb(40, 110, 200));
    p.circle_filled(c + Vec2::new(-r * 0.3, -r * 0.2), r * 0.35, Color32::from_rgb(60, 160, 90));
    p.circle_filled(c + Vec2::new(r * 0.35, r * 0.3), r * 0.25, Color32::from_rgb(60, 160, 90));
    p.circle_stroke(c, r, Stroke::new(1.5, CIANO.gamma_multiply(0.6)));
}

fn stella(p: &Painter, c: Pos2, r: f32, tempo: f64) {
    let pulsa = 1.0 + 0.08 * (tempo as f32 * 2.0).sin();
    for k in (1..=4).rev() {
        p.circle_filled(c, r * (1.0 + 0.45 * k as f32) * pulsa, GIALLO.gamma_multiply(0.07));
    }
    p.circle_filled(c, r, GIALLO);
}

/// Una pista con i tick ogni anno luce. `lunghezza_ly` e' quanto misura la pista per chi guarda.
#[allow(clippy::too_many_arguments)]
fn pista(p: &Painter, riquadro: Rect, x0: f32, x1: f32, y: f32, ly: f64, frazione: f64, verso_destra: bool, fiamma: f32, tempo: f64, titolo: &str, nota: &str) {
    p.text(Pos2::new(riquadro.left() + 30.0, y - 72.0), Align2::LEFT_CENTER, titolo, FontId::proportional(15.0), CHIARO);
    p.text(Pos2::new(riquadro.right() - 30.0, y - 72.0), Align2::RIGHT_CENTER, nota, FontId::proportional(15.0), CIANO);
    p.line_segment([Pos2::new(x0, y), Pos2::new(x1, y)], Stroke::new(2.0, LINEA));
    let passi = ly.floor() as i32;
    for k in 0..=passi {
        let q = x0 + (x1 - x0) * (f64::from(k) / ly) as f32;
        p.line_segment([Pos2::new(q, y - 5.0), Pos2::new(q, y + 5.0)], Stroke::new(1.0, GRIGIO));
    }
    pianeta(p, Pos2::new(x0 - 30.0, y), 28.0);
    stella(p, Pos2::new(x1 + 30.0, y), 21.0, tempo);
    let nave_x = x0 + (x1 - x0) * frazione as f32;
    nave(p, Pos2::new(nave_x, y - 2.0), verso_destra, fiamma);
}

/// Disegna la scena. `avanzamento` va da 0 a 1 lungo il viaggio.
pub fn disegna(p: &Painter, area: Rect, linea: &[Evento], avanzamento: f64, tempo_reale: f64) {
    p.rect_filled(area, 0.0, FONDO);
    for i in 0..110u32 {
        let x = area.left() + caso(i, 1) * area.width();
        let y = area.top() + caso(i, 2) * area.height();
        let lampeggia = 0.55 + 0.45 * ((tempo_reale as f32) * (0.6 + caso(i, 4)) + caso(i, 5) * 6.0).sin();
        p.circle_filled(Pos2::new(x, y), 0.6 + caso(i, 3) * 1.0, Color32::from_white_alpha((70.0 + 90.0 * lampeggia) as u8));
    }
    let durata = linea.last().map_or(1.0, |e| e.t);
    let t = avanzamento * durata;
    let (x_nave, tau_viaggio) = stato_a_tempo(linea, t);
    let beta = velocita(linea, t);
    let a_riposo = avanzamento >= 1.0;
    let g = if a_riposo { 1.0 } else { gamma(beta) };
    let x_max = linea.iter().map(|e| e.x).fold(0.1, f64::max);
    let frazione = (x_nave / x_max).clamp(0.0, 1.0);
    let destra = beta >= 0.0;
    let fiamma = if a_riposo { 0.0 } else { (beta.abs() as f32).clamp(0.0, 1.0) };

    let margine = 110.0;
    let (x0, x1) = (area.left() + margine, area.right() - margine);
    let lung = x1 - x0;

    // Visto da Terra: la pista ha la lunghezza vera.
    let y1 = area.top() + area.height() * 0.27;
    pista(p, area, x0, x1, y1, x_max, frazione, destra, fiamma, tempo_reale, "VISTO DA TERRA", &format!("{} anni luce", numero(x_max)));

    // Visto dalla nave: la stessa pista, accorciata di γ attorno al centro.
    let y2 = area.top() + area.height() * 0.58;
    let corta = lung / g as f32;
    let c = (x0 + x1) * 0.5;
    let (s0, s1) = (c - corta * 0.5, c + corta * 0.5);
    p.line_segment([Pos2::new(x0 - 30.0, y1 + 30.0), Pos2::new(s0 - 30.0, y2 - 30.0)], Stroke::new(1.0, GRIGIO.gamma_multiply(0.35)));
    p.line_segment([Pos2::new(x1 + 30.0, y1 + 30.0), Pos2::new(s1 + 30.0, y2 - 30.0)], Stroke::new(1.0, GRIGIO.gamma_multiply(0.35)));
    let nota = if a_riposo || beta.abs() < 1e-6 { "ferma: nessun accorciamento".to_string() } else { format!("{} anni luce: più corta di {} volte", numero(x_max / g), numero(g)) };
    pista(p, area, s0, s1, y2, x_max, frazione, destra, 0.0, tempo_reale, "VISTO DALLA NAVE", &nota);

    // Cosa succede.
    let frase = if a_riposo {
        "Viaggio finito. Confronta le due età.".to_string()
    } else if beta.abs() < 1e-6 {
        "La nave è ferma.".to_string()
    } else {
        format!("La nave va a {} c: per chi viaggia il tempo scorre {} volte più lentamente.", numero(beta.abs()), numero(g))
    };
    p.text(Pos2::new(area.center().x, area.top() + 24.0), Align2::CENTER_CENTER, frase, FontId::proportional(17.0), CHIARO);

    // Età.
    let largo = (area.width() * 0.5 - 24.0).min(320.0);
    let basso = area.bottom() - 16.0;
    let s1r = Rect::from_min_size(Pos2::new(area.left() + 16.0, basso - 70.0), Vec2::new(largo, 70.0));
    let s2r = Rect::from_min_size(Pos2::new(area.right() - 16.0 - largo, basso - 70.0), Vec2::new(largo, 70.0));
    scheda_eta(p, s1r, t, CIANO, "Chi resta");
    scheda_eta(p, s2r, tau_viaggio, ROSA, "Chi viaggia");
    if a_riposo {
        let differenza = durata - tempo_proprio(linea);
        p.text(
            Pos2::new(area.center().x, basso - 100.0),
            Align2::CENTER_CENTER,
            format!("Chi viaggia è più giovane di {} anni", numero(differenza)),
            FontId::proportional(22.0),
            VERDE,
        );
    }
    let _ = TAU;
}
