//! Banco delle galassie: la materia oscura come interruttore, e gli incontri.

use crate::formato::decimale;
use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE};
use colossi_fisica::galassie::{andromeda, incontro, stelle_lontane, una_galassia, Simulazione, UNITA_IN_MILIARDI_ANNI, VIA_LATTEA};
use egui::{Align2, Color32, FontId, Frame, Margin, Mesh, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const LARGHEZZA: f32 = 430.0;
const N_STELLE: usize = 7000;
const ALTEZZA_CURVA: f32 = 190.0;
const R_CURVA: f64 = 40.0;
const ARANCIO: Color32 = Color32::from_rgb(255, 196, 140);
const AZZURRO: Color32 = Color32::from_rgb(165, 205, 255);
const ROSSO: Color32 = Color32::from_rgb(255, 105, 95);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scena {
    Una,
    Incontro,
    Andromeda,
}

pub struct BancoGalassie {
    pub sim: Simulazione,
    pub scena: Scena,
    pub in_corso: bool,
    pub ritmo: f64,
    passaggio: f64,
    velocita: f64,
    stesso_verso: bool,
    attrito: bool,
    raggio: f64,
    centro: [f64; 2],
}

impl Default for BancoGalassie {
    fn default() -> Self {
        Self {
            sim: una_galassia(N_STELLE),
            scena: Scena::Una,
            in_corso: true,
            ritmo: 0.15,
            passaggio: 30.0,
            velocita: 350.0,
            stesso_verso: true,
            attrito: true,
            raggio: 30.0,
            centro: [0.0, 0.0],
        }
    }
}

impl BancoGalassie {
    pub fn costruisci(&mut self, scena: Scena) {
        self.scena = scena;
        let fattore = self.sim.fattore_alone;
        self.sim = match scena {
            Scena::Una => una_galassia(N_STELLE),
            Scena::Incontro => incontro(N_STELLE, self.passaggio, self.velocita, if self.stesso_verso { 1.0 } else { -1.0 }),
            Scena::Andromeda => andromeda(N_STELLE),
        };
        self.sim.fattore_alone = fattore;
        if scena != Scena::Una {
            self.sim.attrito = self.attrito;
        }
        (self.raggio, self.ritmo) = match scena {
            Scena::Una => (30.0, 0.15),
            Scena::Incontro => (130.0, 0.2),
            Scena::Andromeda => (480.0, 0.5),
        };
        self.centro = [if scena == Scena::Andromeda { 385.0 } else { 0.0 }, 0.0];
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            let durata = self.ritmo * dt / UNITA_IN_MILIARDI_ANNI;
            self.sim.avanza(durata, 40);
        }
    }
}

pub fn mostra(ui: &mut egui::Ui, g: &mut BancoGalassie) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        g.in_corso = !g.in_corso;
    }
    egui::Panel::bottom("tempo_galassie")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            if pannello::barra_tempo(ui, &mut g.in_corso, &mut g.ritmo, 0.01..=2.0, "miliardi di anni al secondo") {
                g.costruisci(g.scena);
                g.in_corso = true;
            }
        });
    egui::Panel::right("colonna_galassie")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, g));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        cielo(ui, area, g);
    });
}

fn cielo(ui: &mut egui::Ui, area: Rect, g: &mut BancoGalassie) {
    let r = ui.interact(area, ui.id().with("galassie"), Sense::click_and_drag());
    if r.hovered() {
        let rotella = ui.input(|i| i.smooth_scroll_delta.y);
        g.raggio = (g.raggio * f64::from(-rotella * 0.0025).exp()).clamp(5.0, 3000.0);
    }
    let px = f64::from(area.height()) * 0.5 / g.raggio;
    if r.dragged_by(PointerButton::Primary) || r.dragged_by(PointerButton::Secondary) {
        let d = r.drag_delta();
        g.centro[0] -= f64::from(d.x) / px;
        g.centro[1] += f64::from(d.y) / px;
    }
    let schermo = |x: f64, y: f64| area.center() + Vec2::new(((x - g.centro[0]) * px) as f32, -(((y - g.centro[1]) * px) as f32));

    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);

    let mut mesh = Mesh::default();
    mesh.reserve_triangles(g.sim.stelle.len() * 2);
    mesh.reserve_vertices(g.sim.stelle.len() * 4);
    let lato = 0.9_f32;
    for s in &g.sim.stelle {
        let q = schermo(f64::from(s.x[0]), f64::from(s.x[1]));
        if !area.contains(q) {
            continue;
        }
        let tinta = if s.madre == 0 { AZZURRO } else { ARANCIO };
        let k = mesh.vertices.len() as u32;
        mesh.colored_vertex(q + Vec2::new(-lato, -lato), tinta);
        mesh.colored_vertex(q + Vec2::new(lato, -lato), tinta);
        mesh.colored_vertex(q + Vec2::new(lato, lato), tinta);
        mesh.colored_vertex(q + Vec2::new(-lato, lato), tinta);
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k, k + 2, k + 3);
    }
    p.add(mesh);

    for c in &g.sim.galassie {
        let q = schermo(c.x[0], c.x[1]);
        p.circle_stroke(q, 7.0, Stroke::new(1.2, CHIARO.gamma_multiply(0.6)));
    }

    // Scala.
    let kpc = crate::banco::passo_tondo(130.0 / px);
    let o = area.left_bottom() + Vec2::new(16.0, -18.0);
    p.line_segment([o, o + Vec2::new((kpc * px) as f32, 0.0)], Stroke::new(2.0, CHIARO));
    p.text(
        o + Vec2::new(0.0, -6.0),
        Align2::LEFT_BOTTOM,
        format!("{kpc} kpc = {} anni luce", crate::formato::grande(kpc * 3261.6)),
        FontId::proportional(12.0),
        CHIARO,
    );

    let (uno, due) = match g.scena {
        Scena::Una => ("Spegni la materia oscura e guarda cosa tiene insieme la galassia.", "Le stelle si vedono, ma da sole non pesano abbastanza per trattenersi a vicenda."),
        Scena::Incontro => ("Due galassie si sfiorano. Cambia il verso di rotazione e confronta le code.", "Le stelle non si scontrano quasi mai: è la gravità a strapparle."),
        Scena::Andromeda => ("Noi e Andromeda, partendo da oggi. L'aspetto è semplificato, i tempi no.", "Si stanno avvicinando a circa 110 km/s."),
    };
    let f = FontId::proportional(13.0);
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, uno, f.clone(), GRIGIO);
    p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, due, f, CIANO);
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

fn curva_rotazione(ui: &mut egui::Ui, fattore: f64) {
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ALTEZZA_CURVA), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 3.0, FONDO);
    p.rect_stroke(area, 3.0, Stroke::new(1.0, LINEA), egui::StrokeKind::Inside);
    let dentro = area.shrink2(Vec2::new(10.0, 18.0));
    let v_massima = 320.0;
    let a_schermo = |r: f64, v: f64| {
        Pos2::new(
            dentro.left() + dentro.width() * (r / R_CURVA) as f32,
            dentro.bottom() - dentro.height() * (v / v_massima) as f32,
        )
    };
    let m = VIA_LATTEA;
    let linea = |funzione: &dyn Fn(f64) -> f64, tratto: Stroke| {
        let punti: Vec<Pos2> = (1..=200)
            .map(|i| {
                let r = R_CURVA * f64::from(i) / 200.0;
                a_schermo(r, funzione(r))
            })
            .collect();
        p.add(Shape::line(punti, tratto));
    };
    linea(&|r| m.v_circolare(r, 0.0), Stroke::new(1.4, GIALLO));
    linea(&|r| (colossi_fisica::galassie::G * fattore * m.massa_alone_entro(r) / r).sqrt(), Stroke::new(1.2, GRIGIO));
    linea(&|r| m.v_circolare(r, fattore), Stroke::new(2.4, CIANO));

    let f = FontId::proportional(11.0);
    p.text(area.left_top() + Vec2::new(8.0, 4.0), Align2::LEFT_TOP, "velocità delle orbite, km/s, fino a 40 kpc", FontId::monospace(10.0), GRIGIO);
    p.text(area.right_top() + Vec2::new(-8.0, 18.0), Align2::RIGHT_TOP, "totale", f.clone(), CIANO);
    p.text(area.right_top() + Vec2::new(-8.0, 32.0), Align2::RIGHT_TOP, "solo stelle", f.clone(), GIALLO);
    p.text(area.right_top() + Vec2::new(-8.0, 46.0), Align2::RIGHT_TOP, "solo alone", f, GRIGIO);
}

fn strumenti(ui: &mut egui::Ui, g: &mut BancoGalassie) {
    titoletto(ui, "SCENE");
    ui.horizontal(|ui| {
        for (scena, nome) in [(Scena::Una, "Una galassia"), (Scena::Incontro, "Due che si sfiorano"), (Scena::Andromeda, "Noi e Andromeda")] {
            if ui.selectable_label(g.scena == scena, nome).clicked() {
                g.costruisci(scena);
            }
        }
    });

    titoletto(ui, "MATERIA OSCURA");
    let mut acceso = g.sim.fattore_alone > 0.0;
    let accesa = ui.checkbox(&mut acceso, "Accesa");
    crate::visite::segna(ui, "alone", &accesa);
    if accesa.changed() {
        g.sim.fattore_alone = if acceso { 1.0 } else { 0.0 };
    }
    ui.add(egui::Slider::new(&mut g.sim.fattore_alone, 0.0..=3.0).text("quantità, rispetto al vero").fixed_decimals(2));
    curva_rotazione(ui, g.sim.fattore_alone);

    if g.scena == Scena::Incontro {
        titoletto(ui, "L'INCONTRO");
        let prima = (g.passaggio, g.velocita, g.stesso_verso);
        ui.add(egui::Slider::new(&mut g.passaggio, 0.0..=120.0).text("distanza di passaggio, kpc").fixed_decimals(0));
        ui.add(egui::Slider::new(&mut g.velocita, 100.0..=1000.0).text("velocità relativa, km/s").fixed_decimals(0));
        ui.checkbox(&mut g.stesso_verso, "Le due ruotano nello stesso verso");
        if prima != (g.passaggio, g.velocita, g.stesso_verso) && ui.input(|i| !i.pointer.any_down()) {
            g.costruisci(Scena::Incontro);
        }
    }
    if g.scena != Scena::Una {
        if ui.checkbox(&mut g.attrito, "Attrito dinamico: rallentano e si fondono").changed() {
            g.sim.attrito = g.attrito;
        }
    }

    titoletto(ui, "LETTURE");
    let lontane = stelle_lontane(&g.sim, 45.0);
    let totale = g.sim.stelle.len().max(1);
    egui::Grid::new("letture_galassie").num_columns(2).spacing([14.0, 3.0]).show(ui, |ui| {
        riga(ui, "Tempo", format!("{} miliardi di anni", decimale(g.sim.tempo_miliardi())));
        riga(ui, "Stelle disperse", format!("{:.1} %", lontane as f64 / totale as f64 * 100.0).replace('.', ","));
        riga(ui, "Velocità a 8 kpc", format!("{} km/s", decimale(VIA_LATTEA.v_circolare(8.0, g.sim.fattore_alone))));
        if g.sim.galassie.len() > 1 {
            riga(ui, "Distanza tra i centri", format!("{} kpc", decimale(g.sim.separazione())));
        }
    });
    if g.sim.galassie.len() > 1 && g.sim.separazione() < 15.0 {
        ui.label(RichText::new("I due centri sono ormai uno solo.").color(CIANO));
    }
    if g.sim.fattore_alone == 0.0 {
        ui.label(RichText::new("Senza l'alone le stelle esterne hanno troppa velocità per restare.").color(ROSSO));
    }
    ui.add_space(4.0);
    ui.label(
        RichText::new("Galassie come potenziali e stelle come particelle di prova, in un piano. Attrito approssimato.")
            .small()
            .color(GRIGIO),
    );
}
