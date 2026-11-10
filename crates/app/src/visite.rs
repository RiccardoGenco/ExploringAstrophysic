//! Visite guidate: tappe in linguaggio semplice che preparano i banchi.

use crate::fondo;
use crate::galassie::Scena;
use crate::pannello::{CHIARO, CIANO, GRIGIO, SUPERFICIE};
use crate::stato::{Stanza, Stato};
use crate::trecorpi::Pagina;
use colossi_fisica::onde::GW150914;
use colossi_fisica::trecorpi::GALLERIA;
use egui::{Align2, Frame, Margin, RichText, Stroke, Vec2};

pub struct Tappa {
    pub titolo: &'static str,
    pub testo: &'static str,
    pub prepara: fn(&mut Stato),
}

pub struct Visita {
    pub nome: &'static str,
    pub durata: &'static str,
    pub tappe: &'static [Tappa],
}

fn scena(nome: &str) -> usize {
    GALLERIA.iter().position(|v| v.nome == nome).unwrap_or(0)
}

const fn t(titolo: &'static str, testo: &'static str, prepara: fn(&mut Stato)) -> Tappa {
    Tappa { titolo, testo, prepara }
}

const COLOSSI: &[Tappa] = &[
    t("Parti da te", "Premi Viaggia e allontanati da una persona fino all'universo. Ogni cerchio ha la grandezza vera. Fermati quando un numero ti sorprende.", |s| {
        s.stanza = Stanza::Scala;
        s.scala = Default::default();
        s.scala.viaggio = true;
    }),
    t("Un buco nero", "L'ombra al centro non è un disegno: ogni punto dell'immagine è un raggio di luce seguito all'indietro. Il disco sembra piegato sopra e sotto, ma è piatto.", |s| {
        s.stanza = Stanza::BucoNero;
        s.masse = 4.3e6;
        s.telescopio_grande = true;
    }),
    t("Orbite che ruotano", "Sul banco questa sonda gira attorno al buco nero. L'ellisse non si chiude mai: Newton direbbe il contrario. Prova a passare a Newton in alto.", |s| {
        s.stanza = Stanza::BucoNero;
        s.telescopio_grande = false;
        s.lab.carica(0);
    }),
    t("Un cucchiaino da due miliardi di tonnellate", "Questa stella è grande quanto Roma e pesa più del Sole. Tieni premuto Aggiungi materia: oltre un certo punto niente può impedirle di crollare.", |s| {
        s.stanza = Stanza::Neutroni;
    }),
    t("Giganti nati troppo presto", "Questi buchi neri erano già enormi quando l'universo era giovane. Trascina il seme: come hanno fatto a crescere in tempo? Non lo sappiamo ancora del tutto.", |s| {
        s.stanza = Stanza::Giganti;
    }),
    t("Materia che non si vede", "Spegni la materia oscura. Le stelle che si vedono da sole non bastano a tenere insieme una galassia.", |s| {
        s.stanza = Stanza::Galassie;
        s.galassie.sim.fattore_alone = 1.0;
        s.galassie.costruisci(Scena::Una);
    }),
    t("Lo spazio che vibra", "Due buchi neri si fondono e tre soli di massa diventano onde. A Terra un braccio di 4 km si è mosso di una frazione di protone. Premi Ascolta.", |s| {
        s.stanza = Stanza::Onde;
        s.onde.coppia = GW150914;
        s.onde.costruisci();
    }),
    t("La luce più antica", "Scorri il tempo. All'inizio l'universo è una nebbia luminosa, poi diventa trasparente: quella stessa luce arriva ancora, stirata, da ogni direzione.", |s| {
        s.stanza = Stanza::Fondo;
        s.fondo.pagina = fondo::Pagina::Nebbia;
    }),
];

const TEMPO: &[Tappa] = &[
    t("Due gemelli", "Quello che viaggia torna più giovane: 6 anni contro 10. Trascina il punto rosa e allunga il viaggio. Più corri, meno tempo vivi.", |s| {
        s.stanza = Stanza::Gemelli;
        s.gemelli.carica(0);
    }),
    t("Nessuno ha ragione da solo", "Sposta il cursore di chi guarda: le coordinate si inclinano e gli \"adesso\" cambiano. Ma quanti anni ha vissuto ognuno non cambia.", |s| {
        s.stanza = Stanza::Gemelli;
        s.gemelli.beta_vista = 0.5;
        s.gemelli.segnali = true;
    }),
    t("Perché cadiamo", "Disegna il percorso di una palla. Salire fa guadagnare tempo, correre ne fa perdere. La palla segue il percorso in cui il suo orologio segna di più.", |s| {
        s.stanza = Stanza::Cadere;
    }),
    t("Cade, senza nessuna forza", "Premi Lascia fare alla natura: la curva diventa da sola una parabola. La gravità, qui, è soprattutto tempo che scorre a ritmi diversi.", |s| {
        s.stanza = Stanza::Cadere;
        s.cadere.natura = true;
    }),
    t("Cosa curva lo spazio-tempo", "Lascia cadere una palla di particelle. Nel vuoto si deforma ma il volume resta. Dentro la materia si restringe: è questo che la materia fa allo spazio-tempo.", |s| {
        s.stanza = Stanza::Curva;
    }),
    t("Il tempo si ferma, per chi guarda", "Una sonda cade nel buco nero. Da lontano la vedi fermarsi sull'orizzonte. Cambia l'orologio in basso e ripeti: lei attraversa e arriva al centro.", |s| {
        s.stanza = Stanza::BucoNero;
        s.telescopio_grande = false;
        s.lab.carica(7);
    }),
];

const SCIENZIATO: &[Tappa] = &[
    t("Pesa un buco nero", "Queste croci sono misure vere di una stella. Muovi i cursori finché la curva ci passa. Hai appena fatto ciò per cui è stato dato un Nobel.", |s| {
        s.stanza = Stanza::Osservatorio;
        s.osservatorio.sezione = crate::osservatorio::Sezione::Orbita;
    }),
    t("Il primo suono dello spazio", "La curva bianca è ciò che ha registrato davvero LIGO nel 2015. Premi Calcola: dalla sola forma dell'onda esce la massa dei due buchi neri.", |s| {
        s.stanza = Stanza::Osservatorio;
        s.osservatorio.sezione = crate::osservatorio::Sezione::Altra(0);
    }),
    t("Inventa una materia", "Muovi i cursori della materia. Se la curva manca i pulsar misurati, la tua idea è esclusa dai dati: ecco come funziona la fisica.", |s| {
        s.stanza = Stanza::Neutroni;
    }),
    t("Tre corpi, nessuna formula", "Due sistemi partono quasi identici. Guarda la differenza crescere: oltre un certo tempo, prevedere è impossibile.", |s| {
        s.stanza = Stanza::TreCorpi;
        s.trecorpi.pagina = Pagina::Banco;
        s.trecorpi.carica(scena("Problema pitagorico"));
        s.trecorpi.caos = true;
        s.trecorpi.ricomincia();
    }),
    t("La mappa del caos", "Ogni casella è una simulazione completa. Dove i colori si mescolano, spostare un corpo di un soffio cambia tutto.", |s| {
        s.stanza = Stanza::TreCorpi;
        s.trecorpi.caos = false;
        s.trecorpi.pagina = Pagina::Mappa;
    }),
    t("Di cosa è fatto l'universo", "Muovi materia e curvatura: i picchi si spostano. Confrontando questa curva con il cielo si misura la composizione dell'universo.", |s| {
        s.stanza = Stanza::Fondo;
        s.fondo.pagina = fondo::Pagina::Macchie;
    }),
];

pub const VISITE: [Visita; 3] = [
    Visita { nome: "Il giro dei colossi", durata: "8 tappe", tappe: COLOSSI },
    Visita { nome: "Che cos'è il tempo", durata: "6 tappe", tappe: TEMPO },
    Visita { nome: "Scienziato per un giorno", durata: "6 tappe", tappe: SCIENZIATO },
];

/// Punto in cui ci si trova in una visita.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct InCorso {
    pub visita: usize,
    pub tappa: usize,
}

pub fn inizia(s: &mut Stato, visita: usize) {
    s.visita = Some(InCorso { visita, tappa: 0 });
    (VISITE[visita].tappe[0].prepara)(s);
}

fn vai(s: &mut Stato, corso: InCorso, delta: i32) {
    let totale = VISITE[corso.visita].tappe.len() as i32;
    let nuova = corso.tappa as i32 + delta;
    if nuova >= totale {
        esci(s);
    } else {
        let tappa = nuova.max(0) as usize;
        s.visita = Some(InCorso { tappa, ..corso });
        (VISITE[corso.visita].tappe[tappa].prepara)(s);
    }
}

/// Si lascia la visita: il telescopio ingrandito era solo per quella tappa.
fn esci(s: &mut Stato) {
    s.visita = None;
    s.telescopio_grande = false;
}

/// La finestrella della tappa, sopra qualunque banco.
pub fn mostra(ctx: &egui::Context, s: &mut Stato) {
    let Some(corso) = s.visita else { return };
    let visita = &VISITE[corso.visita];
    let tappa = &visita.tappe[corso.tappa];
    let mut azione = None;
    egui::Area::new(egui::Id::new("visita"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(-180.0, -62.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(SUPERFICIE)
                .stroke(Stroke::new(1.5, CIANO))
                .corner_radius(8)
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    ui.set_width(540.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(tappa.titolo).size(17.0).strong().color(CIANO));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} di {}", corso.tappa + 1, visita.tappe.len())).color(GRIGIO));
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(tappa.testo).size(15.0).color(CHIARO));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add_enabled(corso.tappa > 0, egui::Button::new("Indietro")).clicked() {
                            azione = Some(-1);
                        }
                        let ultima = corso.tappa + 1 == visita.tappe.len();
                        if ui.button(RichText::new(if ultima { "Fine" } else { "Avanti" }).strong()).clicked() {
                            azione = Some(1);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("Esci dalla visita").clicked() {
                                azione = Some(i32::MAX);
                            }
                        });
                    });
                });
        });
    match azione {
        Some(i32::MAX) => esci(s),
        Some(d) => vai(s, corso, d),
        None => {}
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ogni_tappa_si_prepara_senza_errori() {
        for (i, v) in VISITE.iter().enumerate() {
            let mut s = Stato::default();
            inizia(&mut s, i);
            for k in 0..v.tappe.len() {
                let corso = s.visita.expect("in corso");
                assert_eq!(corso.tappa, k);
                s.avanza(0.02);
                vai(&mut s, corso, 1);
            }
            assert!(s.visita.is_none(), "{}", v.nome);
        }
    }

    #[test]
    fn la_durata_scritta_corrisponde() {
        for v in &VISITE {
            assert!(v.durata.starts_with(&v.tappe.len().to_string()), "{}", v.nome);
        }
    }
}
