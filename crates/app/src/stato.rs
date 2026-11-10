//! Stato dell'applicazione e sua traduzione nei parametri dello shader.

use crate::cadere::Cadere;
use crate::fondo::BancoFondo;
use crate::galassie::BancoGalassie;
use crate::onde::BancoOnde;
use crate::curva::Curva;
use crate::gemelli::Gemelli;
use crate::giganti::Giganti;
use crate::lab::Lab;
use crate::neutroni::Neutroni;
use crate::osservatorio::Osservatorio;
use crate::orizzonte::BancoOrizzonte;
use crate::scala::Scala;
use crate::stelle::BancoStelle;
use crate::trecorpi::TreCorpi;
use crate::visite::InCorso;
use bytemuck::{Pod, Zeroable};
use colossi_fisica::buco_nero::{BucoNero, RIFERIMENTI};

pub struct Preset {
    pub nome: &'static str,
    pub masse: f64,
}

pub const PRESET: [Preset; 4] = [
    Preset { nome: "Stellare", masse: 10.0 },
    Preset { nome: "Sagittarius A*", masse: 4.3e6 },
    Preset { nome: "M87*", masse: 6.5e9 },
    Preset { nome: "TON 618", masse: 4.07e10 },
];

/// Quanti rs/c scorrono nel disco per ogni secondo reale.
const RITMO_DISCO: f64 = 4.0;
const R_INTERNO_DISCO: f32 = 3.0;
pub const R_ESTERNO_DISCO: f32 = 14.0;
/// Oltre questa distanza, in raggi, un riferimento non verrebbe inquadrato.
const R_MASSIMO_ANELLI: f64 = 3000.0;
pub const R_CAM_MIN: f64 = 1.6;
pub const R_CAM_MAX: f64 = 3000.0;
const MAX_SORGENTI: usize = 16;

pub const COLORI_ANELLI: [[f32; 3]; 6] = [
    [0.15, 0.85, 1.0],
    [1.0, 0.8, 0.2],
    [0.45, 1.0, 0.4],
    [1.0, 0.4, 0.75],
    [0.65, 0.55, 1.0],
    [1.0, 0.55, 0.3],
];

pub struct Stato {
    pub masse: f64,
    pub azimut: f64,
    pub elevazione: f64,
    pub r_cam: f64,
    pub disco: bool,
    pub doppler: bool,
    pub confronti: bool,
    pub scie: bool,
    pub qualita: f32,
    pub tempo: f64,
    pub telescopio_grande: bool,
    pub lab: Lab,
    pub stanza: Stanza,
    pub gemelli: Gemelli,
    pub cadere: Cadere,
    pub curva: Curva,
    pub neutroni: Neutroni,
    pub giganti: Giganti,
    pub osservatorio: Osservatorio,
    pub trecorpi: TreCorpi,
    pub galassie: BancoGalassie,
    pub onde: BancoOnde,
    pub fondo: BancoFondo,
    pub scala: Scala,
    pub stelle: BancoStelle,
    pub orizzonte: BancoOrizzonte,
    pub visita: Option<InCorso>,
    /// Sfide della guida gia' riuscite: banco e numero della sfida.
    pub sfide_fatte: std::collections::HashSet<(Stanza, usize)>,
    pub guida_aperta: bool,
    pub guida_visibile: bool,
}

/// Quale banco di prova e' aperto.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stanza {
    Benvenuto,
    BucoNero,
    Gemelli,
    Cadere,
    Curva,
    Neutroni,
    Giganti,
    Osservatorio,
    TreCorpi,
    Galassie,
    Onde,
    Fondo,
    Scala,
    Stelle,
    Orizzonte,
}

/// Un'area raccoglie banchi sullo stesso tema.
pub struct Area {
    pub nome: &'static str,
    pub breve: &'static str,
    pub descrizione: &'static str,
    pub banchi: &'static [(Stanza, &'static str)],
}

pub const AREE: [Area; 4] = [
    Area {
        nome: "I colossi",
        breve: "Colossi",
        descrizione: "Buchi neri, stelle che crollano, galassie, onde dello spazio e la luce più antica.",
        banchi: &[
            (Stanza::Scala, "Scala"),
            (Stanza::BucoNero, "Buco nero"),
            (Stanza::Neutroni, "Stelle di neutroni"),
            (Stanza::Giganti, "Giganti"),
            (Stanza::Stelle, "Stelle"),
            (Stanza::Galassie, "Galassie"),
            (Stanza::Onde, "Onde"),
            (Stanza::Fondo, "Fondo cosmico"),
        ],
    },
    Area {
        nome: "Il tempo e lo spazio",
        breve: "Tempo e spazio",
        descrizione: "Gemelli che invecchiano diversamente, perché cadiamo, cosa piega lo spazio, dentro l'orizzonte.",
        banchi: &[
            (Stanza::Gemelli, "Gemelli"),
            (Stanza::Cadere, "Perché cadiamo"),
            (Stanza::Curva, "Cosa curva lo spazio"),
            (Stanza::Orizzonte, "Orizzonte"),
        ],
    },
    Area {
        nome: "Osservatorio",
        breve: "Osservatorio",
        descrizione: "Misure vere degli astronomi: pesa un buco nero, ascolta LIGO, trova la materia oscura.",
        banchi: &[(Stanza::Osservatorio, "Osservatorio")],
    },
    Area {
        nome: "Tre corpi",
        breve: "Tre corpi",
        descrizione: "Tre corpi che si attraggono: soluzioni rare, caos, punti di Lagrange, un pianeta fra tre soli.",
        banchi: &[(Stanza::TreCorpi, "Tre corpi")],
    },
];

impl Stanza {
    /// L'area a cui appartiene il banco.
    pub fn area(self) -> Option<usize> {
        AREE.iter().position(|a| a.banchi.iter().any(|b| b.0 == self))
    }
}

impl Default for Stato {
    fn default() -> Self {
        Self {
            masse: 4.3e6,
            azimut: 0.0,
            elevazione: 0.14,
            r_cam: 20.0,
            disco: true,
            doppler: true,
            confronti: false,
            scie: true,
            qualita: 1.0,
            tempo: 37.0,
            telescopio_grande: false,
            lab: Lab::default(),
            stanza: Stanza::Benvenuto,
            gemelli: Gemelli::default(),
            cadere: Cadere::default(),
            curva: Curva::default(),
            neutroni: Neutroni::default(),
            giganti: Giganti::default(),
            osservatorio: Osservatorio::default(),
            trecorpi: TreCorpi::default(),
            galassie: BancoGalassie::default(),
            onde: BancoOnde::default(),
            fondo: BancoFondo::default(),
            scala: Scala::default(),
            stelle: BancoStelle::default(),
            orizzonte: BancoOrizzonte::default(),
            visita: None,
            sfide_fatte: std::collections::HashSet::new(),
            guida_aperta: true,
            guida_visibile: true,
        }
    }
}

/// Un riferimento familiare mostrato nella scena.
pub struct AnelloVisibile {
    pub nome: &'static str,
    pub raggi: f64,
    pub colore: [f32; 3],
}

impl Stato {
    pub fn buco_nero(&self) -> BucoNero {
        BucoNero::da_masse_solari(self.masse)
    }

    pub fn avanza(&mut self, dt: f64) {
        match self.stanza {
            Stanza::Benvenuto => {}
            Stanza::BucoNero => {
                if self.lab.in_corso {
                    self.tempo += dt * RITMO_DISCO;
                }
                self.lab.avanza(dt);
            }
            Stanza::Gemelli => self.gemelli.avanza(dt),
            Stanza::Cadere => self.cadere.avanza(dt),
            Stanza::Curva => self.curva.avanza(dt),
            Stanza::Neutroni => self.neutroni.avanza(dt),
            Stanza::Giganti => {}
            Stanza::Osservatorio => self.osservatorio.ammasso.avanza(dt),
            Stanza::TreCorpi => self.trecorpi.avanza(dt),
            Stanza::Galassie => self.galassie.avanza(dt),
            Stanza::Onde => self.onde.avanza(dt),
            Stanza::Fondo => self.fondo.avanza(dt),
            Stanza::Scala => self.scala.avanza(dt),
            Stanza::Stelle => self.stelle.avanza(dt),
            Stanza::Orizzonte => self.orizzonte.avanza(dt),
        }
        crate::guida::controlla(self);
    }

    /// Riferimenti abbastanza grandi da stare fuori dall'orizzonte.
    pub fn anelli(&self) -> Vec<AnelloVisibile> {
        let rs = self.buco_nero().raggio();
        RIFERIMENTI
            .iter()
            .map(|r| (r.nome, r.raggio_m / rs))
            .filter(|(_, raggi)| *raggi > 1.05 && *raggi < R_MASSIMO_ANELLI)
            .take(COLORI_ANELLI.len())
            .zip(COLORI_ANELLI)
            .map(|((nome, raggi), colore)| AnelloVisibile { nome, raggi, colore })
            .collect()
    }

    /// Posizione della camera nel piano del banco, se serve disegnarla.
    pub fn camera_sul_banco(&self) -> [f64; 2] {
        let proiezione = self.r_cam * self.elevazione.cos();
        [proiezione * self.azimut.sin(), proiezione * self.azimut.cos()]
    }

    fn sorgenti(&self) -> ([[f32; 4]; MAX_SORGENTI], [[f32; 4]; MAX_SORGENTI], usize) {
        let mut posizioni = [[0.0; 4]; MAX_SORGENTI];
        let mut colori = [[0.0; 4]; MAX_SORGENTI];
        let raggio = (0.005 * self.r_cam).max(0.025) as f32;
        let selezionato = self.lab.selezionato;
        // Prima l'oggetto selezionato, poi gli altri ancora attivi.
        let attivi = self
            .lab
            .oggetti
            .iter()
            .filter(|o| o.id == selezionato.unwrap_or(0))
            .chain(self.lab.oggetti.iter().filter(|o| o.id != selezionato.unwrap_or(0)))
            .filter(|o| o.fisso || !o.moto.esito.fermo())
            .take(MAX_SORGENTI);
        let mut n = 0;
        for o in attivi {
            let [x, y, z] = o.moto.x;
            posizioni[n] = [x as f32, y as f32, z as f32, raggio];
            let f = (1.0 - 1.0 / o.moto.raggio()).max(0.0) as f32;
            let [r, g, b] = o.colore.map(|c| (f32::from(c) / 255.0).powf(2.2));
            let luminosita = f * f * 5.0;
            colori[n] = [r * luminosita, g * luminosita, b * luminosita, 0.0];
            n += 1;
        }
        (posizioni, colori, n)
    }

    pub fn parametri(&self) -> Parametri {
        let (sa, ca) = (self.azimut.sin(), self.azimut.cos());
        let (se, ce) = (self.elevazione.sin(), self.elevazione.cos());
        let radiale = [ce * sa, se, ce * ca];
        let pos = radiale.map(|c| c * self.r_cam);
        let avanti = radiale.map(|c| -c);
        let destra = normalizza([-avanti[2], 0.0, avanti[0]]);
        let su = [
            destra[1] * avanti[2] - destra[2] * avanti[1],
            destra[2] * avanti[0] - destra[0] * avanti[2],
            destra[0] * avanti[1] - destra[1] * avanti[0],
        ];

        let mut anelli = [[0.0_f32; 4]; 6];
        let mut n_anelli = 0;
        if self.confronti {
            for (uscita, anello) in anelli.iter_mut().zip(self.anelli()) {
                *uscita = [anello.raggi as f32, anello.colore[0], anello.colore[1], anello.colore[2]];
                n_anelli += 1;
            }
        }
        let (sorgenti, colori_sorgenti, n_sorgenti) = self.sorgenti();

        Parametri {
            cam_pos: v4(pos, (1.0 - 1.0 / self.r_cam).sqrt()),
            cam_destra: v4(destra, 0.0),
            cam_su: v4(su, 0.0),
            cam_avanti: v4(avanti, 0.0),
            schermo: [0.0, 0.0, 0.5, self.tempo as f32],
            disco: [
                R_INTERNO_DISCO,
                R_ESTERNO_DISCO,
                f32::from(u8::from(self.disco)),
                f32::from(u8::from(self.doppler)),
            ],
            varie: [
                (2.0 * self.r_cam).max(200.0) as f32,
                n_anelli as f32,
                600.0,
                n_sorgenti as f32,
            ],
            scie: [f32::from(u8::from(self.scie)), 0.0, 0.0, 0.0],
            anelli,
            sorgenti,
            colori_sorgenti,
        }
    }
}

fn normalizza(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    v.map(|c| c / n)
}

fn v4(v: [f64; 3], w: f64) -> [f32; 4] {
    [v[0] as f32, v[1] as f32, v[2] as f32, w as f32]
}

/// Deve corrispondere a `Parametri` in `nero.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Parametri {
    pub cam_pos: [f32; 4],
    pub cam_destra: [f32; 4],
    pub cam_su: [f32; 4],
    pub cam_avanti: [f32; 4],
    pub schermo: [f32; 4],
    pub disco: [f32; 4],
    pub varie: [f32; 4],
    pub scie: [f32; 4],
    pub anelli: [[f32; 4]; 6],
    pub sorgenti: [[f32; 4]; MAX_SORGENTI],
    pub colori_sorgenti: [[f32; 4]; MAX_SORGENTI],
}
