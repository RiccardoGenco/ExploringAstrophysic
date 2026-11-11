//! La pagina d'ingresso: quattro aree, le visite guidate e due righe su come si usa.

use crate::guida;
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use crate::stato::{Stato, AREE};
use crate::visite;
use egui::{Align2, Color32, CornerRadius, FontId, Frame, Margin, Painter, Pos2, Rect, RichText, Sense, Stroke, UiBuilder, Vec2};
use std::f32::consts::TAU;

const ALTEZZA_SCHEDA: f32 = 262.0;
const SPAZIO: f32 = 18.0;
const ROSA: Color32 = Color32::from_rgb(255, 130, 190);

/// Un piccolo disegno per ogni area.
fn icona(p: &Painter, c: Pos2, area: usize) {
    match area {
        0 => {
            p.circle_filled(c, 26.0, Color32::BLACK);
            p.circle_stroke(c, 26.0, Stroke::new(2.0, CIANO));
            for k in 0..3 {
                let r = 34.0 + 7.0 * k as f32;
                p.circle_stroke(c, r, Stroke::new(1.0, GIALLO.gamma_multiply(0.7 - 0.2 * k as f32)));
            }
        }
        1 => {
            for (dx, tinta) in [(-24.0, CIANO), (24.0, ROSA)] {
                let centro = c + Vec2::new(dx, 0.0);
                p.circle_stroke(centro, 20.0, Stroke::new(2.0, tinta));
                p.line_segment([centro, centro + Vec2::new(0.0, -13.0)], Stroke::new(2.0, tinta));
                let a = if dx < 0.0 { 1.2_f32 } else { 0.4 };
                p.line_segment([centro, centro + Vec2::new(a.cos(), -a.sin()) * 9.0], Stroke::new(2.0, tinta));
            }
        }
        2 => {
            p.circle_stroke(c, 28.0, Stroke::new(1.4, LINEA.gamma_multiply(2.5)));
            let punti: Vec<Pos2> = (0..=40)
                .map(|i| {
                    let a = i as f32 / 40.0 * TAU;
                    c + Vec2::new(a.cos() * 30.0, a.sin() * 14.0)
                })
                .collect();
            p.add(egui::Shape::line(punti, Stroke::new(2.0, Color32::from_rgb(255, 160, 90))));
            for k in 0..7 {
                let a = k as f32 * 0.9;
                let q = c + Vec2::new(a.cos() * 30.0, a.sin() * 14.0) + Vec2::new(1.5, -1.0);
                p.line_segment([q - Vec2::new(3.0, 0.0), q + Vec2::new(3.0, 0.0)], Stroke::new(1.2, CHIARO));
                p.line_segment([q - Vec2::new(0.0, 3.0), q + Vec2::new(0.0, 3.0)], Stroke::new(1.2, CHIARO));
            }
        }
        _ => {
            for (k, tinta) in [CIANO, Color32::from_rgb(255, 160, 90), ROSA].into_iter().enumerate() {
                let a = k as f32 / 3.0 * TAU + 0.4;
                let q = c + Vec2::new(a.cos(), a.sin()) * 22.0;
                let punti: Vec<Pos2> = (0..=24)
                    .map(|i| {
                        let b = a + i as f32 / 24.0 * 2.2;
                        c + Vec2::new(b.cos(), b.sin()) * (14.0 + i as f32 * 0.6)
                    })
                    .collect();
                p.add(egui::Shape::line(punti, Stroke::new(1.2, tinta.gamma_multiply(0.6))));
                p.circle_filled(q, 6.0, tinta);
            }
        }
    }
}

pub fn mostra(ui: &mut egui::Ui, s: &mut Stato) {
    egui::CentralPanel::default().frame(Frame::new().fill(FONDO)).show(ui, |ui| {
        egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| contenuto(ui, s));
    });
}

fn contenuto(ui: &mut egui::Ui, s: &mut Stato) {
    let larghezza = ui.available_width().min(1240.0);
    let margine = ((ui.available_width() - larghezza) * 0.5).max(16.0);
    ui.add_space(26.0);
    ui.horizontal(|ui| {
        ui.add_space(margine);
        ui.vertical(|ui| {
            ui.set_width(larghezza - 32.0);
            ui.label(RichText::new("Colossi").size(38.0).strong().color(CIANO));
            ui.label(
                RichText::new("Un laboratorio per toccare con mano le cose più grandi dell'universo: buchi neri, stelle che crollano, il tempo che rallenta, la luce più antica.")
                    .size(17.0)
                    .color(CHIARO),
            );
            ui.label(RichText::new("Non servono formule. Ogni cosa che vedi è calcolata, e ogni banco ti dice cosa provare.").size(15.0).color(GRIGIO));
            ui.add_space(22.0);
            visite_in_evidenza(ui, s, larghezza - 32.0);
            ui.add_space(26.0);
            ui.label(RichText::new("Oppure scegli un banco").size(19.0).strong().color(CHIARO));
            ui.add_space(8.0);
            schede(ui, s, larghezza - 32.0);
            ui.add_space(26.0);
            consigli(ui);
            ui.add_space(30.0);
        });
    });
}

fn schede(ui: &mut egui::Ui, s: &mut Stato, larghezza: f32) {
    let colonne = if larghezza > 1000.0 { 4 } else { 2 };
    let w = (larghezza - SPAZIO * (colonne as f32 - 1.0)) / colonne as f32;
    let righe = AREE.len().div_ceil(colonne);
    let (area_tot, _) = ui.allocate_exact_size(Vec2::new(larghezza, righe as f32 * (ALTEZZA_SCHEDA + SPAZIO)), Sense::hover());
    let mut scelta = None;
    for (i, area) in AREE.iter().enumerate() {
        let (r, c) = (i / colonne, i % colonne);
        let rett = Rect::from_min_size(
            area_tot.min + Vec2::new(c as f32 * (w + SPAZIO), r as f32 * (ALTEZZA_SCHEDA + SPAZIO)),
            Vec2::new(w, ALTEZZA_SCHEDA),
        );
        let risposta = ui.interact(rect_testa(rett), ui.id().with(("area", i)), Sense::click());
        let sopra = risposta.hovered();
        let p = ui.painter();
        p.rect_filled(rett, CornerRadius::same(10), if sopra { SUPERFICIE.gamma_multiply(1.4) } else { SUPERFICIE });
        p.rect_stroke(rett, CornerRadius::same(10), Stroke::new(1.0, if sopra { CIANO } else { LINEA }), egui::StrokeKind::Inside);
        icona(p, rett.center_top() + Vec2::new(0.0, 58.0), i);
        if risposta.clicked() {
            scelta = Some(area.banchi[0].0);
        }

        let interno = Rect::from_min_max(rett.min + Vec2::new(16.0, 112.0), rett.max - Vec2::new(16.0, 12.0));
        let mut figlio = ui.new_child(UiBuilder::new().max_rect(interno));
        figlio.label(RichText::new(area.nome).size(19.0).strong().color(CHIARO));
        figlio.add_space(2.0);
        figlio.label(RichText::new(area.descrizione).color(GRIGIO));
        figlio.add_space(8.0);
        figlio.horizontal_wrapped(|ui| {
            for (stanza, nome) in area.banchi {
                let n = guida::fatte(s, *stanza);
                let totale = guida::guida(*stanza).map_or(0, |g| g.sfide.len());
                let tinta = if totale > 0 && n == totale { VERDE } else { CIANO };
                if ui.link(RichText::new(*nome).color(tinta)).clicked() {
                    scelta = Some(*stanza);
                }
            }
        });
    }
    if let Some(stanza) = scelta {
        s.stanza = stanza;
    }
}

/// La parte alta della scheda, quella col disegno: cliccandola si entra.
fn rect_testa(r: Rect) -> Rect {
    Rect::from_min_max(r.min, Pos2::new(r.max.x, r.min.y + 140.0))
}

/// Le visite guidate in cima: sono il modo piu' facile per cominciare.
fn visite_in_evidenza(ui: &mut egui::Ui, s: &mut Stato, larghezza: f32) {
    ui.label(RichText::new("Comincia da qui").size(19.0).strong().color(CHIARO));
    ui.label(RichText::new("Una visita ti fa provare le cose davvero: ti indica il comando e va avanti quando ci riesci.").color(GRIGIO));
    ui.add_space(8.0);
    let colonne = if larghezza > 900.0 { 3 } else { 1 };
    let w = ((larghezza - SPAZIO * (colonne as f32 - 1.0)) / colonne as f32).floor() - 8.0;
    let mut scelta = None;
    ui.horizontal_top(|ui| {
        for (i, v) in visite::VISITE.iter().enumerate().take(colonne) {
            visita_scheda(ui, v, w, &mut scelta, i);
            ui.add_space(SPAZIO);
        }
    });
    if colonne == 1 {
        for (i, v) in visite::VISITE.iter().enumerate().skip(1) {
            ui.add_space(SPAZIO);
            visita_scheda(ui, v, w, &mut scelta, i);
        }
    }
    if let Some(i) = scelta {
        visite::inizia(s, i);
    }
}

fn visita_scheda(ui: &mut egui::Ui, v: &visite::Visita, w: f32, scelta: &mut Option<usize>, i: usize) {
    ui.vertical(|ui| {
        Frame::new()
            .fill(SUPERFICIE)
            .stroke(Stroke::new(1.0, CIANO.gamma_multiply(0.6)))
            .corner_radius(10)
            .inner_margin(Margin::same(14))
            .show(ui, |ui| {
                ui.set_width(w - 30.0);
                ui.set_min_height(150.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new(v.durata.to_uppercase()).small().strong().color(GRIGIO));
                    ui.label(RichText::new(v.nome).size(19.0).strong().color(CIANO));
                    ui.add_space(2.0);
                    ui.add(egui::Label::new(RichText::new(v.descrizione).color(CHIARO)).wrap());
                    ui.add_space(10.0);
                    if ui.add(egui::Button::new(RichText::new("Comincia").strong()).min_size(Vec2::new(120.0, 32.0))).clicked() {
                        *scelta = Some(i);
                    }
                });
            });
    });
}

fn consigli(ui: &mut egui::Ui) {
    ui.add_space(22.0);
    ui.label(RichText::new("Come si usa").size(19.0).strong().color(CHIARO));
    ui.add_space(6.0);
    for (titolo, testo) in [
        ("Trascina", "Quasi tutto si sposta col mouse: lanci, partenze, curve, il punto di vista."),
        ("Rotella", "Avvicina e allontana nelle mappe e nei telescopi."),
        ("La guida", "In fondo a ogni banco: poche cose da provare, che si spuntano da sole quando ci riesci."),
        ("I numeri", "Dove un numero è approssimato o un colore è inventato, il banco lo scrive."),
    ] {
        Frame::new().inner_margin(Margin::symmetric(0, 3)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_sized([110.0, 18.0], egui::Label::new(RichText::new(titolo).strong().color(CIANO)));
                ui.label(RichText::new(testo).color(CHIARO));
            });
        });
    }
    let _ = (FontId::default(), Align2::LEFT_TOP);
}
