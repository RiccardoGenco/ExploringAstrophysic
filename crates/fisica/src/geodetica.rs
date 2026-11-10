//! Moto libero di luce e materia attorno a un buco nero non rotante.
//!
//! Unita': rs = 1, c = 1, quindi GM = 1/2. In coordinate cartesiane costruite su
//! (r, theta, phi) le equazioni sono
//!   materia: x'' = -x / (2 r^3) - 3/2 L^2 x / r^5   (derivate nel tempo proprio)
//!   luce:    x'' =              - 3/2 L^2 x / r^5   (parametro affine)
//! con L = |x x x'| costante. Il tempo di chi guarda da lontano si ottiene da
//! dt/ds = E / (1 - 1/r). La legge di Newton toglie il termine in L^2.
//!
//! Il moto avviene nel piano y = 0, lo stesso del disco nel telescopio.

use crate::luce::{norma, punto, vettoriale, V3};
use std::f64::consts::TAU;

/// Oltre questa distanza, in allontanamento, l'oggetto e' considerato perso.
pub const R_FUGA: f64 = 200_000.0;
/// Distanza dall'orizzonte a cui un oggetto, visto da lontano, si considera fermo.
pub const MARGINE_ORIZZONTE: f64 = 1e-4;
const R_SINGOLARITA: f64 = 0.02;
const PASSO: f64 = 0.02;
const F_MINIMA: f64 = 1e-5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Specie {
    Luce,
    Materia,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Legge {
    Einstein,
    Newton,
}

/// Quale orologio comanda l'avanzamento.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orologio {
    /// Tempo di chi guarda da lontano: nessuno attraversa mai l'orizzonte.
    Lontano,
    /// Tempo di ciascun oggetto: si puo' entrare e arrivare al centro.
    Proprio,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Esito {
    InVolo,
    /// Visto da lontano, si e' fermato sull'orizzonte.
    AllOrizzonte,
    /// Ha attraversato l'orizzonte e prosegue verso il centro.
    OltreOrizzonte,
    Singolarita,
    /// Con la legge di Newton non c'e' orizzonte: ha urtato il corpo centrale.
    Schiantato,
    Fuggito,
}

impl Esito {
    pub fn fermo(self) -> bool {
        !matches!(self, Esito::InVolo | Esito::OltreOrizzonte)
    }
}

/// Condizioni iniziali, misurate da un osservatore fermo nel punto di partenza.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lancio {
    /// Coordinate (x, z) nel piano, in raggi di Schwarzschild.
    pub posizione: [f64; 2],
    /// Direzione nel piano, in radianti: atan2(z, x).
    pub direzione: f64,
    /// Velocita' locale in frazioni di c. Ignorata per la luce.
    pub beta: f64,
    pub specie: Specie,
}

impl Lancio {
    pub fn raggio(&self) -> f64 {
        self.posizione[0].hypot(self.posizione[1])
    }

    pub fn azimut(&self) -> f64 {
        self.posizione[1].atan2(self.posizione[0])
    }

    /// Direzione perpendicolare al raggio, nel verso degli angoli crescenti.
    pub fn direzione_tangente(&self) -> f64 {
        self.azimut() + TAU / 4.0
    }
}

/// Velocita' locale di un'orbita circolare a distanza `r`. Oltre 1 non esiste.
pub fn beta_circolare(r: f64, legge: Legge) -> f64 {
    match legge {
        Legge::Einstein if r > 1.0 => (0.5 / (r - 1.0)).sqrt().min(1.0),
        Legge::Einstein => 1.0,
        Legge::Newton => (0.5 / r).sqrt(),
    }
}

/// Velocita' locale necessaria per sfuggire da `r` partendo verso l'esterno.
pub fn beta_fuga(r: f64, legge: Legge) -> f64 {
    match legge {
        Legge::Einstein => (1.0 / r).sqrt().min(1.0),
        Legge::Newton => (1.0 / r).sqrt(),
    }
}

/// Potenziale efficace: per Einstein V^2, confrontabile con E^2; per Newton
/// l'energia per unita' di massa, confrontabile con E.
pub fn potenziale_efficace(r: f64, l2: f64, specie: Specie, legge: Legge) -> f64 {
    match (legge, specie) {
        (Legge::Einstein, Specie::Materia) => (1.0 - 1.0 / r) * (1.0 + l2 / (r * r)),
        (Legge::Einstein, Specie::Luce) => (1.0 - 1.0 / r) * l2 / (r * r),
        (Legge::Newton, _) => -0.5 / r + 0.5 * l2 / (r * r),
    }
}

fn piu(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn per(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}

#[derive(Clone, Debug)]
pub struct Moto {
    pub specie: Specie,
    pub legge: Legge,
    pub x: V3,
    pub v: V3,
    /// Tempo proprio per la materia, parametro affine per la luce.
    pub s: f64,
    /// Tempo di chi guarda da lontano.
    pub t: f64,
    pub esito: Esito,
    /// Angolo percorso, senza riavvolgerlo a ogni giro.
    pub angolo: f64,
    /// Valori di `angolo` a ogni passaggio per il punto piu' vicino al centro.
    pub periastri: Vec<f64>,
    l2: f64,
    energia: f64,
    v0: V3,
}

impl Moto {
    pub fn nuovo(lancio: &Lancio, legge: Legge) -> Self {
        let r = lancio.raggio().max(1.0 + 10.0 * MARGINE_ORIZZONTE);
        let verso = lancio.raggio().max(1e-9);
        let x = [
            lancio.posizione[0] / verso * r,
            0.0,
            lancio.posizione[1] / verso * r,
        ];
        let locale = [lancio.direzione.cos(), 0.0, lancio.direzione.sin()];
        let beta = match lancio.specie {
            Specie::Luce => 1.0,
            Specie::Materia => lancio.beta.clamp(0.0, 0.9999),
        };

        let radiale = per(x, 1.0 / r);
        let vr = punto(locale, radiale);
        let tangenziale = piu(locale, per(radiale, -vr));
        let radice = (1.0 - 1.0 / r).sqrt();
        let (v, energia) = match legge {
            Legge::Einstein => {
                let spinta = match lancio.specie {
                    Specie::Luce => 1.0,
                    Specie::Materia => beta / (1.0 - beta * beta).sqrt(),
                };
                let v = per(piu(per(radiale, vr * radice), tangenziale), spinta);
                let energia = match lancio.specie {
                    Specie::Luce => radice,
                    Specie::Materia => radice / (1.0 - beta * beta).sqrt(),
                };
                (v, energia)
            }
            Legge::Newton => (per(locale, beta), 0.5 * beta * beta - 0.5 / r),
        };

        let h = vettoriale(x, v);
        Self {
            specie: lancio.specie,
            legge,
            x,
            v,
            s: 0.0,
            t: 0.0,
            esito: Esito::InVolo,
            angolo: x[2].atan2(x[0]),
            periastri: Vec::new(),
            l2: punto(h, h),
            energia,
            v0: v,
        }
    }

    pub fn raggio(&self) -> f64 {
        norma(self.x)
    }

    pub fn l2(&self) -> f64 {
        self.l2
    }

    pub fn momento_angolare(&self) -> f64 {
        self.l2.sqrt()
    }

    /// Energia conservata per unita' di massa (per la luce, per unita' di E).
    pub fn energia(&self) -> f64 {
        self.energia
    }

    /// Velocita' radiale coordinata.
    pub fn r_punto(&self) -> f64 {
        punto(self.x, self.v) / self.raggio()
    }

    /// Scarto relativo tra l'energia ricalcolata ora e quella di partenza: misura
    /// quanto l'integrazione numerica sta sbagliando.
    pub fn errore_energia(&self) -> f64 {
        let r = self.raggio();
        let ora = match self.legge {
            Legge::Einstein => {
                let dr = self.r_punto();
                let v2 = dr * dr + potenziale_efficace(r, self.l2, self.specie, self.legge);
                v2.max(0.0).sqrt()
            }
            Legge::Newton => 0.5 * punto(self.v, self.v) - 0.5 / r,
        };
        (ora / self.energia - 1.0).abs()
    }

    /// Velocita' misurata da un osservatore fermo qui, in frazioni di c.
    /// `None` dove un osservatore fermo non puo' esistere.
    pub fn beta(&self) -> Option<f64> {
        match (self.legge, self.specie) {
            (Legge::Newton, _) => Some(norma(self.v)),
            (Legge::Einstein, Specie::Luce) => Some(1.0),
            (Legge::Einstein, Specie::Materia) => {
                let f = 1.0 - 1.0 / self.raggio();
                (f > 0.0).then(|| {
                    let gamma = self.energia / f.sqrt();
                    (1.0 - 1.0 / (gamma * gamma).max(1.0)).sqrt()
                })
            }
        }
    }

    /// Ritmo del proprio orologio rispetto a quello lontano, dtau/dt.
    pub fn ritmo(&self) -> Option<f64> {
        match (self.legge, self.specie) {
            (Legge::Newton, _) => Some(1.0),
            (Legge::Einstein, Specie::Luce) => None,
            (Legge::Einstein, Specie::Materia) => {
                let f = 1.0 - 1.0 / self.raggio();
                (f > 0.0).then_some(f / self.energia)
            }
        }
    }

    /// Angolo tra la direzione attuale e quella di partenza.
    pub fn deviazione(&self) -> f64 {
        let n = norma(self.v) * norma(self.v0);
        if n > 0.0 {
            (punto(self.v, self.v0) / n).clamp(-1.0, 1.0).acos()
        } else {
            0.0
        }
    }

    /// Quanto avanza il punto piu' vicino al centro a ogni giro, oltre i 360 gradi.
    pub fn anticipo_periastro(&self) -> Option<f64> {
        let n = self.periastri.len();
        (n >= 2).then(|| {
            let medio = (self.periastri[n - 1] - self.periastri[0]) / (n - 1) as f64;
            medio.abs() - TAU
        })
    }

    fn accelerazione(&self, x: V3) -> V3 {
        let r2 = punto(x, x);
        let r = r2.sqrt();
        let newton = per(x, -0.5 / (r2 * r));
        let einstein = per(x, -1.5 * self.l2 / (r2 * r2 * r));
        match (self.legge, self.specie) {
            (Legge::Newton, _) => newton,
            (Legge::Einstein, Specie::Luce) => einstein,
            (Legge::Einstein, Specie::Materia) => piu(newton, einstein),
        }
    }

    fn dt_ds(&self, x: V3) -> f64 {
        match self.legge {
            Legge::Newton => 1.0,
            Legge::Einstein if self.esito == Esito::OltreOrizzonte => 0.0,
            Legge::Einstein => self.energia / (1.0 - 1.0 / norma(x)).max(F_MINIMA),
        }
    }

    fn passo_naturale(&self) -> f64 {
        let r = self.raggio();
        let v = norma(self.v).max(1e-12);
        let cinematico = PASSO * r / v;
        match self.specie {
            Specie::Luce => cinematico,
            Specie::Materia => cinematico.min(PASSO * r * r.sqrt()),
        }
    }

    fn integra(&mut self, h: f64) {
        let (x, v) = (self.x, self.v);
        let k1x = v;
        let k1v = self.accelerazione(x);
        let x2 = piu(x, per(k1x, h / 2.0));
        let k2x = piu(v, per(k1v, h / 2.0));
        let k2v = self.accelerazione(x2);
        let x3 = piu(x, per(k2x, h / 2.0));
        let k3x = piu(v, per(k2v, h / 2.0));
        let k3v = self.accelerazione(x3);
        let x4 = piu(x, per(k3x, h));
        let k4x = piu(v, per(k3v, h));
        let k4v = self.accelerazione(x4);

        let dt = self.dt_ds(x) + 2.0 * self.dt_ds(x2) + 2.0 * self.dt_ds(x3) + self.dt_ds(x4);
        self.t += dt * h / 6.0;
        self.s += h;

        let dx = piu(piu(k1x, per(piu(k2x, k3x), 2.0)), k4x);
        let dv = piu(piu(k1v, per(piu(k2v, k3v), 2.0)), k4v);
        self.x = piu(x, per(dx, h / 6.0));
        self.v = piu(v, per(dv, h / 6.0));
    }

    fn classifica(&mut self, modo: Orologio) {
        let r = self.raggio();
        if r >= R_FUGA && self.r_punto() > 0.0 {
            self.esito = Esito::Fuggito;
            return;
        }
        match self.legge {
            Legge::Newton => {
                if r <= 1.0 {
                    self.esito = Esito::Schiantato;
                }
            }
            Legge::Einstein => match modo {
                Orologio::Lontano => {
                    if r <= 1.0 + MARGINE_ORIZZONTE {
                        self.x = per(self.x, (1.0 + MARGINE_ORIZZONTE) / r);
                        self.esito = Esito::AllOrizzonte;
                    }
                }
                Orologio::Proprio => {
                    if r <= R_SINGOLARITA {
                        self.esito = Esito::Singolarita;
                    } else if r <= 1.0 {
                        self.esito = Esito::OltreOrizzonte;
                    }
                }
            },
        }
    }

    fn un_passo(&mut self, modo: Orologio, massimo: f64) {
        let mut h = self.passo_naturale();
        match modo {
            Orologio::Lontano => {
                let dt = self.dt_ds(self.x);
                if dt > 0.0 {
                    h = h.min(massimo / dt);
                }
            }
            Orologio::Proprio => h = h.min(massimo),
        }

        let prima_dr = self.r_punto();
        let prima_angolo = self.angolo;
        let prima = self.x;
        self.integra(h);

        let incremento = (prima[0] * self.x[2] - prima[2] * self.x[0])
            .atan2(prima[0] * self.x[0] + prima[2] * self.x[2]);
        self.angolo += incremento;

        let dr = self.r_punto();
        if prima_dr < 0.0 && dr >= 0.0 {
            let frazione = -prima_dr / (dr - prima_dr);
            self.periastri.push(prima_angolo + incremento * frazione);
        }
        self.classifica(modo);
    }

    /// Avanza finche' l'orologio scelto segna `bersaglio`, per al piu' `budget`
    /// passi. `ogni_passo` viene chiamata dopo ciascun passo.
    pub fn avanza(
        &mut self,
        modo: Orologio,
        bersaglio: f64,
        budget: usize,
        mut ogni_passo: impl FnMut(&Moto),
    ) {
        for _ in 0..budget {
            if self.esito.fermo() {
                return;
            }
            let attuale = match modo {
                Orologio::Lontano => self.t,
                Orologio::Proprio => self.s,
            };
            let resto = bersaglio - attuale;
            if resto <= 1e-9 {
                return;
            }
            self.un_passo(modo, resto);
            ogni_passo(self);
        }
    }

    /// Avanza di un numero fissato di passi naturali, senza bersaglio.
    pub fn avanza_passi(&mut self, modo: Orologio, passi: usize, mut ogni_passo: impl FnMut(&Moto)) {
        for _ in 0..passi {
            if self.esito.fermo() {
                return;
            }
            self.un_passo(modo, f64::INFINITY);
            ogni_passo(self);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::caduta::Caduta;

    const CAPIENTE: usize = 5_000_000;

    fn sonda(r: f64, beta: f64, direzione: f64) -> Lancio {
        Lancio {
            posizione: [r, 0.0],
            direzione,
            beta,
            specie: Specie::Materia,
        }
    }

    fn circolare(r: f64) -> Lancio {
        sonda(r, beta_circolare(r, Legge::Einstein), TAU / 4.0)
    }

    #[test]
    fn la_velocita_locale_torna_quella_lanciata() {
        let m = Moto::nuovo(&sonda(9.0, 0.37, 1.1), Legge::Einstein);
        assert!((m.beta().unwrap() - 0.37).abs() < 1e-12);
    }

    #[test]
    fn orbita_circolare_resta_circolare_col_periodo_giusto() {
        let r = 6.0;
        let mut m = Moto::nuovo(&circolare(r), Legge::Einstein);
        let omega = (0.5 / (r * r * r)).sqrt();
        let durata = 5.0 * TAU / omega;
        m.avanza(Orologio::Lontano, durata, CAPIENTE, |_| {});
        assert!((m.raggio() / r - 1.0).abs() < 1e-5, "r = {}", m.raggio());
        assert!((m.angolo - 5.0 * TAU).abs() < 1e-3, "angolo = {}", m.angolo);
    }

    #[test]
    fn orologio_in_orbita_circolare() {
        let r = 10.0;
        let m = Moto::nuovo(&circolare(r), Legge::Einstein);
        let atteso = (1.0_f64 - 1.5 / r).sqrt();
        assert!((m.ritmo().unwrap() - atteso).abs() < 1e-12);
    }

    #[test]
    fn caduta_libera_coincide_con_la_soluzione_esatta() {
        let c = Caduta::new(10.0);
        let eta = 2.0;
        let mut m = Moto::nuovo(&sonda(10.0, 0.0, 0.0), Legge::Einstein);
        m.avanza(Orologio::Proprio, c.tempo_proprio(eta), CAPIENTE, |_| {});
        assert!((m.raggio() / c.raggio(eta) - 1.0).abs() < 1e-6);
        assert!((m.t / c.tempo_lontano(eta) - 1.0).abs() < 1e-4, "{} {}", m.t, c.tempo_lontano(eta));
    }

    #[test]
    fn da_lontano_si_ferma_sull_orizzonte_da_dentro_arriva_al_centro() {
        let lancio = sonda(10.0, 0.0, 0.0);
        let mut lontano = Moto::nuovo(&lancio, Legge::Einstein);
        lontano.avanza(Orologio::Lontano, 1e5, CAPIENTE, |_| {});
        assert_eq!(lontano.esito, Esito::AllOrizzonte);

        let mut proprio = Moto::nuovo(&lancio, Legge::Einstein);
        proprio.avanza(Orologio::Proprio, 1e5, CAPIENTE, |_| {});
        assert_eq!(proprio.esito, Esito::Singolarita);
        let totale = Caduta::new(10.0).tempo_proprio_totale();
        assert!((proprio.s / totale - 1.0).abs() < 1e-3, "{} {totale}", proprio.s);
    }

    #[test]
    fn anticipo_del_periastro_come_la_formula_di_einstein() {
        // Con GM = 1/2: 3 pi / p, con p = 2 L^2 al primo ordine.
        let mut m = Moto::nuovo(&sonda(1000.0, 0.015, TAU / 4.0), Legge::Einstein);
        let p = 2.0 * m.l2();
        m.avanza(Orologio::Lontano, 2e6, CAPIENTE, |_| {});
        assert!(m.periastri.len() >= 3, "{}", m.periastri.len());
        let misurato = m.anticipo_periastro().unwrap();
        let atteso = 3.0 * std::f64::consts::PI / p;
        assert!((misurato / atteso - 1.0).abs() < 0.03, "{misurato} contro {atteso}");
    }

    #[test]
    fn con_newton_l_ellisse_si_chiude() {
        let mut m = Moto::nuovo(&sonda(1000.0, 0.015, TAU / 4.0), Legge::Newton);
        m.avanza(Orologio::Lontano, 2e6, CAPIENTE, |_| {});
        assert!(m.periastri.len() >= 3);
        assert!(m.anticipo_periastro().unwrap().abs() < 1e-4);
    }

    fn deviazione_luce(legge: Legge) -> f64 {
        let b = 400.0;
        let lancio = Lancio {
            posizione: [-10_000.0, b],
            direzione: 0.0,
            beta: 1.0,
            specie: Specie::Luce,
        };
        let mut m = Moto::nuovo(&lancio, legge);
        m.avanza(Orologio::Lontano, 1e6, CAPIENTE, |_| {});
        assert_eq!(m.esito, Esito::Fuggito);
        m.deviazione()
    }

    #[test]
    fn la_luce_devia_il_doppio_che_con_newton() {
        let einstein = deviazione_luce(Legge::Einstein);
        let newton = deviazione_luce(Legge::Newton);
        assert!((einstein / (2.0 / 400.0) - 1.0).abs() < 0.01, "{einstein}");
        assert!((newton / (1.0 / 400.0) - 1.0).abs() < 0.01, "{newton}");
    }

    #[test]
    fn la_luce_gira_sulla_sfera_dei_fotoni() {
        let lancio = Lancio {
            posizione: [1.5, 0.0],
            direzione: TAU / 4.0,
            beta: 1.0,
            specie: Specie::Luce,
        };
        let mut m = Moto::nuovo(&lancio, Legge::Einstein);
        m.avanza_passi(Orologio::Lontano, 400, |_| {});
        assert!(m.angolo > TAU, "{}", m.angolo);
        assert!((m.raggio() - 1.5).abs() < 1e-3, "{}", m.raggio());
    }

    #[test]
    fn l_energia_si_conserva_su_un_orbita_stretta() {
        let mut m = Moto::nuovo(&sonda(12.0, 0.2, TAU / 4.0), Legge::Einstein);
        m.avanza(Orologio::Lontano, 3000.0, CAPIENTE, |_| {});
        assert_eq!(m.esito, Esito::InVolo);
        assert!(m.errore_energia() < 1e-5, "{}", m.errore_energia());
    }
}
