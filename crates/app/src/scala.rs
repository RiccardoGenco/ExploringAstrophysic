//! Banco della scala: zoom continuo da una persona all'universo osservabile.

use crate::formato::{lunghezza, tempo};
use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, SUPERFICIE};
use crate::stato::{Stanza, Stato};
use colossi_fisica::costanti::{ANNO_LUCE, C, UA};
use egui::{Align2, Color32, FontId, Frame, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const LARGHEZZA: f32 = 380.0;
const LOG_MIN: f64 = 0.0;
const LOG_MAX: f64 = 27.2;
/// Sopra e sotto questo rapporto col campo visivo un oggetto non si disegna.
const VISIBILE_MIN: f64 = 0.012;
const VISIBILE_MAX: f64 = 4.0;
const MPC: f64 = 3.085_677_6e22;
const KPC: f64 = MPC / 1e3;
const RIGHE_ETICHETTE: usize = 7;
/// Metri al secondo di chi cammina.
const VELOCITA_A_PIEDI: f64 = 1.4;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Pieno,
    Orbita,
    BucoNero,
}

/// Dove porta un clic sull'oggetto.
#[derive(Clone, Copy)]
pub enum Apre {
    Niente,
    BucoNero(f64),
    Banco(Stanza),
}

pub struct Oggetto {
    pub nome: &'static str,
    /// Raggio, o distanza per le orbite, in metri.
    pub raggio: f64,
    pub tipo: Tipo,
    pub nota: &'static str,
    pub apre: Apre,
}

const fn o(nome: &'static str, raggio: f64, tipo: Tipo, nota: &'static str, apre: Apre) -> Oggetto {
    Oggetto { nome, raggio, tipo, nota, apre }
}

/// Dal piu' piccolo al piu' grande. Valori arrotondati.
pub const OGGETTI: &[Oggetto] = &[
    o("Una persona", 0.9, Tipo::Pieno, "Il punto di partenza.", Apre::Niente),
    o("Una balena azzurra", 13.0, Tipo::Pieno, "Il più grande animale mai esistito.", Apre::Niente),
    o("Torre Eiffel", 165.0, Tipo::Pieno, "Circa 330 metri da cima a fondo.", Apre::Niente),
    o("Monte Everest, altezza", 8849.0, Tipo::Pieno, "Il cerchio ha per raggio la sua altezza.", Apre::Niente),
    o("Grande Raccordo Anulare", 1.08e4, Tipo::Orbita, "Circa 68 km di giro.", Apre::Niente),
    o("Stella di neutroni", 1.2e4, Tipo::Pieno, "Più massa del Sole, grande come una città.", Apre::Banco(Stanza::Neutroni)),
    o("Buco nero di 10 soli", 2.95e4, Tipo::BucoNero, "Un orizzonte grande come una provincia.", Apre::BucoNero(10.0)),
    o("Italia, da capo a capo", 6.0e5, Tipo::Orbita, "Il cerchio ha per raggio metà della sua lunghezza.", Apre::Niente),
    o("Luna", 1.737e6, Tipo::Pieno, "", Apre::Niente),
    o("Terra", 6.371e6, Tipo::Pieno, "", Apre::Niente),
    o("Giove", 6.991e7, Tipo::Pieno, "", Apre::Niente),
    o("Orbita della Luna", 3.844e8, Tipo::Orbita, "La luce la percorre in 1,3 secondi.", Apre::Niente),
    o("Sole", 6.957e8, Tipo::Pieno, "", Apre::Niente),
    o("Sagittarius A*", 1.27e10, Tipo::BucoNero, "Il buco nero al centro della Galassia.", Apre::BucoNero(4.3e6)),
    o("Orbita della Terra", UA, Tipo::Orbita, "Otto minuti di luce.", Apre::Niente),
    o("Betelgeuse", 5.0e11, Tipo::Pieno, "Una supergigante. Il raggio è incerto.", Apre::Niente),
    o("Orbita di Nettuno", 30.07 * UA, Tipo::Orbita, "", Apre::Niente),
    o("M87*", 1.9e13, Tipo::BucoNero, "Il primo buco nero fotografato.", Apre::BucoNero(6.5e9)),
    o("Sonda Voyager 1", 170.0 * UA, Tipo::Orbita, "La cosa più lontana mai costruita.", Apre::Niente),
    o("TON 618", 1.2e14, Tipo::BucoNero, "Tra i più massicci noti. La massa è incerta.", Apre::BucoNero(4.07e10)),
    o("Nube di Oort, bordo interno", 3.0e14, Tipo::Orbita, "Dove iniziano le comete.", Apre::Niente),
    o("Stella più vicina", 4.25 * ANNO_LUCE, Tipo::Orbita, "Proxima Centauri.", Apre::Niente),
    o("Le Pleiadi, distanza", 444.0 * ANNO_LUCE, Tipo::Orbita, "Un ammasso visibile a occhio nudo.", Apre::Niente),
    o("Centro della Galassia", 8.2 * KPC, Tipo::Orbita, "Da qui, 26 mila anni di luce.", Apre::Banco(Stanza::Osservatorio)),
    o("Via Lattea", 15.0 * KPC, Tipo::Pieno, "Il suo disco di stelle.", Apre::Banco(Stanza::Galassie)),
    o("Andromeda, distanza", 770.0 * KPC, Tipo::Orbita, "Si sta avvicinando.", Apre::Banco(Stanza::Galassie)),
    o("Gruppo Locale", 1.5 * MPC, Tipo::Orbita, "Il nostro gruppo di galassie.", Apre::Niente),
    o("Ammasso della Vergine", 16.5 * MPC, Tipo::Orbita, "L'ammasso di galassie più vicino.", Apre::Niente),
    o("Laniakea", 80.0 * MPC, Tipo::Orbita, "Il superammasso di cui facciamo parte.", Apre::Niente),
    o("Ammasso della Chioma", 100.0 * MPC, Tipo::Orbita, "Da qui nacque l'idea della materia oscura.", Apre::Banco(Stanza::Galassie)),
    o("Universo osservabile", 4.4e26, Tipo::Orbita, "Fin dove è arrivata la luce: da lì viene il fondo cosmico.", Apre::Banco(Stanza::Fondo)),
];

pub struct Scala {
    /// Logaritmo della meta' altezza visibile, in metri.
    pub log_vista: f64,
    /// Destinazione del viaggio automatico.
    verso: Option<f64>,
    pub viaggio: bool,
}

impl Default for Scala {
    fn default() -> Self {
        Self { log_vista: 0.6, verso: None, viaggio: false }
    }
}

impl Scala {
    pub fn vai(&mut self, oggetto: &Oggetto) {
        self.verso = Some((oggetto.raggio * 1.35).log10());
        self.viaggio = false;
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.viaggio {
            self.log_vista += dt * 0.45;
            if self.log_vista >= LOG_MAX {
                self.log_vista = LOG_MAX;
                self.viaggio = false;
            }
        } else if let Some(d) = self.verso {
            let k = 1.0 - (-dt * 4.5).exp();
            self.log_vista += (d - self.log_vista) * k;
            if (d - self.log_vista).abs() < 1e-3 {
                self.verso = None;
            }
        }
    }
}

fn tinta(o: &Oggetto) -> Color32 {
    match o.tipo {
        Tipo::Pieno => Color32::from_rgb(255, 196, 140),
        Tipo::Orbita => GIALLO,
        Tipo::BucoNero => CIANO,
    }
}

fn cerchio_a_tratti(p: &egui::Painter, c: Pos2, r: f32, tinta: Color32) {
    let n = ((r * 0.5) as usize).clamp(60, 480);
    let punti: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            c + Vec2::new(a.cos(), a.sin()) * r
        })
        .collect();
    p.extend(Shape::dashed_line(&punti, Stroke::new(1.2, tinta), 6.0, 5.0));
}

/// Quanto un oggetto e' presente: sfuma ai due estremi invece di comparire di colpo.
fn presenza(rapporto: f64) -> f32 {
    let basso = ((rapporto / VISIBILE_MIN).log10() / 0.5).clamp(0.0, 1.0);
    let alto = ((VISIBILE_MAX / rapporto).log10() / 0.3).clamp(0.0, 1.0);
    (basso * alto) as f32
}

fn vista(ui: &mut egui::Ui, area: Rect, s: &mut Scala, indice_click: &mut Option<usize>) {
    let r = ui.interact(area, ui.id().with("scala"), Sense::click_and_drag());
    if r.hovered() {
        let rotella = ui.input(|i| i.smooth_scroll_delta.y);
        if rotella != 0.0 {
            s.log_vista -= f64::from(rotella) * 0.0016;
            s.verso = None;
            s.viaggio = false;
        }
    }
    if r.dragged_by(egui::PointerButton::Primary) {
        s.log_vista += f64::from(r.drag_delta().y) * 0.012;
        s.verso = None;
        s.viaggio = false;
    }
    s.log_vista = s.log_vista.clamp(LOG_MIN, LOG_MAX);

    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    let centro = area.center() + Vec2::new(0.0, 22.0);
    let meta = 10f64.powf(s.log_vista);
    let px_per_m = f64::from(area.height()) * 0.42 / meta;

    // Gli oggetti pieni si disegnano dal grande al piccolo, cosi' i piccoli restano sopra.
    for ogg in OGGETTI.iter().rev() {
        let rapporto = ogg.raggio / meta;
        let k = presenza(rapporto);
        if k <= 0.0 {
            continue;
        }
        let raggio = (ogg.raggio * px_per_m) as f32;
        let c = tinta(ogg);
        match ogg.tipo {
            Tipo::Pieno => {
                p.circle_filled(centro, raggio, c.gamma_multiply(0.14 * k));
                p.circle_stroke(centro, raggio, Stroke::new(1.5, c.gamma_multiply(k)));
            }
            Tipo::Orbita => cerchio_a_tratti(&p, centro, raggio, c.gamma_multiply(0.85 * k)),
            Tipo::BucoNero => {
                p.circle_filled(centro, raggio, Color32::BLACK.gamma_multiply(k));
                p.circle_stroke(centro, raggio, Stroke::new(1.8, c.gamma_multiply(k)));
            }
        }
    }

    // Etichette a raggiera, per non accavallarle quando i cerchi sono vicini.
    let mut riga = 0;
    let mut sopra = None;
    let cursore = r.hover_pos();
    for (i, ogg) in OGGETTI.iter().enumerate() {
        let rapporto = ogg.raggio / meta;
        let k = presenza(rapporto);
        if k < 0.4 {
            continue;
        }
        let raggio = (ogg.raggio * px_per_m) as f32;
        let angolo = -0.95 + 0.34 * riga as f32;
        riga = (riga + 1) % RIGHE_ETICHETTE;
        let punto = centro + Vec2::new(angolo.cos(), angolo.sin()) * raggio;
        if !area.expand(-6.0).contains(punto) {
            continue;
        }
        let fine = punto + Vec2::new(26.0, angolo.sin() * 10.0);
        p.line_segment([punto, fine], Stroke::new(0.8, CHIARO.gamma_multiply(0.5 * k)));
        let colore = tinta(ogg);
        let galleggia = cursore.is_some_and(|q| (q - fine).length() < 70.0 && (q.y - fine.y).abs() < 11.0 && q.x > fine.x - 4.0);
        let scritta = p.text(
            fine + Vec2::new(3.0, 0.0),
            Align2::LEFT_CENTER,
            ogg.nome,
            FontId::proportional(if galleggia { 15.0 } else { 13.0 }),
            colore.gamma_multiply(k),
        );
        if cursore.is_some_and(|q| scritta.expand(3.0).contains(q)) {
            sopra = Some(i);
        }
    }
    if r.clicked() {
        *indice_click = sopra;
    }

    // Barra di scala con lunghezza leggibile.
    let lungo = 10f64.powf((meta * 0.5).log10().floor());
    let px = (lungo * px_per_m) as f32;
    let o = area.left_bottom() + Vec2::new(18.0, -20.0);
    p.line_segment([o, o + Vec2::new(px, 0.0)], Stroke::new(2.0, CHIARO));
    p.text(o + Vec2::new(0.0, -7.0), Align2::LEFT_BOTTOM, lunghezza(lungo), FontId::proportional(13.0), CHIARO);

    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, "Rotella, o trascina su e giù. Ogni cerchio ha la grandezza vera. Clic su un nome.", f.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, "I cerchi tratteggiati sono distanze o orbite, i neri sono orizzonti.", f, CIANO);
}

/// Disegna il banco e, se richiesto, apre un altro banco.
pub fn mostra(ui: &mut egui::Ui, s: &mut Stato) {
    let mut scelta = None;
    egui::Panel::bottom("barra_scala")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| barra(ui, &mut s.scala));
    egui::Panel::right("colonna_scala")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| colonna(ui, &mut s.scala, &mut scelta));
        });
    let mut click = None;
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        vista(ui, area, &mut s.scala, &mut click);
    });
    let indice = click.or(scelta);
    if let Some(i) = indice {
        let ogg = &OGGETTI[i];
        s.scala.vai(ogg);
        if click.is_some() {
            match ogg.apre {
                Apre::Niente => {}
                Apre::BucoNero(m) => {
                    s.masse = m;
                    s.stanza = Stanza::BucoNero;
                }
                Apre::Banco(b) => s.stanza = b,
            }
        }
    }
}

fn barra(ui: &mut egui::Ui, s: &mut Scala) {
    ui.horizontal_centered(|ui| {
        let etichetta = if s.viaggio { "Ferma" } else { "Viaggia verso il grande" };
        let viaggia = ui.add_sized([180.0, 30.0], egui::Button::new(RichText::new(etichetta).strong()));
        crate::visite::segna(ui, "viaggia", &viaggia);
        if viaggia.clicked() {
            s.viaggio = !s.viaggio;
            s.verso = None;
        }
        if ui.add_sized([100.0, 30.0], egui::Button::new("Torna a te")).clicked() {
            s.vai(&OGGETTI[0]);
        }
        ui.add_space(14.0);
        ui.spacing_mut().slider_width = (ui.available_width() - 60.0).clamp(120.0, 700.0);
        if ui.add(egui::Slider::new(&mut s.log_vista, LOG_MIN..=LOG_MAX).show_value(false).text("zoom")).changed() {
            s.verso = None;
            s.viaggio = false;
        }
    });
}

fn colonna(ui: &mut egui::Ui, s: &mut Scala, scelta: &mut Option<usize>) {
    let meta = 10f64.powf(s.log_vista);
    ui.label(RichText::new("In questo momento vedi").color(GRIGIO));
    ui.label(RichText::new(lunghezza(meta * 2.0)).size(24.0).strong().color(CIANO));
    ui.label(RichText::new(format!("La luce li attraversa in {}", tempo(meta * 2.0 / C))).color(CHIARO));
    ui.add_space(4.0);
    ui.label(RichText::new(format!("A piedi, senza fermarti: {}", tempo(meta * 2.0 / VELOCITA_A_PIEDI))).color(GRIGIO));
    ui.add_space(8.0);
    ui.separator();
    ui.label(RichText::new("DALLA PERSONA ALL'UNIVERSO").small().strong().color(GRIGIO));
    for (i, ogg) in OGGETTI.iter().enumerate() {
        let vicino = presenza(ogg.raggio / meta) > 0.4;
        let testo = RichText::new(ogg.nome).color(if vicino { tinta(ogg) } else { GRIGIO });
        let risposta = ui.selectable_label(false, testo);
        let risposta = if ogg.nota.is_empty() {
            risposta
        } else {
            risposta.on_hover_text(ogg.nota)
        };
        if risposta.clicked() {
            *scelta = Some(i);
        }
    }
    ui.add_space(6.0);
    ui.label(RichText::new("Grandezze arrotondate.").small().color(GRIGIO));
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn gli_oggetti_sono_in_ordine_di_grandezza() {
        assert!(OGGETTI.windows(2).all(|c| c[0].raggio < c[1].raggio));
    }

    #[test]
    fn il_viaggio_arriva_alla_fine() {
        let mut s = Scala { viaggio: true, ..Default::default() };
        for _ in 0..3000 {
            s.avanza(0.05);
        }
        assert_eq!(s.log_vista, LOG_MAX);
    }

    #[test]
    fn andare_a_un_oggetto_inquadra_la_sua_grandezza() {
        let mut s = Scala::default();
        s.vai(&OGGETTI[12]);
        for _ in 0..400 {
            s.avanza(0.05);
        }
        assert!((s.log_vista - (OGGETTI[12].raggio * 1.35).log10()).abs() < 0.01);
    }

    #[test]
    fn ogni_scala_mostra_qualcosa() {
        let mut l = LOG_MIN;
        while l < LOG_MAX {
            let meta = 10f64.powf(l);
            assert!(OGGETTI.iter().any(|o| presenza(o.raggio / meta) > 0.3), "vuoto a 10^{l}");
            l += 0.1;
        }
    }
}
