//! Il banco: mappa del piano dove si lancia, si misura e si guarda cosa succede.

use crate::formato::lunghezza;
use crate::lab::{Anteprima, Lab, Oggetto, Strumento};
use crate::pannello::{CHIARO, CIANO, FONDO, GIALLO, GRIGIO, LINEA, VERDE};
use crate::stato::{Stato, R_ESTERNO_DISCO};
use colossi_fisica::buco_nero::{ISCO, SFERA_FOTONI};
use colossi_fisica::geodetica::{beta_circolare, Lancio, Legge, Specie};
use egui::{Align2, Color32, FontId, Painter, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

const R_MINIMO: f64 = 1.01;
/// Lunghezza in pixel del trascinamento che corrisponde alla velocita' della luce.
const PIXEL_PER_C: f32 = 300.0;
const LUNGHEZZA_MANIGLIA_LUCE: f32 = 70.0;
const RAGGIO_PRESA: f32 = 12.0;
const MAX_PUNTI_SCIA: usize = 1500;

/// Cosa sta facendo il tasto sinistro, da quando e' stato premuto.
#[derive(Clone, Copy)]
enum Gesto {
    Lancio([f64; 2]),
    Punta(usize),
    Base(usize),
    Sposta,
}

#[derive(Clone, Copy)]
struct Vista {
    schermo: Pos2,
    centro: [f64; 2],
    px: f64,
}

impl Vista {
    fn a_schermo(&self, p: [f64; 2]) -> Pos2 {
        self.schermo
            + Vec2::new(((p[0] - self.centro[0]) * self.px) as f32, ((p[1] - self.centro[1]) * self.px) as f32)
    }

    fn a_mondo(&self, p: Pos2) -> [f64; 2] {
        let d = p - self.schermo;
        [self.centro[0] + f64::from(d.x) / self.px, self.centro[1] + f64::from(d.y) / self.px]
    }
}

fn colore(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn beta_da_pixel(l: f32) -> f64 {
    f64::from((l / PIXEL_PER_C).powi(2)).min(0.99)
}

fn pixel_da_beta(beta: f64) -> f32 {
    (beta.sqrt() as f32 * PIXEL_PER_C).max(18.0)
}

fn fuori_orizzonte(p: [f64; 2]) -> [f64; 2] {
    let r = p[0].hypot(p[1]);
    if r < R_MINIMO {
        let k = R_MINIMO / r.max(1e-6);
        [p[0] * k, p[1] * k]
    } else {
        p
    }
}

/// Traduce un trascinamento in un lancio. Con `aggancia` la sonda parte in orbita circolare.
fn lancio_da(inizio: [f64; 2], fine: Pos2, vista: &Vista, specie: Specie, aggancia: bool, legge: Legge) -> Lancio {
    let partenza = vista.a_schermo(inizio);
    let d = fine - partenza;
    let mut direzione = f64::from(d.y).atan2(f64::from(d.x));
    let mut beta = beta_da_pixel(d.length());
    if aggancia && specie == Specie::Materia {
        let r = inizio[0].hypot(inizio[1]);
        let tangente = inizio[1].atan2(inizio[0]) + std::f64::consts::FRAC_PI_2;
        let verso = (direzione - tangente).cos().signum();
        direzione = if verso > 0.0 { tangente } else { tangente + std::f64::consts::PI };
        beta = beta_circolare(r, legge).min(0.99);
    }
    Lancio { posizione: inizio, direzione, beta, specie }
}

pub fn mostra(ui: &mut egui::Ui, area: Rect, s: &mut Stato) {
    let id = ui.id().with("banco");
    let risposta = ui.interact(area, id, Sense::click_and_drag());
    let px = f64::from(area.height()) * 0.5 / s.lab.vista.raggio;
    let vista = Vista { schermo: area.center(), centro: s.lab.vista.centro, px };

    zoom_e_spostamento(ui, area, &risposta, s, vista);
    let vista = Vista {
        schermo: area.center(),
        centro: s.lab.vista.centro,
        px: f64::from(area.height()) * 0.5 / s.lab.vista.raggio,
    };

    let p = ui.painter_at(area);
    p.rect_filled(area, 0.0, FONDO);
    griglia(&p, area, vista);
    mappa_fisica(&p, area, vista, s);

    let anteprima = gesti(ui, id, &risposta, s, vista, &p);

    for o in &s.lab.oggetti {
        disegna_scia(&p, o, vista, s.lab.selezionato == Some(o.id));
    }
    for o in &s.lab.oggetti {
        disegna_testa(&p, o, vista, s.lab.selezionato == Some(o.id));
    }
    if let Some(indice) = s.lab.indice_selezionato() {
        disegna_maniglie(&p, &s.lab.oggetti[indice], vista);
    }
    if let Some(a) = anteprima {
        disegna_anteprima(&p, &a, vista);
    }
    didascalie(&p, area, vista, s);
}

fn zoom_e_spostamento(ui: &egui::Ui, area: Rect, r: &Response, s: &mut Stato, vista: Vista) {
    if r.hovered() {
        let rotella = ui.input(|i| i.smooth_scroll_delta.y);
        if rotella != 0.0 {
            let cursore = ui.input(|i| i.pointer.hover_pos()).unwrap_or(area.center());
            let sotto = vista.a_mondo(cursore);
            let lab = &mut s.lab;
            lab.vista.raggio = (lab.vista.raggio * f64::from(-rotella * 0.0025).exp()).clamp(1.5, 200_000.0);
            let px = f64::from(area.height()) * 0.5 / lab.vista.raggio;
            let d = cursore - area.center();
            lab.vista.centro = [sotto[0] - f64::from(d.x) / px, sotto[1] - f64::from(d.y) / px];
        }
    }
    if r.dragged_by(PointerButton::Secondary) || r.dragged_by(PointerButton::Middle) {
        let d = r.drag_delta();
        s.lab.vista.centro[0] -= f64::from(d.x) / vista.px;
        s.lab.vista.centro[1] -= f64::from(d.y) / vista.px;
    }
}

fn punta_maniglia(o: &Oggetto, vista: &Vista) -> Pos2 {
    let base = vista.a_schermo(o.lancio.posizione);
    let l = match o.lancio.specie {
        Specie::Luce => LUNGHEZZA_MANIGLIA_LUCE,
        Specie::Materia => pixel_da_beta(o.lancio.beta),
    };
    base + Vec2::new(o.lancio.direzione.cos() as f32, o.lancio.direzione.sin() as f32) * l
}

/// L'oggetto la cui scia passa piu' vicino al punto, entro la presa.
fn oggetto_vicino(lab: &Lab, punto: Pos2, vista: &Vista) -> Option<u32> {
    let mut migliore = (RAGGIO_PRESA * RAGGIO_PRESA, None);
    for o in &lab.oggetti {
        let testa = vista.a_schermo([o.moto.x[0], o.moto.x[2]]);
        let mut prova = |q: Pos2| {
            let d = (q - punto).length_sq();
            if d < migliore.0 {
                migliore = (d, Some(o.id));
            }
        };
        prova(testa);
        let passo = (o.scia.len() / MAX_PUNTI_SCIA).max(1);
        for c in o.scia.iter().step_by(passo) {
            prova(vista.a_schermo([f64::from(c.p[0]), f64::from(c.p[1])]));
        }
    }
    migliore.1
}

fn gesti(
    ui: &egui::Ui,
    id: egui::Id,
    r: &Response,
    s: &mut Stato,
    vista: Vista,
    p: &Painter,
) -> Option<Vec<Anteprima>> {
    let id_gesto = id.with("gesto");
    let lab = &mut s.lab;
    let aggancia = ui.input(|i| i.modifiers.shift);
    let specie_attrezzo = match lab.strumento {
        Strumento::Luce | Strumento::Fascio => Specie::Luce,
        _ => Specie::Materia,
    };

    if r.drag_started_by(PointerButton::Primary) {
        let premuto = ui.input(|i| i.pointer.press_origin()).unwrap_or(r.rect.center());
        let gesto = match lab.strumento {
            Strumento::Seleziona => lab
                .indice_selezionato()
                .and_then(|i| {
                    let o = &lab.oggetti[i];
                    if (punta_maniglia(o, &vista) - premuto).length() < RAGGIO_PRESA && !o.fisso {
                        Some(Gesto::Punta(i))
                    } else if (vista.a_schermo(o.lancio.posizione) - premuto).length() < RAGGIO_PRESA {
                        Some(Gesto::Base(i))
                    } else {
                        None
                    }
                })
                .unwrap_or(Gesto::Sposta),
            Strumento::Luce | Strumento::Sonda | Strumento::Fascio => {
                Gesto::Lancio(fuori_orizzonte(vista.a_mondo(premuto)))
            }
            Strumento::Orologio => Gesto::Sposta,
        };
        ui.data_mut(|d| d.insert_temp(id_gesto, gesto));
    }

    if r.clicked() {
        let punto = r.interact_pointer_pos().unwrap_or(r.rect.center());
        match lab.strumento {
            Strumento::Orologio => {
                let nuovo = lab.orologio(fuori_orizzonte(vista.a_mondo(punto)));
                lab.selezionato = Some(nuovo);
            }
            _ => lab.selezionato = oggetto_vicino(lab, punto, &vista),
        }
    }

    let gesto: Option<Gesto> = ui.data(|d| d.get_temp(id_gesto));
    let Some(gesto) = gesto else { return None };
    let cursore = ui.input(|i| i.pointer.latest_pos()).unwrap_or(r.rect.center());
    let mut anteprime = None;

    if r.dragged_by(PointerButton::Primary) {
        match gesto {
            Gesto::Sposta => {
                let d = r.drag_delta();
                lab.vista.centro[0] -= f64::from(d.x) / vista.px;
                lab.vista.centro[1] -= f64::from(d.y) / vista.px;
            }
            Gesto::Punta(i) => {
                if i < lab.oggetti.len() {
                    let mut lancio = lab.oggetti[i].lancio;
                    let d = cursore - vista.a_schermo(lancio.posizione);
                    lancio.direzione = f64::from(d.y).atan2(f64::from(d.x));
                    if lancio.specie == Specie::Materia {
                        lancio.beta = beta_da_pixel(d.length());
                    }
                    lab.modifica(i, lancio);
                }
            }
            Gesto::Base(i) => {
                if i < lab.oggetti.len() {
                    let mut lancio = lab.oggetti[i].lancio;
                    lancio.posizione = fuori_orizzonte(vista.a_mondo(cursore));
                    lab.modifica(i, lancio);
                }
            }
            Gesto::Lancio(inizio) => {
                let lancio = lancio_da(inizio, cursore, &vista, specie_attrezzo, aggancia, lab.legge);
                anteprime = Some(if lab.strumento == Strumento::Fascio {
                    Lab::posizioni_fascio(inizio, lancio.direzione)
                        .into_iter()
                        .map(|q| lab.anteprima(&Lancio { posizione: q, ..lancio }, Some(1500)))
                        .collect()
                } else {
                    vec![lab.anteprima(&lancio, None)]
                });
                etichetta_lancio(p, cursore, &lancio, anteprime.as_ref().and_then(|a: &Vec<Anteprima>| a.first()));
            }
        }
    }

    if r.drag_stopped_by(PointerButton::Primary) {
        if let Gesto::Lancio(inizio) = gesto {
            let lontano = (cursore - vista.a_schermo(inizio)).length() > 6.0;
            if lontano {
                let lancio = lancio_da(inizio, cursore, &vista, specie_attrezzo, aggancia, lab.legge);
                if lab.strumento == Strumento::Fascio {
                    lab.fascio(inizio, lancio.direzione);
                } else {
                    let nuovo = lab.aggiungi(lancio, false);
                    lab.selezionato = Some(nuovo);
                }
            }
        }
        ui.data_mut(|d| d.remove::<Gesto>(id_gesto));
    }
    anteprime
}

fn etichetta_lancio(p: &Painter, cursore: Pos2, lancio: &Lancio, a: Option<&Anteprima>) {
    let esito = a.map_or("", |a| a.esito);
    let testo = match lancio.specie {
        Specie::Luce => esito.to_owned(),
        Specie::Materia => format!("{:.3} c  ·  {esito}", lancio.beta).replace('.', ","),
    };
    p.text(cursore + Vec2::new(14.0, -14.0), Align2::LEFT_BOTTOM, testo, FontId::proportional(14.0), CHIARO);
}

/// Distanze 1, 2, 5, 10, 20, ... che cadono nel campo visivo.
pub fn passo_tondo(minimo: f64) -> f64 {
    let base = 10f64.powf(minimo.log10().floor());
    for k in [1.0, 2.0, 5.0, 10.0] {
        if base * k >= minimo {
            return base * k;
        }
    }
    base * 10.0
}

fn griglia(p: &Painter, area: Rect, vista: Vista) {
    let centro = vista.a_schermo([0.0, 0.0]);
    let tratto = Stroke::new(1.0, LINEA);
    p.line_segment([Pos2::new(area.left(), centro.y), Pos2::new(area.right(), centro.y)], tratto);
    p.line_segment([Pos2::new(centro.x, area.top()), Pos2::new(centro.x, area.bottom())], tratto);

    let passo = passo_tondo(70.0 / vista.px);
    let massimo = f64::from(area.width().hypot(area.height())) / vista.px
        + vista.centro[0].hypot(vista.centro[1]);
    let etichette = FontId::monospace(10.0);
    let mut r = passo;
    let mut n = 0;
    while r < massimo && n < 200 {
        p.circle_stroke(centro, (r * vista.px) as f32, tratto);
        let testo = if r.fract() == 0.0 { format!("{r:.0}") } else { format!("{r}").replace('.', ",") };
        p.text(
            centro + Vec2::new((r * vista.px) as f32 + 3.0, 2.0),
            Align2::LEFT_TOP,
            testo,
            etichette.clone(),
            GRIGIO.gamma_multiply(0.7),
        );
        r += passo;
        n += 1;
    }
}

fn cerchio_tratteggiato(p: &Painter, centro: Pos2, raggio: f32, tratto: Stroke) {
    let n = ((raggio * 0.6) as usize).clamp(48, 720);
    let punti: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = i as f32 / n as f32 * TAU;
            centro + Vec2::new(a.cos(), a.sin()) * raggio
        })
        .collect();
    p.extend(Shape::dashed_line(&punti, tratto, 5.0, 5.0));
}

fn nome_cerchio(p: &Painter, centro: Pos2, raggio: f32, testo: &str, c: Color32) {
    if raggio > 34.0 {
        p.text(
            centro - Vec2::new(0.0, raggio + 2.0),
            Align2::CENTER_BOTTOM,
            testo,
            FontId::proportional(11.0),
            c,
        );
    }
}

/// Anello pieno tra due raggi.
fn corona(p: &Painter, centro: Pos2, interno: f32, esterno: f32, tinta: Color32) {
    const SPICCHI: u32 = 120;
    let mut mesh = egui::Mesh::default();
    for i in 0..=SPICCHI {
        let a = i as f32 / SPICCHI as f32 * TAU;
        let d = Vec2::new(a.cos(), a.sin());
        mesh.colored_vertex(centro + d * interno, tinta);
        mesh.colored_vertex(centro + d * esterno, tinta);
    }
    for i in 0..SPICCHI {
        let k = i * 2;
        mesh.add_triangle(k, k + 1, k + 2);
        mesh.add_triangle(k + 1, k + 3, k + 2);
    }
    p.add(mesh);
}

fn mappa_fisica(p: &Painter, area: Rect, vista: Vista, s: &Stato) {
    let centro = vista.a_schermo([0.0, 0.0]);
    let scala = vista.px as f32;

    if s.disco {
        corona(p, centro, 3.0 * scala, R_ESTERNO_DISCO * scala, Color32::from_rgba_unmultiplied(255, 170, 90, 16));
    }
    if s.confronti {
        for anello in s.anelli() {
            let [r, g, b] = anello.colore.map(|c| (c * 255.0) as u8);
            let c = Color32::from_rgb(r, g, b);
            let raggio = anello.raggi as f32 * scala;
            p.circle_stroke(centro, raggio, Stroke::new(1.0, c.gamma_multiply(0.7)));
            nome_cerchio(p, centro, raggio, anello.nome, c);
        }
    }

    match s.lab.legge {
        Legge::Einstein => {
            cerchio_tratteggiato(p, centro, ISCO as f32 * scala, Stroke::new(1.0, VERDE.gamma_multiply(0.6)));
            nome_cerchio(p, centro, ISCO as f32 * scala, "ultima orbita stabile", VERDE.gamma_multiply(0.8));
            cerchio_tratteggiato(p, centro, SFERA_FOTONI as f32 * scala, Stroke::new(1.0, GIALLO.gamma_multiply(0.6)));
            nome_cerchio(p, centro, SFERA_FOTONI as f32 * scala, "la luce può orbitare", GIALLO.gamma_multiply(0.8));
            p.circle_filled(centro, scala, Color32::BLACK);
            p.circle_stroke(centro, scala, Stroke::new(1.5, CIANO));
        }
        Legge::Newton => {
            p.circle_filled(centro, scala, Color32::from_gray(26));
            p.circle_stroke(centro, scala, Stroke::new(1.5, GRIGIO));
        }
    }

    // Da dove guarda il telescopio.
    let camera = vista.a_schermo(s.camera_sul_banco());
    if area.contains(camera) {
        let verso = (centro - camera).normalized();
        let laterale = verso.rot90();
        let a = camera + verso * 13.0 + laterale * 6.0;
        let b = camera + verso * 13.0 - laterale * 6.0;
        p.add(Shape::convex_polygon(vec![camera, a, b], CHIARO.gamma_multiply(0.75), Stroke::NONE));
    }
}

fn scia_a_punti<'a>(campioni: impl Iterator<Item = &'a [f32; 2]>, n: usize, vista: Vista) -> Vec<Pos2> {
    let passo = (n / MAX_PUNTI_SCIA).max(1);
    campioni
        .step_by(passo)
        .map(|q| vista.a_schermo([f64::from(q[0]), f64::from(q[1])]))
        .collect()
}

fn disegna_scia(p: &Painter, o: &Oggetto, vista: Vista, scelto: bool) {
    if !o.scia_gemello.is_empty() {
        let punti = scia_a_punti(o.scia_gemello.iter(), o.scia_gemello.len(), vista);
        p.extend(Shape::dashed_line(&punti, Stroke::new(1.2, GRIGIO), 6.0, 4.0));
    }
    if o.scia.len() < 2 {
        return;
    }
    let passo = (o.scia.len() / MAX_PUNTI_SCIA).max(1);
    let mut punti: Vec<Pos2> = o
        .scia
        .iter()
        .step_by(passo)
        .map(|c| vista.a_schermo([f64::from(c.p[0]), f64::from(c.p[1])]))
        .collect();
    punti.push(vista.a_schermo([o.moto.x[0], o.moto.x[2]]));
    let (spessore, alfa) = if scelto { (2.2, 1.0) } else { (1.3, 0.72) };
    p.add(Shape::line(punti, Stroke::new(spessore, colore(o.colore).gamma_multiply(alfa))));
}

fn disegna_testa(p: &Painter, o: &Oggetto, vista: Vista, scelto: bool) {
    let c = vista.a_schermo([o.moto.x[0], o.moto.x[2]]);
    let tinta = colore(o.colore);
    if o.fisso {
        p.circle_filled(c, 9.0, Color32::from_black_alpha(190));
        p.circle_stroke(c, 9.0, Stroke::new(1.5, tinta));
        let a = (o.moto.s * 0.6) as f32;
        p.line_segment([c, c + Vec2::new(a.sin(), -a.cos()) * 7.0], Stroke::new(1.5, tinta));
    } else if o.moto.esito.fermo() {
        p.circle_stroke(c, 3.5, Stroke::new(1.0, tinta));
    } else {
        p.circle_filled(c, if o.lancio.specie == Specie::Luce { 2.5 } else { 4.0 }, tinta);
    }
    if scelto {
        p.circle_stroke(c, 13.0, Stroke::new(1.2, CHIARO));
        p.text(c + Vec2::new(16.0, -10.0), Align2::LEFT_BOTTOM, &o.nome, FontId::proportional(12.5), CHIARO);
    }
}

fn disegna_maniglie(p: &Painter, o: &Oggetto, vista: Vista) {
    let base = vista.a_schermo(o.lancio.posizione);
    let tinta = colore(o.colore);
    p.circle_stroke(base, 6.0, Stroke::new(1.3, tinta));
    if !o.fisso {
        let punta = punta_maniglia(o, &vista);
        p.line_segment([base, punta], Stroke::new(1.3, tinta.gamma_multiply(0.8)));
        p.circle_filled(punta, 5.5, tinta);
    }
}

fn disegna_anteprima(p: &Painter, anteprime: &[Anteprima], vista: Vista) {
    for a in anteprime {
        if !a.gemello.is_empty() {
            let punti = scia_a_punti(a.gemello.iter(), a.gemello.len(), vista);
            p.extend(Shape::dashed_line(&punti, Stroke::new(1.0, GRIGIO), 6.0, 4.0));
        }
        let punti = scia_a_punti(a.punti.iter(), a.punti.len(), vista);
        if let Some(inizio) = punti.first() {
            p.circle_stroke(*inizio, 5.0, Stroke::new(1.3, CHIARO));
        }
        p.add(Shape::line(punti, Stroke::new(1.4, CHIARO.gamma_multiply(0.85))));
    }
}

fn didascalie(p: &Painter, area: Rect, vista: Vista, s: &Stato) {
    let aiuto = match s.lab.strumento {
        Strumento::Seleziona => "Clic su una traccia per misurarla. Trascina le maniglie per cambiare il lancio.",
        Strumento::Luce => "Trascina: da dove parte e verso dove va.",
        Strumento::Sonda => "Trascina: più lungo, più veloce. Con Maiusc entra in orbita circolare.",
        Strumento::Fascio => "Trascina: tredici raggi paralleli.",
        Strumento::Orologio => "Clic: un orologio tenuto fermo in quel punto.",
    };
    p.text(area.left_top() + Vec2::new(14.0, 12.0), Align2::LEFT_TOP, aiuto, FontId::proportional(13.0), GRIGIO);
    if let Some(nota) = s.lab.nota {
        p.text(area.left_top() + Vec2::new(14.0, 32.0), Align2::LEFT_TOP, nota, FontId::proportional(13.0), CIANO);
    }
    if let Some(avviso) = &s.lab.avviso {
        p.text(area.left_top() + Vec2::new(14.0, 52.0), Align2::LEFT_TOP, avviso, FontId::proportional(12.0), VERDE);
    }

    // Scala reale.
    let raggi = passo_tondo(120.0 / vista.px);
    let larghezza = (raggi * vista.px) as f32;
    let origine = area.left_bottom() + Vec2::new(16.0, -18.0);
    p.line_segment([origine, origine + Vec2::new(larghezza, 0.0)], Stroke::new(2.0, CHIARO));
    for x in [0.0, larghezza] {
        p.line_segment(
            [origine + Vec2::new(x, -4.0), origine + Vec2::new(x, 4.0)],
            Stroke::new(1.5, CHIARO),
        );
    }
    let reale = lunghezza(raggi * s.buco_nero().raggio());
    let testo = format!("{raggi} raggi = {reale}").replace('.', ",");
    p.text(origine + Vec2::new(0.0, -7.0), Align2::LEFT_BOTTOM, testo, FontId::proportional(12.0), CHIARO);
}
