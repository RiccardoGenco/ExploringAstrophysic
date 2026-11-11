//! La guida di ogni banco: cosa si vede, piccole sfide che si spuntano da sole,
//! perche' conta. Pensata per chi non e' del mestiere.

use crate::cadere;
use crate::fondo::Pagina as PaginaFondo;
use crate::galassie::Scena;
use crate::osservatorio::Sezione;
use crate::pannello::{CHIARO, CIANO, GRIGIO, SUPERFICIE, VERDE};
use crate::stato::{Stanza, Stato, AREE};
use crate::stelle::Pagina as PaginaStelle;
use crate::trecorpi::Pagina as PaginaTre;
use colossi_fisica::geodetica::{Esito, Legge};
use egui::{Frame, Margin, RichText, Sense, Stroke, Vec2};

pub struct Sfida {
    pub testo: &'static str,
    pub fatta: fn(&Stato) -> bool,
}

pub struct Guida {
    pub cosa: &'static str,
    pub perche: &'static str,
    pub sfide: &'static [Sfida],
}

const fn s(testo: &'static str, fatta: fn(&Stato) -> bool) -> Sfida {
    Sfida { testo, fatta }
}

const SCALA: Guida = Guida {
    cosa: "Cerchi alla grandezza vera, uno dentro l'altro, da una persona fino all'universo osservabile.",
    perche: "I numeri astronomici non dicono niente finché non li metti accanto a qualcosa che conosci.",
    sfide: &[
        s("Allontanati finché la Terra entra tutta nel riquadro", |s| s.scala.log_vista >= 6.9),
        s("Arriva a vedere il Sole intero", |s| s.scala.log_vista >= 9.0),
        s("Raggiungi la nostra galassia", |s| s.scala.log_vista >= 20.4),
    ],
};

const BUCO_NERO: Guida = Guida {
    cosa: "A sinistra il piano visto dall'alto, dove lanci luce e sonde. A destra come apparirebbe la scena a un telescopio.",
    perche: "Qui la gravità è così forte che piega la luce e rallenta il tempo: tutto ciò che vedi è calcolato.",
    sfide: &[
        s("Lancia qualcosa trascinando sul banco", |s| s.lab.lanci >= 1),
        s("Passa alla gravità di Newton e guarda l'orbita chiudersi", |s| s.lab.legge == Legge::Newton),
        s("Fai cadere qualcosa fino all'orizzonte", |s| {
            s.lab.oggetti.iter().any(|o| matches!(o.moto.esito, Esito::AllOrizzonte | Esito::OltreOrizzonte | Esito::Singolarita))
        }),
        s("Ingrandisci il telescopio", |s| s.telescopio_grande),
    ],
};

const NEUTRONI: Guida = Guida {
    cosa: "Ogni punto della curva è una stella fatta di neutroni. A destra la vedi in sezione, accanto al Raccordo Anulare.",
    perche: "Sono gli oggetti più densi che si possano toccare: un passo in più e diventano buchi neri.",
    sfide: &[
        s("Tieni premuto Aggiungi materia finché la stella crolla", |s| s.neutroni.crollata()),
        s("Scegli un'altra materia nell'elenco", |s| s.neutroni.materia_cambiata()),
        s("Guarda la stessa materia senza Einstein", |s| s.neutroni.senza_einstein()),
    ],
};

const GIGANTI: Guida = Guida {
    cosa: "La crescita di un buco nero nel tempo dell'universo. I punti sono buchi neri veri, già enormi quando l'universo era giovane.",
    perche: "Nessuno sa ancora bene come abbiano fatto a diventare così grandi così in fretta.",
    sfide: &[
        s("Trascina il punto arancione finché la curva raggiunge il quasar più lontano", |s| s.giganti.arriva_al_piu_lontano()),
        s("Spingi la crescita oltre il limite di Eddington", |s| s.giganti.oltre_eddington()),
    ],
};

const STELLE: Guida = Guida {
    cosa: "La vita di una stella in pochi secondi, sopra migliaia di stelle vere misurate dal satellite Gaia.",
    perche: "Quasi tutti gli atomi di cui sei fatto sono stati prodotti dentro stelle come queste.",
    sfide: &[
        s("Porta il Sole fino alla fine della sua vita", |s| (s.stelle.massa - 1.0).abs() < 0.05 && s.stelle.k >= 1.0),
        s("Fai esplodere una stella grande", |s| s.stelle.massa >= 8.0 && s.stelle.k >= 1.0),
        s("Apri la scala delle stelle", |s| s.stelle.pagina == PaginaStelle::Scala),
    ],
};

const GALASSIE: Guida = Guida {
    cosa: "Migliaia di stelle che girano attorno al centro di una galassia, e quello che le tiene insieme.",
    perche: "Le stelle che vediamo non pesano abbastanza: serve qualcosa di invisibile, la materia oscura.",
    sfide: &[
        s("Spegni la materia oscura", |s| s.galassie.sim.fattore_alone == 0.0),
        s("Fai sfiorare due galassie", |s| s.galassie.scena == Scena::Incontro),
        s("Guarda noi e Andromeda", |s| s.galassie.scena == Scena::Andromeda),
    ],
};

const ONDE: Guida = Guida {
    cosa: "Due buchi neri che si avvicinano e si fondono, il segnale arrivato sulla Terra e lo spazio che vibra.",
    perche: "Nel 2015 per la prima volta abbiamo sentito lo spazio stesso vibrare.",
    sfide: &[
        s("Arriva al momento della fusione", |s| s.onde.t >= 0.0),
        s("Ascolta il segnale", |s| s.onde.ascoltato),
        s("Prova due stelle di neutroni", |s| s.onde.coppia.neutroni),
    ],
};

const FONDO: Guida = Guida {
    cosa: "La luce più antica che esista: com'era l'universo giovane, e come appare oggi il cielo in microonde.",
    perche: "È una fotografia dell'universo bambino, e contiene già i grumi da cui nasceranno le galassie.",
    sfide: &[
        s("Scorri finché la nebbia si dirada", |s| s.fondo.pagina == PaginaFondo::Nebbia && s.fondo.z < 1000.0),
        s("Apri il cielo e togli il nostro moto", |s| s.fondo.pagina == PaginaFondo::Cielo && !s.fondo.con_dipolo),
        s("Cambia la curvatura dello spazio e guarda i picchi", |s| s.fondo.cosmo.curvatura.abs() > 0.02),
    ],
};

const GEMELLI: Guida = Guida {
    cosa: "Il tempo sale in verticale, lo spazio va in orizzontale. La linea azzurra resta a casa, quella rosa viaggia.",
    perche: "Chi viaggia veloce invecchia meno: è stato misurato, con orologi su aerei e satelliti.",
    sfide: &[
        s("Trascina la svolta rosa per cambiare il viaggio", |s| {
            s.gemelli.linea.get(1).is_some_and(|e| (e.x - 4.0).abs() > 0.05 || (e.t - 5.0).abs() > 0.05)
        }),
        s("Guarda la scena da un altro osservatore", |s| s.gemelli.beta_vista.abs() > 0.2),
        s("Accendi i lampi di luce annuali", |s| s.gemelli.segnali),
    ],
};

const CADERE: Guida = Guida {
    cosa: "Il percorso di una palla tra due istanti. Più in alto il tempo scorre un poco più in fretta.",
    perche: "La gravità, qui sulla Terra, è soprattutto tempo che scorre a ritmi diversi.",
    sfide: &[
        s("Trascina la curva fino al 90% del tempo massimo", |s| cadere::frazione_del_massimo(&s.cadere) >= 0.9),
        s("Lascia fare alla natura", |s| s.cadere.natura),
        s("Guarda la scena da chi cade", |s| s.cadere.cornice_caduta),
    ],
};

const CURVA: Guida = Guida {
    cosa: "Una palla di sassolini lasciata cadere: vedi come cambia forma mentre cade.",
    perche: "È il modo più semplice di vedere cosa fa la materia allo spazio-tempo.",
    sfide: &[
        s("Lascia cadere una palla dentro la materia", |s| s.curva.partenza[0].hypot(s.curva.partenza[1]) < s.curva.sfera.raggio * 0.95),
        s("Lasciala cadere esattamente al centro", |s| s.curva.partenza[0].hypot(s.curva.partenza[1]) < 0.05),
        s("Cambia la massa della sfera", |s| (s.curva.sfera.massa - 1.0).abs() > 0.05),
    ],
};

const ORIZZONTE: Guida = Guida {
    cosa: "A sinistra i futuri possibili, disegnati come coni. A destra un buco nero che ruota e trascina lo spazio.",
    perche: "Oltre l'orizzonte il centro non è un luogo: è il futuro, inevitabile come domani.",
    sfide: &[
        s("Trascina il cono oltre l'orizzonte", |s| s.orizzonte.r_cono < 1.0),
        s("Fai ruotare il buco nero quasi al massimo", |s| s.orizzonte.spin > 0.95),
    ],
};

const OSSERVATORIO: Guida = Guida {
    cosa: "Misure vere pubblicate dagli astronomi. Muovi i cursori finché la curva passa per le croci.",
    perche: "È così che si scopre davvero: si confronta un'idea con i dati, e i dati hanno sempre l'ultima parola.",
    sfide: &[
        s("Con S2, porta lo scarto sotto 3 muovendo i cursori a mano", |s| s.osservatorio.a_mano_vicino(3.0)),
        s("Premi Calcola e confronta con la pubblicazione", |s| s.osservatorio.confronto_aperto()),
        s("Apri un'altra missione", |s| s.osservatorio.sezione != Sezione::Orbita),
    ],
};

const TRE_CORPI: Guida = Guida {
    cosa: "Tre corpi che si attraggono. Trascina i cerchietti per cambiare la partenza, o scegli una scena.",
    perche: "Con tre corpi non esiste una formula: si può solo calcolare passo per passo, e il futuro diventa imprevedibile.",
    sfide: &[
        s("Accendi un gemello che parte quasi uguale", |s| s.trecorpi.caos),
        s("Apri la mappa del caos", |s| s.trecorpi.pagina == PaginaTre::Mappa),
        s("Accendi la vista che ruota", |s| s.trecorpi.rotante),
    ],
};

pub fn guida(stanza: Stanza) -> Option<&'static Guida> {
    Some(match stanza {
        Stanza::Scala => &SCALA,
        Stanza::BucoNero => &BUCO_NERO,
        Stanza::Neutroni => &NEUTRONI,
        Stanza::Giganti => &GIGANTI,
        Stanza::Stelle => &STELLE,
        Stanza::Galassie => &GALASSIE,
        Stanza::Onde => &ONDE,
        Stanza::Fondo => &FONDO,
        Stanza::Gemelli => &GEMELLI,
        Stanza::Cadere => &CADERE,
        Stanza::Curva => &CURVA,
        Stanza::Orizzonte => &ORIZZONTE,
        Stanza::Osservatorio => &OSSERVATORIO,
        Stanza::TreCorpi => &TRE_CORPI,
        Stanza::Benvenuto => return None,
    })
}

/// Vero se la sfida e' riuscita adesso, a prescindere da quelle spuntate in passato.
/// Serve alle visite guidate, che vogliono vedere il gesto fatto ora.
pub fn sfida_ora(s: &Stato, stanza: Stanza, i: usize) -> bool {
    guida(stanza).and_then(|g| g.sfide.get(i)).is_some_and(|sfida| (sfida.fatta)(s))
}

/// Segna le sfide appena riuscite. Una volta fatte restano fatte.
pub fn controlla(s: &mut Stato) {
    let Some(g) = guida(s.stanza) else { return };
    for (i, sfida) in g.sfide.iter().enumerate() {
        if !s.sfide_fatte.contains(&(s.stanza, i)) && (sfida.fatta)(s) {
            s.sfide_fatte.insert((s.stanza, i));
        }
    }
}

pub fn fatte(s: &Stato, stanza: Stanza) -> usize {
    guida(stanza).map_or(0, |g| (0..g.sfide.len()).filter(|i| s.sfide_fatte.contains(&(stanza, *i))).count())
}

/// Il banco che segue nell'ordine delle aree.
pub fn prossimo(stanza: Stanza) -> Option<Stanza> {
    let tutti: Vec<Stanza> = AREE.iter().flat_map(|a| a.banchi.iter().map(|b| b.0)).collect();
    let i = tutti.iter().position(|b| *b == stanza)?;
    tutti.get(i + 1).copied()
}

fn nome(stanza: Stanza) -> &'static str {
    AREE.iter().flat_map(|a| a.banchi.iter()).find(|b| b.0 == stanza).map_or("", |b| b.1)
}

fn spunta(ui: &mut egui::Ui, fatta: bool, testo: &str) {
    ui.horizontal(|ui| {
        let (area, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
        let p = ui.painter();
        let c = area.center();
        if fatta {
            p.circle_filled(c, 7.0, VERDE);
            p.line_segment([c + Vec2::new(-3.5, 0.0), c + Vec2::new(-1.0, 2.8)], Stroke::new(1.8, SUPERFICIE));
            p.line_segment([c + Vec2::new(-1.0, 2.8), c + Vec2::new(3.8, -2.6)], Stroke::new(1.8, SUPERFICIE));
        } else {
            p.circle_stroke(c, 6.5, Stroke::new(1.3, GRIGIO));
        }
        let colore = if fatta { GRIGIO } else { CHIARO };
        ui.add(egui::Label::new(RichText::new(testo).color(colore)).wrap());
    });
}

const ALTEZZA_CHIUSA: f32 = 34.0;
const ALTEZZA_APERTA: f32 = 132.0;

/// La fascia della guida, agganciata al fondo: non copre mai il banco.
pub fn pannello(ui: &mut egui::Ui, s: &mut Stato) {
    if s.visita.is_some() || !s.guida_visibile {
        return;
    }
    let Some(g) = guida(s.stanza) else { return };
    let (n, totale) = (fatte(s, s.stanza), g.sfide.len());
    let altezza = if s.guida_aperta { ALTEZZA_APERTA } else { ALTEZZA_CHIUSA };
    let mut vai = None;
    egui::Panel::bottom("guida")
        .exact_size(altezza)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE.gamma_multiply(1.25)).inner_margin(Margin::symmetric(14, 6)))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("GUIDA · {}", nome(s.stanza))).small().strong().color(CIANO));
                ui.label(RichText::new(format!("{n} di {totale}")).small().color(if n == totale { VERDE } else { GRIGIO }));
                if !s.guida_aperta {
                    let prossima = g.sfide.iter().enumerate().find(|(i, _)| !s.sfide_fatte.contains(&(s.stanza, *i)));
                    match prossima {
                        Some((_, sfida)) => ui.label(RichText::new(format!("Prova: {}", sfida.testo)).color(CHIARO)),
                        None => ui.label(RichText::new("Tutto provato.").color(VERDE)),
                    };
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Nascondi").on_hover_text("La riapri dal pulsante Guida in alto").clicked() {
                        s.guida_visibile = false;
                    }
                    let testo = if s.guida_aperta { "Riduci" } else { "Apri" };
                    if ui.small_button(testo).clicked() {
                        s.guida_aperta = !s.guida_aperta;
                    }
                    if n == totale {
                        if let Some(dopo) = prossimo(s.stanza) {
                            if ui.small_button(RichText::new(format!("Prossimo: {}", nome(dopo))).color(VERDE)).clicked() {
                                vai = Some(dopo);
                            }
                        }
                    }
                });
            });
            if !s.guida_aperta {
                return;
            }
            ui.add_space(2.0);
            let larghezza = ui.available_width();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(larghezza * 0.30);
                    ui.label(RichText::new("COSA VEDI").small().strong().color(GRIGIO));
                    ui.add(egui::Label::new(RichText::new(g.cosa).color(CHIARO)).wrap());
                });
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.set_width(larghezza * 0.40);
                    ui.label(RichText::new("PROVA COSÌ").small().strong().color(GRIGIO));
                    for (i, sfida) in g.sfide.iter().enumerate() {
                        spunta(ui, s.sfide_fatte.contains(&(s.stanza, i)), sfida.testo);
                    }
                });
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.set_width((larghezza * 0.30 - 40.0).max(120.0));
                    ui.label(RichText::new("PERCHÉ CONTA").small().strong().color(GRIGIO));
                    ui.add(egui::Label::new(RichText::new(g.perche).italics().color(CHIARO)).wrap());
                });
            });
        });
    if let Some(dopo) = vai {
        s.stanza = dopo;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ogni_banco_tranne_il_benvenuto_ha_la_sua_guida() {
        for area in &AREE {
            for (stanza, nome) in area.banchi {
                let g = guida(*stanza).unwrap_or_else(|| panic!("{nome}"));
                assert!(!g.sfide.is_empty() && g.sfide.len() <= 4, "{nome}");
            }
        }
    }

    #[test]
    fn all_inizio_nessuna_sfida_e_gia_fatta() {
        let mut s = Stato::default();
        for area in &AREE {
            for (stanza, nome) in area.banchi {
                s.stanza = *stanza;
                controlla(&mut s);
                assert_eq!(fatte(&s, *stanza), 0, "{nome} parte con sfide gia' fatte");
            }
        }
    }

    #[test]
    fn una_sfida_fatta_resta_fatta() {
        let mut s = Stato::default();
        s.stanza = Stanza::BucoNero;
        s.lab.legge = Legge::Newton;
        controlla(&mut s);
        s.lab.legge = Legge::Einstein;
        controlla(&mut s);
        assert_eq!(fatte(&s, Stanza::BucoNero), 1);
    }

    #[test]
    fn dopo_l_ultimo_banco_non_c_e_altro() {
        let ultimo = AREE[AREE.len() - 1].banchi.last().unwrap().0;
        assert!(prossimo(ultimo).is_none());
        assert!(prossimo(Stanza::Scala).is_some());
    }
}
