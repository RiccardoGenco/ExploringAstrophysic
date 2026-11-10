//! Radiazione cosmica di fondo: la storia termica e la dimensione delle macchie.

use crate::costanti::C;
use std::f64::consts::PI;

/// Temperatura attuale, in kelvin.
pub const T_OGGI: f64 = 2.725_5;
/// Fotoni per cm^3 oggi.
pub const FOTONI_OGGI: f64 = 410.7;
/// Costante di Boltzmann in eV/K.
const K_EV: f64 = 8.617_333e-5;
const ENERGIA_IDROGENO: f64 = 13.598_4;
const MASSA_ELETTRONE_EV: f64 = 510_998.95;
const HBARC_CM_EV: f64 = 1.973_27e-5;
const SIGMA_T_CM2: f64 = 6.652_46e-25;
const BARIONI_PER_FOTONE_FATTORE: f64 = 2.74e-8;
const MPC_KM: f64 = 3.085_677_6e19;
const ANNO_S: f64 = 3.155_76e7;
/// Densita' dei fotoni e dei neutrini, come Omega h^2.
const OMEGA_FOTONI_H2: f64 = 2.473e-5;
const OMEGA_RADIAZIONE_H2: f64 = 4.184e-5;
const PASSI_INTEGRALE: usize = 4000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cosmologia {
    pub barioni_h2: f64,
    pub oscura_h2: f64,
    pub h: f64,
    /// Curvatura: positivo significa spazio aperto.
    pub curvatura: f64,
}

/// Planck 2018.
pub const PLANCK: Cosmologia = Cosmologia { barioni_h2: 0.022_37, oscura_h2: 0.120_0, h: 0.673_6, curvatura: 0.0 };

/// Redshift a cui la luce ha smesso di urtare (Planck 2018).
pub const Z_ULTIMO_URTO: f64 = 1089.9;

pub fn temperatura(z: f64) -> f64 {
    T_OGGI * (1.0 + z)
}

pub fn fotoni_per_cm3(z: f64) -> f64 {
    FOTONI_OGGI * (1.0 + z).powi(3)
}

/// Lunghezza d'onda del massimo di corpo nero, in micrometri (legge di Wien).
pub fn lunghezza_picco_um(z: f64) -> f64 {
    2897.77 / temperatura(z)
}

/// Spettro di corpo nero alla lunghezza d'onda data, normalizzato a un massimo unitario.
pub fn planck_relativa(lambda_um: f64, t: f64) -> f64 {
    let x = 14_387.77 / (lambda_um * t);
    let massimo = 14_387.77 / 2897.77_f64 / 1.0;
    let forma = |y: f64| y.powi(5) / (y.min(600.0).exp() - 1.0);
    forma(x) / forma(massimo)
}

impl Cosmologia {
    fn materia_h2(&self) -> f64 {
        self.barioni_h2 + self.oscura_h2
    }

    /// Densita' di energia oscura, per chiudere il bilancio.
    fn lambda_h2(&self) -> f64 {
        self.h * self.h * (1.0 - self.curvatura) - self.materia_h2() - OMEGA_RADIAZIONE_H2
    }

    /// H(a) / (100 km/s/Mpc).
    fn e_h(&self, a: f64) -> f64 {
        let k = self.curvatura * self.h * self.h;
        (OMEGA_RADIAZIONE_H2 / a.powi(4) + self.materia_h2() / a.powi(3) + k / (a * a) + self.lambda_h2()).sqrt()
    }

    /// Eta' dell'universo a redshift `z`, in anni.
    pub fn eta(&self, z: f64) -> f64 {
        let fine = 1.0 / (1.0 + z);
        // Variabile s = sqrt(a): regolare all'inizio.
        let s_fine = fine.sqrt();
        let n = PASSI_INTEGRALE;
        let mut somma = 0.0;
        for i in 0..n {
            let s = (i as f64 + 0.5) / n as f64 * s_fine;
            let a = s * s;
            somma += 2.0 * s / (a * self.e_h(a));
        }
        somma * s_fine / n as f64 * MPC_KM / 100.0 / ANNO_S
    }

    /// Frazione di idrogeno ionizzato (equazione di Saha).
    pub fn ionizzazione(&self, z: f64) -> f64 {
        let t = temperatura(z);
        let kt = K_EV * t;
        let n_barioni = BARIONI_PER_FOTONE_FATTORE * self.barioni_h2 * fotoni_per_cm3(z);
        let quanti = (MASSA_ELETTRONE_EV * kt / (2.0 * PI * HBARC_CM_EV * HBARC_CM_EV)).powf(1.5);
        let s = quanti / n_barioni * (-ENERGIA_IDROGENO / kt).exp();
        if s > 1e12 {
            return 1.0;
        }
        0.5 * ((s * s + 4.0 * s).sqrt() - s)
    }

    /// Distanza che un fotone percorre fra due urti, in anni luce.
    pub fn cammino_libero_anni_luce(&self, z: f64) -> f64 {
        let n_e = self.ionizzazione(z) * BARIONI_PER_FOTONE_FATTORE * self.barioni_h2 * fotoni_per_cm3(z);
        let cm = 1.0 / (n_e.max(1e-40) * SIGMA_T_CM2);
        cm / 9.460_73e17
    }

    fn velocita_suono(&self, a: f64) -> f64 {
        let r = 0.75 * self.barioni_h2 / OMEGA_FOTONI_H2 * a;
        1.0 / (3.0 * (1.0 + r)).sqrt()
    }

    /// Orizzonte del suono al momento dell'ultimo urto, in Mpc.
    pub fn orizzonte_suono(&self, z: f64) -> f64 {
        let fine = 1.0 / (1.0 + z);
        let s_fine = fine.sqrt();
        let n = PASSI_INTEGRALE;
        let mut somma = 0.0;
        for i in 0..n {
            let s = (i as f64 + 0.5) / n as f64 * s_fine;
            let a = s * s;
            somma += self.velocita_suono(a) * 2.0 * s / (a * a * self.e_h(a));
        }
        somma * s_fine / n as f64 * (C / 1e3) / 100.0
    }

    /// Distanza comovente di curvatura fino a `z`, in Mpc.
    pub fn distanza(&self, z: f64) -> f64 {
        let inizio = 1.0 / (1.0 + z);
        let n = PASSI_INTEGRALE * 4;
        let mut somma = 0.0;
        // In ln a: l'integrando varia con dolcezza.
        let (la, lb) = (inizio.ln(), 0.0);
        for i in 0..n {
            let l = la + (lb - la) * (i as f64 + 0.5) / n as f64;
            let a = l.exp();
            somma += 1.0 / (a * self.e_h(a));
        }
        let chi = somma * (lb - la) / n as f64 * (C / 1e3) / 100.0;
        let k = self.curvatura * (self.h * 100.0 / (C / 1e3)).powi(2);
        if k.abs() < 1e-12 {
            chi
        } else if k > 0.0 {
            (k.sqrt() * chi).sinh() / k.sqrt()
        } else {
            ((-k).sqrt() * chi).sin() / (-k).sqrt()
        }
    }

    /// Scala acustica: l'angolo delle macchie, espresso come multipolo.
    pub fn scala_acustica(&self) -> f64 {
        PI * self.distanza(Z_ULTIMO_URTO) / self.orizzonte_suono(Z_ULTIMO_URTO)
    }

    /// Multipolo del picco `n` (1, 2, 3...), con lo sfasamento tipico.
    pub fn picco(&self, n: usize) -> f64 {
        self.scala_acustica() * (n as f64 - 0.267)
    }

    /// Forma indicativa dello spettro delle macchie, in microkelvin quadri.
    /// Non e' il risultato di un calcolo completo: posizione dei picchi vera,
    /// altezze approssimate e tarate sul primo picco misurato.
    pub fn spettro(&self, l: f64) -> f64 {
        let la = self.scala_acustica();
        // Il carico dei barioni, ridotto per dare ai picchi altezze vicine a quelle misurate.
        let r = 0.32 * 0.75 * self.barioni_h2 / OMEGA_FOTONI_H2 / (1.0 + Z_ULTIMO_URTO);
        let fase = PI * (l / la + 0.267);
        // I picchi dispari (compressioni) crescono con i barioni.
        let oscillazione = ((1.0 + r) * fase.cos() - r).powi(2) + 0.16 * fase.sin().powi(2);
        let smorzamento = (-(l / 1000.0).powf(1.6)).exp();
        let salita = (l / 60.0).min(1.0).powf(0.6) * 0.18 + (l / 220.0).min(1.0).powi(2) * 0.82;
        5720.0 * salita * smorzamento * oscillazione / (1.0 + 2.0 * r).powi(2) + 950.0 * (-(l / 80.0)).exp()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn circa(a: f64, b: f64, tolleranza: f64) {
        assert!((a / b - 1.0).abs() < tolleranza, "{a} non e' circa {b}");
    }

    #[test]
    fn eta_dell_universo_e_dell_ultimo_urto() {
        circa(PLANCK.eta(0.0), 13.8e9, 0.01);
        circa(PLANCK.eta(Z_ULTIMO_URTO), 3.75e5, 0.05);
    }

    #[test]
    fn circa_quattrocento_fotoni_per_centimetro_cubo() {
        circa(fotoni_per_cm3(0.0), 411.0, 0.01);
    }

    #[test]
    fn allora_era_caldo_come_la_superficie_di_una_stella() {
        circa(temperatura(Z_ULTIMO_URTO), 2973.0, 0.01);
    }

    #[test]
    fn l_idrogeno_si_forma_attorno_a_z_1300() {
        let p = PLANCK;
        assert!(p.ionizzazione(2000.0) > 0.99);
        assert!(p.ionizzazione(1000.0) < 0.01);
        let mezza = (900..2000).find(|&z| p.ionizzazione(f64::from(z)) > 0.5).unwrap();
        assert!((1250..1500).contains(&mezza), "{mezza}");
    }

    #[test]
    fn orizzonte_del_suono_e_scala_delle_macchie_come_planck() {
        circa(PLANCK.orizzonte_suono(Z_ULTIMO_URTO), 144.4, 0.015);
        // 100 theta* = 1.0411: scala acustica pi / theta*.
        circa(PLANCK.scala_acustica(), PI / 0.010_411, 0.015);
    }

    #[test]
    fn il_primo_picco_cade_vicino_a_220() {
        assert!((PLANCK.picco(1) - 220.0).abs() < 8.0, "{}", PLANCK.picco(1));
    }

    #[test]
    fn la_curvatura_sposta_le_macchie() {
        let aperto = Cosmologia { curvatura: 0.1, ..PLANCK };
        let chiuso = Cosmologia { curvatura: -0.1, ..PLANCK };
        assert!(aperto.picco(1) > PLANCK.picco(1) && chiuso.picco(1) < PLANCK.picco(1));
    }

    #[test]
    fn la_forma_ha_il_massimo_al_primo_picco() {
        let p = PLANCK;
        let massimo = (2..1500)
            .map(f64::from)
            .max_by(|a, b| p.spettro(*a).total_cmp(&p.spettro(*b)))
            .unwrap();
        assert!((massimo - p.picco(1)).abs() < 30.0, "{massimo}");
    }

    #[test]
    fn lo_spettro_di_corpo_nero_ha_massimo_a_wien() {
        let t = 3000.0;
        let picco = 2897.77 / t;
        assert!((planck_relativa(picco, t) - 1.0).abs() < 1e-3);
        assert!(planck_relativa(picco * 3.0, t) < 1.0);
    }
}
