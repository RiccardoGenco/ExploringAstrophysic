//! Banco delle stelle: una vita accelerata e la scala delle grandezze.

use crate::formato::{decimale, grande, tempo};
use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::costanti::ANNO;
use colossi_fisica::stelle_vita::{colore_gaia, durata_principale, durata_totale, fine, istante, magnitudine, massa_resto, Fine, STELLE_NOTE};
use egui::{Align2, Color32, FontId, Frame, Margin, Mesh, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const LARGHEZZA: f32 = 400.0;
const DURATA_VITA: f64 = 14.0;
const ALTEZZA_DISCO: f32 = 210.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pagina {
    Vita,
    Scala,
}

pub struct BancoStelle {
    pub pagina: Pagina,
    pub massa: f64,
    pub k: f64,
    pub in_corso: bool,
    pub scelta: usize,
    gaia: Vec<[f32; 2]>,
}

impl Default for BancoStelle {
    fn default() -> Self {
        let gaia = include_str!("../dati/gaia.csv")
            .lines()
            .filter_map(|r| {
                let (a, b) = r.split_once(',')?;
                Some([a.parse().ok()?, b.parse().ok()?])
            })
            .collect();
        Self { pagina: Pagina::Vita, massa: 1.0, k: 0.0, in_corso: true, scelta: 7, gaia }
    }
}

impl BancoStelle {
    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso && self.pagina == Pagina::Vita {
            self.k += dt / DURATA_VITA;
            if self.k >= 1.0 {
                self.k = 1.0;
                self.in_corso = false;
            }
        }
    }
}

fn colore_stella(t: f64) -> Color32 {
    let k = ((t - 3000.0) / 9000.0).clamp(0.0, 1.0) as f32;
    let (a, b) = ([255.0, 150.0, 90.0], [190.0, 215.0, 255.0]);
    Color32::from_rgb((a[0] + (b[0] - a[0]) * k) as u8, (a[1] + (b[1] - a[1]) * k) as u8, (a[2] + (b[2] - a[2]) * k) as u8)
}

pub fn mostra(ui: &mut egui::Ui, b: &mut BancoStelle) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        b.in_corso = !b.in_corso;
    }
    egui::Panel::bottom("tempo_stelle")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.add_sized([70.0, 30.0], egui::Button::new(if b.in_corso { "Pausa" } else { "Avvia" })).clicked() {
                    if b.k >= 1.0 {
                        b.k = 0.0;
                    }
                    b.in_corso = !b.in_corso;
                }
                ui.spacing_mut().slider_width = (ui.available_width() - 90.0).clamp(120.0, 700.0);
                ui.add(egui::Slider::new(&mut b.k, 0.0..=1.0).show_value(false).text("la sua vita"));
            });
        });
    egui::Panel::right("colonna_stelle")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| colonna(ui, b));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        match b.pagina {
            Pagina::Vita => diagramma(ui, area, b),
            Pagina::Scala => scala(ui, area, b),
        }
    });
}

fn diagramma(ui: &mut egui::Ui, area: Rect, b: &BancoStelle) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let piano = Rect::from_min_max(area.min + Vec2::new(46.0, 60.0), area.max - Vec2::new(18.0, 40.0));
    let (x0, x1, y0, y1) = (-0.6_f64, 5.2_f64, -9.5_f64, 17.0_f64);
    let a_schermo = |c: f64, m: f64| {
        Pos2::new(
            piano.left() + piano.width() * ((c - x0) / (x1 - x0)) as f32,
            piano.top() + piano.height() * ((m - y0) / (y1 - y0)) as f32,
        )
    };
    p.rect_stroke(piano, 2.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);

    let f = FontId::monospace(10.0);
    p.text(piano.right_bottom() + Vec2::new(0.0, 6.0), Align2::RIGHT_TOP, "colore BP-RP: blu a sinistra, rosso a destra", f.clone(), GRIGIO);
    p.text(piano.left_top() + Vec2::new(6.0, 4.0), Align2::LEFT_TOP, "più luminose in alto", f.clone(), GRIGIO);

    let mut mesh = Mesh::default();
    let tinta = Color32::from_rgb(120, 128, 150);
    for s in &b.gaia {
        let q = a_schermo(f64::from(s[0]), f64::from(s[1]));
        if !piano.contains(q) {
            continue;
        }
        let k = mesh.vertices.len() as u32;
        for d in [Vec2::new(-0.8, -0.8), Vec2::new(0.8, -0.8), Vec2::new(0.8, 0.8), Vec2::new(-0.8, 0.8)] {
            mesh.colored_vertex(q + d, tinta);
        }
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k, k + 2, k + 3);
    }
    p.add(mesh);

    let n = 300;
    let punti: Vec<Pos2> = (0..=n)
        .map(|i| {
            let q = istante(b.massa, f64::from(i) / f64::from(n));
            a_schermo(colore_gaia(q.temperatura), magnitudine(q.luminosita))
        })
        .collect();
    p.with_clip_rect(piano).add(Shape::line(punti, Stroke::new(1.4, GIALLO.gamma_multiply(0.75))));

    let ora = istante(b.massa, b.k);
    let q = a_schermo(colore_gaia(ora.temperatura), magnitudine(ora.luminosita));
    p.circle_filled(q, 7.0, colore_stella(ora.temperatura));
    p.circle_stroke(q, 11.0, Stroke::new(1.2, CHIARO));

    let t = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Ogni puntino è una stella vera misurata da Gaia. La linea gialla è la strada della tua stella in questo diagramma.", t.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, ora.fase, t, CIANO);
}

fn scala(ui: &mut egui::Ui, area: Rect, b: &BancoStelle) {
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let centro = area.center() + Vec2::new(0.0, area.height() * 0.42);
    let massimo = STELLE_NOTE[b.scelta].1;
    let scala = f64::from(area.height()) * 0.78 / (2.0 * massimo);
    for (i, (nome, r)) in STELLE_NOTE.iter().enumerate().rev() {
        let raggio = (r * scala) as f32;
        if i > b.scelta {
            continue;
        }
        // Le stelle poggiano sulla stessa linea di base.
        let c = centro - Vec2::new(0.0, raggio);
        let colore = if i == b.scelta { Color32::from_rgb(255, 170, 110) } else { Color32::from_rgb(120, 170, 255) };
        p.circle_filled(c, raggio.max(1.0), colore.gamma_multiply(0.28));
        p.circle_stroke(c, raggio.max(1.0), Stroke::new(1.3, colore));
        if raggio > 14.0 || i == b.scelta {
            p.text(c - Vec2::new(0.0, raggio + 3.0), Align2::CENTER_BOTTOM, *nome, FontId::proportional(12.5), CHIARO);
        }
    }
    let t = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Scegli una stella a destra: tutte le altre si rimpiccioliscono di conseguenza. Grandezze vere, raggi arrotondati.", t, GRIGIO);
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.end_row();
}

fn nome_fine(f: Fine) -> &'static str {
    match f {
        Fine::NanaBianca => "Nana bianca, senza esplosione",
        Fine::StellaDiNeutroni => "Supernova, poi stella di neutroni",
        Fine::BucoNero => "Supernova o crollo, poi buco nero",
        Fine::AncoraViva => "Nessuna: vive più dell'età dell'universo",
    }
}

fn colonna(ui: &mut egui::Ui, b: &mut BancoStelle) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut b.pagina, Pagina::Vita, RichText::new("Vita").strong());
        ui.selectable_value(&mut b.pagina, Pagina::Scala, RichText::new("Scala delle stelle").strong());
    });
    ui.add_space(6.0);
    if b.pagina == Pagina::Scala {
        for (i, (nome, r)) in STELLE_NOTE.iter().enumerate() {
            let testo = format!("{nome}: {} volte il Sole", decimale(*r));
            if ui.selectable_label(b.scelta == i, testo).clicked() {
                b.scelta = i;
            }
        }
        ui.add_space(6.0);
        ui.label(RichText::new("Il raggio delle supergiganti è incerto anche del 20-30%.").small().color(GRIGIO));
        return;
    }

    let prima = b.massa;
    ui.add(egui::Slider::new(&mut b.massa, 0.1..=60.0).logarithmic(true).text("massa, soli").fixed_decimals(2));
    if prima != b.massa {
        b.k = 0.0;
        b.in_corso = true;
    }
    ui.horizontal(|ui| {
        for (nome, m) in [("Sole", 1.0), ("Sirio", 2.0), ("Rigel", 21.0), ("Nana rossa", 0.2)] {
            if ui.small_button(nome).clicked() {
                b.massa = m;
                b.k = 0.0;
                b.in_corso = true;
            }
        }
    });

    let ora = istante(b.massa, b.k);
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_DISCO), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    let c = area.center();
    let finita = b.k >= 1.0;
    let fatale = fine(b.massa);
    if finita && matches!(fatale, Fine::StellaDiNeutroni | Fine::BucoNero) {
        p.circle_filled(c, area.height() * 0.45, Color32::from_rgb(250, 245, 225).gamma_multiply(0.22));
        p.circle_filled(c, 5.0, CIANO);
    } else if finita && fatale == Fine::NanaBianca {
        p.circle_filled(c, 4.0, Color32::WHITE);
        p.circle_stroke(c, area.height() * 0.38, Stroke::new(1.0, CIANO.gamma_multiply(0.4)));
    } else {
        // La grandezza e' logaritmica: da una nana a una supergigante.
        let raggio = (20.0 + 26.0 * ora.raggio.max(0.05).log10() as f32).clamp(5.0, area.height() * 0.46);
        p.circle_filled(c, raggio, colore_stella(ora.temperatura));
    }
    p.text(area.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, "grandezza su scala compressa", FontId::proportional(10.5), GRIGIO);

    ui.add_space(6.0);
    egui::Grid::new("letture_stelle").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Età", tempo(durata_totale(b.massa) * b.k * ANNO));
        riga(ui, "Brucia idrogeno per", tempo(durata_principale(b.massa) * ANNO));
        riga(ui, "Brilla come", format!("{} soli", grande(ora.luminosita)));
        riga(ui, "Raggio", format!("{} soli", grande(ora.raggio)));
        riga(ui, "Superficie", format!("{:.0} K", ora.temperatura));
    });
    ui.add_space(4.0);
    ui.label(RichText::new(format!("Finale: {}.", nome_fine(fatale))).color(CIANO));
    if fatale != Fine::AncoraViva {
        ui.label(format!("Resto di {} masse solari.", decimale(massa_resto(b.massa))));
    }
    ui.add_space(4.0);
    ui.label(RichText::new("Relazioni di scala, non un modello stellare completo. Stelle sullo sfondo: Gaia DR3.").small().color(GRIGIO));
}
