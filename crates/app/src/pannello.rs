//! Disposizione del laboratorio: barre, attrezzi, telescopio.

use crate::formato::{densita, grande, lunghezza, tempo};
use crate::lab::{Strumento, ESPERIMENTI};
use crate::stato::{Stanza, Stato, AREE, PRESET, R_CAM_MAX, R_CAM_MIN};
use crate::{banco, benvenuto, cadere, curva, fondo, galassie, gemelli, giganti, neutroni, onde, orizzonte, osservatorio, scala, stelle, strumenti, guida, trecorpi, visite};
use colossi_fisica::costanti::UA;
use colossi_fisica::buco_nero::ISCO;
use colossi_fisica::geodetica::{Legge, Orologio};
use egui::{Color32, CornerRadius, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke, UiBuilder, Vec2};
use std::f32::consts::{FRAC_PI_2, TAU};

pub const FONDO: Color32 = Color32::from_rgb(9, 11, 17);
pub const SUPERFICIE: Color32 = Color32::from_rgb(16, 19, 27);
pub const LINEA: Color32 = Color32::from_rgb(38, 44, 58);
pub const CIANO: Color32 = Color32::from_rgb(90, 215, 255);
pub const VERDE: Color32 = Color32::from_rgb(110, 225, 130);
pub const GIALLO: Color32 = Color32::from_rgb(245, 200, 90);
pub const GRIGIO: Color32 = Color32::from_rgb(142, 149, 165);
pub const CHIARO: Color32 = Color32::from_rgb(226, 230, 240);

const LARGHEZZA_COLONNA: f32 = 450.0;
const QUOTA_MINIATURA: f32 = 0.40;
const ALTEZZA_BARRA: f32 = 44.0;
/// Sotto questa larghezza le schede diventano un menu.
const LARGHEZZA_SCHEDE: f32 = 760.0;
const ALTEZZA_COMANDI: f32 = 42.0;
pub const ALTEZZA_TEMPO: f32 = 50.0;

pub fn cornice() -> Frame {
    Frame::new().fill(SUPERFICIE).inner_margin(Margin::symmetric(10, 6))
}

/// Barra del tempo comune ai banchi di relativita'. Vero se va ricominciato.
pub fn barra_tempo(
    ui: &mut egui::Ui,
    in_corso: &mut bool,
    ritmo: &mut f64,
    campo: std::ops::RangeInclusive<f64>,
    unita: &str,
) -> bool {
    let mut ricomincia = false;
    ui.horizontal_centered(|ui| {
        let etichetta = if *in_corso { "Pausa" } else { "Avvia" };
        if ui.add_sized([70.0, 30.0], egui::Button::new(RichText::new(etichetta).strong())).clicked() {
            *in_corso = !*in_corso;
        }
        if ui.add_sized([86.0, 30.0], egui::Button::new("Ricomincia")).clicked() {
            ricomincia = true;
        }
        ui.add_space(16.0);
        ui.separator();
        ui.label(RichText::new("Ritmo").color(GRIGIO));
        ui.add(egui::Slider::new(ritmo, campo).logarithmic(true).show_value(false));
        ui.label(RichText::new(format!("{:.2} {unita}", *ritmo).replace('.', ",")).color(CHIARO));
    });
    ricomincia
}

/// Disegna il banco aperto. Restituisce l'area del telescopio, se c'e', in punti.
pub fn mostra(ui: &mut egui::Ui, s: &mut Stato, fps: f32) -> Option<Rect> {
    egui::Panel::top("barra").exact_size(ALTEZZA_BARRA).frame(cornice()).show(ui, |ui| barra(ui, s, fps));
    visite::mostra(ui, s);
    guida::pannello(ui, s);
    let area = match s.stanza {
        Stanza::Benvenuto => {
            benvenuto::mostra(ui, s);
            None
        }
        Stanza::Scala => {
            scala::mostra(ui, s);
            None
        }
        Stanza::Stelle => {
            stelle::mostra(ui, &mut s.stelle);
            None
        }
        Stanza::Orizzonte => {
            orizzonte::mostra(ui, &mut s.orizzonte);
            None
        }
        Stanza::BucoNero => Some(buco_nero(ui, s)),
        Stanza::Gemelli => {
            gemelli::mostra(ui, &mut s.gemelli);
            None
        }
        Stanza::Cadere => {
            cadere::mostra(ui, &mut s.cadere);
            None
        }
        Stanza::Curva => {
            curva::mostra(ui, &mut s.curva);
            None
        }
        Stanza::Neutroni => {
            apri_buco_nero(s, |s| neutroni::mostra(ui, &mut s.neutroni));
            None
        }
        Stanza::Giganti => {
            apri_buco_nero(s, |s| giganti::mostra(ui, &mut s.giganti));
            None
        }
        Stanza::TreCorpi => {
            trecorpi::mostra(ui, &mut s.trecorpi);
            None
        }
        Stanza::Galassie => {
            galassie::mostra(ui, &mut s.galassie);
            None
        }
        Stanza::Onde => {
            onde::mostra(ui, &mut s.onde);
            None
        }
        Stanza::Fondo => {
            fondo::mostra(ui, &mut s.fondo);
            None
        }
        Stanza::Osservatorio => {
            if let Some(o) = osservatorio::mostra(ui, &mut s.osservatorio) {
                s.masse = o.masse;
                let rs = s.buco_nero().raggio();
                s.lab.carica_orbita(o.a_ua * UA / rs, o.e);
                s.stanza = Stanza::BucoNero;
            }
            None
        }
    };
    visite::evidenzia(ui.ctx(), s);
    area
}

/// Se il banco chiede di aprire un buco nero, ci si sposta su quello.
fn apri_buco_nero(s: &mut Stato, banco: impl FnOnce(&mut Stato) -> Option<f64>) {
    if let Some(masse) = banco(s) {
        s.masse = masse;
        s.stanza = Stanza::BucoNero;
    }
}

fn buco_nero(ui: &mut egui::Ui, s: &mut Stato) -> Rect {
    tastiera(ui, s);

    egui::Panel::top("barra_buco_nero")
        .exact_size(ALTEZZA_COMANDI)
        .frame(cornice())
        .show(ui, |ui| barra_buco_nero(ui, s));
    egui::Panel::bottom("tempo").exact_size(ALTEZZA_TEMPO).frame(cornice()).show(ui, |ui| orologi(ui, s));
    egui::Panel::left("attrezzi")
        .exact_size(76.0)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(6)))
        .show(ui, |ui| attrezzi(ui, s));

    let mut telescopio = Rect::NOTHING;
    egui::Panel::right("colonna")
        .exact_size(LARGHEZZA_COLONNA)
        .resizable(false)
        .frame(Frame::NONE)
        .show(ui, |ui| {
            let altezza_utile = ui.ctx().content_rect().height() - ALTEZZA_BARRA - ALTEZZA_COMANDI - ALTEZZA_TEMPO;
            let misura = Vec2::new(LARGHEZZA_COLONNA, altezza_utile * QUOTA_MINIATURA);
            let (alto, _) = ui.allocate_exact_size(misura, Sense::hover());
            if s.telescopio_grande {
                banco::mostra(ui, alto, s);
            } else {
                telescopio = vista_telescopio(ui, alto, s);
            }
            Frame::new()
                .fill(SUPERFICIE)
                .inner_margin(Margin::same(10))
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .show(ui, |ui| strumenti::mostra(ui, s));
                });
        });

    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        if s.telescopio_grande {
            telescopio = vista_telescopio(ui, area, s);
        } else {
            visite::segna_rect(ui.ctx(), "banco", area);
            banco::mostra(ui, area, s);
        }
    });
    telescopio
}

fn tastiera(ui: &mut egui::Ui, s: &mut Stato) {
    if ui.ctx().egui_wants_keyboard_input() {
        return;
    }
    let lab = &mut s.lab;
    ui.input(|i| {
        use egui::Key::{Backspace, Delete, Num1, Num2, Num3, Num4, Num5, Period, Space, R};
        for (tasto, strumento) in [
            (Num1, Strumento::Seleziona),
            (Num2, Strumento::Luce),
            (Num3, Strumento::Sonda),
            (Num4, Strumento::Fascio),
            (Num5, Strumento::Orologio),
        ] {
            if i.key_pressed(tasto) {
                lab.strumento = strumento;
            }
        }
        if i.key_pressed(Space) {
            lab.in_corso = !lab.in_corso;
        }
        if i.key_pressed(Period) {
            lab.un_passo();
        }
        if i.key_pressed(R) {
            lab.ricomincia();
        }
        if i.key_pressed(Delete) || i.key_pressed(Backspace) {
            if let Some(indice) = lab.indice_selezionato() {
                lab.elimina(indice);
            }
        }
    });
}

fn barra(ui: &mut egui::Ui, s: &mut Stato, fps: f32) {
    ui.horizontal_centered(|ui| {
        let casa = s.stanza == Stanza::Benvenuto;
        if ui.selectable_label(casa, RichText::new("COLOSSI").strong().color(CIANO).size(15.0)).on_hover_text("Torna alla pagina iniziale").clicked() {
            s.stanza = Stanza::Benvenuto;
        }
        ui.add_space(8.0);
        ui.separator();
        let attuale = s.stanza.area();
        for (i, area) in AREE.iter().enumerate() {
            let scelta = attuale == Some(i);
            if ui.selectable_label(scelta, RichText::new(area.breve).strong()).on_hover_text(area.descrizione).clicked() && !scelta {
                s.stanza = area.banchi[0].0;
            }
        }
        if let Some(i) = s.stanza.area().filter(|i| AREE[*i].banchi.len() > 1) {
            ui.separator();
            let banchi = AREE[i].banchi;
            if ui.available_width() < LARGHEZZA_SCHEDE {
                // Finestra stretta: i banchi dell'area diventano un menu.
                let nome = banchi.iter().find(|q| q.0 == s.stanza).map_or("", |q| q.1);
                egui::ComboBox::from_id_salt("banchi").selected_text(RichText::new(nome).strong()).show_ui(ui, |ui| {
                    for (stanza, nome) in banchi {
                        ui.selectable_value(&mut s.stanza, *stanza, *nome);
                    }
                });
            } else {
                for (stanza, nome) in banchi {
                    let n = crate::guida::fatte(s, *stanza);
                    let totale = crate::guida::guida(*stanza).map_or(0, |g| g.sfide.len());
                    let testo = RichText::new(*nome);
                    let testo = if totale > 0 && n == totale { testo.color(VERDE) } else { testo };
                    ui.selectable_value(&mut s.stanza, *stanza, testo);
                }
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(format!("{fps:.0} fps")).small().color(GRIGIO));
            ui.add_space(10.0);
            if guida::guida(s.stanza).is_some() && !s.guida_visibile && ui.button(RichText::new("Guida").color(CIANO)).clicked() {
                s.guida_visibile = true;
                s.guida_aperta = true;
            }
            let mut scelta = None;
            ui.menu_button(RichText::new("Visite guidate").strong().color(CIANO), |ui| {
                ui.set_min_width(240.0);
                for (i, v) in visite::VISITE.iter().enumerate() {
                    if ui.button(format!("{}, {}", v.nome, v.durata)).clicked() {
                        scelta = Some(i);
                    }
                }
            });
            if let Some(i) = scelta {
                visite::inizia(s, i);
            }
        });
    });
}

fn barra_buco_nero(ui: &mut egui::Ui, s: &mut Stato) {
    ui.horizontal_centered(|ui| {
        ui.label(RichText::new("Massa").color(GRIGIO));
        ui.add(
            egui::Slider::new(&mut s.masse, 1.0..=1e11)
                .logarithmic(true)
                .show_value(false),
        );
        let etichetta = format!("{} masse solari", grande(s.masse));
        ui.menu_button(RichText::new(etichetta).strong(), |ui| {
            ui.set_min_width(280.0);
            for preset in &PRESET {
                if ui.selectable_label(s.masse == preset.masse, preset.nome).clicked() {
                    s.masse = preset.masse;
                }
            }
            ui.separator();
            let bh = s.buco_nero();
            egui::Grid::new("grandezze").num_columns(2).spacing([14.0, 4.0]).show(ui, |ui| {
                ui.label(RichText::new("Raggio dell'orizzonte").color(GRIGIO));
                ui.label(lunghezza(bh.raggio()));
                ui.end_row();
                ui.label(RichText::new("Densità media").color(GRIGIO));
                ui.label(densita(bh.densita_media()));
                ui.end_row();
                ui.label(RichText::new("La luce percorre un raggio in").color(GRIGIO));
                ui.label(tempo(bh.tempo_luce()));
                ui.end_row();
                ui.label(RichText::new("Disco, bordo interno").color(GRIGIO));
                ui.label(format!("{} K (stima)", grande(bh.temperatura_disco(ISCO))));
                ui.end_row();
            });
        });

        ui.separator();
        ui.label(RichText::new("Gravità").color(GRIGIO));
        let prima = (s.lab.legge, s.lab.confronta);
        ui.selectable_value(&mut s.lab.legge, Legge::Einstein, "Einstein");
        let newton = ui.selectable_value(&mut s.lab.legge, Legge::Newton, "Newton");
        visite::segna(ui, "newton", &newton);
        ui.checkbox(&mut s.lab.confronta, "Confronta le due");
        if prima != (s.lab.legge, s.lab.confronta) {
            s.lab.rilancia_tutti();
        }

        ui.separator();
        let mut carica = None;
        ui.menu_button(RichText::new("Esperimenti").strong(), |ui| {
            ui.set_min_width(240.0);
            for (i, esperimento) in ESPERIMENTI.iter().enumerate() {
                if ui.button(esperimento.nome).on_hover_text(esperimento.nota).clicked() {
                    carica = Some(i);
                }
            }
        });
        if let Some(i) = carica {
            s.lab.carica(i);
        }
    });
}

fn orologi(ui: &mut egui::Ui, s: &mut Stato) {
    let unita = s.buco_nero().tempo_luce();
    let lab = &mut s.lab;
    ui.horizontal_centered(|ui| {
        let etichetta = if lab.in_corso { "Pausa" } else { "Avvia" };
        if ui.add_sized([70.0, 30.0], egui::Button::new(RichText::new(etichetta).strong())).clicked() {
            lab.in_corso = !lab.in_corso;
        }
        if ui.add_sized([58.0, 30.0], egui::Button::new("Passo")).on_hover_text("Tasto .").clicked() {
            lab.un_passo();
        }
        if ui.add_sized([86.0, 30.0], egui::Button::new("Ricomincia")).on_hover_text("Tasto R").clicked() {
            lab.ricomincia();
        }
        if ui.add_sized([70.0, 30.0], egui::Button::new("Svuota")).clicked() {
            lab.svuota();
        }

        ui.add_space(16.0);
        ui.separator();
        ui.label(RichText::new("Ritmo").color(GRIGIO));
        ui.add(egui::Slider::new(&mut lab.velocita, 0.1..=1e7).logarithmic(true).show_value(false));
        ui.label(
            RichText::new(format!("1 s qui = {} là", tempo(lab.velocita * unita)))
                .color(CHIARO),
        );

        ui.add_space(16.0);
        ui.separator();
        ui.label(RichText::new("Comanda l'orologio").color(GRIGIO));
        let prima = lab.orologio;
        ui.selectable_value(&mut lab.orologio, Orologio::Lontano, "di chi guarda");
        let di_ciascuno = ui.selectable_value(&mut lab.orologio, Orologio::Proprio, "di ciascuno");
        visite::segna(ui, "di_ciascuno", &di_ciascuno);
        if prima != lab.orologio {
            lab.ricomincia();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(tempo(lab.t * unita)).monospace().strong().color(CIANO));
            ui.label(RichText::new("sono passati").color(GRIGIO));
        });
    });
}

fn attrezzi(ui: &mut egui::Ui, s: &mut Stato) {
    ui.spacing_mut().item_spacing.y = 4.0;
    for (strumento, nome, tasto) in [
        (Strumento::Seleziona, "Misura", "1"),
        (Strumento::Luce, "Luce", "2"),
        (Strumento::Sonda, "Sonda", "3"),
        (Strumento::Fascio, "Fascio", "4"),
        (Strumento::Orologio, "Orologio", "5"),
    ] {
        let attivo = s.lab.strumento == strumento;
        let (area, risposta) = ui.allocate_exact_size(Vec2::new(64.0, 62.0), Sense::click());
        let sopra = risposta.hovered();
        let fondo = if attivo {
            CIANO.gamma_multiply(0.16)
        } else if sopra {
            Color32::from_gray(32)
        } else {
            Color32::TRANSPARENT
        };
        let p = ui.painter();
        p.rect_filled(area, CornerRadius::same(6), fondo);
        if attivo {
            p.rect_stroke(area, CornerRadius::same(6), Stroke::new(1.0, CIANO), egui::StrokeKind::Inside);
        }
        let colore = if attivo { CIANO } else { CHIARO };
        icona(p, Pos2::new(area.center().x, area.top() + 23.0), strumento, colore);
        p.text(
            Pos2::new(area.center().x, area.bottom() - 11.0),
            egui::Align2::CENTER_CENTER,
            nome,
            egui::FontId::proportional(12.0),
            colore,
        );
        p.text(
            area.right_top() + Vec2::new(-7.0, 8.0),
            egui::Align2::CENTER_CENTER,
            tasto,
            egui::FontId::monospace(9.0),
            GRIGIO,
        );
        if risposta.clicked() {
            s.lab.strumento = strumento;
        }
    }
}

fn icona(p: &egui::Painter, c: Pos2, strumento: Strumento, colore: Color32) {
    let tratto = Stroke::new(1.7, colore);
    match strumento {
        Strumento::Seleziona => {
            p.circle_stroke(c, 10.0, tratto);
            p.line_segment([c - Vec2::new(14.0, 0.0), c + Vec2::new(14.0, 0.0)], tratto);
            p.line_segment([c - Vec2::new(0.0, 14.0), c + Vec2::new(0.0, 14.0)], tratto);
        }
        Strumento::Luce => {
            let punti: Vec<Pos2> = (0..=24)
                .map(|i| {
                    let k = i as f32 / 24.0;
                    c + Vec2::new(-14.0 + 28.0 * k, 4.0 * (k * TAU * 2.0).sin())
                })
                .collect();
            p.add(egui::Shape::line(punti, tratto));
        }
        Strumento::Sonda => {
            p.circle_filled(c + Vec2::new(-9.0, 5.0), 4.5, colore);
            p.line_segment([c + Vec2::new(-5.0, 2.0), c + Vec2::new(12.0, -9.0)], tratto);
            p.line_segment([c + Vec2::new(12.0, -9.0), c + Vec2::new(5.0, -9.0)], tratto);
            p.line_segment([c + Vec2::new(12.0, -9.0), c + Vec2::new(9.0, -3.0)], tratto);
        }
        Strumento::Fascio => {
            for y in [-9.0, -3.0, 3.0, 9.0] {
                p.line_segment([c + Vec2::new(-14.0, y), c + Vec2::new(14.0, y)], tratto);
            }
        }
        Strumento::Orologio => {
            p.circle_stroke(c, 12.0, tratto);
            p.line_segment([c, c + Vec2::new(0.0, -8.0)], tratto);
            let a: f32 = FRAC_PI_2 * 0.7;
            p.line_segment([c, c + Vec2::new(a.cos() * 6.0, a.sin() * 6.0)], tratto);
        }
    }
}

fn vista_telescopio(ui: &mut egui::Ui, area: Rect, s: &mut Stato) -> Rect {
    let risposta = ui.interact(area, ui.id().with("telescopio"), Sense::drag());
    visite::segna_rect(ui.ctx(), "telescopio", area);
    let delta = risposta.drag_delta();
    s.azimut -= f64::from(delta.x) * 0.006;
    s.elevazione = (s.elevazione + f64::from(delta.y) * 0.006).clamp(-1.5, 1.5);
    if risposta.hovered() {
        let rotella = ui.input(|i| i.smooth_scroll_delta.y);
        s.r_cam = (s.r_cam * f64::from(-rotella * 0.0025).exp()).clamp(R_CAM_MIN, R_CAM_MAX);
    }

    let comandi = Rect::from_min_size(area.min + Vec2::new(8.0, 6.0), Vec2::new(area.width() - 16.0, 26.0));
    // Un figlio separato: non deve spostare il cursore della colonna.
    let mut riga = ui.new_child(UiBuilder::new().max_rect(comandi));
    riga.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        ui.label(RichText::new("TELESCOPIO").small().strong().color(CHIARO));
        ui.add_space(8.0);
        ui.toggle_value(&mut s.disco, RichText::new("Disco").small());
        ui.toggle_value(&mut s.doppler, RichText::new("Doppler").small());
        ui.toggle_value(&mut s.scie, RichText::new("Scie").small());
        ui.toggle_value(&mut s.confronti, RichText::new("Righelli").small());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let testo = if s.telescopio_grande { "Riduci" } else { "Ingrandisci" };
            if ui.small_button(testo).clicked() {
                s.telescopio_grande = !s.telescopio_grande;
            }
        });
    });
    ui.painter_at(area).text(
        area.left_bottom() + Vec2::new(10.0, -8.0),
        egui::Align2::LEFT_BOTTOM,
        "Luce deviata: calcolata. Colori del disco e stelle: illustrativi.",
        egui::FontId::proportional(10.5),
        GRIGIO.gamma_multiply(0.85),
    );
    area
}

#[cfg(test)]
mod prove {
    //! Interazioni vere, senza finestra: eventi di input dati direttamente a egui.
    use super::*;
    use colossi_fisica::geodetica::{beta_circolare, Specie};
    use colossi_fisica::relativita::tempo_proprio;
    use egui::{Event, Key, Modifiers, PointerButton, RawInput};

    struct Simulatore {
        contesto: egui::Context,
        stato: Stato,
        eventi: Vec<Event>,
        modificatori: Modifiers,
        tempo: f64,
        texture: std::collections::HashMap<egui::TextureId, (usize, usize, Vec<egui::Color32>)>,
        ultimo: Vec<egui::epaint::ClippedShape>,
    }

    impl Simulatore {
        fn nuovo() -> Self {
            let mut s = Self {
                contesto: egui::Context::default(),
                stato: Stato::default(),
                eventi: Vec::new(),
                modificatori: Modifiers::NONE,
                tempo: 0.0,
                texture: Default::default(),
                ultimo: Vec::new(),
            };
            s.stato.stanza = Stanza::BucoNero;
            s.stato.guida_visibile = false;
            s.stato.lab.svuota();
            s.stato.lab.in_corso = false;
            s.giro();
            s.giro();
            s
        }

        fn giro(&mut self) {
            self.tempo += 1.0 / 60.0;
            let ingresso = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1536.0, 840.0))),
                events: std::mem::take(&mut self.eventi),
                modifiers: self.modificatori,
                time: Some(self.tempo),
                predicted_dt: 1.0 / 60.0,
                ..Default::default()
            };
            let stato = &mut self.stato;
            let uscita = self.contesto.run_ui(ingresso, |ui| {
                mostra(ui, stato, 60.0);
            });
            for (id, delta) in &uscita.textures_delta.set {
                let egui::ImageData::Color(img) = &delta.image;
                let voce = self.texture.entry(*id).or_insert((img.size[0], img.size[1], vec![egui::Color32::TRANSPARENT; img.size[0] * img.size[1]]));
                match delta.pos {
                    None => *voce = (img.size[0], img.size[1], img.pixels.clone()),
                    Some([x0, y0]) => {
                        for y in 0..img.size[1] {
                            for x in 0..img.size[0] {
                                voce.2[(y0 + y) * voce.0 + x0 + x] = img.pixels[y * img.size[0] + x];
                            }
                        }
                    }
                }
            }
            self.ultimo = uscita.shapes;
        }

        /// Disegna l'ultimo fotogramma con un rasterizzatore minimo e lo scrive in PPM.
        fn scatta(&self, percorso: &str) {
            let (w, h) = (1536usize, 840usize);
            let mut px = vec![[9.0f32, 11.0, 17.0]; w * h];
            let primitive = self.contesto.tessellate(self.ultimo.clone(), 1.0);
            for prim in primitive {
                let egui::epaint::Primitive::Mesh(mesh) = prim.primitive else { continue };
                let Some(tex) = self.texture.get(&mesh.texture_id) else { continue };
                let clip = prim.clip_rect;
                for tri in mesh.indices.chunks(3) {
                    let v = [mesh.vertices[tri[0] as usize], mesh.vertices[tri[1] as usize], mesh.vertices[tri[2] as usize]];
                    let area = (v[1].pos.x - v[0].pos.x) * (v[2].pos.y - v[0].pos.y) - (v[2].pos.x - v[0].pos.x) * (v[1].pos.y - v[0].pos.y);
                    if area.abs() < 1e-6 {
                        continue;
                    }
                    let min_x = v.iter().map(|q| q.pos.x).fold(f32::MAX, f32::min).max(clip.left()).max(0.0).floor() as usize;
                    let max_x = v.iter().map(|q| q.pos.x).fold(f32::MIN, f32::max).min(clip.right()).min(w as f32 - 1.0).ceil() as usize;
                    let min_y = v.iter().map(|q| q.pos.y).fold(f32::MAX, f32::min).max(clip.top()).max(0.0).floor() as usize;
                    let max_y = v.iter().map(|q| q.pos.y).fold(f32::MIN, f32::max).min(clip.bottom()).min(h as f32 - 1.0).ceil() as usize;
                    for y in min_y..=max_y {
                        for x in min_x..=max_x {
                            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
                            let b0 = ((v[1].pos.x - cx) * (v[2].pos.y - cy) - (v[2].pos.x - cx) * (v[1].pos.y - cy)) / area;
                            let b1 = ((v[2].pos.x - cx) * (v[0].pos.y - cy) - (v[0].pos.x - cx) * (v[2].pos.y - cy)) / area;
                            let b2 = 1.0 - b0 - b1;
                            if b0 < -0.001 || b1 < -0.001 || b2 < -0.001 {
                                continue;
                            }
                            let u = b0 * v[0].uv.x + b1 * v[1].uv.x + b2 * v[2].uv.x;
                            let vv = b0 * v[0].uv.y + b1 * v[1].uv.y + b2 * v[2].uv.y;
                            let tx = ((u * tex.0 as f32) as usize).min(tex.0 - 1);
                            let ty = ((vv * tex.1 as f32) as usize).min(tex.1 - 1);
                            let t = tex.2[ty * tex.0 + tx];
                            let mut c = [0.0f32; 4];
                            for k in 0..4 {
                                let vert = b0 * v[0].color.to_array()[k] as f32 + b1 * v[1].color.to_array()[k] as f32 + b2 * v[2].color.to_array()[k] as f32;
                                c[k] = vert / 255.0 * t.to_array()[k] as f32 / 255.0;
                            }
                            let dst = &mut px[y * w + x];
                            for k in 0..3 {
                                dst[k] = c[k] * 255.0 + dst[k] * (1.0 - c[3]);
                            }
                        }
                    }
                }
            }
            let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
            for p in px {
                out.extend(p.iter().map(|c| c.clamp(0.0, 255.0) as u8));
            }
            std::fs::write(percorso, out).unwrap();
        }

        fn muovi(&mut self, x: f32, y: f32) {
            self.eventi.push(Event::PointerMoved(Pos2::new(x, y)));
            self.giro();
        }

        fn bottone(&mut self, x: f32, y: f32, tasto: PointerButton, premuto: bool) {
            self.eventi.push(Event::PointerButton {
                pos: Pos2::new(x, y),
                button: tasto,
                pressed: premuto,
                modifiers: self.modificatori,
            });
            self.giro();
        }

        fn trascina(&mut self, da: (f32, f32), a: (f32, f32), tasto: PointerButton) {
            self.muovi(da.0, da.1);
            self.bottone(da.0, da.1, tasto, true);
            for i in 1..=8 {
                let k = i as f32 / 8.0;
                self.muovi(da.0 + (a.0 - da.0) * k, da.1 + (a.1 - da.1) * k);
            }
            self.bottone(a.0, a.1, tasto, false);
            self.giro();
        }

        fn premi(&mut self, tasto: Key) {
            self.eventi.push(Event::Key {
                key: tasto,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: self.modificatori,
            });
            self.giro();
        }
    }

    #[test]
    fn i_tasti_scelgono_gli_attrezzi_e_la_pausa() {
        let mut s = Simulatore::nuovo();
        s.premi(Key::Num4);
        assert!(s.stato.lab.strumento == Strumento::Fascio);
        s.premi(Key::Num2);
        assert!(s.stato.lab.strumento == Strumento::Luce);
        assert!(!s.stato.lab.in_corso);
        s.premi(Key::Space);
        assert!(s.stato.lab.in_corso);
    }

    #[test]
    fn trascinare_lancia_luce_e_fascio() {
        let mut s = Simulatore::nuovo();
        s.stato.lab.strumento = Strumento::Luce;
        s.trascina((300.0, 300.0), (420.0, 360.0), PointerButton::Primary);
        assert_eq!(s.stato.lab.oggetti.len(), 1);
        assert!(s.stato.lab.oggetti[0].lancio.specie == Specie::Luce);

        s.stato.lab.svuota();
        s.stato.lab.strumento = Strumento::Fascio;
        s.trascina((300.0, 300.0), (420.0, 300.0), PointerButton::Primary);
        assert_eq!(s.stato.lab.oggetti.len(), 13);
    }

    #[test]
    fn maiusc_aggancia_l_orbita_circolare() {
        let mut s = Simulatore::nuovo();
        s.stato.lab.strumento = Strumento::Sonda;
        s.modificatori = Modifiers::SHIFT;
        s.trascina((700.0, 200.0), (730.0, 240.0), PointerButton::Primary);
        let lancio = s.stato.lab.oggetti[0].lancio;
        let atteso = beta_circolare(lancio.raggio(), s.stato.lab.legge);
        assert!((lancio.beta - atteso).abs() < 1e-9);
        let scarto = (lancio.direzione - lancio.azimut()).sin().abs();
        assert!((scarto - 1.0).abs() < 1e-9);
    }

    #[test]
    fn il_tasto_destro_sposta_la_mappa() {
        let mut s = Simulatore::nuovo();
        let prima = s.stato.lab.vista.centro;
        s.trascina((500.0, 500.0), (620.0, 500.0), PointerButton::Secondary);
        let dopo = s.stato.lab.vista.centro;
        assert!(dopo[0] < prima[0] && (dopo[1] - prima[1]).abs() < 1e-9);
        assert!(s.stato.lab.oggetti.is_empty());
    }

    #[test]
    fn un_clic_con_l_orologio_mette_un_orologio() {
        let mut s = Simulatore::nuovo();
        s.stato.lab.strumento = Strumento::Orologio;
        s.muovi(500.0, 300.0);
        s.bottone(500.0, 300.0, PointerButton::Primary, true);
        s.bottone(500.0, 300.0, PointerButton::Primary, false);
        s.giro();
        assert_eq!(s.stato.lab.oggetti.len(), 1);
        assert!(s.stato.lab.oggetti[0].fisso);
    }

    fn clic(s: &mut Simulatore, x: f32, y: f32) {
        s.muovi(x, y);
        s.bottone(x, y, PointerButton::Primary, true);
        s.bottone(x, y, PointerButton::Primary, false);
        s.giro();
    }

    const Y_COMANDI: f32 = 65.0;

    /// Le schede cambiano larghezza: si cerca quella giusta dal comportamento.
    fn apri(s: &mut Simulatore, stanza: Stanza) {
        // Prima l'area, poi il banco dentro l'area.
        let area = stanza.area().expect("banco senza area");
        for x in (100..1500).step_by(6) {
            if s.stato.stanza.area() == Some(area) {
                break;
            }
            clic(s, x as f32, 22.0);
        }
        for x in (100..1500).step_by(6) {
            if s.stato.stanza == stanza {
                return;
            }
            clic(s, x as f32, 22.0);
            // Un clic su un'altra area cambia la fila dei banchi: si torna indietro.
            if s.stato.stanza.area() != Some(area) {
                s.stato.stanza = AREE[area].banchi[0].0;
                s.giro();
            }
        }
        assert!(s.stato.stanza == stanza, "scheda non trovata");
    }

    #[test]
    fn dalla_pagina_iniziale_si_entra_in_un_area() {
        let mut s = Simulatore::nuovo();
        s.stato.stanza = Stanza::Benvenuto;
        s.giro();
        s.giro();
        // La parte alta della prima scheda porta al primo banco dei colossi.
        for y in (330..900).step_by(10) {
            clic(&mut s, 300.0, y as f32);
            if s.stato.stanza != Stanza::Benvenuto {
                break;
            }
        }
        assert!(s.stato.stanza == Stanza::Scala, "non si e' entrati");
        assert!(s.stato.visita.is_none(), "si e' entrati da una visita e non da una scheda");
        clic(&mut s, 40.0, 22.0);
        assert!(s.stato.stanza == Stanza::Benvenuto, "il nome in alto riporta all'inizio");
    }

    #[test]
    fn la_fascia_della_guida_si_riduce_e_si_nasconde() {
        let mut s = Simulatore::nuovo();
        s.stato.guida_visibile = true;
        s.giro();
        // Il pulsante Riduci sta a destra nella fascia in fondo.
        let mut ridotta = false;
        for x in (1200..1536).rev().step_by(6) {
            for y in [840.0 - 132.0 + 14.0] {
                clic(&mut s, x as f32, y);
                if !s.stato.guida_aperta {
                    ridotta = true;
                    break;
                }
            }
            if ridotta || !s.stato.guida_visibile {
                break;
            }
        }
        assert!(ridotta || !s.stato.guida_visibile, "nessun pulsante della fascia risponde");
    }

    #[test]
    fn la_guida_si_spunta_davvero_usando_il_banco() {
        let mut s = Simulatore::nuovo();
        assert_eq!(crate::guida::fatte(&s.stato, Stanza::BucoNero), 0);
        s.stato.lab.strumento = Strumento::Sonda;
        s.trascina((400.0, 300.0), (440.0, 340.0), PointerButton::Primary);
        s.stato.avanza(0.02);
        assert!(crate::guida::fatte(&s.stato, Stanza::BucoNero) >= 1);
    }

    #[test]
    fn la_barra_cambia_banco_e_il_menu_carica_gli_esperimenti() {
        let mut s = Simulatore::nuovo();
        apri(&mut s, Stanza::Gemelli);
        assert!(s.stato.stanza == Stanza::Gemelli);
        apri(&mut s, Stanza::Cadere);
        assert!(s.stato.stanza == Stanza::Cadere);
        apri(&mut s, Stanza::Curva);
        assert!(s.stato.stanza == Stanza::Curva);
        apri(&mut s, Stanza::BucoNero);
        assert!(s.stato.stanza == Stanza::BucoNero);

        // Il pulsante si sposta con la barra: lo si cerca dal comportamento.
        let x_menu = (300..1200)
            .step_by(8)
            .map(|x| x as f32)
            .find(|&x| {
                s.stato.lab.nota = None;
                clic(&mut s, x, Y_COMANDI);
                clic(&mut s, x + 30.0, Y_COMANDI + 26.0);
                s.stato.stanza == Stanza::BucoNero && s.stato.lab.nota.is_some()
            })
            .expect("menu Esperimenti");

        // Si apre il menu e si cerca la riga di ogni esperimento scendendo.
        let mut trovati = std::collections::HashSet::new();
        for y in (80..360).step_by(6) {
            s.stato.lab.nota = None;
            clic(&mut s, x_menu, Y_COMANDI);
            clic(&mut s, x_menu + 30.0, y as f32);
            if let Some(nota) = s.stato.lab.nota {
                trovati.insert(nota);
            }
            s.giro();
        }
        assert_eq!(trovati.len(), ESPERIMENTI.len(), "{trovati:?}");
    }

    #[test]
    fn tenere_premuto_aggiungi_materia_fa_crollare_la_stella() {
        let mut s = Simulatore::nuovo();
        apri(&mut s, Stanza::Neutroni);
        assert!(s.stato.stanza == Stanza::Neutroni);
        s.muovi(1218.0, 330.0);
        s.bottone(1218.0, 330.0, PointerButton::Primary, true);
        for _ in 0..900 {
            s.stato.avanza(1.0 / 60.0);
            s.giro();
        }
        s.bottone(1218.0, 330.0, PointerButton::Primary, false);
        assert!(s.stato.neutroni.e_buco_nero());
    }

    #[test]
    fn nel_banco_giganti_si_trascina_il_seme_e_si_sposta_l_istante() {
        let mut s = Simulatore::nuovo();
        apri(&mut s, Stanza::Giganti);
        assert!(s.stato.stanza == Stanza::Giganti);
        s.giro();
        let massa = s.stato.giganti.massa_seme;
        s.trascina((144.0, 727.0), (300.0, 520.0), PointerButton::Primary);
        assert!(s.stato.giganti.massa_seme > massa * 10.0);
        let t = s.stato.giganti.t_cursore;
        clic(&mut s, 880.0, 300.0);
        assert!(s.stato.giganti.t_cursore > t);
    }

    #[test]
    fn i_cursori_non_alterano_il_risultato_del_calcolo() {
        let mut s = Simulatore::nuovo();
        s.stato.stanza = Stanza::Osservatorio;
        s.stato.osservatorio.calcola();
        let orbita = s.stato.osservatorio.orbita;
        for _ in 0..5 {
            s.giro();
        }
        assert_eq!(s.stato.osservatorio.orbita, orbita);
        assert!(s.stato.osservatorio.incertezze.is_some());
    }

    #[test]
    fn le_visite_disegnano_ogni_tappa_senza_errori() {
        let mut s = Simulatore::nuovo();
        for (i, v) in visite::VISITE.iter().enumerate() {
            visite::inizia(&mut s.stato, i);
            for k in 0..v.tappe.len() {
                (v.tappe[k].prepara)(&mut s.stato);
                s.stato.avanza(0.02);
                s.giro();
            }
            s.stato.visita = None;
        }
    }

    #[test]
    fn nella_scala_la_rotella_cambia_la_vista() {
        let mut s = Simulatore::nuovo();
        s.stato.stanza = Stanza::Scala;
        s.giro();
        let prima = s.stato.scala.log_vista;
        s.trascina((500.0, 400.0), (500.0, 200.0), PointerButton::Primary);
        assert!(s.stato.scala.log_vista < prima || s.stato.scala.log_vista == 0.0);
    }

    #[test]
    fn il_banco_tre_corpi_si_apre_e_una_partenza_si_trascina() {
        let mut s = Simulatore::nuovo();
        apri(&mut s, Stanza::TreCorpi);
        s.giro();
        s.stato.trecorpi.in_corso = false;
        let area = Rect::from_min_max(Pos2::new(0.0, 44.0), Pos2::new(1106.0, 790.0));
        let q = s.stato.trecorpi.schermo_partenza(area, 2);
        let prima = s.stato.trecorpi.iniziale[2].x;
        s.trascina((q.x, q.y), (q.x + 60.0, q.y - 40.0), PointerButton::Primary);
        assert_ne!(s.stato.trecorpi.iniziale[2].x, prima);
        assert!(s.stato.trecorpi.sistema.t == 0.0);
    }

    #[test]
    fn nell_osservatorio_un_clic_toglie_una_misura() {
        let mut s = Simulatore::nuovo();
        s.stato.stanza = Stanza::Osservatorio;
        s.giro();
        s.giro();
        let centrale = Rect::from_min_max(Pos2::new(0.0, 44.0), Pos2::new(1106.0, 840.0));
        let q = s.stato.osservatorio.schermo_prima_posizione(centrale);
        clic(&mut s, q.x, q.y);
        assert!(!s.stato.osservatorio.dati.posizioni[0].attiva);
    }

    #[test]
    fn i_banchi_di_relativita_si_aprono_e_rispondono() {
        let mut s = Simulatore::nuovo();
        s.stato.stanza = Stanza::Gemelli;
        s.giro();
        s.giro();
        // Si trascina la svolta del viaggio verso destra e si misura il tempo proprio.
        s.stato.gemelli.scena = false;
        let prima = tempo_proprio(&s.stato.gemelli.linea);
        let centrale = Rect::from_min_max(Pos2::new(0.0, 44.0), Pos2::new(1136.0, 790.0));
        let q = s.stato.gemelli.posizione_vertice(centrale, 1);
        s.trascina((q.x, q.y), (q.x + 30.0, q.y), PointerButton::Primary);
        let dopo = tempo_proprio(&s.stato.gemelli.linea);
        assert!(dopo < prima, "piu' lontano nello stesso tempo: {dopo} contro {prima}");

        s.stato.stanza = Stanza::Cadere;
        s.giro();
        let quota = s.stato.cadere.h[24];
        s.trascina((900.0, 420.0), (900.0, 300.0), PointerButton::Primary);
        assert!(s.stato.cadere.h[24] > quota, "trascinare in alto alza la curva");
        s.stato.stanza = Stanza::Curva;
        s.giro();
        let partenza = s.stato.curva.partenza;
        s.muovi(300.0, 300.0);
        s.bottone(300.0, 300.0, PointerButton::Primary, true);
        s.bottone(300.0, 300.0, PointerButton::Primary, false);
        s.giro();
        assert_ne!(s.stato.curva.partenza, partenza);
    }

    #[test]
    fn anteprime_su_file() {
        let Ok(cartella) = std::env::var("ANTEPRIMA") else { return };
        let stanze = [("gemelli", Stanza::Gemelli), ("cadere", Stanza::Cadere), ("curva", Stanza::Curva), ("osservatorio", Stanza::Osservatorio), ("benvenuto", Stanza::Benvenuto), ("neutroni", Stanza::Neutroni), ("giganti", Stanza::Giganti), ("galassie", Stanza::Galassie), ("onde", Stanza::Onde), ("fondo", Stanza::Fondo), ("scala", Stanza::Scala), ("trecorpi", Stanza::TreCorpi)];
        for (nome, stanza) in stanze {
            let mut s = Simulatore::nuovo();
            s.stato.stanza = stanza;
            match nome {
                "cadere" => s.stato.cadere.avanzamento = 0.5,
                "gemelli" => s.stato.gemelli.avanzamento = 0.5,
                _ => {}
            }
            for _ in 0..90 {
                s.giro();
            }
            s.scatta(&format!("{cartella}/{nome}.ppm"));
        }
    }
}
