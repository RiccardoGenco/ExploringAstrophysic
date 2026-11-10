//! Banco delle stelle di neutroni: scegli la materia, costruisci una stella,
//! aggiungi massa finche' non crolla, confronta con i pulsar misurati.

use crate::audio::Audio;
use crate::formato::{decimale, grande};
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use colossi_fisica::costanti::{DENSITA_ACQUA, G_TERRA};
use colossi_fisica::stelle::{Curva, Gravita, Materia, Misura, Parametri, Stella, CATALOGO, MISURE, SOLE_IN_KM};
use egui::{Align2, Color32, FontId, Frame, Margin, Painter, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const LARGHEZZA: f32 = 420.0;
const R_MIN: f64 = 6.0;
const R_MAX: f64 = 20.0;
const M_MAX: f64 = 3.2;
const ALTEZZA_STELLA: f32 = 250.0;
/// Scala della vista della stella: mezza altezza, in km.
const KM_VISTA: f32 = 15.0;
/// Il Grande Raccordo Anulare, circa 68 km di giro.
const RAGGIO_RACCORDO: f32 = 10.8;
const DURATA_CROLLO: f64 = 1.1;
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);
const INDICI_AL_SECONDO: f64 = 9.0;

/// Pulsar reali e il loro periodo di rotazione, in secondi.
const PULSAR: [(&str, f64); 5] = [
    ("B0329+54", 0.714_5),
    ("Vela", 0.089_3),
    ("Granchio", 0.033_5),
    ("J0437-4715", 0.005_76),
    ("J1748-2446ad", 0.001_396),
];
const FREQUENZA_AUDIO: u32 = 44_100;
const SECONDI_AUDIO: f64 = 3.0;

/// Un breve impulso a ogni giro della stella.
fn impulsi(periodo: f64) -> Vec<f32> {
    let n = (SECONDI_AUDIO * f64::from(FREQUENZA_AUDIO)) as usize;
    (0..n)
        .map(|i| {
            let t = i as f64 / f64::from(FREQUENZA_AUDIO);
            let dentro = t % periodo;
            let durata = (periodo * 0.3).min(0.004);
            let inviluppo = (-dentro / durata).exp() as f32;
            let fine = ((SECONDI_AUDIO - t) / 0.1).min(1.0) as f32;
            inviluppo * fine * 0.7 * if dentro < durata * 6.0 { 1.0 } else { 0.0 }
        })
        .collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Fase {
    Stella,
    Crollo(f64),
    BucoNero,
}

pub struct Neutroni {
    parametri: Parametri,
    catalogo: Option<usize>,
    curva: Curva,
    curva_newton: Curva,
    mostra_newton: bool,
    indice: f64,
    fase: Fase,
    impossibile: bool,
    aggiungi: bool,
    audio: Audio,
    avviso: Option<String>,
}

impl Default for Neutroni {
    fn default() -> Self {
        let parametri = CATALOGO[0].1;
        let materia = Materia::nuova(parametri).expect("il catalogo e' valido");
        let curva = materia.curva(Gravita::Einstein);
        let curva_newton = materia.curva(Gravita::Newton);
        let mut n = Self {
            parametri,
            catalogo: Some(0),
            curva,
            curva_newton,
            mostra_newton: false,
            indice: 0.0,
            fase: Fase::Stella,
            impossibile: false,
            aggiungi: false,
            audio: Audio::default(),
            avviso: None,
        };
        n.scegli_massa(1.4);
        n
    }
}

impl Neutroni {
    fn scegli_massa(&mut self, m: f64) {
        let stabili = &self.curva.stelle[..=self.curva.indice_massimo];
        let vicina = stabili
            .iter()
            .enumerate()
            .rev()
            .min_by(|a, b| (a.1.massa - m).abs().total_cmp(&(b.1.massa - m).abs()))
            .map_or(0, |(i, _)| i);
        self.indice = vicina as f64;
    }

    fn stella(&self) -> &Stella {
        &self.curva.stelle[(self.indice as usize).min(self.curva.stelle.len() - 1)]
    }

    fn cambia_materia(&mut self, p: Parametri, catalogo: Option<usize>) {
        self.parametri = p;
        self.catalogo = catalogo;
        match Materia::nuova(p) {
            Some(materia) => {
                let massa = self.stella().massa;
                self.curva = materia.curva(Gravita::Einstein);
                self.curva_newton = materia.curva(Gravita::Newton);
                self.impossibile = false;
                self.fase = Fase::Stella;
                self.scegli_massa(massa);
            }
            None => self.impossibile = true,
        }
    }

    pub fn avanza(&mut self, dt: f64) {
        match self.fase {
            Fase::Crollo(t) => {
                let t = t + dt;
                self.fase = if t >= DURATA_CROLLO { Fase::BucoNero } else { Fase::Crollo(t) };
            }
            Fase::Stella if self.aggiungi => {
                self.indice += INDICI_AL_SECONDO * dt;
                if self.indice >= self.curva.indice_massimo as f64 + 0.999 {
                    self.indice = self.curva.indice_massimo as f64;
                    self.fase = Fase::Crollo(0.0);
                }
            }
            _ => {}
        }
    }

    #[cfg(test)]
    pub fn e_buco_nero(&self) -> bool {
        self.fase == Fase::BucoNero
    }

    fn ricomincia(&mut self) {
        self.fase = Fase::Stella;
        self.scegli_massa(1.4);
    }
}

fn piano_a_schermo(piano: Rect, r: f64, m: f64) -> Pos2 {
    Pos2::new(
        piano.left() + piano.width() * ((r - R_MIN) / (R_MAX - R_MIN)) as f32,
        piano.bottom() - piano.height() * (m / M_MAX) as f32,
    )
}

fn linea_curva(p: &Painter, piano: Rect, stelle: &[Stella], tratto: Stroke) {
    let punti: Vec<Pos2> = stelle.iter().map(|s| piano_a_schermo(piano, s.raggio, s.massa)).collect();
    p.add(Shape::line(punti, tratto));
}

/// Restituisce la massa da aprire nel banco del buco nero, se richiesto.
pub fn mostra(ui: &mut egui::Ui, n: &mut Neutroni) -> Option<f64> {
    let mut apri = None;
    egui::Panel::right("colonna_neutroni")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| apri = strumenti(ui, n));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        grafico(ui, area, n);
    });
    apri
}

fn grafico(ui: &mut egui::Ui, area: Rect, n: &mut Neutroni) {
    let risposta = ui.interact(area, ui.id().with("neutroni"), Sense::click_and_drag());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let piano = Rect::from_min_max(area.min + Vec2::new(54.0, 64.0), area.max - Vec2::new(22.0, 46.0));

    if risposta.is_pointer_button_down_on() && n.fase == Fase::Stella {
        if let Some(q) = ui.input(|i| i.pointer.latest_pos()) {
            let vicino = n.curva.stelle.iter().enumerate().min_by(|a, b| {
                let d = |s: &Stella| (piano_a_schermo(piano, s.raggio, s.massa) - q).length_sq();
                d(a.1).total_cmp(&d(b.1))
            });
            if let Some((i, _)) = vicino {
                n.indice = i as f64;
            }
        }
    }

    // Griglia.
    let f = FontId::monospace(10.0);
    for r in (6..=20).step_by(2) {
        let x = piano_a_schermo(piano, f64::from(r), 0.0).x;
        p.line_segment([Pos2::new(x, piano.top()), Pos2::new(x, piano.bottom())], Stroke::new(1.0, LINEA));
        p.text(Pos2::new(x, piano.bottom() + 5.0), Align2::CENTER_TOP, format!("{r}"), f.clone(), GRIGIO);
    }
    for i in 0..=6 {
        let m = f64::from(i) * 0.5;
        let y = piano_a_schermo(piano, R_MIN, m).y;
        p.line_segment([Pos2::new(piano.left(), y), Pos2::new(piano.right(), y)], Stroke::new(1.0, LINEA));
        p.text(Pos2::new(piano.left() - 6.0, y), Align2::RIGHT_CENTER, decimale(m), f.clone(), GRIGIO);
    }
    p.text(piano.right_bottom() + Vec2::new(0.0, 20.0), Align2::RIGHT_TOP, "raggio, km", f.clone(), GRIGIO);
    p.text(piano.left_top() + Vec2::new(4.0, 4.0), Align2::LEFT_TOP, "massa, in masse solari", f.clone(), GRIGIO);

    // Zone proibite: a sinistra della prima, nulla puo' essere stabile.
    let zona = |rapporto: f64, tinta: Color32, testo: &str, m_testo: f64| {
        let m_inizio = R_MIN / (rapporto * SOLE_IN_KM);
        let r_fine = rapporto * SOLE_IN_KM * M_MAX;
        p.add(Shape::convex_polygon(
            vec![
                piano_a_schermo(piano, R_MIN, m_inizio),
                piano_a_schermo(piano, r_fine, M_MAX),
                piano_a_schermo(piano, R_MIN, M_MAX),
            ],
            tinta,
            Stroke::NONE,
        ));
        p.text(
            piano_a_schermo(piano, R_MIN + 0.2, m_testo),
            Align2::LEFT_CENTER,
            testo,
            FontId::proportional(11.5),
            CHIARO.gamma_multiply(0.75),
        );
    };
    zona(2.9, Color32::from_rgba_unmultiplied(255, 200, 90, 14), "qui il suono supererebbe la luce", 2.25);
    zona(2.0, Color32::from_rgba_unmultiplied(90, 215, 255, 26), "buco nero", 3.05);

    // Misure reali, con le loro barre d'errore.
    for m in &MISURE {
        let ok = m.compatibile(&n.curva);
        let tinta = if ok { VERDE } else { ROSSO };
        let (sx, dx) = (m.raggio - m.raggio_giu, m.raggio + m.raggio_su);
        let centro = piano_a_schermo(piano, m.raggio, m.massa);
        p.line_segment(
            [piano_a_schermo(piano, sx, m.massa), piano_a_schermo(piano, dx, m.massa)],
            Stroke::new(1.6, tinta),
        );
        p.line_segment(
            [
                piano_a_schermo(piano, m.raggio, m.massa - m.massa_errore),
                piano_a_schermo(piano, m.raggio, m.massa + m.massa_errore),
            ],
            Stroke::new(1.6, tinta),
        );
        p.circle_filled(centro, 3.5, tinta);
        p.text(centro + Vec2::new(8.0, -6.0), Align2::LEFT_BOTTOM, m.nome, FontId::proportional(11.5), tinta);
    }

    // Massa massima.
    let massima = n.curva.massa_massima();
    let y = piano_a_schermo(piano, R_MIN, massima).y;
    p.extend(Shape::dashed_line(
        &[Pos2::new(piano.left(), y), Pos2::new(piano.right(), y)],
        Stroke::new(1.0, GIALLO.gamma_multiply(0.7)),
        6.0,
        5.0,
    ));
    p.text(
        Pos2::new(piano.right() - 4.0, y - 3.0),
        Align2::RIGHT_BOTTOM,
        format!("massa massima {}", decimale(massima)),
        FontId::proportional(11.5),
        GIALLO,
    );

    // Curve: oltre il massimo la stella non regge.
    let (stabile, instabile) = n.curva.stelle.split_at(n.curva.indice_massimo + 1);
    if n.mostra_newton {
        linea_curva(&p, piano, &n.curva_newton.stelle, Stroke::new(1.3, GRIGIO));
        if let Some(s) = n.curva_newton.stelle.iter().rev().find(|s| s.massa < M_MAX * 0.98) {
            let q = piano_a_schermo(piano, s.raggio, s.massa);
            p.text(q + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, "senza Einstein", FontId::proportional(11.5), GRIGIO);
        }
    }
    let mut instabile_con_giunto = vec![stabile[stabile.len() - 1].clone()];
    instabile_con_giunto.extend_from_slice(instabile);
    linea_curva(&p, piano, &instabile_con_giunto, Stroke::new(1.2, ROSSO.gamma_multiply(0.55)));
    linea_curva(&p, piano, stabile, Stroke::new(3.0, CIANO));

    let s = n.stella();
    let q = piano_a_schermo(piano, s.raggio, s.massa);
    if n.fase == Fase::Stella {
        p.circle_filled(q, 7.5, CIANO);
        p.circle_stroke(q, 11.0, Stroke::new(1.2, CHIARO));
    }

    p.text(
        area.left_top() + Vec2::new(14.0, 12.0),
        Align2::LEFT_TOP,
        "Ogni punto della curva è una stella possibile con questa materia: clicca per sceglierne una.",
        FontId::proportional(13.0),
        GRIGIO,
    );
    p.text(
        area.left_top() + Vec2::new(14.0, 32.0),
        Align2::LEFT_TOP,
        "Le croci sono pulsar misurati davvero. Inventa una materia a destra: se la curva le manca di molto, quella materia è esclusa.",
        FontId::proportional(13.0),
        CIANO,
    );
}

fn colore_densita(rho: f64) -> Color32 {
    let k = (((rho.max(1.0)).log10() - 9.0) / 6.5).clamp(0.0, 1.0) as f32;
    let (a, b) = ([140.0, 60.0, 30.0], [120.0, 140.0, 185.0]);
    let c: Vec<u8> = (0..3).map(|i| (a[i] + (b[i] - a[i]) * k) as u8).collect();
    Color32::from_rgb(c[0], c[1], c[2])
}

fn vista_stella(ui: &mut egui::Ui, n: &Neutroni) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_STELLA), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let centro = area.center();
    let scala = area.height() * 0.5 / KM_VISTA;
    let s = n.stella();
    let rs = s.raggio_schwarzschild() as f32;

    let cerchio_tratteggiato = |r: f32, tinta: Color32| {
        let punti: Vec<Pos2> = (0..=160)
            .map(|i| {
                let a = i as f32 / 160.0 * TAU;
                centro + Vec2::new(a.cos(), a.sin()) * r
            })
            .collect();
        p.extend(Shape::dashed_line(&punti, Stroke::new(1.2, tinta), 5.0, 4.0));
    };

    match n.fase {
        Fase::BucoNero => {
            p.circle_filled(centro, rs * scala, Color32::BLACK);
            p.circle_stroke(centro, rs * scala, Stroke::new(2.0, CIANO));
        }
        fase => {
            let k = match fase {
                Fase::Crollo(t) => 1.0 - ((t / DURATA_CROLLO) as f32).powi(3),
                _ => 1.0,
            };
            let r_vis = rs + (s.raggio as f32 - rs) * k;
            let fattore = r_vis / s.raggio as f32;
            for strato in s.profilo.iter().rev() {
                let raggio = if strato.r == 0.0 { 0.0 } else { strato.r as f32 };
                if raggio > 0.0 {
                    p.circle_filled(centro, raggio * fattore * scala, colore_densita(strato.rho));
                }
            }
            cerchio_tratteggiato(rs * scala, CIANO.gamma_multiply(0.85));
        }
    }

    cerchio_tratteggiato(RAGGIO_RACCORDO * scala, VERDE);
    let f = FontId::proportional(11.0);
    p.text(area.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, "SEZIONE, IN SCALA", FontId::proportional(10.5), CHIARO);
    p.text(area.left_bottom() + Vec2::new(8.0, -22.0), Align2::LEFT_BOTTOM, "- - Grande Raccordo Anulare", f.clone(), VERDE);
    p.text(area.left_bottom() + Vec2::new(8.0, -7.0), Align2::LEFT_BOTTOM, "- - dove sarebbe l'orizzonte", f, CIANO);
}

fn riga(ui: &mut egui::Ui, nome: &str, valore: String) {
    ui.label(RichText::new(nome).color(GRIGIO));
    ui.label(RichText::new(valore).monospace().color(CHIARO));
    ui.end_row();
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(8.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

fn conseguenze(ui: &mut egui::Ui, s: &Stella) {
    let g = s.gravita_superficie();
    let tonnellate = s.densita_media() * 5e-6 / 1e3;
    egui::Grid::new("conseguenze").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Massa", format!("{} masse solari", decimale(s.massa)));
        riga(ui, "Raggio", format!("{} km", decimale(s.raggio)));
        riga(ui, "Densità al centro", format!("{} volte l'acqua", grande(s.rho_centrale * 1e3 / DENSITA_ACQUA)));
        riga(ui, "Un cucchiaino pesa", format!("{} tonnellate", grande(tonnellate)));
        riga(ui, "Il tuo peso", format!("{} volte tanto", grande(g / G_TERRA)));
        riga(ui, "Cadere da un metro", format!("{} km/s", grande((2.0 * g).sqrt() / 1e3)));
        riga(ui, "Per scappare serve", format!("{} c", decimale(s.fuga())));
        riga(ui, "Un anno sulla superficie", format!("{} anni fuori", decimale(s.anni_lontano())));
        riga(ui, "Superficie che si vede", format!("{:.0} %", s.superficie_visibile() * 100.0));
        riga(ui, "Massa persa nel legarsi", format!("{} soli", decimale(s.massa_barionica - s.massa)));
    });
}

fn verdetto_misura(ui: &mut egui::Ui, m: &Misura, curva: &Curva) {
    let ok = m.compatibile(curva);
    ui.horizontal(|ui| {
        let (testo, tinta) = if ok { ("regge", VERDE) } else { ("esclusa", ROSSO) };
        ui.label(RichText::new(testo).strong().color(tinta));
        ui.label(RichText::new(m.nome).color(CHIARO))
            .on_hover_text(format!(
                "{}: {} masse solari, {} km. {}. Regge se ci passa entro due barre d'errore.",
                m.nome,
                decimale(m.massa),
                decimale(m.raggio),
                m.fonte
            ));
    });
}

fn strumenti(ui: &mut egui::Ui, n: &mut Neutroni) -> Option<f64> {
    let mut apri = None;
    vista_stella(ui, n);
    ui.add_space(6.0);

    match n.fase {
        Fase::Stella => {
            ui.horizontal(|ui| {
                let bottone = ui.add_sized([180.0, 30.0], egui::Button::new(RichText::new("Aggiungi materia").strong()));
                // Un pulsante smette di risultare premuto dopo un attimo: si guarda il mouse.
                n.aggiungi = bottone.contains_pointer() && ui.input(|i| i.pointer.primary_down());
                ui.label(RichText::new("tieni premuto").color(GRIGIO));
            });
            let s = n.stella();
            if (n.indice as usize) > n.curva.indice_massimo {
                ui.label(RichText::new("Questa stella non regge: al primo disturbo crollerebbe.").color(ROSSO));
            }
            titoletto(ui, "SE FOSSI LÌ");
            conseguenze(ui, s);
        }
        _ => {
            n.aggiungi = false;
            let s = n.stella();
            ui.label(
                RichText::new(format!(
                    "Oltre {} masse solari la pressione non basta più. Ora è un buco nero con un orizzonte di {} km.",
                    decimale(n.curva.massa_massima()),
                    decimale(s.raggio_schwarzschild())
                ))
                .color(CIANO),
            );
            ui.horizontal(|ui| {
                if ui.button("Ricomincia").clicked() {
                    n.ricomincia();
                }
                if n.fase == Fase::BucoNero && ui.button("Aprilo nel banco Buco nero").clicked() {
                    apri = Some(n.stella().massa);
                }
            });
        }
    }

    titoletto(ui, "LE MISURE VERE");
    for m in &MISURE {
        verdetto_misura(ui, m, &n.curva);
    }
    ui.checkbox(&mut n.mostra_newton, "Mostra la stessa materia senza Einstein");

    titoletto(ui, "PULSAR DA ASCOLTARE");
    ui.horizontal_wrapped(|ui| {
        for (nome, periodo) in PULSAR {
            let testo = if periodo >= 0.1 {
                format!("{nome}, {} giri al secondo", decimale(1.0 / periodo))
            } else {
                format!("{nome}, {} giri al secondo", grande(1.0 / periodo))
            };
            if ui.small_button(testo).clicked() {
                n.avviso = n.audio.suona(impulsi(periodo), FREQUENZA_AUDIO).err();
            }
        }
    });
    if let Some(e) = &n.avviso {
        ui.label(RichText::new(format!("Audio non disponibile: {e}")).color(GIALLO));
    }

    titoletto(ui, "LA MATERIA");
    ui.horizontal_wrapped(|ui| {
        let mut scelta = None;
        for (i, (nome, _, _)) in CATALOGO.iter().enumerate() {
            if ui.selectable_label(n.catalogo == Some(i), *nome).clicked() {
                scelta = Some(i);
            }
        }
        if let Some(i) = scelta {
            n.cambia_materia(CATALOGO[i].1, Some(i));
        }
    });

    let mut p = n.parametri;
    let mut cambiato = false;
    cambiato |= ui.add(egui::Slider::new(&mut p.log_p1, 33.6..=35.2).text("durezza media").custom_formatter(|v, _| format!("{v:.2}"))).changed();
    cambiato |= ui.add(egui::Slider::new(&mut p.gamma1, 1.8..=4.5).text("indice, basse").custom_formatter(|v, _| format!("{v:.2}"))).changed();
    cambiato |= ui.add(egui::Slider::new(&mut p.gamma2, 1.2..=4.5).text("indice, medie").custom_formatter(|v, _| format!("{v:.2}"))).changed();
    cambiato |= ui.add(egui::Slider::new(&mut p.gamma3, 1.2..=4.5).text("indice, alte").custom_formatter(|v, _| format!("{v:.2}"))).changed();
    if cambiato {
        n.cambia_materia(p, None);
    }
    if n.impossibile {
        ui.label(RichText::new("Questo nucleo non si raccorda con la crosta: curva non aggiornata.").color(ROSSO));
    }
    let centrale = n.curva.stelle[n.curva.indice_massimo].suono2_centrale.sqrt();
    if centrale >= 1.0 {
        ui.label(RichText::new(format!("Al centro il suono andrebbe a {} c: impossibile.", decimale(centrale))).color(ROSSO));
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new("Materia: Read et al. 2009, crosta SLy. Stelle ferme, senza rotazione.")
            .small()
            .color(GRIGIO),
    );
    apri
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn gli_impulsi_hanno_il_ritmo_della_pulsar() {
        let a = impulsi(0.1);
        let conta = a.windows(2).filter(|c| c[0] < 0.3 && c[1] >= 0.3).count();
        assert!((28..=32).contains(&conta), "{conta}");
    }

    #[test]
    fn aggiungere_materia_porta_al_crollo() {
        let mut n = Neutroni::default();
        n.aggiungi = true;
        for _ in 0..2000 {
            n.avanza(1.0 / 60.0);
        }
        assert!(n.fase == Fase::BucoNero);
        n.aggiungi = false;
        n.ricomincia();
        assert!(n.fase == Fase::Stella);
        assert!((n.stella().massa - 1.4).abs() < 0.1);
    }

    #[test]
    fn la_materia_del_catalogo_regge_le_misure() {
        let n = Neutroni::default();
        assert!(MISURE.iter().all(|m| m.compatibile(&n.curva)));
    }

    #[test]
    fn cambiare_materia_ricalcola_la_curva() {
        let mut n = Neutroni::default();
        let prima = n.curva.massa_massima();
        n.cambia_materia(CATALOGO[6].1, Some(6));
        assert!(n.curva.massa_massima() > prima + 0.4);
    }
}
