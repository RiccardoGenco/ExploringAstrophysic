//! Onde gravitazionali di una coppia che si fonde.
//!
//! Spirale: formula di quadrupolo all'ordine piu' basso. Fusione e assestamento:
//! raccordo fenomenologico con le formule approssimate della massa e rotazione finali.
//! Serve a percepire il fenomeno, non a misurare parametri.

use crate::costanti::{C, G, MASSA_SOLE};
use std::f64::consts::PI;

const MPC: f64 = 3.085_677_6e22;
/// Secondi corrispondenti a G M / c^3 per una massa solare.
const TEMPO_SOLE: f64 = G * MASSA_SOLE / (C * C * C);
/// Metri corrispondenti a G M / c^2 per una massa solare.
const LUNGHEZZA_SOLE: f64 = G * MASSA_SOLE / (C * C);
const LUMINOSITA_PLANCK: f64 = C * C * C * C * C / G;
const CAMPIONI_AL_SECONDO: f64 = 4096.0;
const MAX_CAMPIONI: usize = 200_000;
const FREQUENZA_INIZIALE: f64 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coppia {
    pub m1: f64,
    pub m2: f64,
    pub distanza_mpc: f64,
    /// Due stelle di neutroni: niente assestamento di un buco nero.
    pub neutroni: bool,
}

pub const GW150914: Coppia = Coppia { m1: 36.0, m2: 29.0, distanza_mpc: 410.0, neutroni: false };
pub const GW170817: Coppia = Coppia { m1: 1.46, m2: 1.27, distanza_mpc: 40.0, neutroni: true };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Campione {
    /// Secondi, zero alla fusione.
    pub t: f64,
    pub h: f64,
    pub f: f64,
    pub fase: f64,
}

impl Coppia {
    pub fn massa(&self) -> f64 {
        self.m1 + self.m2
    }

    fn eta(&self) -> f64 {
        self.m1 * self.m2 / self.massa().powi(2)
    }

    pub fn massa_chirp(&self) -> f64 {
        self.eta().powf(0.6) * self.massa()
    }

    /// Redshift stimato dalla distanza: stira il segnale che arriva a noi.
    fn redshift(&self) -> f64 {
        self.distanza_mpc * 67.4 / (C / 1e3)
    }

    /// Una massa come appare da lontano, allungata dall'espansione.
    fn osservata(&self, m: f64) -> f64 {
        m * (1.0 + self.redshift())
    }

    fn tempo_chirp(&self) -> f64 {
        self.osservata(self.massa_chirp()) * TEMPO_SOLE
    }

    /// Tempo che manca alla fusione quando l'onda ha frequenza `f`.
    pub fn tempo_alla_fusione(&self, f: f64) -> f64 {
        5.0 / 256.0 * self.tempo_chirp().powf(-5.0 / 3.0) * (PI * f).powf(-8.0 / 3.0)
    }

    pub fn frequenza(&self, tempo_alla_fusione: f64) -> f64 {
        (5.0 / (256.0 * tempo_alla_fusione)).powf(3.0 / 8.0) * self.tempo_chirp().powf(-5.0 / 8.0) / PI
    }

    /// Ampiezza del segnale a Terra alla frequenza `f`.
    pub fn ampiezza(&self, f: f64) -> f64 {
        let lunghezza = self.osservata(self.massa_chirp()) * LUNGHEZZA_SOLE;
        4.0 * lunghezza / (self.distanza_mpc * MPC)
            * (lunghezza / C * PI * f).powf(2.0 / 3.0)
    }

    /// Frequenza dell'onda all'ultima orbita stabile.
    pub fn frequenza_fine_spirale(&self) -> f64 {
        1.0 / (6.0_f64.powf(1.5) * PI * self.osservata(self.massa()) * TEMPO_SOLE)
    }

    /// Massa del buco nero finale, in masse solari.
    pub fn massa_finale(&self) -> f64 {
        let e = self.eta();
        self.massa() * (1.0 - (1.0 - (8.0_f64 / 9.0).sqrt()) * e - 0.498 * e * e)
    }

    pub fn rotazione_finale(&self) -> f64 {
        let e = self.eta();
        2.0 * 3.0_f64.sqrt() * e - 3.871 * e * e + 4.028 * e * e * e
    }

    /// Masse solari trasformate in onde.
    pub fn energia_irraggiata(&self) -> f64 {
        self.massa() - self.massa_finale()
    }

    /// Frequenza e tempo di smorzamento del buco nero appena nato.
    pub fn assestamento(&self) -> (f64, f64) {
        let a = self.rotazione_finale();
        let m = self.osservata(self.massa_finale()) * TEMPO_SOLE;
        let f = (1.0 - 0.63 * (1.0 - a).powf(0.3)) / (2.0 * PI * m);
        let qualita = 2.0 * (1.0 - a).powf(-0.45);
        (f, qualita / (PI * f))
    }

    /// Potenza dell'onda alla frequenza `f`, in watt (quadrupolo).
    pub fn potenza(&self, f: f64) -> f64 {
        let e = self.eta();
        32.0 / 5.0 * e * e * (PI * f * self.osservata(self.massa()) * TEMPO_SOLE).powf(10.0 / 3.0) * LUMINOSITA_PLANCK
    }

    /// Distanza tra i due corpi quando l'onda ha frequenza `f`, in km.
    pub fn separazione(&self, f: f64) -> f64 {
        let m = self.osservata(self.massa()) * LUNGHEZZA_SOLE;
        let omega = PI * f / C;
        (m / (omega * omega)).cbrt() / 1e3
    }

    /// Il segnale nel tempo, con al massimo `durata` secondi prima della fusione.
    pub fn segnale(&self, durata: f64) -> Vec<Campione> {
        let f_fine = self.frequenza_fine_spirale();
        let t_fine = self.tempo_alla_fusione(f_fine);
        let inizio = self.tempo_alla_fusione(FREQUENZA_INIZIALE).min(durata);
        let dt = (inizio / MAX_CAMPIONI as f64).max(1.0 / CAMPIONI_AL_SECONDO);

        let mut campioni = Vec::new();
        let (mut fase, mut t) = (0.0, -inizio);
        // Spirale.
        while -t > t_fine {
            let f = self.frequenza(-t);
            fase += 2.0 * PI * f * dt;
            campioni.push(Campione { t, h: self.ampiezza(f) * fase.cos(), f, fase });
            t += dt;
        }
        if self.neutroni {
            return campioni;
        }
        // Il picco e l'assestamento.
        let (f_a, tau) = self.assestamento();
        let a_fine = self.ampiezza(f_fine);
        let durata_raccordo = 3.0 / f_a;
        let picco = a_fine * 1.4;
        let t_raccordo = t;
        while t < t_raccordo + durata_raccordo {
            let k = (t - t_raccordo) / durata_raccordo;
            let liscio = k * k * (3.0 - 2.0 * k);
            let f = f_fine + (f_a - f_fine) * liscio;
            fase += 2.0 * PI * f * dt;
            let a = a_fine + (picco - a_fine) * liscio;
            campioni.push(Campione { t, h: a * fase.cos(), f, fase });
            t += dt;
        }
        let t_picco = t;
        while t < t_picco + 8.0 * tau {
            fase += 2.0 * PI * f_a * dt;
            campioni.push(Campione { t, h: picco * (-(t - t_picco) / tau).exp() * fase.cos(), f: f_a, fase });
            t += dt;
        }
        for c in &mut campioni {
            c.t -= t_picco;
        }
        campioni
    }

    pub fn ampiezza_massima(campioni: &[Campione]) -> f64 {
        campioni.iter().map(|c| c.h.abs()).fold(0.0, f64::max)
    }
}

/// Lo stiramento di uno dei bracci di LIGO (4 km) al picco, in metri.
pub fn stiramento_ligo(h: f64) -> f64 {
    0.5 * h * 4000.0
}

/// Il segnale riportato a campioni audio.
pub fn in_audio(campioni: &[Campione], frequenza: u32) -> Vec<f32> {
    if campioni.len() < 2 {
        return Vec::new();
    }
    let massimo = Coppia::ampiezza_massima(campioni).max(1e-40);
    let (t0, t1) = (campioni[0].t, campioni[campioni.len() - 1].t);
    let n = ((t1 - t0) * f64::from(frequenza)) as usize;
    let mut k = 0;
    (0..n)
        .map(|i| {
            let t = t0 + i as f64 / f64::from(frequenza);
            while k + 2 < campioni.len() && campioni[k + 1].t < t {
                k += 1;
            }
            let (a, b) = (campioni[k], campioni[k + 1]);
            let u = ((t - a.t) / (b.t - a.t)).clamp(0.0, 1.0);
            ((a.h + (b.h - a.h) * u) / massimo * 0.7) as f32
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    fn circa(a: f64, b: f64, tolleranza: f64) {
        assert!((a / b - 1.0).abs() < tolleranza, "{a} non e' circa {b}");
    }

    #[test]
    fn massa_chirp_di_gw150914() {
        circa(GW150914.massa_chirp(), 28.1, 0.03);
    }

    #[test]
    fn dura_circa_due_decimi_di_secondo_nella_banda_di_ligo() {
        let t = GW150914.tempo_alla_fusione(35.0);
        assert!(t > 0.12 && t < 0.3, "{t}");
    }

    #[test]
    fn gw170817_resta_nella_banda_per_quasi_un_minuto() {
        let t = GW170817.tempo_alla_fusione(30.0);
        assert!(t > 40.0 && t < 75.0, "{t}");
    }

    #[test]
    fn massa_e_rotazione_finali() {
        circa(GW150914.massa_finale(), 62.0, 0.02);
        circa(GW150914.energia_irraggiata(), 3.0, 0.15);
        circa(GW150914.rotazione_finale(), 0.68, 0.03);
    }

    #[test]
    fn suona_a_circa_250_hertz_alla_fine() {
        let (f, tau) = GW150914.assestamento();
        circa(f, 250.0, 0.1);
        assert!(tau > 1e-3 && tau < 1e-2, "{tau}");
    }

    #[test]
    fn l_ampiezza_e_quella_misurata() {
        let s = GW150914.segnale(0.3);
        let h = Coppia::ampiezza_massima(&s);
        assert!(h > 5e-22 && h < 2.5e-21, "{h}");
    }

    #[test]
    fn la_frequenza_sale_e_il_segnale_e_continuo() {
        let s = GW150914.segnale(0.3);
        assert!(s.windows(2).all(|c| c[1].f >= c[0].f - 1e-6 || c[1].t > 0.0));
        assert!(s.iter().any(|c| c.t > 0.0), "deve esserci l'assestamento");
    }

    #[test]
    fn inversione_tra_tempo_e_frequenza() {
        let c = GW150914;
        circa(c.frequenza(c.tempo_alla_fusione(70.0)), 70.0, 1e-9);
    }

    #[test]
    fn l_audio_non_satura() {
        let a = in_audio(&GW150914.segnale(0.3), 44_100);
        assert!(a.len() > 1000 && a.iter().all(|x| x.abs() <= 0.71));
    }
}
