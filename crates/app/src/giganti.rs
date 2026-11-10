//! Banco dei giganti: far crescere un seme abbastanza in fretta.

use crate::formato::{decimale, densita, grande, lunghezza};
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use colossi_fisica::buco_nero::{BucoNero, RIFERIMENTI};
use colossi_fisica::crescita::{redshift, tempo_cosmico, Quasar, Ritmo, QUASAR};
use egui::{Align2, Color32, FontId, Frame, Margin, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const LARGHEZZA: f32 = 420.0;
const T_MAX: f64 = 1200.0;
const LOG_MIN: f64 = 1.0;
const LOG_MAX: f64 = 11.0;
const ALTEZZA_SCALA: f32 = 220.0;
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);
const ARANCIO: Color32 = Color32::from_rgb(255, 160, 90);
const PRESA: f32 = 14.0;
const MASSA_MASSIMA_SEME: f64 = 1e9;

const SEMI: [(&str, f64, f64); 3] = [
    ("Resto di una prima stella", 30.0, 100.0),
    ("Ammasso che si fonde", 20.0, 3e3),
    ("Nube che crolla direttamente", 20.0, 1e5),
];

pub struct Giganti {
    /// Eta' dell'universo alla nascita del seme, in milioni di anni.
    pub t_seme: f64,
    pub massa_seme: f64,
    efficienza: f64,
    eddington: f64,
    /// Istante osservato, in milioni di anni.
    pub t_cursore: f64,
    trascina_seme: bool,
}

impl Default for Giganti {
    fn default() -> Self {
        Self {
            t_seme: tempo_cosmico(SEMI[0].1),
            massa_seme: SEMI[0].2,
            efficienza: 0.1,
            eddington: 1.0,
            t_cursore: tempo_cosmico(QUASAR[0].z),
            trascina_seme: false,
        }
    }
}

impl Giganti {
    fn ritmo(&self) -> Ritmo {
        Ritmo { efficienza: self.efficienza, eddington: self.eddington }
    }

    fn massa_a(&self, t: f64) -> f64 {
        self.ritmo().massa_dopo(self.massa_seme, (t - self.t_seme).max(0.0))
    }
}

fn a_schermo(piano: Rect, t: f64, log_m: f64) -> Pos2 {
    Pos2::new(
        piano.left() + piano.width() * (t / T_MAX) as f32,
        piano.bottom() - piano.height() * ((log_m - LOG_MIN) / (LOG_MAX - LOG_MIN)) as f32,
    )
}

fn da_schermo(piano: Rect, q: Pos2) -> (f64, f64) {
    (
        f64::from((q.x - piano.left()) / piano.width()) * T_MAX,
        LOG_MIN + f64::from((piano.bottom() - q.y) / piano.height()) * (LOG_MAX - LOG_MIN),
    )
}

/// Restituisce la massa da aprire nel banco del buco nero, se richiesto.
pub fn mostra(ui: &mut egui::Ui, g: &mut Giganti) -> Option<f64> {
    let mut apri = None;
    egui::Panel::right("colonna_giganti")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| apri = strumenti(ui, g));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        grafico(ui, area, g);
    });
    apri
}

fn griglia(p: &Painter, piano: Rect) {
    let f = FontId::monospace(10.0);
    let mut t = 0;
    while t <= T_MAX as i32 {
        let x = a_schermo(piano, f64::from(t), LOG_MIN).x;
        p.line_segment([Pos2::new(x, piano.top()), Pos2::new(x, piano.bottom())], Stroke::new(1.0, LINEA));
        p.text(Pos2::new(x, piano.bottom() + 5.0), Align2::CENTER_TOP, format!("{t}"), f.clone(), GRIGIO);
        if t > 0 {
            let z = redshift(f64::from(t));
            p.text(Pos2::new(x, piano.top() - 3.0), Align2::CENTER_BOTTOM, format!("z {z:.1}").replace('.', ","), f.clone(), GRIGIO.gamma_multiply(0.8));
        }
        t += 200;
    }
    for e in 1..=11 {
        let y = a_schermo(piano, 0.0, f64::from(e)).y;
        p.line_segment([Pos2::new(piano.left(), y), Pos2::new(piano.right(), y)], Stroke::new(1.0, LINEA));
        p.text(Pos2::new(piano.left() - 6.0, y), Align2::RIGHT_CENTER, format!("10^{e}"), f.clone(), GRIGIO);
    }
    p.text(piano.right_bottom() + Vec2::new(0.0, 20.0), Align2::RIGHT_TOP, "età dell'universo, milioni di anni", f.clone(), GRIGIO);
    p.text(piano.left_top() + Vec2::new(4.0, 4.0), Align2::LEFT_TOP, "massa, in masse solari", f, GRIGIO);
}

fn quasar_raggiunto(g: &Giganti, q: &Quasar) -> bool {
    let t = tempo_cosmico(q.z);
    t > g.t_seme && g.massa_a(t) >= q.massa - q.errore
}

fn grafico(ui: &mut egui::Ui, area: Rect, g: &mut Giganti) {
    let risposta = ui.interact(area, ui.id().with("giganti"), Sense::click_and_drag());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let piano = Rect::from_min_max(area.min + Vec2::new(58.0, 78.0), area.max - Vec2::new(22.0, 46.0));

    if risposta.drag_started_by(PointerButton::Primary) {
        let premuto = ui.input(|i| i.pointer.press_origin()).unwrap_or(area.center());
        let seme = a_schermo(piano, g.t_seme, g.massa_seme.log10());
        g.trascina_seme = (seme - premuto).length() < PRESA;
    }
    if risposta.is_pointer_button_down_on() {
        if let Some(q) = ui.input(|i| i.pointer.latest_pos()) {
            let (t, log_m) = da_schermo(piano, q);
            if g.trascina_seme {
                g.t_seme = t.clamp(tempo_cosmico(60.0), T_MAX * 0.8);
                g.massa_seme = 10f64.powf(log_m.clamp(LOG_MIN, MASSA_MASSIMA_SEME.log10()));
            } else {
                g.t_cursore = t.clamp(1.0, T_MAX);
            }
        }
    }
    if risposta.drag_stopped_by(PointerButton::Primary) {
        g.trascina_seme = false;
    }

    griglia(&p, piano);

    // La crescita.
    let n = 200;
    let inizio = g.t_seme;
    let punti: Vec<Pos2> = (0..=n)
        .filter_map(|i| {
            let t = inizio + (T_MAX - inizio) * f64::from(i) / f64::from(n);
            let m = g.massa_a(t).log10();
            (m <= LOG_MAX).then(|| a_schermo(piano, t, m))
        })
        .collect();
    p.add(Shape::line(punti, Stroke::new(3.0, ARANCIO)));

    // Quasar misurati.
    for (i, q) in QUASAR.iter().enumerate() {
        let t = tempo_cosmico(q.z);
        let ok = quasar_raggiunto(g, q);
        let tinta = if ok { VERDE } else { ROSSO };
        let c = a_schermo(piano, t, q.massa.log10());
        if q.errore > 0.0 {
            p.line_segment(
                [
                    a_schermo(piano, t, (q.massa - q.errore).log10()),
                    a_schermo(piano, t, (q.massa + q.errore).log10()),
                ],
                Stroke::new(1.8, tinta),
            );
        }
        p.circle_filled(c, 5.0, tinta);
        // Due quasar sono quasi nello stesso punto: etichette su lati opposti.
        let (scarto, allineamento) = if i == 1 {
            (Vec2::new(9.0, 3.0), Align2::LEFT_TOP)
        } else {
            (Vec2::new(-8.0, -5.0), Align2::RIGHT_BOTTOM)
        };
        p.text(c + scarto, allineamento, q.nome, FontId::proportional(11.5), tinta);
    }

    // Seme e istante osservato.
    let seme = a_schermo(piano, g.t_seme, g.massa_seme.log10());
    p.circle_filled(seme, 8.0, ARANCIO);
    p.circle_stroke(seme, 12.0, Stroke::new(1.2, CHIARO));
    p.text(seme + Vec2::new(14.0, 8.0), Align2::LEFT_TOP, "seme: trascinami", FontId::proportional(11.5), CHIARO);

    let x = a_schermo(piano, g.t_cursore, LOG_MIN).x;
    p.line_segment([Pos2::new(x, piano.top()), Pos2::new(x, piano.bottom())], Stroke::new(1.0, CIANO.gamma_multiply(0.7)));
    let massa = g.massa_a(g.t_cursore);
    if massa.log10() <= LOG_MAX && g.t_cursore >= g.t_seme {
        p.circle_filled(a_schermo(piano, g.t_cursore, massa.log10()), 5.5, CIANO);
    }

    p.text(
        area.left_top() + Vec2::new(14.0, 12.0),
        Align2::LEFT_TOP,
        "Trascina il seme nel tempo e nella massa, clicca altrove per guardare un altro istante. Il ritmo si cambia a destra.",
        FontId::proportional(13.0),
        GRIGIO,
    );
    p.text(
        area.left_top() + Vec2::new(14.0, 32.0),
        Align2::LEFT_TOP,
        "I punti sono buchi neri misurati, già enormi quando l'universo era giovane. Il seme deve arrivarci in tempo.",
        FontId::proportional(13.0),
        CIANO,
    );
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

/// Orizzonte e orbite familiari, in scala vera.
fn scala(ui: &mut egui::Ui, bh: &BucoNero) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_SCALA), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let centro = area.center();
    let rs = bh.raggio();
    let raggio_orizzonte = area.height() * 0.3;
    p.circle_filled(centro, raggio_orizzonte, Color32::BLACK);
    p.circle_stroke(centro, raggio_orizzonte, Stroke::new(2.0, CIANO));

    let mut riga_testo = 0.0;
    for r in RIFERIMENTI {
        let rapporto = (r.raggio_m / rs) as f32;
        if !(0.04..=1.6).contains(&rapporto) {
            continue;
        }
        let raggio = raggio_orizzonte * rapporto;
        p.circle_stroke(centro, raggio, Stroke::new(1.0, GIALLO.gamma_multiply(0.8)));
        p.text(
            area.right_top() + Vec2::new(-8.0, 6.0 + riga_testo),
            Align2::RIGHT_TOP,
            r.nome,
            FontId::proportional(10.5),
            GIALLO,
        );
        riga_testo += 14.0;
    }
    p.text(area.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, "ORIZZONTE, IN SCALA", FontId::proportional(10.5), CHIARO);
    p.text(
        area.left_bottom() + Vec2::new(8.0, -6.0),
        Align2::LEFT_BOTTOM,
        format!("raggio {}", lunghezza(rs)),
        FontId::proportional(11.5),
        CIANO,
    );
}

fn strumenti(ui: &mut egui::Ui, g: &mut Giganti) -> Option<f64> {
    let mut apri = None;
    let massa = if g.t_cursore >= g.t_seme { g.massa_a(g.t_cursore) } else { g.massa_seme };
    let bh = BucoNero::da_masse_solari(massa);
    let r = g.ritmo();

    scala(ui, &bh);
    titoletto(ui, &format!("A {:.0} MILIONI DI ANNI DAL BIG BANG", g.t_cursore));
    egui::Grid::new("adesso").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Massa", format!("{} soli", grande(massa)));
        riga(ui, "Densità media", densita(bh.densita_media()));
        riga(ui, "Ingoia ogni anno", format!("{} soli", grande(r.soli_all_anno(massa))));
        riga(ui, "Brilla come", format!("{} soli", grande(r.luminosita_soli(massa))));
        riga(ui, "Raddoppia in", format!("{} milioni di anni", decimale(r.tempo_e() * std::f64::consts::LN_2)));
    });
    if ui.small_button("Aprilo nel banco Buco nero").clicked() {
        apri = Some(massa);
    }

    titoletto(ui, "RITMO DI CRESCITA");
    ui.add(egui::Slider::new(&mut g.eddington, 0.05..=5.0).logarithmic(true).text("rispetto al limite di Eddington").fixed_decimals(2));
    ui.add(egui::Slider::new(&mut g.efficienza, 0.04..=0.42).text("massa che diventa luce").fixed_decimals(2));
    if g.eddington > 1.0 {
        ui.label(RichText::new("Sopra il limite: la luce dovrebbe respingere la materia in caduta.").color(GIALLO));
    }

    titoletto(ui, "DA DOVE PARTE");
    ui.horizontal_wrapped(|ui| {
        for (nome, z, m) in SEMI {
            if ui.small_button(nome).clicked() {
                g.t_seme = tempo_cosmico(z);
                g.massa_seme = m;
            }
        }
    });
    ui.label(RichText::new(format!(
        "Seme di {} soli, a {:.0} milioni di anni.",
        grande(g.massa_seme),
        g.t_seme
    )).color(GRIGIO));

    titoletto(ui, "BUCHI NERI MISURATI");
    egui::Grid::new("quasar").num_columns(4).spacing([10.0, 3.0]).striped(true).show(ui, |ui| {
        for t in ["", "Massa", "Ci arrivi?", "Seme minimo"] {
            ui.label(RichText::new(t).small().color(GRIGIO));
        }
        ui.end_row();
        for q in &QUASAR {
            let t = tempo_cosmico(q.z);
            ui.label(RichText::new(q.nome).color(CHIARO)).on_hover_text(format!(
                "z = {}, circa {:.0} milioni di anni dopo il Big Bang. {}",
                decimale(q.z),
                t,
                q.fonte
            ));
            ui.label(RichText::new(grande(q.massa)).monospace());
            let ok = quasar_raggiunto(g, q);
            ui.label(RichText::new(if ok { "sì" } else { "no" }).strong().color(if ok { VERDE } else { ROSSO }));
            if t > g.t_seme {
                ui.label(RichText::new(grande(r.seme_necessario(q.massa, t - g.t_seme))).monospace());
            } else {
                ui.label(RichText::new("—").color(GRIGIO));
            }
            ui.end_row();
        }
    });
    ui.add_space(4.0);
    ui.label(RichText::new("Crescita continua, cosmologia Planck 2018 piatta.").small().color(GRIGIO));
    apri
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn un_seme_stellare_non_arriva_e_uno_grande_si() {
        let mut g = Giganti::default();
        assert!(!quasar_raggiunto(&g, &QUASAR[0]));
        g.t_seme = tempo_cosmico(SEMI[2].1);
        g.massa_seme = SEMI[2].2;
        assert!(quasar_raggiunto(&g, &QUASAR[0]));
    }
}
