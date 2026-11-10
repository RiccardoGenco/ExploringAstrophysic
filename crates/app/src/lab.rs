//! Il banco di prova: oggetti lanciati, tempo condiviso, esperimenti pronti.
//! Nessuna interfaccia qui: solo stato e regole del laboratorio.

use colossi_fisica::buco_nero::BucoNero;
use colossi_fisica::geodetica::{
    beta_circolare, Esito, Lancio, Legge, Moto, Orologio, Specie,
};
use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;
use std::path::PathBuf;

const MAX_OGGETTI: usize = 96;
const MAX_CAMPIONI: usize = 6000;
/// Passi massimi per oggetto e per fotogramma.
const BUDGET: usize = 40_000;
const PASSI_ANTEPRIMA: usize = 4000;
const RAGGI_FASCIO: i32 = 6;
const PASSO_FASCIO: f64 = 0.667;

/// Mappa delle scie nel piano, in coordinate log-polari, per il telescopio.
pub const MAPPA_L: usize = 2048;
pub const MAPPA_A: usize = 512;
const R_MAPPA: f64 = 1000.0;

const COLORE_LUCE: [u8; 3] = [255, 224, 140];
const COLORE_OROLOGIO: [u8; 3] = [245, 200, 90];
const COLORI_SONDE: [[u8; 3]; 6] = [
    [90, 215, 255],
    [120, 235, 150],
    [255, 130, 190],
    [175, 150, 255],
    [255, 160, 90],
    [130, 240, 225],
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Strumento {
    Seleziona,
    Luce,
    Sonda,
    Fascio,
    Orologio,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Grafico {
    RaggioLontano,
    RaggioProprio,
    Ritmo,
    Potenziale,
}

#[derive(Clone, Copy)]
pub struct Campione {
    pub p: [f32; 2],
    pub r: f64,
    pub s: f64,
    pub t: f64,
}

pub struct Oggetto {
    pub id: u32,
    pub nome: String,
    pub colore: [u8; 3],
    pub lancio: Lancio,
    /// Tenuto fermo da un motore: non segue una geodetica.
    pub fisso: bool,
    /// Istante del laboratorio in cui e' stato lanciato.
    pub t0: f64,
    pub moto: Moto,
    /// Lo stesso lancio con l'altra legge di gravita'.
    pub gemello: Option<Moto>,
    pub scia: Vec<Campione>,
    pub scia_gemello: Vec<[f32; 2]>,
    disegnati: usize,
}

impl Oggetto {
    pub fn stato_testo(&self) -> &'static str {
        if self.fisso {
            return "fermo";
        }
        match self.moto.esito {
            Esito::InVolo => "in volo",
            Esito::AllOrizzonte => "fermo sull'orizzonte",
            Esito::OltreOrizzonte => "oltre l'orizzonte",
            Esito::Singolarita => "al centro",
            Esito::Schiantato => "schiantato",
            Esito::Fuggito => "fuggito",
        }
    }
}

/// Esito previsto di un lancio, mostrato mentre lo si prepara.
pub struct Anteprima {
    pub punti: Vec<[f32; 2]>,
    pub gemello: Vec<[f32; 2]>,
    pub esito: &'static str,
}

pub struct VistaBanco {
    pub centro: [f64; 2],
    /// Meta' altezza visibile, in raggi di Schwarzschild.
    pub raggio: f64,
}

pub struct Mappa {
    pub pixel: Vec<u8>,
    pub sporca: bool,
}

impl Mappa {
    fn nuova() -> Self {
        Self { pixel: vec![0; MAPPA_L * MAPPA_A * 4], sporca: true }
    }

    fn svuota(&mut self) {
        self.pixel.fill(0);
        self.sporca = true;
    }

    fn cella(p: [f64; 2]) -> Option<(f64, f64)> {
        let r = p[0].hypot(p[1]);
        (r > 1.0 && r < R_MAPPA).then(|| {
            let u = (p[1].atan2(p[0]) / TAU + 0.5) * MAPPA_L as f64;
            let v = r.ln() / R_MAPPA.ln() * MAPPA_A as f64;
            (u, v)
        })
    }

    fn segmento(&mut self, a: [f64; 2], b: [f64; 2], colore: [u8; 3]) {
        let (Some((ua, va)), Some((ub, vb))) = (Self::cella(a), Self::cella(b)) else {
            return;
        };
        let larghezza = MAPPA_L as f64;
        let mut du = ub - ua;
        if du > larghezza / 2.0 {
            du -= larghezza;
        } else if du < -larghezza / 2.0 {
            du += larghezza;
        }
        let dv = vb - va;
        let passi = du.abs().max(dv.abs()).ceil().clamp(1.0, 256.0) as usize;
        for i in 0..=passi {
            let k = i as f64 / passi as f64;
            let x = (ua + du * k).rem_euclid(larghezza) as usize % MAPPA_L;
            let y = ((va + dv * k) as usize).min(MAPPA_A - 1);
            let indice = (y * MAPPA_L + x) * 4;
            self.pixel[indice..indice + 3].copy_from_slice(&colore);
            self.pixel[indice + 3] = 255;
        }
        self.sporca = true;
    }
}

pub struct Esperimento {
    pub nome: &'static str,
    pub nota: &'static str,
    prepara: fn(&mut Lab),
}

pub const ESPERIMENTI: &[Esperimento] = &[
    Esperimento {
        nome: "Un'orbita che ruota",
        nota: "L'ellisse non si chiude: leggi di quanto avanza a ogni giro, poi passa a Newton.",
        prepara: |l| {
            l.vista.raggio = 48.0;
            l.velocita = 160.0;
            l.sonda([40.0, 0.0], TAU / 4.0, 0.09);
        },
    },
    Esperimento {
        nome: "Luce in equilibrio",
        nota: "A 1,5 raggi la luce gira in tondo, ma basta l'errore di calcolo per farla uscire o cadere.",
        prepara: |l| {
            l.vista.raggio = 4.0;
            l.velocita = 3.0;
            l.luce([1.5, 0.0], TAU / 4.0);
            l.luce([1.52, 0.0], TAU / 4.0);
            l.luce([1.48, 0.0], TAU / 4.0);
        },
    },
    Esperimento {
        nome: "L'ultima orbita stabile",
        nota: "Stessa spinta, -2%. Sopra 3 raggi l'orbita regge, sotto precipita.",
        prepara: |l| {
            l.vista.raggio = 6.0;
            l.velocita = 18.0;
            for r in [3.6, 2.5] {
                l.sonda([r, 0.0], TAU / 4.0, beta_circolare(r, Legge::Einstein) * 0.98);
            }
        },
    },
    Esperimento {
        nome: "Fascio parallelo",
        nota: "Raggi identici, distanze diverse dal centro: guarda dove si separano i catturati dagli altri.",
        prepara: |l| {
            l.vista.raggio = 12.0;
            l.velocita = 8.0;
            l.fascio([-30.0, 0.0], 0.0);
        },
    },
    Esperimento {
        nome: "Einstein contro Newton",
        nota: "La linea tratteggiata e' lo stesso lancio con la gravita' di Newton.",
        prepara: |l| {
            l.vista.raggio = 30.0;
            l.velocita = 24.0;
            l.confronta = true;
            l.luce([-40.0, 8.0], 0.0);
            l.luce([-40.0, 5.0], 0.0);
            l.sonda([24.0, 0.0], TAU / 4.0, 0.12);
        },
    },
    Esperimento {
        nome: "Orologi a quote diverse",
        nota: "Tutti fermi, tutti precisi. Confronta la colonna del ritmo nel registro.",
        prepara: |l| {
            l.vista.raggio = 14.0;
            l.velocita = 10.0;
            for (i, r) in [1.05, 1.5, 3.0, 6.0, 12.0].into_iter().enumerate() {
                let a = i as f64 * TAU / 5.0;
                l.orologio([r * a.cos(), r * a.sin()]);
            }
        },
    },
    Esperimento {
        nome: "Gemelli: chi sale, chi resta",
        nota: "La sonda sale e ricade. Quando torna a 10 raggi, quale dei due orologi segna di piu'?",
        prepara: |l| {
            l.vista.raggio = 48.0;
            l.velocita = 60.0;
            l.orologio([10.0, 0.0]);
            l.sonda([10.0, 0.0], 0.0, 0.28);
        },
    },
    Esperimento {
        nome: "Caduta: due orologi",
        nota: "Da lontano si ferma sull'orizzonte. Passa all'orologio proprio e ripeti: arriva al centro.",
        prepara: |l| {
            l.vista.raggio = 13.0;
            l.velocita = 6.0;
            l.sonda([10.0, 0.0], 0.0, 0.0);
        },
    },
];

pub struct Lab {
    pub oggetti: Vec<Oggetto>,
    pub selezionato: Option<u32>,
    pub strumento: Strumento,
    pub grafico: Grafico,
    /// Tempo del laboratorio, in rs/c.
    pub t: f64,
    pub in_corso: bool,
    /// Quanti rs/c scorrono per ogni secondo reale.
    pub velocita: f64,
    pub orologio: Orologio,
    pub legge: Legge,
    pub confronta: bool,
    pub vista: VistaBanco,
    pub nota: Option<&'static str>,
    pub avviso: Option<String>,
    pub mappa: Mappa,
    rifare_mappa: bool,
    prossimo_id: u32,
    contatori: [u32; 3],
}

impl Default for Lab {
    fn default() -> Self {
        let mut lab = Self {
            oggetti: Vec::new(),
            selezionato: None,
            strumento: Strumento::Sonda,
            grafico: Grafico::RaggioLontano,
            t: 0.0,
            in_corso: true,
            velocita: 8.0,
            orologio: Orologio::Lontano,
            legge: Legge::Einstein,
            confronta: false,
            vista: VistaBanco { centro: [0.0, 0.0], raggio: 30.0 },
            nota: None,
            avviso: None,
            mappa: Mappa::nuova(),
            rifare_mappa: false,
            prossimo_id: 1,
            contatori: [0; 3],
        };
        lab.carica(0);
        lab
    }
}

fn punto(p: &[f64; 3]) -> [f64; 2] {
    [p[0], p[2]]
}

fn registra(scia: &mut Vec<Campione>, m: &Moto) {
    let p = punto(&m.x);
    let r = m.raggio();
    if let Some(ultimo) = scia.last() {
        let (dx, dz) = (p[0] - f64::from(ultimo.p[0]), p[1] - f64::from(ultimo.p[1]));
        let soglia = 0.008 * r;
        if dx * dx + dz * dz < soglia * soglia && !m.esito.fermo() {
            return;
        }
    }
    scia.push(Campione { p: [p[0] as f32, p[1] as f32], r, s: m.s, t: m.t });
}

/// Se la scia e' troppo lunga tiene un campione su due. Restituisce vero se l'ha ridotta.
fn sfoltisci(scia: &mut Vec<Campione>) -> bool {
    if scia.len() <= MAX_CAMPIONI {
        return false;
    }
    let mut indice = 0;
    scia.retain(|_| {
        indice += 1;
        indice % 2 == 1
    });
    true
}

impl Lab {
    pub fn indice_selezionato(&self) -> Option<usize> {
        let id = self.selezionato?;
        self.oggetti.iter().position(|o| o.id == id)
    }

    pub fn carica(&mut self, indice: usize) {
        self.svuota();
        self.legge = Legge::Einstein;
        self.orologio = Orologio::Lontano;
        self.confronta = false;
        self.vista.centro = [0.0, 0.0];
        let esperimento = &ESPERIMENTI[indice];
        (esperimento.prepara)(self);
        self.nota = Some(esperimento.nota);
        self.selezionato = self.oggetti.last().map(|o| o.id);
        self.in_corso = true;
        self.sincronizza();
    }

    pub fn svuota(&mut self) {
        self.oggetti.clear();
        self.selezionato = None;
        self.t = 0.0;
        self.nota = None;
        self.contatori = [0; 3];
        self.rifare_mappa = true;
    }

    /// Mette sul banco un'orbita misurata: parte dal punto piu' lontano.
    pub fn carica_orbita(&mut self, a: f64, e: f64) {
        self.svuota();
        self.legge = Legge::Einstein;
        self.orologio = Orologio::Lontano;
        self.confronta = false;
        self.vista.centro = [0.0, 0.0];
        let lontano = a * (1.0 + e);
        self.vista.raggio = lontano * 1.2;
        // Velocita' all'apoastro, con GM = 1/2.
        let beta = ((1.0 - e) / (1.0 + e) * 0.5 / a).sqrt();
        let periodo = TAU * (2.0 * a * a * a).sqrt();
        self.velocita = periodo / 20.0;
        let id = self.sonda([lontano, 0.0], TAU / 4.0, beta);
        self.selezionato = Some(id);
        self.nota = Some("L'orbita che hai misurato. Leggi di quanto avanza a ogni giro: lo strumento GRAVITY l'ha misurato nel 2020.");
        self.in_corso = true;
        self.sincronizza();
    }

    /// Riporta il tempo a zero e rilancia tutto dalle stesse condizioni.
    pub fn ricomincia(&mut self) {
        self.t = 0.0;
        for o in &mut self.oggetti {
            o.t0 = 0.0;
        }
        self.rilancia_tutti();
    }

    pub fn rilancia_tutti(&mut self) {
        let (legge, confronta) = (self.legge, self.confronta);
        for o in &mut self.oggetti {
            azzera(o, legge, confronta);
        }
        self.rifare_mappa = true;
        self.sincronizza();
    }

    pub fn modifica(&mut self, indice: usize, lancio: Lancio) {
        let (legge, confronta) = (self.legge, self.confronta);
        let o = &mut self.oggetti[indice];
        o.lancio = lancio;
        azzera(o, legge, confronta);
        self.rifare_mappa = true;
        self.sincronizza();
    }

    /// Rilancia un oggetto da adesso, senza toccare gli altri.
    pub fn rilancia(&mut self, indice: usize) {
        let (legge, confronta) = (self.legge, self.confronta);
        let t = self.t;
        let o = &mut self.oggetti[indice];
        o.t0 = t;
        azzera(o, legge, confronta);
        self.rifare_mappa = true;
        self.sincronizza();
    }

    pub fn elimina(&mut self, indice: usize) {
        self.oggetti.remove(indice);
        self.selezionato = None;
        self.rifare_mappa = true;
    }

    pub fn duplica(&mut self, indice: usize) {
        let (lancio, fisso) = (self.oggetti[indice].lancio, self.oggetti[indice].fisso);
        self.aggiungi(lancio, fisso);
    }

    pub fn aggiungi(&mut self, lancio: Lancio, fisso: bool) -> u32 {
        if self.oggetti.len() >= MAX_OGGETTI {
            self.oggetti.remove(0);
            self.rifare_mappa = true;
        }
        let categoria = match (fisso, lancio.specie) {
            (true, _) => 2,
            (false, Specie::Luce) => 0,
            (false, Specie::Materia) => 1,
        };
        self.contatori[categoria] += 1;
        let n = self.contatori[categoria];
        let (nome, colore) = match categoria {
            0 => (format!("Luce {n}"), COLORE_LUCE),
            1 => (format!("Sonda {n}"), COLORI_SONDE[(n as usize - 1) % COLORI_SONDE.len()]),
            _ => (format!("Orologio {n}"), COLORE_OROLOGIO),
        };
        let id = self.prossimo_id;
        self.prossimo_id += 1;
        let mut o = Oggetto {
            id,
            nome,
            colore,
            lancio,
            fisso,
            t0: self.t,
            moto: Moto::nuovo(&lancio, self.legge),
            gemello: None,
            scia: Vec::new(),
            scia_gemello: Vec::new(),
            disegnati: 0,
        };
        azzera(&mut o, self.legge, self.confronta);
        self.oggetti.push(o);
        id
    }

    pub fn luce(&mut self, posizione: [f64; 2], direzione: f64) -> u32 {
        self.aggiungi(Lancio { posizione, direzione, beta: 1.0, specie: Specie::Luce }, false)
    }

    pub fn sonda(&mut self, posizione: [f64; 2], direzione: f64, beta: f64) -> u32 {
        self.aggiungi(Lancio { posizione, direzione, beta, specie: Specie::Materia }, false)
    }

    pub fn orologio(&mut self, posizione: [f64; 2]) -> u32 {
        self.aggiungi(
            Lancio { posizione, direzione: 0.0, beta: 0.0, specie: Specie::Materia },
            true,
        )
    }

    /// Posizioni di partenza dei raggi di un fascio.
    pub fn posizioni_fascio(centro: [f64; 2], direzione: f64) -> Vec<[f64; 2]> {
        let normale = [-direzione.sin(), direzione.cos()];
        (-RAGGI_FASCIO..=RAGGI_FASCIO)
            .map(|i| {
                let d = f64::from(i) * PASSO_FASCIO;
                [centro[0] + normale[0] * d, centro[1] + normale[1] * d]
            })
            .filter(|p| p[0].hypot(p[1]) > 1.01)
            .collect()
    }

    pub fn fascio(&mut self, centro: [f64; 2], direzione: f64) {
        for p in Self::posizioni_fascio(centro, direzione) {
            self.luce(p, direzione);
        }
    }

    /// Calcola dove andrebbe un lancio, senza farlo.
    pub fn anteprima(&self, lancio: &Lancio, passi: Option<usize>) -> Anteprima {
        let passi = passi.unwrap_or(PASSI_ANTEPRIMA);
        let mut punti = vec![[lancio.posizione[0] as f32, lancio.posizione[1] as f32]];
        let mut moto = Moto::nuovo(lancio, self.legge);
        moto.avanza_passi(Orologio::Lontano, passi, |m| {
            punti.push([m.x[0] as f32, m.x[2] as f32]);
        });
        let mut gemello = Vec::new();
        if self.confronta {
            let mut altro = Moto::nuovo(lancio, altra_legge(self.legge));
            altro.avanza_passi(Orologio::Lontano, passi, |m| {
                gemello.push([m.x[0] as f32, m.x[2] as f32]);
            });
        }
        let legato = moto.energia() < if self.legge == Legge::Einstein { 1.0 } else { 0.0 };
        let esito = match moto.esito {
            Esito::AllOrizzonte | Esito::OltreOrizzonte | Esito::Singolarita => "cade dentro",
            Esito::Schiantato => "si schianta",
            Esito::Fuggito => "sfugge",
            Esito::InVolo if lancio.specie == Specie::Materia && legato => "resta in orbita",
            Esito::InVolo => "si allontana",
        };
        Anteprima { punti, gemello, esito }
    }

    pub fn avanza(&mut self, dt: f64) {
        if self.in_corso {
            self.t += self.velocita * dt;
        }
        self.sincronizza();
    }

    pub fn un_passo(&mut self) {
        self.in_corso = false;
        self.t += self.velocita / 30.0;
        self.sincronizza();
    }

    fn sincronizza(&mut self) {
        let (t, modo) = (self.t, self.orologio);
        for o in &mut self.oggetti {
            porta(o, modo, t - o.t0);
        }
        if self.rifare_mappa {
            self.rifare_mappa = false;
            self.mappa.svuota();
            for o in &mut self.oggetti {
                o.disegnati = 0;
            }
        }
        for o in &mut self.oggetti {
            let mappa = &mut self.mappa;
            for coppia in o.scia[o.disegnati.saturating_sub(1)..].windows(2) {
                let a = [f64::from(coppia[0].p[0]), f64::from(coppia[0].p[1])];
                let b = [f64::from(coppia[1].p[0]), f64::from(coppia[1].p[1])];
                mappa.segmento(a, b, o.colore);
            }
            o.disegnati = o.scia.len();
        }
    }

    /// Salva le misure di un oggetto in `misure/` e restituisce il percorso.
    pub fn esporta(&self, indice: usize, bh: &BucoNero) -> std::io::Result<PathBuf> {
        let o = &self.oggetti[indice];
        let mut testo = String::from("t_lontano_rs_c,tempo_proprio_rs_c,x_rs,z_rs,r_rs,t_lontano_s,tempo_proprio_s,r_m\n");
        let (unita, rs) = (bh.tempo_luce(), bh.raggio());
        for c in &o.scia {
            let _ = writeln!(
                testo,
                "{},{},{},{},{},{},{},{}",
                c.t, c.s, c.p[0], c.p[1], c.r, c.t * unita, c.s * unita, c.r * rs
            );
        }
        let cartella = std::env::current_dir()?.join("misure");
        std::fs::create_dir_all(&cartella)?;
        let percorso = cartella.join(format!("{}.csv", o.nome.replace(' ', "_")));
        std::fs::write(&percorso, testo)?;
        Ok(percorso)
    }
}

pub fn altra_legge(legge: Legge) -> Legge {
    match legge {
        Legge::Einstein => Legge::Newton,
        Legge::Newton => Legge::Einstein,
    }
}

fn azzera(o: &mut Oggetto, legge: Legge, confronta: bool) {
    o.moto = Moto::nuovo(&o.lancio, legge);
    o.gemello = (confronta && !o.fisso).then(|| Moto::nuovo(&o.lancio, altra_legge(legge)));
    o.scia.clear();
    o.scia_gemello.clear();
    registra(&mut o.scia, &o.moto);
    o.disegnati = 0;
}

fn porta(o: &mut Oggetto, modo: Orologio, bersaglio: f64) {
    if o.fisso {
        let ritmo = o.moto.ritmo().unwrap_or(1.0);
        match modo {
            Orologio::Lontano => {
                o.moto.t = bersaglio;
                o.moto.s = bersaglio * ritmo;
            }
            Orologio::Proprio => {
                o.moto.s = bersaglio;
                o.moto.t = bersaglio / ritmo;
            }
        }
        return;
    }
    let scia = &mut o.scia;
    o.moto.avanza(modo, bersaglio, BUDGET, |m| registra(scia, m));
    if sfoltisci(&mut o.scia) {
        o.disegnati /= 2;
    }
    if let Some(gemello) = &mut o.gemello {
        let scia = &mut o.scia_gemello;
        gemello.avanza(modo, bersaglio, BUDGET, |m| {
            let p = [m.x[0] as f32, m.x[2] as f32];
            let vicino = scia.last().is_some_and(|u: &[f32; 2]| {
                let (dx, dz) = (p[0] - u[0], p[1] - u[1]);
                let soglia = 0.008 * m.raggio() as f32;
                dx * dx + dz * dz < soglia * soglia
            });
            if !vicino {
                scia.push(p);
            }
        });
    }
}

/// Angolo riportato tra -180 e 180 gradi.
pub fn gradi_relativi(radianti: f64) -> f64 {
    let a = (radianti + PI).rem_euclid(TAU) - PI;
    a.to_degrees()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ogni_esperimento_si_carica_e_avanza() {
        for i in 0..ESPERIMENTI.len() {
            let mut lab = Lab::default();
            lab.carica(i);
            assert!(!lab.oggetti.is_empty(), "{}", ESPERIMENTI[i].nome);
            for _ in 0..30 {
                lab.avanza(1.0 / 30.0);
            }
            assert!(lab.oggetti.iter().all(|o| o.moto.raggio().is_finite()));
        }
    }

    #[test]
    fn i_gemelli_chi_sale_invecchia_di_piu() {
        let mut lab = Lab::default();
        lab.carica(6);
        lab.velocita = 1.0;
        // La sonda sale e torna a 10 raggi dopo aver superato l'apice.
        let mut risalita = false;
        loop {
            lab.avanza(20.0);
            let sonda = &lab.oggetti[1].moto;
            risalita |= sonda.r_punto() < 0.0;
            if risalita && sonda.raggio() <= 10.0 {
                break;
            }
            assert!(lab.t < 1e5);
        }
        let (fermo, sonda) = (&lab.oggetti[0].moto, &lab.oggetti[1].moto);
        assert!(sonda.s > fermo.s, "{} contro {}", sonda.s, fermo.s);
    }

    #[test]
    fn un_orbita_come_s2_avanza_di_circa_dodici_primi() {
        let mut lab = Lab::default();
        let (a, e) = (11_800.0, 0.88);
        lab.carica_orbita(a, e);
        let periodo = TAU * (2.0_f64 * a * a * a).sqrt();
        for _ in 0..120 {
            lab.velocita = 1.0;
            lab.avanza(periodo / 40.0);
        }
        let m = &lab.oggetti[0].moto;
        assert_eq!(m.esito, Esito::InVolo);
        let misurato = m.anticipo_periastro().expect("almeno due passaggi").to_degrees() * 60.0;
        let atteso = (3.0 * PI / (a * (1.0 - e * e))).to_degrees() * 60.0;
        assert!((misurato / atteso - 1.0).abs() < 0.05, "{misurato} contro {atteso}");
    }

    #[test]
    fn ricalcolare_un_lancio_non_cambia_il_risultato() {
        let mut lab = Lab::default();
        lab.carica(0);
        lab.avanza(1.0);
        let prima = lab.oggetti[0].moto.x;
        let lancio = lab.oggetti[0].lancio;
        lab.modifica(0, lancio);
        let dopo = lab.oggetti[0].moto.x;
        assert!((prima[0] - dopo[0]).abs() < 1e-9 && (prima[2] - dopo[2]).abs() < 1e-9);
    }

    #[test]
    fn la_mappa_copre_il_giro_senza_uscire() {
        let mut m = Mappa::nuova();
        m.segmento([-5.0, 0.001], [-5.0, -0.001], [255, 255, 255]);
        m.segmento([999.0, 0.0], [1.5, 0.0], [255, 255, 255]);
        assert!(m.pixel.iter().any(|&b| b > 0));
    }
}
