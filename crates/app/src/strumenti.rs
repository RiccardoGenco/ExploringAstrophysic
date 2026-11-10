//! Strumenti di misura: condizioni iniziali, letture, grafici e registro.

use crate::formato::{grande, lunghezza, tempo};
use crate::lab::{gradi_relativi, Grafico, Lab, Oggetto};
use crate::pannello::{CHIARO, CIANO, GRIGIO, LINEA};
use crate::stato::Stato;
use colossi_fisica::buco_nero::{BucoNero, ALTEZZA_PERSONA};
use colossi_fisica::costanti::G_TERRA;
use colossi_fisica::geodetica::{
    beta_circolare, beta_fuga, potenziale_efficace, Esito, Legge, Specie,
};
use egui::{Align2, Color32, DragValue, FontId, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const ALTEZZA_GRAFICO: f32 = 130.0;

pub fn mostra(ui: &mut egui::Ui, s: &mut Stato) {
    match s.lab.indice_selezionato() {
        Some(indice) => scheda(ui, s, indice),
        None => {
            ui.label(RichText::new("Nessun oggetto selezionato").strong().color(CHIARO));
            ui.label(RichText::new("Lancia qualcosa sul banco, oppure clicca una traccia con lo strumento Misura.").color(GRIGIO));
        }
    }
    ui.add_space(10.0);
    ui.separator();
    registro(ui, s);
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(6.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

fn scheda(ui: &mut egui::Ui, s: &mut Stato, indice: usize) {
    let bh = s.buco_nero();
    let lab = &mut s.lab;

    ui.horizontal(|ui| {
        let o = &lab.oggetti[indice];
        let [r, g, b] = o.colore;
        ui.label(RichText::new(&o.nome).strong().size(16.0).color(Color32::from_rgb(r, g, b)));
        ui.label(RichText::new(o.stato_testo()).color(GRIGIO));
    });

    let mut azione: Option<fn(&mut Lab, usize)> = None;
    let mut esporta = false;
    ui.horizontal(|ui| {
        if ui.small_button("Rilancia").clicked() {
            azione = Some(Lab::rilancia);
        }
        if ui.small_button("Duplica").clicked() {
            azione = Some(Lab::duplica);
        }
        if ui.small_button("Esporta CSV").clicked() {
            esporta = true;
        }
        if ui.small_button("Elimina").clicked() {
            azione = Some(Lab::elimina);
        }
    });

    condizioni_iniziali(ui, lab, indice);
    misure(ui, &lab.oggetti[indice], &bh);
    if !lab.oggetti[indice].fisso {
        grafico(ui, lab, indice);
    }

    if esporta {
        lab.avviso = Some(match lab.esporta(indice, &bh) {
            Ok(percorso) => format!("Dati salvati in {}", percorso.display()),
            Err(e) => format!("Esportazione non riuscita: {e}"),
        });
    }
    if let Some(azione) = azione {
        azione(lab, indice);
    }
}

fn condizioni_iniziali(ui: &mut egui::Ui, lab: &mut Lab, indice: usize) {
    titoletto(ui, "CONDIZIONI DI PARTENZA");
    let legge = lab.legge;
    let o = &lab.oggetti[indice];
    let (fisso, specie) = (o.fisso, o.lancio.specie);
    let mut lancio = o.lancio;
    let mut r = lancio.raggio();
    let mut azimut = lancio.azimut().to_degrees();
    let mut beta = lancio.beta;
    let mut relativa = gradi_relativi(lancio.direzione - lancio.azimut());
    let mut cambiato = false;

    egui::Grid::new("partenza").num_columns(4).spacing([10.0, 4.0]).show(ui, |ui| {
        ui.label(RichText::new("Distanza").color(GRIGIO));
        let velocita = (r * 0.005).max(0.001);
        cambiato |= ui.add(DragValue::new(&mut r).speed(velocita).range(1.01..=15_000.0).suffix(" raggi")).changed();
        ui.label(RichText::new("Posizione").color(GRIGIO));
        cambiato |= ui.add(DragValue::new(&mut azimut).speed(0.5).suffix("°")).changed();
        ui.end_row();
        if !fisso {
            if specie == Specie::Materia {
                ui.label(RichText::new("Velocità").color(GRIGIO));
                cambiato |= ui.add(DragValue::new(&mut beta).speed(0.001).range(0.0..=0.9999).max_decimals(4).suffix(" c")).changed();
            } else {
                ui.label(RichText::new("Velocità").color(GRIGIO));
                ui.label("c");
            }
            ui.label(RichText::new("Rotta").color(GRIGIO));
            cambiato |= ui
                .add(DragValue::new(&mut relativa).speed(0.5).suffix("°"))
                .on_hover_text("0° verso l'esterno, 90° di lato, 180° verso il centro")
                .changed();
            ui.end_row();
        }
    });

    if !fisso && specie == Specie::Materia {
        ui.horizontal(|ui| {
            if ui.small_button("Orbita circolare").clicked() && r > 1.5 {
                beta = beta_circolare(r, legge).min(0.9999);
                relativa = 90.0;
                cambiato = true;
            }
            if ui.small_button("Velocità di fuga").clicked() {
                beta = beta_fuga(r, legge).min(0.9999);
                relativa = 0.0;
                cambiato = true;
            }
            if ui.small_button("Da fermo").clicked() {
                beta = 0.0;
                cambiato = true;
            }
        });
    }

    if cambiato {
        let a = azimut.to_radians();
        lancio.posizione = [r * a.cos(), r * a.sin()];
        lancio.beta = beta;
        lancio.direzione = a + relativa.to_radians();
        lab.modifica(indice, lancio);
    }
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String, reale: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.label(RichText::new(reale).color(GRIGIO));
    ui.end_row();
}

fn numero(x: f64) -> String {
    format!("{x:.4}").replace('.', ",")
}

fn misure(ui: &mut egui::Ui, o: &Oggetto, bh: &BucoNero) {
    titoletto(ui, "LETTURE");
    let m = &o.moto;
    let r = m.raggio();
    let unita = bh.tempo_luce();
    let luce = o.lancio.specie == Specie::Luce && !o.fisso;
    let oltre = matches!(m.esito, Esito::OltreOrizzonte | Esito::Singolarita | Esito::AllOrizzonte);

    egui::Grid::new("letture").num_columns(3).spacing([12.0, 3.0]).show(ui, |ui| {
        riga(ui, "Distanza", format!("{} raggi", numero(r)), lunghezza(r * bh.raggio()));
        if let Some(b) = m.beta() {
            let gamma = 1.0 / (1.0 - (b * b).min(0.999_999_9)).sqrt();
            if luce {
                riga(ui, "Velocità locale", "1 c".to_owned(), String::new());
            } else {
                riga(ui, "Velocità locale", format!("{} c", numero(b)), format!("γ = {}", numero(gamma)));
            }
        }
        if !luce {
            riga(ui, "Il suo orologio", numero(m.s), tempo(m.s * unita));
        }
        let lontano = if oltre { "∞".to_owned() } else { numero(m.t) };
        let reale = if oltre { "non arriva mai".to_owned() } else { tempo(m.t * unita) };
        riga(ui, "Orologio lontano", lontano, reale);
        if let Some(ritmo) = m.ritmo() {
            riga(ui, "Ritmo rispetto a lontano", numero(ritmo), String::new());
        }

        if !o.fisso {
            riga(ui, "Momento angolare", numero(m.momento_angolare()), String::new());
            riga(ui, "Energia conservata", numero(m.energia()), String::new());
            riga(ui, "Errore numerico energia", format!("{:.1e}", m.errore_energia()), String::new());
            if let Some(anticipo) = m.anticipo_periastro() {
                riga(
                    ui,
                    "Avanzamento per giro",
                    format!("{:.3}°", anticipo.to_degrees()).replace('.', ","),
                    format!("{} passaggi", m.periastri.len()),
                );
            }
            if luce {
                riga(ui, "Parametro d'impatto", numero(m.momento_angolare() / m.energia()), String::new());
                riga(ui, "Deviazione", format!("{:.3}°", m.deviazione().to_degrees()).replace('.', ","), String::new());
            }
        }
        if !luce && r > 0.0 {
            let marea = bh.marea(r * bh.raggio(), ALTEZZA_PERSONA) / G_TERRA;
            let testo = if marea < 0.01 { "impercettibile".to_owned() } else { format!("{} g", grande(marea)) };
            riga(ui, "Marea su una persona", testo, String::new());
        }
    });
}

fn grafico(ui: &mut egui::Ui, lab: &mut Lab, indice: usize) {
    titoletto(ui, "GRAFICO");
    let luce = lab.oggetti[indice].lancio.specie == Specie::Luce;
    ui.horizontal(|ui| {
        let g = &mut lab.grafico;
        ui.selectable_value(g, Grafico::RaggioLontano, "r / t lontano");
        if !luce {
            ui.selectable_value(g, Grafico::RaggioProprio, "r / suo tempo");
            ui.selectable_value(g, Grafico::Ritmo, "ritmo");
        }
        ui.selectable_value(g, Grafico::Potenziale, "potenziale");
    });
    if luce && matches!(lab.grafico, Grafico::RaggioProprio | Grafico::Ritmo) {
        lab.grafico = Grafico::RaggioLontano;
    }

    let (area, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ALTEZZA_GRAFICO),
        Sense::hover(),
    );
    let o = &lab.oggetti[indice];
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, Color32::from_rgb(9, 11, 17));
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let tinta = Color32::from_rgb(o.colore[0], o.colore[1], o.colore[2]);
    let dentro = area.shrink2(Vec2::new(8.0, 14.0));

    let (punti, nomi): (Vec<[f64; 2]>, [&str; 2]) = match lab.grafico {
        Grafico::RaggioLontano => (o.scia.iter().map(|c| [c.t, c.r]).collect(), ["t lontano", "distanza"]),
        Grafico::RaggioProprio => (o.scia.iter().map(|c| [c.s, c.r]).collect(), ["suo tempo", "distanza"]),
        Grafico::Ritmo => (
            o.scia
                .windows(2)
                .filter(|c| c[1].t > c[0].t)
                .map(|c| [c[1].t, (c[1].s - c[0].s) / (c[1].t - c[0].t)])
                .collect(),
            ["t lontano", "ritmo"],
        ),
        Grafico::Potenziale => {
            potenziale(&p, dentro, o, tinta);
            return;
        }
    };
    curva(&p, dentro, &punti, tinta, nomi);
}

fn estremi(valori: impl Iterator<Item = f64>) -> (f64, f64) {
    let (mut basso, mut alto) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in valori.filter(|v| v.is_finite()) {
        basso = basso.min(v);
        alto = alto.max(v);
    }
    if !basso.is_finite() {
        return (0.0, 1.0);
    }
    if alto - basso < 1e-9 {
        return (basso - 0.5, alto + 0.5);
    }
    (basso, alto)
}

fn in_area(area: Rect, x: f64, y: f64, ex: (f64, f64), ey: (f64, f64)) -> Pos2 {
    let u = (x - ex.0) / (ex.1 - ex.0);
    let v = (y - ey.0) / (ey.1 - ey.0);
    Pos2::new(
        area.left() + area.width() * u as f32,
        area.bottom() - area.height() * v as f32,
    )
}

fn curva(p: &egui::Painter, area: Rect, punti: &[[f64; 2]], tinta: Color32, nomi: [&str; 2]) {
    if punti.len() < 2 {
        return;
    }
    let ex = estremi(punti.iter().map(|q| q[0]));
    let ey = estremi(punti.iter().map(|q| q[1]));
    let passo = (punti.len() / 800).max(1);
    let linea: Vec<Pos2> = punti
        .iter()
        .step_by(passo)
        .filter(|q| q[1].is_finite())
        .map(|q| in_area(area, q[0], q[1], ex, ey))
        .collect();
    p.add(Shape::line(linea, Stroke::new(1.6, tinta)));
    assi(p, area, ex, ey, nomi);
}

fn assi(p: &egui::Painter, area: Rect, ex: (f64, f64), ey: (f64, f64), nomi: [&str; 2]) {
    let f = FontId::monospace(10.0);
    let g = GRIGIO;
    let bordo = area.expand2(Vec2::new(4.0, 12.0));
    p.text(bordo.left_top(), Align2::LEFT_TOP, format!("{}  max {:.3}", nomi[1], ey.1).replace('.', ","), f.clone(), g);
    p.text(bordo.left_bottom(), Align2::LEFT_BOTTOM, format!("{:.3}", ey.0).replace('.', ","), f.clone(), g);
    p.text(bordo.right_bottom(), Align2::RIGHT_BOTTOM, format!("{} {:.1}", nomi[0], ex.1).replace('.', ","), f, g);
}

/// La curva che la traiettoria non puo' scavalcare, con l'oggetto sopra.
fn potenziale(p: &egui::Painter, area: Rect, o: &Oggetto, tinta: Color32) {
    let m = &o.moto;
    let (minimo, massimo) = estremi(o.scia.iter().map(|c| c.r));
    let basso = match m.legge {
        Legge::Einstein => (minimo * 0.6).max(1.02),
        Legge::Newton => (minimo * 0.5).max(0.5),
    };
    let alto = (massimo * 1.6).max(basso * 3.0);
    let (lb, la) = (basso.ln(), alto.ln());
    let n = 240;
    let curva: Vec<[f64; 2]> = (0..=n)
        .map(|i| {
            let r = (lb + (la - lb) * f64::from(i) / f64::from(n)).exp();
            [r.ln(), potenziale_efficace(r, m.l2(), m.specie, m.legge)]
        })
        .collect();
    let livello = match m.legge {
        Legge::Einstein => m.energia() * m.energia(),
        Legge::Newton => m.energia(),
    };

    let ex = (lb, la);
    let (b, a) = estremi(curva.iter().map(|q| q[1]).chain([livello]));
    let margine = (a - b) * 0.08;
    let ey = (b - margine, a + margine);

    let linea: Vec<Pos2> = curva.iter().map(|q| in_area(area, q[0], q[1], ex, ey)).collect();
    p.add(Shape::line(linea, Stroke::new(1.6, CIANO)));
    let y = in_area(area, lb, livello, ex, ey).y;
    p.add(Shape::dashed_line(
        &[Pos2::new(area.left(), y), Pos2::new(area.right(), y)],
        Stroke::new(1.0, CHIARO.gamma_multiply(0.6)),
        5.0,
        4.0,
    ));

    let r = m.raggio().clamp(basso, alto);
    p.circle_filled(in_area(area, r.ln(), livello, ex, ey), 4.5, tinta);
    assi(p, area, (basso, alto), ey, ["distanza", "potenziale efficace"]);
}

fn registro(ui: &mut egui::Ui, s: &mut Stato) {
    let unita = s.buco_nero().tempo_luce();
    let lab = &mut s.lab;
    titoletto(ui, "REGISTRO");
    if lab.oggetti.is_empty() {
        ui.label(RichText::new("Vuoto.").color(GRIGIO));
        return;
    }
    let mut scelto = None;
    egui::Grid::new("registro").num_columns(5).spacing([10.0, 3.0]).striped(true).show(ui, |ui| {
        for intestazione in ["", "Distanza", "Suo tempo", "Ritmo", "Stato"] {
            ui.label(RichText::new(intestazione).small().color(GRIGIO));
        }
        ui.end_row();
        for o in &lab.oggetti {
            let [r, g, b] = o.colore;
            let attivo = lab.selezionato == Some(o.id);
            let etichetta = RichText::new(&o.nome).color(Color32::from_rgb(r, g, b));
            if ui.selectable_label(attivo, etichetta).clicked() {
                scelto = Some(o.id);
            }
            ui.label(RichText::new(numero(o.moto.raggio())).monospace());
            if o.lancio.specie == Specie::Luce && !o.fisso {
                ui.label(RichText::new("—").color(GRIGIO));
            } else {
                ui.label(RichText::new(tempo(o.moto.s * unita)).monospace());
            }
            match o.moto.ritmo() {
                Some(ritmo) if !o.moto.esito.fermo() || o.fisso => {
                    ui.label(RichText::new(numero(ritmo)).monospace())
                }
                _ => ui.label(RichText::new("—").color(GRIGIO)),
            };
            ui.label(RichText::new(o.stato_testo()).small().color(GRIGIO));
            ui.end_row();
        }
    });
    if scelto.is_some() {
        lab.selezionato = scelto;
    }
}

