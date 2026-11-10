//! Banco delle onde gravitazionali: la fusione, il segnale, il suono.

use crate::audio::Audio;
use crate::formato::{decimale, grande};
use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::onde::{in_audio, stiramento_ligo, Campione, Coppia, GW150914, GW170817};
use egui::{Align2, Color32, FontId, Frame, Margin, Mesh, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const LARGHEZZA: f32 = 430.0;
const QUOTA_CAMPO: f32 = 0.70;
const CELLE_X: usize = 150;
const CELLE_Y: usize = 90;
/// Quanto l'immagine stira l'anello al picco del segnale.
const STIRAMENTO_VISIBILE: f64 = 0.35;
const LUMINOSITA_SOLE: f64 = 3.828e26;
const RAGGIO_PROTONE: f64 = 0.84e-15;
const FREQUENZA_AUDIO: u32 = 44_100;
const ALTEZZA_ANELLO: f32 = 150.0;

pub struct BancoOnde {
    pub coppia: Coppia,
    pub segnale: Vec<Campione>,
    h_massimo: f64,
    pub t: f64,
    pub in_corso: bool,
    pub ritmo: f64,
    audio: Audio,
    avviso: Option<String>,
    pub ascoltato: bool,
}

impl Default for BancoOnde {
    fn default() -> Self {
        let mut b = Self {
            coppia: GW150914,
            segnale: Vec::new(),
            h_massimo: 1.0,
            t: 0.0,
            in_corso: true,
            ritmo: 0.05,
            audio: Audio::default(),
            avviso: None,
            ascoltato: false,
        };
        b.costruisci();
        b
    }
}

impl BancoOnde {
    fn durata(&self) -> f64 {
        if self.coppia.neutroni {
            60.0
        } else {
            1.0
        }
    }

    pub fn costruisci(&mut self) {
        self.segnale = self.coppia.segnale(self.durata());
        self.h_massimo = Coppia::ampiezza_massima(&self.segnale).max(1e-40);
        self.t = self.inizio();
        self.ritmo = if self.coppia.neutroni { 2.0 } else { 0.05 };
        self.in_corso = true;
    }

    fn inizio(&self) -> f64 {
        self.segnale.first().map_or(0.0, |c| c.t)
    }

    fn fine(&self) -> f64 {
        self.segnale.last().map_or(0.0, |c| c.t)
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            self.t += self.ritmo * dt;
            if self.t >= self.fine() {
                self.t = self.fine();
                self.in_corso = false;
            }
        }
    }

    fn campione(&self, t: f64) -> Option<Campione> {
        let i = self.segnale.partition_point(|c| c.t < t);
        self.segnale.get(i.min(self.segnale.len().saturating_sub(1))).copied()
    }

    fn suona(&mut self) {
        let campioni = in_audio(&self.segnale, FREQUENZA_AUDIO);
        // Le stelle di neutroni durano un minuto: si ascolta la parte finale.
        let massimo = FREQUENZA_AUDIO as usize * 12;
        let inizio = campioni.len().saturating_sub(massimo);
        self.avviso = self.audio.suona(campioni[inizio..].to_vec(), FREQUENZA_AUDIO).err();
        self.ascoltato = true;
    }
}

pub fn mostra(ui: &mut egui::Ui, b: &mut BancoOnde) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        b.in_corso = !b.in_corso;
    }
    egui::Panel::bottom("tempo_onde")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            if pannello::barra_tempo(ui, &mut b.in_corso, &mut b.ritmo, 0.005..=20.0, "volte la velocità reale") {
                b.t = b.inizio();
                b.in_corso = true;
            }
        });
    egui::Panel::right("colonna_onde")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, b));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        let taglio = area.top() + area.height() * QUOTA_CAMPO;
        let sopra = Rect::from_min_max(area.min, Pos2::new(area.right(), taglio));
        let sotto = Rect::from_min_max(Pos2::new(area.left(), taglio), area.max);
        ui.painter_at(area).rect_filled(area, 0.0, FONDO);
        campo(ui, sopra, b);
        onda(ui, sotto, b);
    });
}

fn tinta_campo(v: f32) -> Color32 {
    let a = v.abs().min(1.0);
    let base = 12.0;
    let (r, g, b) = if v > 0.0 {
        (base + 80.0 * a, base + 170.0 * a, base + 243.0 * a)
    } else {
        (base + 243.0 * a, base + 110.0 * a, base + 90.0 * a)
    };
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

fn campo(ui: &mut egui::Ui, area: Rect, b: &BancoOnde) {
    let Some(ora) = b.campione(b.t) else { return };
    let p = ui.painter_at(area);
    let centro = area.center();
    let scala = area.height().min(area.width()) * 0.5;
    // Il ritardo e' visivo: l'onda attraversa il riquadro in qualche ciclo.
    let durata_attraversamento = (6.0 / ora.f.max(1.0)).min(0.25);

    let mut mesh = Mesh::default();
    mesh.reserve_vertices((CELLE_X + 1) * (CELLE_Y + 1));
    mesh.reserve_triangles(CELLE_X * CELLE_Y * 2);
    for j in 0..=CELLE_Y {
        for i in 0..=CELLE_X {
            let q = Pos2::new(
                area.left() + area.width() * i as f32 / CELLE_X as f32,
                area.top() + area.height() * j as f32 / CELLE_Y as f32,
            );
            let d = (q - centro) / scala;
            let r = d.length();
            let t_rit = b.t - f64::from(r) * durata_attraversamento;
            let valore = b.campione(t_rit).filter(|_| t_rit >= b.inizio()).map_or(0.0, |c| {
                let ampiezza = (c.h.abs().max(b.coppia.ampiezza(c.f)) / b.h_massimo) as f32;
                let angolo = d.y.atan2(d.x);
                ampiezza * (2.0 * angolo - c.fase as f32).cos() / (1.0 + 2.2 * r)
            });
            mesh.colored_vertex(q, tinta_campo(valore * 1.6));
        }
    }
    let w = (CELLE_X + 1) as u32;
    for j in 0..CELLE_Y as u32 {
        for i in 0..CELLE_X as u32 {
            let k = j * w + i;
            mesh.add_triangle(k, k + 1, k + w);
            mesh.add_triangle(k + 1, k + w + 1, k + w);
        }
    }
    p.add(mesh);

    // I due corpi.
    if ora.t < -3.0 / ora.f.max(1.0) || b.coppia.neutroni {
        let separazione0 = b.coppia.separazione(b.segnale[0].f);
        let k = (b.coppia.separazione(ora.f) / separazione0) as f32;
        let raggio_orbita = scala * 0.32 * k.powf(0.5);
        let angolo = (ora.fase * 0.5) as f32;
        let d = Vec2::new(angolo.cos(), angolo.sin()) * raggio_orbita;
        let totale = (b.coppia.m1 + b.coppia.m2) as f32;
        for (segno, m) in [(1.0_f32, b.coppia.m2 as f32), (-1.0, b.coppia.m1 as f32)] {
            p.circle_filled(centro + d * segno * (m / totale) * 2.0, 8.0, Color32::BLACK);
            p.circle_stroke(centro + d * segno * (m / totale) * 2.0, 8.0, Stroke::new(1.4, CHIARO));
        }
    } else {
        p.circle_filled(centro, 13.0, Color32::BLACK);
        p.circle_stroke(centro, 13.0, Stroke::new(1.6, CHIARO));
    }

    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Lo schema delle onde nel piano dell'orbita. Forma vera, scala e ritardo esagerati per poterli vedere.", f.clone(), CHIARO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, "Ogni orbita ne produce due: l'onda vibra al doppio della frequenza del giro.", f, CIANO);
}

fn onda(ui: &mut egui::Ui, area: Rect, b: &BancoOnde) {
    let p = ui.painter_at(area);
    let piano = area.shrink2(Vec2::new(22.0, 22.0));
    p.rect_stroke(piano, 2.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let (t0, t1) = (b.inizio(), b.fine().max(b.inizio() + 1e-6));
    let a_schermo = |t: f64, h: f64| {
        Pos2::new(
            piano.left() + piano.width() * ((t - t0) / (t1 - t0)) as f32,
            piano.center().y - (piano.height() * 0.46) * (h / b.h_massimo) as f32,
        )
    };
    let passo = (b.segnale.len() / 4000).max(1);
    let punti: Vec<Pos2> = b.segnale.iter().step_by(passo).map(|c| a_schermo(c.t, c.h)).collect();
    p.add(Shape::line(punti, Stroke::new(1.0, CIANO)));
    let x = a_schermo(b.t, 0.0).x;
    p.line_segment([Pos2::new(x, piano.top()), Pos2::new(x, piano.bottom())], Stroke::new(1.2, GIALLO));
    let f = FontId::monospace(10.0);
    p.text(piano.left_top() + Vec2::new(6.0, 4.0), Align2::LEFT_TOP, format!("deformazione dello spazio a Terra, massimo {:.1e}", b.h_massimo), f.clone(), GRIGIO);
    p.text(piano.right_bottom() + Vec2::new(-4.0, 3.0), Align2::RIGHT_TOP, format!("{:.2} s", t1 - t0).replace('.', ","), f, GRIGIO);
}

/// Un anello di masse libere attraversato dall'onda.
fn anello(ui: &mut egui::Ui, b: &BancoOnde) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_ANELLO), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let c = area.center();
    let r = area.height() * 0.32;
    let h = b.campione(b.t).map_or(0.0, |q| q.h / b.h_massimo) as f32;
    let e = h * STIRAMENTO_VISIBILE as f32;
    p.circle_stroke(c, r, Stroke::new(1.0, GRIGIO.gamma_multiply(0.5)));
    let mut punti: Vec<Pos2> = (0..=48)
        .map(|i| {
            let a = i as f32 / 48.0 * TAU;
            c + Vec2::new(a.cos() * r * (1.0 + e), a.sin() * r * (1.0 - e))
        })
        .collect();
    punti.push(punti[0]);
    p.add(Shape::line(punti.clone(), Stroke::new(2.2, CIANO)));
    for q in punti.iter().step_by(6) {
        p.circle_filled(*q, 2.5, CHIARO);
    }
    let fattore = STIRAMENTO_VISIBILE / b.h_massimo;
    p.text(area.left_top() + Vec2::new(6.0, 4.0), Align2::LEFT_TOP, "UN ANELLO DI OGGETTI LIBERI", FontId::proportional(10.5), CHIARO);
    p.text(
        area.left_bottom() + Vec2::new(6.0, -4.0),
        Align2::LEFT_BOTTOM,
        format!("esagerato {} volte", grande(fattore)),
        FontId::proportional(11.0),
        GIALLO,
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

fn strumenti(ui: &mut egui::Ui, b: &mut BancoOnde) {
    anello(ui, b);

    titoletto(ui, "LA COPPIA");
    ui.horizontal(|ui| {
        if ui.small_button("Primo segnale, 2015").clicked() {
            b.coppia = GW150914;
            b.costruisci();
        }
        if ui.small_button("Due stelle di neutroni, 2017").clicked() {
            b.coppia = GW170817;
            b.costruisci();
        }
    });
    let prima = b.coppia;
    ui.add(egui::Slider::new(&mut b.coppia.m1, 1.0..=100.0).logarithmic(true).text("massa 1, soli").fixed_decimals(1));
    ui.add(egui::Slider::new(&mut b.coppia.m2, 1.0..=100.0).logarithmic(true).text("massa 2, soli").fixed_decimals(1));
    ui.add(egui::Slider::new(&mut b.coppia.distanza_mpc, 10.0..=3000.0).logarithmic(true).text("distanza, Mpc").fixed_decimals(0));
    b.coppia.neutroni = b.coppia.m1 < 3.0 && b.coppia.m2 < 3.0;
    if prima != b.coppia && ui.input(|i| !i.pointer.any_down()) {
        b.costruisci();
    }

    ui.add_space(4.0);
    if ui.button(RichText::new("Ascolta il segnale").strong()).clicked() {
        b.suona();
    }
    if let Some(e) = &b.avviso {
        ui.label(RichText::new(format!("Audio non disponibile: {e}")).color(GIALLO));
    }

    titoletto(ui, "LETTURE");
    let ora = b.campione(b.t);
    let c = b.coppia;
    egui::Grid::new("letture_onde").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        if let Some(q) = ora {
            riga(ui, "Frequenza dell'onda", format!("{} Hz", decimale(q.f)));
            if q.t < 0.0 {
                riga(ui, "Distanza tra i due", format!("{} km", grande(c.separazione(q.f))));
            }
        }
        riga(ui, "Massa chirp", format!("{} soli", decimale(c.massa_chirp())));
        if !c.neutroni {
            riga(ui, "Massa finale", format!("{} soli", decimale(c.massa_finale())));
            riga(ui, "Diventate onde", format!("{} soli", decimale(c.energia_irraggiata())));
            riga(ui, "Rotazione finale", format!("{} su 1", decimale(c.rotazione_finale())));
            let (f, _) = c.assestamento();
            riga(ui, "Nota finale", format!("{} Hz", decimale(f)));
        }
        riga(ui, "Potenza all'ultima orbita", format!("{} soli", grande(c.potenza(c.frequenza_fine_spirale()) / LUMINOSITA_SOLE)));
        let s = stiramento_ligo(b.h_massimo);
        riga(ui, "Braccio di 4 km", format!("{} protoni", format!("{:.4}", s / RAGGIO_PROTONE).replace('.', ",")));
    });
    ui.add_space(4.0);
    ui.label(
        RichText::new("Spirale: formula di quadrupolo. Fusione e assestamento: raccordo approssimato. Non è il dato misurato.")
            .small()
            .color(GRIGIO),
    );
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_banco_arriva_alla_fine_e_si_ferma() {
        let mut b = BancoOnde::default();
        b.ritmo = 1.0;
        for _ in 0..200 {
            b.avanza(0.05);
        }
        assert!(!b.in_corso && b.t >= b.fine());
    }

    #[test]
    fn cambiare_coppia_ricostruisce_il_segnale() {
        let mut b = BancoOnde::default();
        let n = b.segnale.len();
        b.coppia = GW170817;
        b.costruisci();
        assert!(b.segnale.len() > n);
    }
}
