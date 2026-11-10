//! Come cresce un buco nero: limite di Eddington e tempo cosmico.

use crate::costanti::{ANNO, C, G};
use std::f64::consts::PI;

const THOMSON: f64 = 6.652_458_7e-29;
const MASSA_PROTONE: f64 = 1.672_621_9e-27;
const LUMINOSITA_SOLE: f64 = 3.828e26;
const MASSA_SOLE_KG: f64 = crate::costanti::MASSA_SOLE;
const MILIONE_DI_ANNI: f64 = 1e6 * ANNO;

/// Parametri cosmologici di Planck 2018, universo piatto.
const H0: f64 = 67.4;
const OMEGA_M: f64 = 0.315;
const OMEGA_L: f64 = 1.0 - OMEGA_M;
const MPC: f64 = 3.085_677_6e22;

fn h0_si() -> f64 {
    H0 * 1e3 / MPC
}

/// Eta' dell'universo a redshift `z`, in milioni di anni.
pub fn tempo_cosmico(z: f64) -> f64 {
    let x = (OMEGA_L / OMEGA_M).sqrt() * (1.0 + z).powf(-1.5);
    2.0 / (3.0 * h0_si() * OMEGA_L.sqrt()) * x.asinh() / MILIONE_DI_ANNI
}

/// Il redshift a cui l'universo aveva `t` milioni di anni.
pub fn redshift(t: f64) -> f64 {
    let s = (1.5 * h0_si() * OMEGA_L.sqrt() * t * MILIONE_DI_ANNI).sinh();
    ((OMEGA_L / OMEGA_M).sqrt() / s).powf(2.0 / 3.0) - 1.0
}

/// Ritmo di crescita, descritto da efficienza radiativa e rapporto di Eddington.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ritmo {
    /// Frazione della massa ingoiata che diventa luce.
    pub efficienza: f64,
    /// Luminosita' in unita' di quella di Eddington.
    pub eddington: f64,
}

/// Tempo di Eddington, in milioni di anni.
pub fn tempo_eddington() -> f64 {
    THOMSON * C / (4.0 * PI * G * MASSA_PROTONE) / MILIONE_DI_ANNI
}

impl Ritmo {
    /// Tempo in cui la massa si moltiplica per e, in milioni di anni.
    pub fn tempo_e(&self) -> f64 {
        tempo_eddington() * self.efficienza / (1.0 - self.efficienza) / self.eddington
    }

    pub fn massa_dopo(&self, iniziale: f64, milioni_di_anni: f64) -> f64 {
        iniziale * (milioni_di_anni / self.tempo_e()).exp()
    }

    /// Il seme che servirebbe per arrivare a `finale` in quel tempo.
    pub fn seme_necessario(&self, finale: f64, milioni_di_anni: f64) -> f64 {
        finale * (-milioni_di_anni / self.tempo_e()).exp()
    }

    /// Luminosita' in unita' di quella del Sole.
    pub fn luminosita_soli(&self, masse: f64) -> f64 {
        let eddington = 4.0 * PI * G * masse * MASSA_SOLE_KG * MASSA_PROTONE * C / THOMSON;
        self.eddington * eddington / LUMINOSITA_SOLE
    }

    /// Masse solari ingoiate ogni anno.
    pub fn soli_all_anno(&self, masse: f64) -> f64 {
        self.luminosita_soli(masse) * LUMINOSITA_SOLE / (self.efficienza * C * C) * ANNO / MASSA_SOLE_KG
    }
}

/// Un buco nero lontano misurato davvero.
pub struct Quasar {
    pub nome: &'static str,
    pub fonte: &'static str,
    pub z: f64,
    pub massa: f64,
    pub errore: f64,
}

pub const QUASAR: [Quasar; 3] = [
    Quasar { nome: "J0313-1806", fonte: "Wang et al. 2021", z: 7.642, massa: 1.6e9, errore: 0.4e9 },
    Quasar { nome: "J1120+0641", fonte: "Mortlock et al. 2011", z: 7.085, massa: 2.0e9, errore: 0.0 },
    Quasar { nome: "J0100+2802", fonte: "Wu et al. 2015", z: 6.30, massa: 1.24e10, errore: 0.19e10 },
];

#[cfg(test)]
mod test {
    use super::*;

    fn circa(a: f64, b: f64, tolleranza: f64) {
        assert!((a / b - 1.0).abs() < tolleranza, "{a} non e' circa {b}");
    }

    #[test]
    fn eta_dell_universo() {
        circa(tempo_cosmico(0.0), 13_790.0, 0.01);
    }

    #[test]
    fn eta_alle_distanze_pubblicate() {
        // Dagli articoli: circa 670 e 770 milioni di anni dopo il Big Bang.
        circa(tempo_cosmico(7.642), 670.0, 0.03);
        circa(tempo_cosmico(7.085), 770.0, 0.05);
    }

    #[test]
    fn redshift_e_tempo_sono_inversi() {
        for z in [0.5, 2.0, 7.6, 30.0] {
            circa(redshift(tempo_cosmico(z)), z, 1e-9);
        }
    }

    #[test]
    fn tempo_di_salpeter() {
        circa(tempo_eddington(), 451.0, 0.01);
        let r = Ritmo { efficienza: 0.1, eddington: 1.0 };
        circa(r.tempo_e(), 50.1, 0.01);
    }

    #[test]
    fn crescita_e_seme_sono_inversi() {
        let r = Ritmo { efficienza: 0.1, eddington: 0.8 };
        circa(r.seme_necessario(r.massa_dopo(1e3, 400.0), 400.0), 1e3, 1e-12);
    }

    #[test]
    fn luminosita_di_eddington() {
        let r = Ritmo { efficienza: 0.1, eddington: 1.0 };
        circa(r.luminosita_soli(1.0), 3.29e4, 0.02);
    }

    #[test]
    fn un_seme_stellare_non_arriva_in_tempo_al_quasar_piu_lontano() {
        let r = Ritmo { efficienza: 0.1, eddington: 1.0 };
        let q = &QUASAR[0];
        let durata = tempo_cosmico(q.z) - tempo_cosmico(30.0);
        assert!(r.massa_dopo(100.0, durata) < q.massa);
    }
}
