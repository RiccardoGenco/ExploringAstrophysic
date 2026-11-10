//! Grandezze di un buco nero non rotante (Schwarzschild).

use crate::costanti::{C, G, G_TERRA, MASSA_SOLE};
use std::f64::consts::PI;

/// Altezza convenzionale di una persona, in metri.
pub const ALTEZZA_PERSONA: f64 = 1.8;
/// Soglia didattica di letalita': differenza di accelerazione tra testa e piedi.
pub const SOGLIA_MAREA: f64 = 10.0 * G_TERRA;

/// Raggio della sfera dei fotoni, in raggi di Schwarzschild.
pub const SFERA_FOTONI: f64 = 1.5;
/// Orbita circolare stabile piu' interna, in raggi di Schwarzschild.
pub const ISCO: f64 = 3.0;

/// Parametro d'impatto critico, in raggi di Schwarzschild: 3*sqrt(3)/2.
pub fn impatto_critico() -> f64 {
    1.5 * 3.0_f64.sqrt()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BucoNero {
    pub massa_kg: f64,
}

impl BucoNero {
    pub fn da_masse_solari(masse: f64) -> Self {
        Self {
            massa_kg: masse * MASSA_SOLE,
        }
    }

    pub fn masse_solari(&self) -> f64 {
        self.massa_kg / MASSA_SOLE
    }

    /// Raggio dell'orizzonte in metri.
    pub fn raggio(&self) -> f64 {
        2.0 * G * self.massa_kg / (C * C)
    }

    /// Massa divisa per il volume euclideo racchiuso dall'orizzonte, in kg/m^3.
    pub fn densita_media(&self) -> f64 {
        let r = self.raggio();
        self.massa_kg / (4.0 / 3.0 * PI * r * r * r)
    }

    /// Differenza di accelerazione radiale tra due punti distanti `lunghezza`
    /// metri, a `r` metri dal centro. Vale anche per chi cade, in Schwarzschild.
    pub fn marea(&self, r: f64, lunghezza: f64) -> f64 {
        self.raggio() * C * C * lunghezza / (r * r * r)
    }

    /// Distanza dal centro, in metri, a cui la marea raggiunge `soglia` su `lunghezza`.
    pub fn raggio_marea(&self, lunghezza: f64, soglia: f64) -> f64 {
        (self.raggio() * C * C * lunghezza / soglia).cbrt()
    }

    /// Vero se una persona attraversa l'orizzonte sotto la soglia di letalita'.
    pub fn si_sopravvive_all_orizzonte(&self) -> bool {
        self.raggio_marea(ALTEZZA_PERSONA, SOGLIA_MAREA) < self.raggio()
    }

    /// Tempo che la luce impiega a percorrere un raggio dell'orizzonte, in secondi.
    pub fn tempo_luce(&self) -> f64 {
        self.raggio() / C
    }

    /// Tempo proprio massimo dall'orizzonte al centro, in secondi.
    pub fn tempo_massimo_dentro(&self) -> f64 {
        PI / 2.0 * self.tempo_luce()
    }

    /// Temperatura di un disco sottile con accrescimento al 10% del limite di
    /// Eddington, a `r` raggi di Schwarzschild. Stima di ordine di grandezza.
    pub fn temperatura_disco(&self, r: f64) -> f64 {
        6.3e5 * 0.1_f64.powf(0.25) * (self.masse_solari() / 1e8).powf(-0.25) * r.powf(-0.75)
    }
}

/// Massa, in masse solari, sotto la quale la marea uccide prima dell'orizzonte.
pub fn massa_soglia_sopravvivenza() -> f64 {
    let raggio = (C * C * ALTEZZA_PERSONA / SOGLIA_MAREA).sqrt();
    raggio * C * C / (2.0 * G * MASSA_SOLE)
}

/// Velocita' angolare di un'orbita circolare a `r` raggi, in unita' c/rs.
pub fn omega_orbita(r: f64) -> f64 {
    (0.5 / (r * r * r)).sqrt()
}

/// Rapporto tra energia ricevuta all'infinito ed energia emessa da materia in
/// orbita circolare a `r`, per un fotone con momento angolare specifico `l`.
pub fn fattore_g_disco(r: f64, l: f64) -> f64 {
    (1.0 - 1.5 / r).sqrt() / (1.0 - omega_orbita(r) * l)
}

/// Riferimento familiare da sovrapporre alla scena.
pub struct Riferimento {
    pub nome: &'static str,
    pub raggio_m: f64,
}

/// Dal piu' piccolo al piu' grande.
pub const RIFERIMENTI: &[Riferimento] = &[
    Riferimento { nome: "Italia (da capo a capo)", raggio_m: 6.0e5 },
    Riferimento { nome: "Terra", raggio_m: 6.371e6 },
    Riferimento { nome: "Orbita della Luna", raggio_m: 3.844e8 },
    Riferimento { nome: "Sole", raggio_m: 6.957e8 },
    Riferimento { nome: "Orbita della Terra", raggio_m: 1.496e11 },
    Riferimento { nome: "Orbita di Nettuno", raggio_m: 4.495e12 },
    Riferimento { nome: "Sonda Voyager 1", raggio_m: 2.5e13 },
    Riferimento { nome: "Nube di Oort (bordo interno)", raggio_m: 3.0e14 },
    Riferimento { nome: "Stella piu' vicina", raggio_m: 4.0e16 },
];

#[cfg(test)]
mod test {
    use super::*;
    use crate::costanti::DENSITA_ARIA;

    fn circa(a: f64, b: f64, tolleranza: f64) {
        assert!((a / b - 1.0).abs() < tolleranza, "{a} non e' circa {b}");
    }

    #[test]
    fn raggio_del_sole() {
        circa(BucoNero::da_masse_solari(1.0).raggio(), 2953.25, 1e-4);
    }

    #[test]
    fn densita_del_sole_compattato() {
        circa(BucoNero::da_masse_solari(1.0).densita_media(), 1.85e19, 0.01);
    }

    #[test]
    fn un_gigante_e_meno_denso_dell_aria() {
        assert!(BucoNero::da_masse_solari(4.07e10).densita_media() < DENSITA_ARIA);
    }

    #[test]
    fn stellare_uccide_prima_supermassiccio_no() {
        assert!(!BucoNero::da_masse_solari(10.0).si_sopravvive_all_orizzonte());
        assert!(BucoNero::da_masse_solari(4.3e6).si_sopravvive_all_orizzonte());
    }

    #[test]
    fn la_soglia_separa_i_due_casi() {
        let m = massa_soglia_sopravvivenza();
        circa(m, 1.38e4, 0.02);
        assert!(!BucoNero::da_masse_solari(m * 0.99).si_sopravvive_all_orizzonte());
        assert!(BucoNero::da_masse_solari(m * 1.01).si_sopravvive_all_orizzonte());
    }

    #[test]
    fn disco_visto_di_faccia() {
        // Con l = 0 resta solo il redshift gravitazionale e trasverso.
        circa(fattore_g_disco(6.0, 0.0), (0.75_f64).sqrt(), 1e-12);
    }

    #[test]
    fn disco_lontano_si_riduce_a_doppler_classico() {
        let r: f64 = 1e8;
        let v = (0.5 / r).sqrt();
        // Un fotone emesso lungo il moto ha l = r: g = 1 + v al primo ordine.
        circa(fattore_g_disco(r, r), 1.0 + v, 1e-7);
    }
}
