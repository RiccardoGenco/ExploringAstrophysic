//! Perche' cadiamo: fra tutti i percorsi possibili, quello libero e' quello in
//! cui l'orologio segna di piu'.

use crate::pannello::{self, CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, SUPERFICIE, VERDE};
use colossi_fisica::costanti::C;
use colossi_fisica::relativita::{caduta_libera, rilassa, scarto_orologio};
use egui::{Align2, Color32, FontId, Frame, Margin, Mesh, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2};

const INTERVALLI: usize = 48;
const LARGHEZZA: f32 = 400.0;
const LARGHEZZA_TORRE: f32 = 60.0;
const ALLARGAMENTO: f64 = 4.0;
const GIRI_PER_FOTOGRAMMA: usize = 24;
const PALLA: Color32 = Color32::from_rgb(255, 160, 90);

const GRAVITA: [(&str, f64); 4] = [("Terra", 9.81), ("Luna", 1.62), ("Giove", 24.8), ("Sole", 274.0)];

pub struct Cadere {
    pub h: Vec<f64>,
    pub durata: f64,
    pub g: f64,
    pub natura: bool,
    pub soluzione: bool,
    pub cornice_caduta: bool,
    pub in_corso: bool,
    pub avanzamento: f64,
    pub ritmo: f64,
    trascinato: Option<usize>,
    scala_ferma: Option<(f64, f64)>,
}

impl Default for Cadere {
    fn default() -> Self {
        let mut c = Self {
            h: Vec::new(),
            durata: 2.0,
            g: 9.81,
            natura: false,
            soluzione: false,
            cornice_caduta: false,
            in_corso: true,
            avanzamento: 0.0,
            ritmo: 0.35,
            trascinato: None,
            scala_ferma: None,
        };
        c.tenda();
        c
    }
}

impl Cadere {
    /// Un percorso a punta, chiaramente non naturale.
    fn tenda(&mut self) {
        let n = INTERVALLI as f64;
        self.h = (0..=INTERVALLI)
            .map(|i| {
                let k = i as f64 / n;
                9.0 * (1.0 - (2.0 * k - 1.0).abs())
            })
            .collect();
        self.natura = false;
    }

    fn piatto(&mut self) {
        let (a, b) = (self.h[0], self.h[INTERVALLI]);
        let n = INTERVALLI as f64;
        for (i, h) in self.h.iter_mut().enumerate() {
            *h = a + (b - a) * i as f64 / n;
        }
        self.natura = false;
    }

    fn dt(&self) -> f64 {
        self.durata / INTERVALLI as f64
    }

    fn libera(&self) -> Vec<f64> {
        caduta_libera(self.h[0], self.h[INTERVALLI], INTERVALLI, self.dt(), self.g)
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            self.avanzamento = (self.avanzamento + self.ritmo * dt).fract();
        }
        if self.natura {
            let passo = self.dt();
            rilassa(&mut self.h, passo, self.g, GIRI_PER_FOTOGRAMMA);
        }
    }

    /// Quanto togliere a ogni istante per guardare da chi cade.
    fn scostamento(&self, i: usize) -> f64 {
        if self.cornice_caduta {
            let t = i as f64 * self.dt();
            0.5 * self.g * t * (self.durata - t)
        } else {
            0.0
        }
    }

    fn altezza_interpolata(&self, curva: &[f64], k: f64) -> f64 {
        let x = k * INTERVALLI as f64;
        let i = (x as usize).min(INTERVALLI - 1);
        curva[i] + (curva[i + 1] - curva[i]) * (x - i as f64)
    }

    fn visibile(&self, curva: &[f64]) -> Vec<f64> {
        curva.iter().enumerate().map(|(i, h)| h - self.scostamento(i)).collect()
    }

    fn scala(&self) -> (f64, f64) {
        let suolo = self.visibile(&[0.0; INTERVALLI + 1]);
        let (mut basso, mut alto) = (0.0_f64, 1.0_f64);
        for v in self.visibile(&self.h).iter().chain(&self.visibile(&self.libera())).chain(&suolo) {
            basso = basso.min(*v);
            alto = alto.max(*v);
        }
        let margine = (alto - basso) * 0.14;
        (basso - margine, alto + margine)
    }
}

/// Tempi piccolissimi con l'unita' adatta.
fn minuscolo(s: f64) -> String {
    let a = s.abs();
    let (valore, unita) = if a >= 1e-9 {
        (s * 1e9, "ns")
    } else if a >= 1e-12 {
        (s * 1e12, "ps")
    } else if a >= 1e-15 {
        (s * 1e15, "fs")
    } else {
        (s * 1e18, "as")
    };
    format!("{valore:+.1} {unita}").replace('.', ",")
}

pub fn mostra(ui: &mut egui::Ui, c: &mut Cadere) {
    if !ui.ctx().egui_wants_keyboard_input() && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        c.in_corso = !c.in_corso;
    }
    egui::Panel::bottom("tempo_cadere")
        .exact_size(pannello::ALTEZZA_TEMPO)
        .frame(pannello::cornice())
        .show(ui, |ui| {
            if pannello::barra_tempo(ui, &mut c.in_corso, &mut c.ritmo, 0.05..=2.0, "giri al secondo") {
                c.avanzamento = 0.0;
                c.in_corso = true;
            }
        });
    egui::Panel::right("colonna_cadere")
        .exact_size(LARGHEZZA)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE).inner_margin(Margin::same(12)))
        .show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| strumenti(ui, c));
        });
    egui::CentralPanel::default().frame(Frame::NONE).show(ui, |ui| {
        let area = ui.max_rect();
        grafico(ui, area, c);
    });
}

fn titoletto(ui: &mut egui::Ui, testo: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(testo).small().strong().color(GRIGIO));
}

fn barra(ui: &mut egui::Ui, nome: &str, valore: f64, massimo: f64, tinta: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(nome).color(GRIGIO));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(minuscolo(valore)).monospace().color(CHIARO));
        });
    });
    let (area, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 8.0), Sense::hover());
    let p = ui.painter_at(area);
    p.rect_filled(area, 2.0, LINEA);
    let k = (valore.abs() / massimo.max(1e-30)).clamp(0.0, 1.0) as f32;
    p.rect_filled(Rect::from_min_size(area.min, Vec2::new(area.width() * k, 8.0)), 2.0, tinta);
    ui.add_space(4.0);
}

fn strumenti(ui: &mut egui::Ui, c: &mut Cadere) {
    let dt = c.dt();
    let (quota, velocita) = scarto_orologio(&c.h, dt, c.g);
    let totale = quota - velocita;
    let (quota_l, velocita_l) = scarto_orologio(&c.libera(), dt, c.g);
    let migliore = quota_l - velocita_l;

    ui.label(RichText::new("Il suo orologio, rispetto a chi resta a terra").color(GRIGIO));
    let tinta = if (totale - migliore).abs() < migliore.abs() * 0.002 { VERDE } else { CHIARO };
    ui.label(RichText::new(minuscolo(totale)).monospace().size(30.0).strong().color(tinta));
    ui.add_space(4.0);

    let massimo = quota.abs().max(velocita.abs()).max(migliore.abs());
    barra(ui, "Guadagnato stando in alto", quota, massimo, CIANO);
    barra(ui, "Perso andando veloce", velocita, massimo, GIALLO);
    barra(ui, "Il massimo possibile", migliore, massimo, VERDE);

    titoletto(ui, "PROVA");
    ui.horizontal(|ui| {
        ui.toggle_value(&mut c.natura, "Lascia fare alla natura");
        ui.toggle_value(&mut c.soluzione, "Mostra la caduta libera");
    });
    ui.horizontal(|ui| {
        if ui.small_button("A punta").clicked() {
            c.tenda();
        }
        if ui.small_button("Immobile").clicked() {
            c.piatto();
        }
    });

    titoletto(ui, "CHI GUARDA");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut c.cornice_caduta, false, "Da terra");
        ui.selectable_value(&mut c.cornice_caduta, true, "Da chi cade");
    });

    titoletto(ui, "DOVE");
    ui.horizontal(|ui| {
        for (nome, g) in GRAVITA {
            ui.selectable_value(&mut c.g, g, nome);
        }
    });
    ui.add(egui::Slider::new(&mut c.durata, 0.5..=8.0).text("secondi di volo").fixed_decimals(1));
    ui.label(
        RichText::new(format!(
            "Ogni metro di quota accelera l'orologio di {} parti su un miliardo di miliardi.",
            format!("{:.1}", c.g / (C * C) * 1e18).replace('.', ",")
        ))
        .color(GRIGIO),
    );
}

fn sfondo(p: &Painter, piano: Rect) {
    let mut mesh = Mesh::default();
    let alto = Color32::from_rgb(20, 42, 62);
    let basso = Color32::from_rgb(9, 11, 17);
    for (punto, tinta) in [
        (piano.left_top(), alto),
        (piano.right_top(), alto),
        (piano.left_bottom(), basso),
        (piano.right_bottom(), basso),
    ] {
        mesh.colored_vertex(punto, tinta);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    p.add(mesh);
}

fn grafico(ui: &mut egui::Ui, area: Rect, c: &mut Cadere) {
    let risposta = ui.interact(area, ui.id().with("cadere"), Sense::drag());
    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);

    let piano = Rect::from_min_max(
        area.min + Vec2::new(LARGHEZZA_TORRE + 34.0, 62.0),
        area.max - Vec2::new(24.0, 42.0),
    );
    let (basso, alto) = c.scala_ferma.unwrap_or_else(|| c.scala());
    let n = INTERVALLI as f32;
    let x_di = |i: usize| piano.left() + piano.width() * i as f32 / n;
    let y_di = |h: f64| piano.bottom() - piano.height() * ((h - basso) / (alto - basso)) as f32;
    let metri_per_px = (alto - basso) / f64::from(piano.height());

    if risposta.drag_started_by(PointerButton::Primary) {
        let premuto = ui.input(|i| i.pointer.press_origin()).unwrap_or(area.center());
        let k = ((premuto.x - piano.left()) / piano.width() * n).round();
        if (0.0..=n).contains(&k) {
            c.trascinato = Some(k as usize);
            c.scala_ferma = Some((basso, alto));
            c.natura = false;
        }
    }
    if let (Some(i), true) = (c.trascinato, risposta.dragged_by(PointerButton::Primary)) {
        let dy = -f64::from(risposta.drag_delta().y) * metri_per_px;
        if i == 0 || i == INTERVALLI {
            c.h[i] = (c.h[i] + dy).max(0.0);
        } else {
            for j in 1..INTERVALLI {
                let d = j as f64 - i as f64;
                let peso = (-d * d / (2.0 * ALLARGAMENTO * ALLARGAMENTO)).exp();
                c.h[j] = (c.h[j] + dy * peso).max(0.0);
            }
        }
    }
    if risposta.drag_stopped_by(PointerButton::Primary) {
        c.trascinato = None;
        c.scala_ferma = None;
    }

    sfondo(&p, piano);
    let etichetta = FontId::proportional(12.0);
    p.text(piano.right_top() + Vec2::new(-8.0, 8.0), Align2::RIGHT_TOP, "qui il tempo scorre più in fretta", etichetta.clone(), CIANO);
    p.text(piano.right_bottom() + Vec2::new(-8.0, -8.0), Align2::RIGHT_BOTTOM, "qui più piano", etichetta.clone(), GRIGIO);

    // Il suolo.
    let suolo = c.visibile(&[0.0; INTERVALLI + 1]);
    let linea_suolo: Vec<Pos2> = suolo.iter().enumerate().map(|(i, h)| Pos2::new(x_di(i), y_di(*h))).collect();
    p.add(Shape::line(linea_suolo, Stroke::new(2.0, GRIGIO)));

    if c.soluzione {
        let libera = c.visibile(&c.libera());
        let punti: Vec<Pos2> = libera.iter().enumerate().map(|(i, h)| Pos2::new(x_di(i), y_di(*h))).collect();
        p.extend(Shape::dashed_line(&punti, Stroke::new(1.6, VERDE), 7.0, 5.0));
    }

    let visibile = c.visibile(&c.h);
    let punti: Vec<Pos2> = visibile.iter().enumerate().map(|(i, h)| Pos2::new(x_di(i), y_di(*h))).collect();
    p.add(Shape::line(punti, Stroke::new(3.0, PALLA)));

    // La palla sulla curva e nella torre, alla stessa quota.
    let k = c.avanzamento;
    let quota = c.altezza_interpolata(&visibile, k);
    let qx = piano.left() + piano.width() * k as f32;
    let centro = Pos2::new(qx, y_di(quota));
    p.circle_filled(centro, 8.0, PALLA);
    let torre_x = area.left() + 18.0 + LARGHEZZA_TORRE * 0.5;
    p.line_segment(
        [Pos2::new(torre_x, piano.top()), Pos2::new(torre_x, piano.bottom())],
        Stroke::new(1.0, LINEA),
    );
    let suolo_ora = c.altezza_interpolata(&suolo, k);
    p.line_segment(
        [Pos2::new(torre_x - 22.0, y_di(suolo_ora)), Pos2::new(torre_x + 22.0, y_di(suolo_ora))],
        Stroke::new(3.0, GRIGIO),
    );
    p.circle_filled(Pos2::new(torre_x, y_di(quota)), 11.0, PALLA);
    p.line_segment([Pos2::new(torre_x + 11.0, y_di(quota)), centro], Stroke::new(0.8, PALLA.gamma_multiply(0.35)));

    // Scala.
    let f = FontId::monospace(10.0);
    p.text(piano.left_top() + Vec2::new(-6.0, 0.0), Align2::RIGHT_TOP, format!("{alto:.0} m"), f.clone(), GRIGIO);
    p.text(piano.left_bottom() + Vec2::new(-6.0, 0.0), Align2::RIGHT_BOTTOM, format!("{basso:.0} m"), f.clone(), GRIGIO);
    p.text(piano.right_bottom() + Vec2::new(0.0, 6.0), Align2::RIGHT_TOP, format!("{:.1} s", c.durata).replace('.', ","), f, GRIGIO);

    p.text(
        area.left_top() + Vec2::new(14.0, 12.0),
        Align2::LEFT_TOP,
        "Trascina la curva arancione: una palla deve partire e arrivare in questi due istanti. Cerca di far segnare di più al suo orologio.",
        etichetta.clone(),
        GRIGIO,
    );
    p.text(
        area.left_top() + Vec2::new(14.0, 32.0),
        Align2::LEFT_TOP,
        "Nessuna forza la tira giù: segue il percorso in cui il suo tempo è il più lungo, e quel percorso è una caduta.",
        FontId::proportional(13.0),
        CIANO,
    );
}
