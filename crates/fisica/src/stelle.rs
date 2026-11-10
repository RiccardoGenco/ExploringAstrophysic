//! Stelle di neutroni: equazione di stato a tratti ed equilibrio relativistico.
//!
//! Equazione di stato: polítropi a tratti di Read, Lackey, Owen, Friedman (2009),
//! con la crosta SLy. La struttura si integra nell'entalpia specifica h, che va
//! da un valore centrale a zero sulla superficie (forma di Lindblom, 1992):
//!   dr/dh = -r (r - 2m) / (m + 4 pi r^3 P),   dm/dh = 4 pi r^2 e dr/dh.

use crate::costanti::{C, G, MASSA_SOLE};
use std::f64::consts::PI;

/// Da g/cm^3 a km^-2, cioe' G/c^2 in unita' geometriche.
const DA_CGS_A_KM2: f64 = G / (C * C) * 1e9;
/// Massa del Sole in km (G M / c^2).
pub const SOLE_IN_KM: f64 = G * MASSA_SOLE / (C * C) * 1e-3;

const RHO_1: f64 = 5.011_872_336_272_722e14; // 10^14.7 g/cm^3
const RHO_2: f64 = 1e15;
const H_FINALE: f64 = 1e-6;
const PASSI_CENTRO: usize = 60;
const PASSI_STELLA: usize = 500;
const LOG_RHO_MIN: f64 = 14.35;
const LOG_RHO_MAX: f64 = 16.0;
const PUNTI_CURVA: usize = 90;
const OGNI_PROFILO: usize = 10;

/// Crosta SLy: K (con P/c^2 in g/cm^3), Gamma, densita' a cui il tratto finisce.
const CROSTA: [(f64, f64, f64); 4] = [
    (6.80110e-9, 1.58425, 2.44034e7),
    (1.06186e-6, 1.28733, 3.78358e11),
    (5.32697e1, 0.62223, 2.62780e12),
    (3.99874e-8, 1.35692, f64::INFINITY),
];

#[derive(Clone, Copy, Debug)]
struct Tratto {
    k: f64,
    gamma: f64,
    rho_inizio: f64,
    a: f64,
    h_inizio: f64,
}

impl Tratto {
    fn pressione(&self, rho: f64) -> f64 {
        self.k * rho.powf(self.gamma)
    }

    fn energia(&self, rho: f64) -> f64 {
        (1.0 + self.a) * rho + self.pressione(rho) / (self.gamma - 1.0)
    }

    fn e_alla_h(&self, rho: f64) -> f64 {
        1.0 + self.a + self.gamma / (self.gamma - 1.0) * self.k * rho.powf(self.gamma - 1.0)
    }
}

/// Punto dell'equazione di stato, in g/cm^3.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Punto {
    pub rho: f64,
    pub energia: f64,
    pub pressione: f64,
    /// Velocita' del suono al quadrato, in unita' di c^2.
    pub suono2: f64,
}

/// I quattro numeri che descrivono il nucleo: log10 di p1 in dyn/cm^2 e tre Gamma.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parametri {
    pub log_p1: f64,
    pub gamma1: f64,
    pub gamma2: f64,
    pub gamma3: f64,
}

/// Equazioni di stato realistiche, con la massa massima pubblicata (Read 2009).
pub const CATALOGO: [(&str, Parametri, f64); 7] = [
    ("SLy", Parametri { log_p1: 34.384, gamma1: 3.005, gamma2: 2.988, gamma3: 2.851 }, 2.049),
    ("AP4", Parametri { log_p1: 34.269, gamma1: 2.830, gamma2: 3.445, gamma3: 3.348 }, 2.213),
    ("WFF1", Parametri { log_p1: 34.031, gamma1: 2.519, gamma2: 3.791, gamma3: 3.660 }, 2.133),
    ("MPA1", Parametri { log_p1: 34.495, gamma1: 3.446, gamma2: 3.572, gamma3: 2.887 }, 2.461),
    ("H4", Parametri { log_p1: 34.669, gamma1: 2.909, gamma2: 2.246, gamma3: 2.144 }, 2.032),
    ("GNH3", Parametri { log_p1: 34.648, gamma1: 2.664, gamma2: 2.194, gamma3: 2.304 }, 1.962),
    ("MS1", Parametri { log_p1: 34.858, gamma1: 3.224, gamma2: 3.033, gamma3: 1.325 }, 2.767),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gravita {
    Einstein,
    Newton,
}

#[derive(Clone, Debug)]
pub struct Materia {
    tratti: Vec<Tratto>,
}

/// Uno strato della stella.
#[derive(Clone, Copy, Debug)]
pub struct Strato {
    /// Distanza dal centro, in km.
    pub r: f64,
    /// Densita', in g/cm^3.
    pub rho: f64,
}

#[derive(Clone, Debug)]
pub struct Stella {
    pub rho_centrale: f64,
    /// Massa gravitazionale, in masse solari.
    pub massa: f64,
    pub massa_barionica: f64,
    /// Raggio, in km.
    pub raggio: f64,
    pub suono2_centrale: f64,
    /// Dal centro verso la superficie.
    pub profilo: Vec<Strato>,
}

impl Stella {
    /// Raggio di Schwarzschild della stessa massa, in km.
    pub fn raggio_schwarzschild(&self) -> f64 {
        2.0 * SOLE_IN_KM * self.massa
    }

    fn compattezza(&self) -> f64 {
        self.raggio_schwarzschild() / self.raggio
    }

    /// Accelerazione sulla superficie, in m/s^2.
    pub fn gravita_superficie(&self) -> f64 {
        let r = self.raggio * 1e3;
        G * self.massa * MASSA_SOLE / (r * r) / (1.0 - self.compattezza()).sqrt()
    }

    /// Densita' media, in kg/m^3.
    pub fn densita_media(&self) -> f64 {
        let r = self.raggio * 1e3;
        self.massa * MASSA_SOLE / (4.0 / 3.0 * PI * r * r * r)
    }

    /// Velocita' di fuga in frazioni di c.
    pub fn fuga(&self) -> f64 {
        self.compattezza().sqrt()
    }

    /// Anni che passano lontano per ogni anno vissuto sulla superficie.
    pub fn anni_lontano(&self) -> f64 {
        1.0 / (1.0 - self.compattezza()).sqrt()
    }

    /// Frazione di superficie visibile da lontano, grazie alla luce deviata.
    /// Approssimazione di Beloborodov (2002).
    pub fn superficie_visibile(&self) -> f64 {
        (0.5 / (1.0 - self.compattezza())).min(1.0)
    }
}

/// Una curva massa-raggio e l'indice della stella piu' pesante stabile.
#[derive(Clone, Debug)]
pub struct Curva {
    pub stelle: Vec<Stella>,
    pub indice_massimo: usize,
}

impl Curva {
    pub fn massa_massima(&self) -> f64 {
        self.stelle[self.indice_massimo].massa
    }

    /// Raggio delle stelle stabili di massa `m`, se la materia ne ammette.
    pub fn raggio_a_massa(&self, m: f64) -> Option<f64> {
        self.stelle[..=self.indice_massimo].windows(2).rev().find_map(|c| {
            let (basso, alto) = (c[0].massa.min(c[1].massa), c[0].massa.max(c[1].massa));
            (m >= basso && m <= alto && alto > basso).then(|| {
                let k = (m - c[0].massa) / (c[1].massa - c[0].massa);
                c[0].raggio + (c[1].raggio - c[0].raggio) * k
            })
        })
    }
}

impl Materia {
    /// `None` se il nucleo chiesto non si raccorda con la crosta.
    pub fn nuova(p: Parametri) -> Option<Self> {
        let p1 = 10f64.powf(p.log_p1) / (C * C * 1e4);
        let k1 = p1 / RHO_1.powf(p.gamma1);
        let k2 = p1 / RHO_1.powf(p.gamma2);
        let k3 = k2 * RHO_2.powf(p.gamma2 - p.gamma3);

        let (k_crosta, gamma_crosta, _) = CROSTA[3];
        let rho_giunzione = (k_crosta / k1).powf(1.0 / (p.gamma1 - gamma_crosta));
        if !(rho_giunzione > CROSTA[2].2 && rho_giunzione < RHO_1) {
            return None;
        }
        if [p.gamma1, p.gamma2, p.gamma3].iter().any(|g| *g <= 1.0) {
            return None;
        }

        let pezzi = [
            (CROSTA[0].0, CROSTA[0].1, 0.0),
            (CROSTA[1].0, CROSTA[1].1, CROSTA[0].2),
            (CROSTA[2].0, CROSTA[2].1, CROSTA[1].2),
            (CROSTA[3].0, CROSTA[3].1, CROSTA[2].2),
            (k1, p.gamma1, rho_giunzione),
            (k2, p.gamma2, RHO_1),
            (k3, p.gamma3, RHO_2),
        ];
        let mut tratti: Vec<Tratto> = Vec::with_capacity(pezzi.len());
        for (k, gamma, rho_inizio) in pezzi {
            let mut t = Tratto { k, gamma, rho_inizio, a: 0.0, h_inizio: 0.0 };
            if let Some(prima) = tratti.last() {
                // Energia e' continua: da qui si ricava la costante del tratto.
                let e = prima.energia(rho_inizio);
                t.a = e / rho_inizio - 1.0 - k * rho_inizio.powf(gamma - 1.0) / (gamma - 1.0);
                t.h_inizio = t.e_alla_h(rho_inizio).ln();
            }
            tratti.push(t);
        }
        Some(Self { tratti })
    }

    fn tratto_da_rho(&self, rho: f64) -> &Tratto {
        self.tratti.iter().rev().find(|t| rho >= t.rho_inizio).unwrap_or(&self.tratti[0])
    }

    fn tratto_da_h(&self, h: f64) -> &Tratto {
        self.tratti.iter().rev().find(|t| h >= t.h_inizio).unwrap_or(&self.tratti[0])
    }

    pub fn da_rho(&self, rho: f64) -> Punto {
        Self::punto(self.tratto_da_rho(rho), rho)
    }

    fn punto(t: &Tratto, rho: f64) -> Punto {
        let pressione = t.pressione(rho);
        let energia = t.energia(rho);
        Punto { rho, energia, pressione, suono2: t.gamma * pressione / (energia + pressione) }
    }

    pub fn entalpia(&self, rho: f64) -> f64 {
        self.tratto_da_rho(rho).e_alla_h(rho).ln()
    }

    pub fn da_entalpia(&self, h: f64) -> Punto {
        let t = self.tratto_da_h(h);
        let base = (h.exp() - 1.0 - t.a) * (t.gamma - 1.0) / (t.gamma * t.k);
        Self::punto(t, base.max(0.0).powf(1.0 / (t.gamma - 1.0)))
    }

    /// Derivate di (r, m, massa barionica) rispetto a h. In km e km^-2.
    fn derivate(&self, h: f64, y: [f64; 3], gravita: Gravita) -> [f64; 3] {
        let q = self.da_entalpia(h);
        let (rho, e, p) = (
            q.rho * DA_CGS_A_KM2,
            q.energia * DA_CGS_A_KM2,
            q.pressione * DA_CGS_A_KM2,
        );
        let [r, m, _] = y;
        let quattro_pi_r2 = 4.0 * PI * r * r;
        match gravita {
            Gravita::Einstein => {
                let dr = -r * (r - 2.0 * m) / (m + quattro_pi_r2 * r * p);
                [dr, quattro_pi_r2 * e * dr, quattro_pi_r2 * rho * dr / (1.0 - 2.0 * m / r).sqrt()]
            }
            Gravita::Newton => {
                // Per Newton pesa solo la massa a riposo, non energia e pressione.
                let dr = -(e + p) * r * r / (rho * m);
                [dr, quattro_pi_r2 * rho * dr, quattro_pi_r2 * rho * dr]
            }
        }
    }

    fn rk4(&self, h: f64, dh: f64, y: [f64; 3], g: Gravita) -> [f64; 3] {
        let somma = |a: [f64; 3], b: [f64; 3], k: f64| [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k];
        let k1 = self.derivate(h, y, g);
        let k2 = self.derivate(h + dh / 2.0, somma(y, k1, dh / 2.0), g);
        let k3 = self.derivate(h + dh / 2.0, somma(y, k2, dh / 2.0), g);
        let k4 = self.derivate(h + dh, somma(y, k3, dh), g);
        [
            y[0] + dh / 6.0 * (k1[0] + 2.0 * k2[0] + 2.0 * k3[0] + k4[0]),
            y[1] + dh / 6.0 * (k1[1] + 2.0 * k2[1] + 2.0 * k3[1] + k4[1]),
            y[2] + dh / 6.0 * (k1[2] + 2.0 * k2[2] + 2.0 * k3[2] + k4[2]),
        ]
    }

    /// Risolve l'equilibrio di una stella con la densita' centrale data.
    pub fn stella(&self, rho_centrale: f64, gravita: Gravita) -> Stella {
        let hc = self.entalpia(rho_centrale);
        let c = self.da_entalpia(hc);
        let (ec, pc, rc) = (
            c.energia * DA_CGS_A_KM2,
            c.pressione * DA_CGS_A_KM2,
            c.rho * DA_CGS_A_KM2,
        );

        // Attorno al centro r va come la radice di (hc - h): nodi fitti lì.
        let inizio = hc * 1e-4;
        let r0 = (3.0 * inizio / (2.0 * PI * (ec + 3.0 * pc))).sqrt();
        let volume = 4.0 / 3.0 * PI * r0 * r0 * r0;
        let mut y = [r0, volume * ec, volume * rc];

        let mut profilo = vec![Strato { r: 0.0, rho: c.rho }];
        let mut h = hc - inizio;
        let mut avanza = |nuovo: f64, y: &mut [f64; 3]| {
            *y = self.rk4(h, nuovo - h, *y, gravita);
            h = nuovo;
        };
        for k in 1..=PASSI_CENTRO {
            let distanza = inizio * 1000f64.powf(k as f64 / PASSI_CENTRO as f64);
            avanza(hc - distanza, &mut y);
        }
        let h_medio = hc * 0.9;
        let rapporto = (H_FINALE / h_medio).ln() / PASSI_STELLA as f64;
        for k in 1..=PASSI_STELLA {
            let nuovo = h_medio * (rapporto * k as f64).exp();
            avanza(nuovo, &mut y);
            if k % OGNI_PROFILO == 0 {
                profilo.push(Strato { r: y[0], rho: self.da_entalpia(nuovo).rho });
            }
        }

        Stella {
            rho_centrale,
            massa: y[1] / SOLE_IN_KM,
            massa_barionica: y[2] / SOLE_IN_KM,
            raggio: y[0],
            suono2_centrale: c.suono2,
            profilo,
        }
    }

    /// Tutte le stelle possibili con questa materia, dalle leggere alle densissime.
    pub fn curva(&self, gravita: Gravita) -> Curva {
        let stelle: Vec<Stella> = (0..PUNTI_CURVA)
            .map(|i| {
                let k = i as f64 / (PUNTI_CURVA - 1) as f64;
                self.stella(10f64.powf(LOG_RHO_MIN + (LOG_RHO_MAX - LOG_RHO_MIN) * k), gravita)
            })
            .collect();
        // Il ramo stabile finisce alla massa piu' alta: oltre, la stella crolla.
        // Non basta il primo massimo: a densita' basse la curva puo' scendere.
        let indice_massimo = stelle
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.massa.total_cmp(&b.1.massa))
            .map_or(0, |(i, _)| i);
        Curva { stelle, indice_massimo }
    }
}

/// Una misura reale di massa e raggio, con le sue incertezze a 68%.
pub struct Misura {
    pub nome: &'static str,
    pub fonte: &'static str,
    pub massa: f64,
    pub massa_errore: f64,
    pub raggio: f64,
    pub raggio_su: f64,
    pub raggio_giu: f64,
}

pub const MISURE: [Misura; 2] = [
    Misura {
        nome: "PSR J0740+6620",
        fonte: "NICER e XMM-Newton, Riley et al. 2021",
        massa: 2.072,
        massa_errore: 0.067,
        raggio: 12.39,
        raggio_su: 1.30,
        raggio_giu: 0.98,
    },
    Misura {
        nome: "PSR J0030+0451",
        fonte: "NICER, Miller et al. 2019",
        massa: 1.44,
        massa_errore: 0.15,
        raggio: 13.02,
        raggio_su: 1.24,
        raggio_giu: 1.06,
    },
];

impl Misura {
    /// Vero se la materia puo' produrre una stella entro due volte l'incertezza,
    /// cioe' circa il 95% di confidenza: una barra sola sarebbe troppo severa.
    pub fn compatibile(&self, curva: &Curva) -> bool {
        curva.stelle[..=curva.indice_massimo].iter().any(|s| {
            (s.massa - self.massa).abs() <= 2.0 * self.massa_errore
                && s.raggio <= self.raggio + 2.0 * self.raggio_su
                && s.raggio >= self.raggio - 2.0 * self.raggio_giu
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn la_massa_massima_coincide_con_quella_pubblicata() {
        for (nome, parametri, attesa) in CATALOGO {
            let m = Materia::nuova(parametri).expect(nome).curva(Gravita::Einstein).massa_massima();
            assert!((m / attesa - 1.0).abs() < 0.015, "{nome}: {m} contro {attesa}");
        }
    }

    #[test]
    fn una_stella_di_1_4_masse_solari_con_sly() {
        let curva = Materia::nuova(CATALOGO[0].1).unwrap().curva(Gravita::Einstein);
        let r = curva.raggio_a_massa(1.4).unwrap();
        assert!((r - 11.75).abs() < 0.4, "{r}");
    }

    #[test]
    fn energia_e_entalpia_sono_continue_alle_giunzioni() {
        let m = Materia::nuova(CATALOGO[0].1).unwrap();
        for t in &m.tratti[1..] {
            let (a, b) = (t.rho_inizio * (1.0 - 1e-9), t.rho_inizio * (1.0 + 1e-9));
            let (pa, pb) = (m.da_rho(a), m.da_rho(b));
            assert!((pa.energia / pb.energia - 1.0).abs() < 1e-7);
            // La tabella pubblicata ha sei cifre: la pressione combacia a meno di quelle.
            assert!((pa.pressione / pb.pressione - 1.0).abs() < 1e-3);
        }
    }

    #[test]
    fn l_entalpia_si_inverte() {
        let m = Materia::nuova(CATALOGO[1].1).unwrap();
        for rho in [1e6, 1e9, 1e12, 3e12, 1e14, 7e14, 3e15] {
            let tornato = m.da_entalpia(m.entalpia(rho)).rho;
            assert!((tornato / rho - 1.0).abs() < 1e-5, "{rho} -> {tornato}");
        }
    }

    #[test]
    fn densita_vicine_danno_stelle_vicine() {
        let m = Materia::nuova(CATALOGO[0].1).unwrap();
        let s = m.stella(1e15, Gravita::Einstein);
        let t = m.stella(1.002e15, Gravita::Einstein);
        assert!((s.massa - t.massa).abs() < 0.01 && (s.raggio - t.raggio).abs() < 0.05);
    }

    #[test]
    fn senza_einstein_non_c_e_massa_massima() {
        let m = Materia::nuova(CATALOGO[0].1).unwrap();
        let newton = m.curva(Gravita::Newton);
        assert_eq!(newton.indice_massimo, newton.stelle.len() - 1);
        assert!(newton.massa_massima() > 2.0 * m.curva(Gravita::Einstein).massa_massima());
    }

    #[test]
    fn le_conseguenze_di_una_stella_tipica() {
        let curva = Materia::nuova(CATALOGO[0].1).unwrap().curva(Gravita::Einstein);
        let s = curva
            .stelle
            .iter()
            .min_by(|a, b| (a.massa - 1.4).abs().total_cmp(&(b.massa - 1.4).abs()))
            .unwrap();
        // Una caduta da un metro: circa 2000 km/s.
        let v = (2.0 * s.gravita_superficie()).sqrt() / 1e3;
        assert!(v > 1500.0 && v < 3000.0, "{v}");
        assert!(s.superficie_visibile() > 0.5);
        assert!(s.massa_barionica > s.massa, "il legame toglie massa");
    }

    #[test]
    fn la_misura_pesante_esclude_la_materia_troppo_morbida() {
        let morbida = Parametri { log_p1: 34.1, gamma1: 2.4, gamma2: 2.0, gamma3: 2.0 };
        let curva = Materia::nuova(morbida).unwrap().curva(Gravita::Einstein);
        assert!(curva.massa_massima() < 1.9, "{}", curva.massa_massima());
        assert!(!MISURE[0].compatibile(&curva));
    }

    #[test]
    fn un_nucleo_impossibile_viene_rifiutato() {
        let p = Parametri { log_p1: 33.0, gamma1: 1.2, gamma2: 2.0, gamma3: 2.0 };
        assert!(Materia::nuova(p).is_none());
    }
}
