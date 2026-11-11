//! Visite guidate. Ogni tappa dà un compito vero: indica il comando da usare, si
//! accorge da sola quando l'hai fatto e solo allora accende "Avanti" (con "Salta
//! questa prova" sempre a portata). La scheda sta in un pannello in fondo: restringe
//! il banco invece di coprirlo, quindi nessun comando resta bloccato.

use crate::fondo;
use crate::galassie::Scena;
use crate::guida;
use crate::osservatorio::Sezione;
use crate::pannello::{CHIARO, CIANO, GRIGIO, SUPERFICIE, VERDE};
use crate::stato::{Stanza, Stato};
use crate::trecorpi::Pagina;
use colossi_fisica::fondo::PLANCK;
use colossi_fisica::onde::GW150914;
use colossi_fisica::trecorpi::GALLERIA;
use egui::{Button, Color32, CornerRadius, FontId, Frame, Id, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Vec2};
use std::collections::HashMap;

/// Il comando da indicare: il nome con cui il banco lo segnala e la scritta accanto.
pub struct Indizio {
    pub id: &'static str,
    pub testo: &'static str,
}

pub struct Tappa {
    pub titolo: &'static str,
    /// Una o due frasi per capire cosa si sta guardando.
    pub storia: &'static str,
    /// Cosa fare, all'imperativo.
    pub compito: &'static str,
    /// Cosa si e' scoperto: compare quando il compito e' riuscito.
    pub esito: &'static str,
    pub indizio: Option<Indizio>,
    pub prepara: fn(&mut Stato),
    /// Vero quando il gesto richiesto e' stato fatto. Subito dopo `prepara` deve essere falso.
    pub fatto: fn(&Stato) -> bool,
    /// Quanto manca, da 0 a 1, per i compiti che richiedono tempo.
    pub avanzamento: Option<fn(&Stato) -> f32>,
}

pub struct Visita {
    pub nome: &'static str,
    pub durata: &'static str,
    pub descrizione: &'static str,
    pub tappe: &'static [Tappa],
}

fn scena(nome: &str) -> usize {
    GALLERIA.iter().position(|v| v.nome == nome).unwrap_or(0)
}

const fn indizio(id: &'static str, testo: &'static str) -> Option<Indizio> {
    Some(Indizio { id, testo })
}

fn scala_avanza(s: &Stato) -> f32 {
    ((s.scala.log_vista - 0.6) / (6.9 - 0.6)).clamp(0.0, 1.0) as f32
}

fn nebbia_avanza(s: &Stato) -> f32 {
    let (inizio, fine) = (fondo::Z_MASSIMO.ln_1p(), 1000f64.ln_1p());
    ((inizio - s.fondo.z.ln_1p()) / (inizio - fine)).clamp(0.0, 1.0) as f32
}

fn cadere_avanza(s: &Stato) -> f32 {
    (crate::cadere::frazione_del_massimo(&s.cadere) / 0.9).clamp(0.0, 1.0) as f32
}

const COLOSSI: &[Tappa] = &[
    Tappa {
        titolo: "Parti da te",
        storia: "Ogni cerchio ha la grandezza vera, da una persona fino all'universo.",
        compito: "Premi «Viaggia verso il grande» e aspetta che la Terra entri tutta nel riquadro.",
        esito: "Ecco la Terra intera. Sopra ci sono ancora il Sole, la galassia e tutto il resto.",
        indizio: indizio("viaggia", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::Scala;
            s.scala = Default::default();
        },
        fatto: |s| s.scala.log_vista >= 6.9,
        avanzamento: Some(scala_avanza),
    },
    Tappa {
        titolo: "Un buco nero",
        storia: "L'ombra al centro non è un disegno: ogni punto dell'immagine è un raggio di luce seguito all'indietro. Il disco sembra piegato sopra e sotto, ma è piatto.",
        compito: "Trascina sull'immagine per girare attorno al buco nero.",
        esito: "Da ogni lato il disco sembra piegarsi sopra e sotto l'ombra. A piegarsi è la luce.",
        indizio: indizio("telescopio", "Trascina qui"),
        prepara: |s| {
            s.stanza = Stanza::BucoNero;
            s.masse = 4.3e6;
            s.telescopio_grande = true;
            s.azimut = 0.0;
            s.elevazione = 0.14;
            s.r_cam = 20.0;
        },
        fatto: |s| s.azimut.abs() > 0.5 || (s.elevazione - 0.14).abs() > 0.3,
        avanzamento: None,
    },
    Tappa {
        titolo: "Lancia una sonda",
        storia: "A sinistra la mappa vista dall'alto. Mentre trascini, il banco prevede già dove andrà a finire la sonda.",
        compito: "Trascina sulla mappa per lanciare una sonda vicino al buco nero.",
        esito: "Lanciata troppo piano cade dentro, più veloce gira attorno o scappa. Provane un'altra.",
        indizio: indizio("banco", "Trascina qui"),
        prepara: |s| {
            s.stanza = Stanza::BucoNero;
            s.telescopio_grande = false;
            s.lab.svuota();
            s.lab.lanci = 0;
            s.lab.strumento = crate::lab::Strumento::Sonda;
            s.lab.in_corso = true;
        },
        fatto: |s| s.lab.lanci >= 1,
        avanzamento: None,
    },
    Tappa {
        titolo: "Orbite che ruotano",
        storia: "Sul banco questa sonda gira attorno al buco nero. L'ellisse non si chiude mai: Newton direbbe il contrario.",
        compito: "Passa a «Newton» nella barra in alto e guarda l'orbita chiudersi.",
        esito: "Con Newton l'ellisse si chiude. Con Einstein ruota a ogni giro, come Mercurio ma molto più in fretta.",
        indizio: indizio("newton", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::BucoNero;
            s.telescopio_grande = false;
            s.lab.carica(0);
        },
        fatto: |s| s.lab.legge == colossi_fisica::geodetica::Legge::Newton,
        avanzamento: None,
    },
    Tappa {
        titolo: "Un cucchiaino da due miliardi di tonnellate",
        storia: "Questa stella è grande quanto Roma e pesa più del Sole.",
        compito: "Tieni premuto «Aggiungi materia» finché la stella crolla.",
        esito: "Oltre un certo punto niente può impedirle di crollare: ora è un buco nero.",
        indizio: indizio("aggiungi", "Tieni premuto"),
        prepara: |s| {
            s.stanza = Stanza::Neutroni;
            s.neutroni.ricomincia();
        },
        fatto: |s| s.neutroni.crollata(),
        avanzamento: Some(|s| s.neutroni.progresso_crollo()),
    },
    Tappa {
        titolo: "Giganti nati troppo presto",
        storia: "Questi buchi neri erano già enormi quando l'universo era giovane. Come hanno fatto a crescere in tempo? Non lo sappiamo ancora del tutto.",
        compito: "Trascina il punto arancione finché la curva tocca il quasar più lontano.",
        esito: "Ci sei arrivato. Serve un buco nero di partenza già pesante o una crescita velocissima, e gli astronomi discutono ancora quale.",
        indizio: indizio("seme", "Trascina questo punto"),
        prepara: |s| {
            s.stanza = Stanza::Giganti;
            s.giganti = Default::default();
        },
        fatto: |s| s.giganti.arriva_al_piu_lontano(),
        avanzamento: None,
    },
    Tappa {
        titolo: "Materia che non si vede",
        storia: "Le stelle che si vedono da sole non bastano a tenere insieme una galassia.",
        compito: "Togli la spunta a «Accesa», sotto Materia oscura, e guarda le stelle.",
        esito: "Senza materia oscura le stelle esterne volano via.",
        indizio: indizio("alone", "Spegni qui"),
        prepara: |s| {
            s.stanza = Stanza::Galassie;
            s.galassie.sim.fattore_alone = 1.0;
            s.galassie.costruisci(Scena::Una);
        },
        fatto: |s| s.galassie.sim.fattore_alone == 0.0,
        avanzamento: None,
    },
    Tappa {
        titolo: "Lo spazio che vibra",
        storia: "Due buchi neri si fondono e tre soli di massa diventano onde. A Terra un braccio di 4 km si è mosso di una frazione di protone.",
        compito: "Premi «Ascolta il segnale».",
        esito: "È il suono dello spazio che vibra: sale fino alla fusione e poi si spegne.",
        indizio: indizio("ascolta", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::Onde;
            s.onde.coppia = GW150914;
            s.onde.costruisci();
            s.onde.ascoltato = false;
        },
        fatto: |s| s.onde.ascoltato,
        avanzamento: None,
    },
    Tappa {
        titolo: "La luce più antica",
        storia: "All'inizio l'universo è una nebbia luminosa, poi diventa trasparente: quella stessa luce arriva ancora, stirata, da ogni direzione.",
        compito: "Premi «Scorri» e guarda la nebbia diradarsi.",
        esito: "Quella luce arriva fino a noi: è la fotografia più antica dell'universo.",
        indizio: indizio("scorri", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::Fondo;
            s.fondo.pagina = fondo::Pagina::Nebbia;
            s.fondo.z = fondo::Z_MASSIMO;
            s.fondo.in_corso = false;
        },
        fatto: |s| s.fondo.pagina == fondo::Pagina::Nebbia && s.fondo.z < 1000.0,
        avanzamento: Some(nebbia_avanza),
    },
];

const TEMPO: &[Tappa] = &[
    Tappa {
        titolo: "Un viaggio quasi alla velocità della luce",
        storia: "Un gemello parte per Alpha Centauri, l'altro resta. Dalla nave le stelle si avvicinano: lo spazio si accorcia.",
        compito: "Premi «Alpha Centauri a 0,99 c» e guarda il viaggio fino in fondo.",
        esito: "Chi viaggia è tornato più giovane: ha vissuto meno tempo di chi è rimasto.",
        indizio: indizio("alpha", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::Gemelli;
            s.gemelli.scena = true;
            s.gemelli.carica(0);
        },
        fatto: |s| s.gemelli.viaggio == 1 && s.gemelli.avanzamento >= 1.0,
        avanzamento: Some(|s| if s.gemelli.viaggio == 1 { s.gemelli.avanzamento as f32 } else { 0.0 }),
    },
    Tappa {
        titolo: "Due gemelli",
        storia: "Nel diagramma il tempo sale e lo spazio va di lato. Quello che viaggia torna più giovane: 6 anni contro 10.",
        compito: "Trascina il punto rosa verso destra e allunga il viaggio.",
        esito: "Più corre, meno tempo vive.",
        indizio: indizio("vertice", "Trascina il punto"),
        prepara: |s| {
            s.stanza = Stanza::Gemelli;
            s.gemelli.scena = false;
            s.gemelli.carica(0);
        },
        fatto: |s| guida::sfida_ora(s, Stanza::Gemelli, 0),
        avanzamento: None,
    },
    Tappa {
        titolo: "Nessuno ha ragione da solo",
        storia: "Chi guarda da un altro punto di vista inclina le coordinate, e i suoi «adesso» cambiano.",
        compito: "Sposta il cursore «velocità osservatore» a destra o a sinistra.",
        esito: "Le coordinate si inclinano, ma quanti anni ha vissuto ognuno non cambia.",
        indizio: indizio("chi_guarda", "Sposta qui"),
        prepara: |s| {
            s.stanza = Stanza::Gemelli;
            s.gemelli.scena = false;
            s.gemelli.beta_vista = 0.0;
            s.gemelli.segnali = false;
        },
        fatto: |s| s.gemelli.beta_vista.abs() > 0.2,
        avanzamento: None,
    },
    Tappa {
        titolo: "Perché cadiamo",
        storia: "Salire fa guadagnare tempo, correre ne fa perdere. La palla segue il percorso in cui il suo orologio segna di più.",
        compito: "Trascina la curva finché il tempo raggiunge il 90% del massimo possibile.",
        esito: "Ci sei: la palla sceglie davvero il percorso in cui il suo orologio segna di più.",
        indizio: indizio("scena", "Trascina la curva"),
        prepara: |s| {
            s.stanza = Stanza::Cadere;
            s.cadere = Default::default();
        },
        fatto: |s| guida::sfida_ora(s, Stanza::Cadere, 0),
        avanzamento: Some(cadere_avanza),
    },
    Tappa {
        titolo: "Cade, senza nessuna forza",
        storia: "La gravità, qui, è soprattutto tempo che scorre a ritmi diversi.",
        compito: "Premi «Lascia fare alla natura».",
        esito: "La curva diventa da sola una parabola: è la caduta libera.",
        indizio: indizio("natura", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::Cadere;
            s.cadere.natura = false;
        },
        fatto: |s| s.cadere.natura,
        avanzamento: None,
    },
    Tappa {
        titolo: "Cosa curva lo spazio-tempo",
        storia: "Nel vuoto la palla di particelle si deforma ma il volume resta. Dentro la materia si restringe: è questo che la materia fa allo spazio-tempo.",
        compito: "Clicca dentro la sfera di materia per lasciar cadere una palla lì.",
        esito: "Dentro la materia il volume della palla si restringe.",
        indizio: indizio("scena", "Clicca nella sfera"),
        prepara: |s| {
            s.stanza = Stanza::Curva;
            s.curva = Default::default();
        },
        fatto: |s| guida::sfida_ora(s, Stanza::Curva, 0),
        avanzamento: None,
    },
    Tappa {
        titolo: "Il tempo si ferma, per chi guarda",
        storia: "Una sonda cade nel buco nero. Da lontano la vedi fermarsi sull'orizzonte.",
        compito: "Premi «di ciascuno» nella barra in basso e guarda la sonda attraversare l'orizzonte.",
        esito: "Per lei il tempo non si ferma: attraversa l'orizzonte e arriva al centro.",
        indizio: indizio("di_ciascuno", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::BucoNero;
            s.telescopio_grande = false;
            s.lab.carica(7);
        },
        fatto: |s| s.lab.orologio == colossi_fisica::geodetica::Orologio::Proprio,
        avanzamento: None,
    },
];

const SCIENZIATO: &[Tappa] = &[
    Tappa {
        titolo: "Pesa un buco nero",
        storia: "Queste croci sono misure vere di una stella.",
        compito: "Muovi i cursori della colonna a destra finché lo scarto dalle misure scende sotto 3.",
        esito: "Hai appena fatto ciò per cui è stato dato un Nobel.",
        indizio: indizio("cursori", "Muovi i cursori"),
        prepara: |s| {
            s.stanza = Stanza::Osservatorio;
            s.osservatorio.sezione = Sezione::Orbita;
            s.osservatorio.riparti();
        },
        fatto: |s| s.osservatorio.a_mano_vicino(3.0),
        avanzamento: None,
    },
    Tappa {
        titolo: "Il primo suono dello spazio",
        storia: "La curva bianca è ciò che ha registrato davvero LIGO nel 2015.",
        compito: "Premi «Calcola la migliore»: dalla sola forma dell'onda escono le masse dei due buchi neri.",
        esito: "Dalla forma dell'onda hai ricavato le masse dei due buchi neri.",
        indizio: indizio("calcola_missione", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::Osservatorio;
            s.osservatorio.sezione = Sezione::Altra(0);
            s.osservatorio.missioni[0].riparti();
        },
        fatto: |s| s.osservatorio.missioni[0].calcolata(),
        avanzamento: None,
    },
    Tappa {
        titolo: "Inventa una materia",
        storia: "Ogni materia dà una curva diversa. Se la curva manca i pulsar misurati, la tua idea è esclusa dai dati.",
        compito: "Scegli un'altra materia nell'elenco «La materia».",
        esito: "Ecco come funziona la fisica: un'idea resta in piedi solo se i dati la reggono.",
        indizio: indizio("materia", "Scegli qui"),
        prepara: |s| {
            s.stanza = Stanza::Neutroni;
            s.neutroni = Default::default();
        },
        fatto: |s| s.neutroni.materia_cambiata(),
        avanzamento: None,
    },
    Tappa {
        titolo: "Tre corpi, nessuna formula",
        storia: "Con tre corpi che si attraggono non esiste una formula per il futuro: si può solo calcolare passo per passo.",
        compito: "Spunta «Un gemello che parte quasi uguale» e guarda i due sistemi separarsi.",
        esito: "Partivano quasi identici e la differenza cresce: oltre un certo tempo, prevedere è impossibile.",
        indizio: indizio("gemello", "Spunta qui"),
        prepara: |s| {
            s.stanza = Stanza::TreCorpi;
            s.trecorpi.pagina = Pagina::Banco;
            s.trecorpi.carica(scena("Problema pitagorico"));
            s.trecorpi.caos = false;
            s.trecorpi.ricomincia();
        },
        fatto: |s| s.trecorpi.caos,
        avanzamento: None,
    },
    Tappa {
        titolo: "La mappa del caos",
        storia: "Ogni casella è una simulazione completa. Dove i colori si mescolano, spostare un corpo di un soffio cambia tutto.",
        compito: "Apri «Mappa del caos» in alto, nella colonna a destra.",
        esito: "Dove i colori si mescolano, il futuro dipende da un soffio.",
        indizio: indizio("mappa", "Premi qui"),
        prepara: |s| {
            s.stanza = Stanza::TreCorpi;
            s.trecorpi.caos = false;
            s.trecorpi.pagina = Pagina::Banco;
        },
        fatto: |s| s.trecorpi.pagina == Pagina::Mappa,
        avanzamento: None,
    },
    Tappa {
        titolo: "Di cosa è fatto l'universo",
        storia: "Muovi materia e curvatura: i picchi si spostano.",
        compito: "Muovi «curvatura dello spazio» e guarda i picchi spostarsi.",
        esito: "Confrontando questa curva con il cielo si misura la composizione dell'universo.",
        indizio: indizio("curvatura", "Muovi qui"),
        prepara: |s| {
            s.stanza = Stanza::Fondo;
            s.fondo.pagina = fondo::Pagina::Macchie;
            s.fondo.cosmo = PLANCK;
        },
        fatto: |s| s.fondo.cosmo.curvatura.abs() > 0.02,
        avanzamento: None,
    },
];

pub const VISITE: [Visita; 3] = [
    Visita {
        nome: "Il giro dei colossi",
        durata: "9 tappe",
        descrizione: "Dalla grandezza di una persona alla luce più antica: buchi neri, stelle che crollano, galassie, onde dello spazio.",
        tappe: COLOSSI,
    },
    Visita {
        nome: "Che cos'è il tempo",
        durata: "7 tappe",
        descrizione: "Due gemelli che invecchiano in modo diverso, perché le cose cadono e cosa succede vicino a un buco nero.",
        tappe: TEMPO,
    },
    Visita {
        nome: "Scienziato per un giorno",
        durata: "6 tappe",
        descrizione: "Pesa un buco nero, ascolta il primo segnale di LIGO e scopri perché il futuro di tre corpi non si prevede.",
        tappe: SCIENZIATO,
    },
];

/// Punto in cui ci si trova in una visita.
#[derive(Clone, Copy, PartialEq)]
pub struct InCorso {
    pub visita: usize,
    pub tappa: usize,
    /// Istante, in secondi di egui, in cui il compito e' riuscito.
    pub riuscita: Option<f64>,
    /// La scheda ridotta a una riga sola.
    pub ridotta: bool,
}

impl InCorso {
    pub fn nuova(visita: usize, tappa: usize) -> Self {
        Self { visita, tappa, riuscita: None, ridotta: false }
    }
}

pub fn inizia(s: &mut Stato, visita: usize) {
    salta_a(s, visita, 0);
}

/// Si apre una tappa precisa, preparando i banchi.
pub fn salta_a(s: &mut Stato, visita: usize, tappa: usize) {
    let ridotta = s.visita.is_some_and(|c| c.ridotta);
    s.visita = Some(InCorso { ridotta, ..InCorso::nuova(visita, tappa) });
    (VISITE[visita].tappe[tappa].prepara)(s);
}

fn vai(s: &mut Stato, corso: InCorso, delta: i32) {
    let totale = VISITE[corso.visita].tappe.len() as i32;
    let nuova = corso.tappa as i32 + delta;
    if nuova >= totale {
        esci(s);
    } else {
        salta_a(s, corso.visita, nuova.max(0) as usize);
    }
}

/// Si lascia la visita: il telescopio ingrandito era solo per quella tappa.
fn esci(s: &mut Stato) {
    s.visita = None;
    s.telescopio_grande = false;
}

/// Segna il compito come riuscito, la prima volta che lo e'.
pub fn controlla(s: &mut Stato, adesso: f64) {
    let Some(corso) = s.visita else { return };
    if corso.riuscita.is_none() && (VISITE[corso.visita].tappe[corso.tappa].fatto)(s) {
        s.visita = Some(InCorso { riuscita: Some(adesso), ..corso });
    }
}

// ---------------------------------------------------------------------------
// Dove stanno i comandi: i banchi li segnalano, la visita li evidenzia.
// ---------------------------------------------------------------------------

type Registro = HashMap<&'static str, (Rect, u64)>;

fn id_registro() -> Id {
    Id::new("colossi_bersagli")
}

fn id_scorri() -> Id {
    Id::new("colossi_scorri_a")
}

fn id_ultima() -> Id {
    Id::new("colossi_ultima_tappa")
}

/// Un banco dichiara dove sta un'area o un comando che una tappa potrebbe indicare.
pub fn segna_rect(ctx: &egui::Context, id: &'static str, rett: Rect) {
    let passo = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Registro>(id_registro()).insert(id, (rett, passo));
    });
}

/// Come `segna_rect`, per un comando disegnato con egui. Se la tappa lo sta cercando
/// e non si vede (colonna scorsa troppo in alto o in basso), lo porta in vista.
pub fn segna(ui: &egui::Ui, id: &'static str, r: &egui::Response) {
    let ctx = ui.ctx();
    let richiesto = ctx.data_mut(|d| {
        let voce = d.get_temp_mut_or_default::<Option<&'static str>>(id_scorri());
        let tocca_a_lui = *voce == Some(id);
        if tocca_a_lui {
            *voce = None;
        }
        tocca_a_lui
    });
    if richiesto && !ui.clip_rect().contains_rect(r.rect) {
        r.scroll_to_me(Some(egui::Align::Center));
    }
    let visibile = r.rect.intersect(ui.clip_rect());
    if visibile.width() > 1.0 && visibile.height() > 1.0 {
        segna_rect(ctx, id, visibile);
    }
}

/// Dove sta il comando, se in questo fotogramma e' sullo schermo.
pub fn bersaglio(ctx: &egui::Context, id: &str) -> Option<Rect> {
    let passo = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Registro>(id_registro()).get(id).copied())
        .filter(|(_, p)| *p == passo)
        .map(|(r, _)| r)
}

// ---------------------------------------------------------------------------
// La scheda in fondo.
// ---------------------------------------------------------------------------

enum Azione {
    Vai(i32),
    Esci,
    Riduci,
}

fn pulso(adesso: f64) -> f32 {
    (0.5 + 0.5 * (adesso * 4.5).sin()) as f32
}

/// Un puntino per tappa: verdi quelle fatte, cerchiato quella in corso.
fn punti(ui: &mut egui::Ui, totale: usize, attuale: usize, riuscita: bool, pulso: f32) {
    let passo = 16.0;
    let (area, _) = ui.allocate_exact_size(Vec2::new(passo * totale as f32, 18.0), Sense::hover());
    let p = ui.painter();
    for i in 0..totale {
        let c = Pos2::new(area.left() + passo * (i as f32 + 0.5), area.center().y);
        if i < attuale {
            p.circle_filled(c, 4.5, VERDE);
        } else if i == attuale {
            let tinta = if riuscita { VERDE } else { CIANO };
            p.circle_filled(c, 5.5, tinta);
            p.circle_stroke(c, 8.0, Stroke::new(1.2, tinta.gamma_multiply(0.4 + 0.5 * pulso)));
        } else {
            p.circle_stroke(c, 4.0, Stroke::new(1.3, GRIGIO));
        }
    }
}

/// Il cerchietto del compito: vuoto e pulsante, poi verde con la spunta.
fn spunta_grande(ui: &mut egui::Ui, fatta: bool, pulso: f32) {
    let (area, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
    let p = ui.painter();
    let c = area.center();
    if fatta {
        p.circle_filled(c, 11.0, VERDE);
        let tratto = Stroke::new(2.4, SUPERFICIE);
        p.line_segment([c + Vec2::new(-5.0, 0.5), c + Vec2::new(-1.5, 4.0)], tratto);
        p.line_segment([c + Vec2::new(-1.5, 4.0), c + Vec2::new(5.5, -4.0)], tratto);
    } else {
        p.circle_stroke(c, 10.0, Stroke::new(2.0, CIANO.gamma_multiply(0.5 + 0.5 * pulso)));
    }
}

fn riquadro_compito(ui: &mut egui::Ui, tappa: &Tappa, s: &Stato, riuscita: bool, pulso: f32, larghezza: f32) {
    let (tinta, fondo) = if riuscita { (VERDE, VERDE.gamma_multiply(0.14)) } else { (CIANO, CIANO.gamma_multiply(0.10)) };
    let bordo = if riuscita { 1.0 } else { 0.5 + 0.5 * pulso };
    Frame::new()
        .fill(fondo)
        .stroke(Stroke::new(1.4, tinta.gamma_multiply(bordo)))
        .corner_radius(8)
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            ui.set_width((larghezza - 24.0).max(120.0));
            ui.horizontal_top(|ui| {
                spunta_grande(ui, riuscita, pulso);
                ui.vertical(|ui| {
                    ui.label(RichText::new(if riuscita { "FATTO" } else { "DA FARE" }).small().strong().color(tinta));
                    let testo = if riuscita { tappa.esito } else { tappa.compito };
                    ui.add(egui::Label::new(RichText::new(testo).size(15.0).strong().color(CHIARO)).wrap());
                });
            });
            if !riuscita {
                if let Some(f) = tappa.avanzamento {
                    ui.add_space(4.0);
                    ui.add(egui::ProgressBar::new(f(s)).desired_height(6.0).fill(CIANO));
                }
            }
        });
}

fn bottoni(ui: &mut egui::Ui, corso: InCorso, ultima: bool, riuscita: bool, pulso: f32, azione: &mut Option<Azione>) {
    ui.horizontal(|ui| {
        if ui.add_enabled(corso.tappa > 0, Button::new("Indietro")).clicked() {
            *azione = Some(Azione::Vai(-1));
        }
        let testo = if ultima { "Fine" } else { "Avanti" };
        let avanti = if riuscita {
            Button::new(RichText::new(testo).strong().color(Color32::BLACK)).fill(VERDE.gamma_multiply(0.7 + 0.3 * pulso))
        } else {
            Button::new(testo)
        };
        if ui.add_enabled(riuscita, avanti.min_size(Vec2::new(110.0, 30.0))).clicked() {
            *azione = Some(Azione::Vai(1));
        }
        if !riuscita && ui.small_button("Salta questa prova").clicked() {
            *azione = Some(Azione::Vai(1));
        }
    });
}

/// La scheda della tappa, sotto qualunque banco. E' un pannello: il banco si
/// restringe e resta tutto utilizzabile mentre la visita e' aperta.
pub fn mostra(ui: &mut egui::Ui, s: &mut Stato) {
    let ctx = ui.ctx().clone();
    if s.visita.is_none() {
        ctx.data_mut(|d| {
            *d.get_temp_mut_or_default::<Option<&'static str>>(id_scorri()) = None;
            *d.get_temp_mut_or_default::<Option<(usize, usize)>>(id_ultima()) = None;
        });
        return;
    }
    let adesso = ctx.input(|i| i.time);
    controlla(s, adesso);
    let Some(corso) = s.visita else { return };
    let visita = &VISITE[corso.visita];
    let tappa = &visita.tappe[corso.tappa];
    let totale = visita.tappe.len();
    let ultima = corso.tappa + 1 == totale;
    let riuscita = corso.riuscita.is_some();
    let battito = pulso(adesso);

    // A ogni nuova tappa si chiede al comando indicato di mostrarsi, se e' fuori vista.
    let cambiata = ctx.data_mut(|d| {
        let ultima_vista = d.get_temp_mut_or_default::<Option<(usize, usize)>>(id_ultima());
        let adesso = Some((corso.visita, corso.tappa));
        let cambia = *ultima_vista != adesso;
        *ultima_vista = adesso;
        cambia
    });
    if cambiata {
        let primo = tappa.indizio.as_ref().map(|i| i.id);
        ctx.data_mut(|d| *d.get_temp_mut_or_default::<Option<&'static str>>(id_scorri()) = primo);
    }

    let larga = ui.available_width() >= 900.0;
    let altezza = if corso.ridotta {
        48.0
    } else if larga {
        176.0
    } else {
        300.0
    };
    let mut azione = None;
    let stato: &Stato = s;
    egui::Panel::bottom("visita")
        .exact_size(altezza)
        .resizable(false)
        .frame(Frame::new().fill(SUPERFICIE.gamma_multiply(1.25)).inner_margin(Margin::symmetric(16, 8)))
        .show(ui, |ui| {
            if corso.ridotta {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Esci").clicked() {
                        azione = Some(Azione::Esci);
                    }
                    if ui.small_button("Apri").clicked() {
                        azione = Some(Azione::Riduci);
                    }
                    let testo = if ultima { "Fine" } else { "Avanti" };
                    if ui.add_enabled(riuscita, Button::new(testo)).clicked() {
                        azione = Some(Azione::Vai(1));
                    }
                    if !riuscita && ui.small_button("Salta").clicked() {
                        azione = Some(Azione::Vai(1));
                    }
                    let riga = if riuscita { tappa.esito } else { tappa.compito };
                    let titolo = format!("{} di {}  ·  {}", corso.tappa + 1, totale, riga);
                    ui.add(egui::Label::new(RichText::new(titolo).color(if riuscita { VERDE } else { CHIARO })).truncate());
                });
                return;
            }

            ui.horizontal(|ui| {
                punti(ui, totale, corso.tappa, riuscita, battito);
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("{}  ·  TAPPA {} DI {}", visita.nome.to_uppercase(), corso.tappa + 1, totale))
                        .small()
                        .strong()
                        .color(GRIGIO),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Esci dalla visita").clicked() {
                        azione = Some(Azione::Esci);
                    }
                    if ui.small_button("Riduci").on_hover_text("Lascia una riga sola: il comando resta evidenziato").clicked() {
                        azione = Some(Azione::Riduci);
                    }
                });
            });
            ui.add_space(4.0);

            let larghezza = ui.available_width();
            if larga {
                let sinistra = (larghezza * 0.42).min(560.0);
                let destra = larghezza - sinistra - 24.0;
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(sinistra);
                        ui.label(RichText::new(tappa.titolo).size(18.0).strong().color(CIANO));
                        ui.add(egui::Label::new(RichText::new(tappa.storia).size(14.5).color(CHIARO)).wrap());
                    });
                    ui.add_space(24.0);
                    ui.vertical(|ui| {
                        ui.set_width(destra);
                        riquadro_compito(ui, tappa, stato, riuscita, battito, destra);
                        ui.add_space(6.0);
                        bottoni(ui, corso, ultima, riuscita, battito, &mut azione);
                    });
                });
            } else {
                ui.label(RichText::new(tappa.titolo).size(18.0).strong().color(CIANO));
                ui.add(egui::Label::new(RichText::new(tappa.storia).size(14.5).color(CHIARO)).wrap());
                ui.add_space(6.0);
                riquadro_compito(ui, tappa, stato, riuscita, battito, larghezza);
                ui.add_space(6.0);
                bottoni(ui, corso, ultima, riuscita, battito, &mut azione);
            }
        });

    match azione {
        Some(Azione::Esci) => esci(s),
        Some(Azione::Riduci) => s.visita = Some(InCorso { ridotta: !corso.ridotta, ..corso }),
        Some(Azione::Vai(d)) => vai(s, corso, d),
        None => {}
    }
}

/// La scritta con la freccia che accompagna il comando evidenziato.
fn etichetta(p: &egui::Painter, schermo: Rect, centro: Pos2, testo: &str, freccia_in_basso: Option<bool>) {
    let galleria = p.layout_no_wrap(testo.to_string(), FontId::proportional(14.0), Color32::BLACK);
    let misura = galleria.size() + Vec2::new(22.0, 10.0);
    let x = centro.x.clamp(schermo.left() + misura.x * 0.5 + 6.0, schermo.right() - misura.x * 0.5 - 6.0);
    let rett = Rect::from_center_size(Pos2::new(x, centro.y), misura);
    p.rect_filled(rett, CornerRadius::same(10), CIANO);
    p.galley(rett.min + Vec2::new(11.0, 5.0), galleria, Color32::BLACK);
    if let Some(in_basso) = freccia_in_basso {
        let (base, punta) = if in_basso {
            (rett.center_bottom(), rett.center_bottom() + Vec2::new(0.0, 9.0))
        } else {
            (rett.center_top(), rett.center_top() - Vec2::new(0.0, 9.0))
        };
        p.add(Shape::convex_polygon(vec![base + Vec2::new(-7.0, 0.0), base + Vec2::new(7.0, 0.0), punta], CIANO, Stroke::NONE));
    }
}

/// Disegna sopra tutto un contorno che pulsa attorno al comando della tappa e una
/// scritta che dice cosa farci. Non intercetta il mouse: il comando resta usabile.
pub fn evidenzia(ctx: &egui::Context, s: &Stato) {
    let Some(corso) = s.visita else { return };
    if corso.riuscita.is_some() {
        return;
    }
    let Some(indizio) = &VISITE[corso.visita].tappe[corso.tappa].indizio else { return };
    let Some(rett) = bersaglio(ctx, indizio.id) else { return };
    let battito = pulso(ctx.input(|i| i.time));
    let schermo = ctx.content_rect();
    let p = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("visita_evidenzia")));

    if rett.width() > 420.0 || rett.height() > 260.0 {
        // Un'area grande: si accende il bordo e la scritta sta dentro, in alto.
        let dentro = rett.shrink(4.0);
        p.rect_stroke(dentro, CornerRadius::same(6), Stroke::new(3.0, CIANO.gamma_multiply(0.35 + 0.55 * battito)), StrokeKind::Inside);
        p.rect_stroke(dentro.shrink(4.0), CornerRadius::same(4), Stroke::new(1.0, CIANO.gamma_multiply(0.25)), StrokeKind::Inside);
        etichetta(&p, schermo, Pos2::new(dentro.center().x, dentro.top() + 30.0), indizio.testo, None);
    } else {
        let alone = rett.expand(5.0 + 3.0 * battito);
        p.rect_stroke(alone, CornerRadius::same(8), Stroke::new(2.5, CIANO.gamma_multiply(0.5 + 0.5 * battito)), StrokeKind::Outside);
        p.rect_stroke(rett.expand(2.0), CornerRadius::same(6), Stroke::new(1.2, Color32::WHITE.gamma_multiply(0.6)), StrokeKind::Outside);
        let rimbalzo = 3.0 * battito;
        if rett.top() > schermo.top() + 110.0 {
            etichetta(&p, schermo, Pos2::new(rett.center().x, alone.top() - 22.0 - rimbalzo), indizio.testo, Some(true));
        } else {
            etichetta(&p, schermo, Pos2::new(rett.center().x, alone.bottom() + 22.0 + rimbalzo), indizio.testo, Some(false));
        }
    }
    ctx.request_repaint();
}

#[cfg(test)]
mod test {
    use super::*;

    /// Tutti i nomi che i banchi segnalano: un refuso in una tappa si nota qui.
    const NOTI: &[&str] = &[
        "viaggia", "alpha", "telescopio", "banco", "newton", "aggiungi", "seme", "alone", "ascolta", "scorri", "vertice", "chi_guarda", "scena",
        "natura", "di_ciascuno", "cursori", "calcola_missione", "materia", "gemello", "mappa", "curvatura",
    ];

    fn tutte() -> impl Iterator<Item = (usize, usize, &'static Tappa)> {
        VISITE.iter().enumerate().flat_map(|(i, v)| v.tappe.iter().enumerate().map(move |(k, t)| (i, k, t)))
    }

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
    fn nessun_compito_e_gia_fatto_alla_partenza() {
        for (i, k, t) in tutte() {
            let mut s = Stato::default();
            salta_a(&mut s, i, k);
            assert!(!(t.fatto)(&s), "«{}» sarebbe gia' fatto appena si apre la tappa", t.titolo);
        }
    }

    #[test]
    fn l_avanzamento_parte_da_zero() {
        for (i, k, t) in tutte() {
            if let Some(f) = t.avanzamento {
                let mut s = Stato::default();
                salta_a(&mut s, i, k);
                assert!(f(&s) < 0.3, "«{}» parte gia' avanti: {}", t.titolo, f(&s));
            }
        }
    }

    #[test]
    fn la_durata_scritta_corrisponde() {
        for v in &VISITE {
            assert!(v.durata.starts_with(&v.tappe.len().to_string()), "{}", v.nome);
        }
    }

    #[test]
    fn i_testi_stanno_nella_scheda() {
        for (_, _, t) in tutte() {
            assert!(t.storia.chars().count() <= 170, "storia troppo lunga: {}", t.titolo);
            assert!(t.compito.chars().count() <= 100, "compito troppo lungo: {}", t.titolo);
            assert!(t.esito.chars().count() <= 120, "esito troppo lungo: {}", t.titolo);
            assert!(t.titolo.chars().count() <= 48, "titolo troppo lungo: {}", t.titolo);
        }
    }

    #[test]
    fn ogni_tappa_indica_un_comando_noto() {
        for (_, _, t) in tutte() {
            let i = t.indizio.as_ref().unwrap_or_else(|| panic!("{} non indica niente", t.titolo));
            assert!(NOTI.contains(&i.id), "{}: comando «{}» sconosciuto", t.titolo, i.id);
            assert!(i.testo.chars().count() <= 22, "{}: scritta troppo lunga", t.titolo);
        }
    }

    #[test]
    fn il_compito_riuscito_si_riconosce_una_volta_sola() {
        let mut s = Stato::default();
        inizia(&mut s, 0);
        controlla(&mut s, 1.0);
        assert!(s.visita.is_some_and(|c| c.riuscita.is_none()));
        s.scala.log_vista = 7.5;
        controlla(&mut s, 2.0);
        assert_eq!(s.visita.and_then(|c| c.riuscita), Some(2.0));
        // Tornare indietro con il gesto non cancella il risultato.
        s.scala.log_vista = 1.0;
        controlla(&mut s, 3.0);
        assert_eq!(s.visita.and_then(|c| c.riuscita), Some(2.0));
    }

    #[test]
    fn andare_avanti_riparte_dal_compito_da_fare() {
        let mut s = Stato::default();
        inizia(&mut s, 0);
        s.scala.log_vista = 7.5;
        controlla(&mut s, 1.0);
        let corso = s.visita.unwrap();
        vai(&mut s, corso, 1);
        let ora = s.visita.unwrap();
        assert_eq!(ora.tappa, 1);
        assert!(ora.riuscita.is_none());
    }

    #[test]
    fn la_scheda_ridotta_resta_ridotta_cambiando_tappa() {
        let mut s = Stato::default();
        inizia(&mut s, 0);
        s.visita = s.visita.map(|c| InCorso { ridotta: true, ..c });
        let corso = s.visita.unwrap();
        vai(&mut s, corso, 1);
        assert!(s.visita.unwrap().ridotta);
    }
}
