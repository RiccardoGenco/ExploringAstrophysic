//! Galassie: curva di rotazione, alone oscuro, passaggi ravvicinati.
//!
//! Metodo "ristretto" di Toomre: le galassie sono potenziali che si attraggono e le
//! stelle sono particelle di prova. Unita': kpc, km/s, masse solari.

use std::f64::consts::PI;

pub const G: f64 = 4.300_91e-6;
/// Miliardi di anni in un'unita' di tempo (kpc / km/s).
pub const UNITA_IN_MILIARDI_ANNI: f64 = 0.977_8;
const PASSO: f64 = 0.002;
const LOG_COULOMB: f64 = 3.0;
/// Velocita' tipica della materia dell'alone, in km/s.
const DISPERSIONE: f64 = 180.0;

/// Funzione degli errori (Abramowitz e Stegun 7.1.26).
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.470_47 * x);
    1.0 - (0.348_024_2 * t - 0.095_879_8 * t * t + 0.747_855_6 * t * t * t) * (-x * x).exp()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modello {
    pub massa_stelle: f64,
    pub raggio_stelle: f64,
    pub massa_alone: f64,
    pub raggio_alone: f64,
}

/// Simile alla Via Lattea: circa 220 km/s attorno al Sole.
pub const VIA_LATTEA: Modello = Modello {
    massa_stelle: 6e10,
    raggio_stelle: 4.0,
    massa_alone: 1e12,
    raggio_alone: 30.0,
};

impl Modello {
    pub fn massa_stelle_entro(&self, r: f64) -> f64 {
        self.massa_stelle * r.powi(3) / (r * r + self.raggio_stelle * self.raggio_stelle).powf(1.5)
    }

    pub fn massa_alone_entro(&self, r: f64) -> f64 {
        self.massa_alone * r * r / (r + self.raggio_alone).powi(2)
    }

    pub fn massa_entro(&self, r: f64, fattore_alone: f64) -> f64 {
        self.massa_stelle_entro(r) + fattore_alone * self.massa_alone_entro(r)
    }

    pub fn massa_totale(&self, fattore_alone: f64) -> f64 {
        self.massa_stelle + fattore_alone * self.massa_alone
    }

    /// Velocita' di un'orbita circolare a `r`, in km/s.
    pub fn v_circolare(&self, r: f64, fattore_alone: f64) -> f64 {
        (G * self.massa_entro(r, fattore_alone) / r).sqrt()
    }

    fn densita_alone(&self, r: f64) -> f64 {
        let (a, r) = (self.raggio_alone, r.max(1e-3));
        self.massa_alone * a / (2.0 * PI * r * (r + a).powi(3))
    }

    /// Accelerazione verso il centro, data la distanza vettoriale dal centro.
    fn accelerazione(&self, dx: f64, dy: f64, fattore_alone: f64) -> [f64; 2] {
        let r2 = dx * dx + dy * dy + 1e-4;
        let r = r2.sqrt();
        let k = -G * self.massa_entro(r, fattore_alone) / (r2 * r);
        [k * dx, k * dy]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Galassia {
    pub modello: Modello,
    pub x: [f64; 2],
    pub v: [f64; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Stella {
    pub x: [f32; 2],
    pub v: [f32; 2],
    pub madre: u8,
}

#[derive(Clone, Debug)]
pub struct Simulazione {
    pub galassie: Vec<Galassia>,
    pub stelle: Vec<Stella>,
    /// Tempo in unita' interne.
    pub t: f64,
    pub fattore_alone: f64,
    pub attrito: bool,
}

/// Generatore semplice e ripetibile: la stessa scena e' sempre la stessa.
struct Casuale(u64);

impl Casuale {
    fn valore(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f64 + 0.5) / (1u64 << 31) as f64
    }
}

/// Riempie un disco di stelle su orbite circolari attorno alla galassia.
pub fn disco(g: &Galassia, indice: u8, n: usize, verso: f64, seme: u64) -> Vec<Stella> {
    let mut caso = Casuale(seme.wrapping_mul(2) + 1);
    (0..n)
        .map(|_| {
            // Esponenziale con raggio di scala 4 kpc, tagliato a 20.
            let r = (-4.0 * (1.0 - caso.valore() * 0.993).ln()).clamp(0.6, 20.0);
            let a = caso.valore() * 2.0 * PI;
            let v = g.modello.v_circolare(r, 1.0) * verso * (0.98 + 0.04 * caso.valore());
            let (s, c) = a.sin_cos();
            Stella {
                x: [(g.x[0] + r * c) as f32, (g.x[1] + r * s) as f32],
                v: [(g.v[0] - v * s) as f32, (g.v[1] + v * c) as f32],
                madre: indice,
            }
        })
        .collect()
}

impl Simulazione {
    fn acc_stella(&self, x: f64, y: f64) -> [f64; 2] {
        let mut a = [0.0, 0.0];
        for g in &self.galassie {
            let q = g.modello.accelerazione(x - g.x[0], y - g.x[1], self.fattore_alone);
            a[0] += q[0];
            a[1] += q[1];
        }
        a
    }

    fn acc_galassie(&self) -> Vec<[f64; 2]> {
        let n = self.galassie.len();
        let mut acc = vec![[0.0, 0.0]; n];
        for i in 0..n {
            for j in (0..n).filter(|&j| j != i) {
                let (a, b) = (&self.galassie[i], &self.galassie[j]);
                let (dx, dy) = (a.x[0] - b.x[0], a.x[1] - b.x[1]);
                let q = b.modello.accelerazione(dx, dy, self.fattore_alone);
                acc[i][0] += q[0];
                acc[i][1] += q[1];
                if self.attrito && self.fattore_alone > 0.0 {
                    // Attrito dinamico alla Chandrasekhar: formula approssimata.
                    let r = dx.hypot(dy);
                    let (ux, uy) = (a.v[0] - b.v[0], a.v[1] - b.v[1]);
                    let v = ux.hypot(uy).max(1.0);
                    let massa = a.modello.massa_entro(r.min(a.modello.raggio_alone), self.fattore_alone);
                    let rho = self.fattore_alone * b.modello.densita_alone(r);
                    // Solo la materia piu' lenta di chi passa frena: a basse velocita' tende a zero.
                    let x = v / (2.0_f64.sqrt() * DISPERSIONE);
                    let frazione = erf(x) - 2.0 * x / PI.sqrt() * (-x * x).exp();
                    let k = -4.0 * PI * G * G * massa * rho * LOG_COULOMB * frazione / (v * v * v);
                    acc[i][0] += k * ux;
                    acc[i][1] += k * uy;
                }
            }
        }
        acc
    }

    fn passo(&mut self, h: f64) {
        // Salto-rana: mezza spinta, spostamento, mezza spinta.
        let a = self.acc_galassie();
        for (g, q) in self.galassie.iter_mut().zip(&a) {
            g.v[0] += 0.5 * h * q[0];
            g.v[1] += 0.5 * h * q[1];
            g.x[0] += h * g.v[0];
            g.x[1] += h * g.v[1];
        }
        for i in 0..self.stelle.len() {
            let s = self.stelle[i];
            let p = self.acc_stella(f64::from(s.x[0]), f64::from(s.x[1]));
            let v = [f64::from(s.v[0]) + 0.5 * h * p[0], f64::from(s.v[1]) + 0.5 * h * p[1]];
            let x = [f64::from(s.x[0]) + h * v[0], f64::from(s.x[1]) + h * v[1]];
            self.stelle[i].x = [x[0] as f32, x[1] as f32];
            self.stelle[i].v = [v[0] as f32, v[1] as f32];
        }
        let a = self.acc_galassie();
        for (g, q) in self.galassie.iter_mut().zip(&a) {
            g.v[0] += 0.5 * h * q[0];
            g.v[1] += 0.5 * h * q[1];
        }
        for s in &mut self.stelle {
            let (x, y) = (f64::from(s.x[0]), f64::from(s.x[1]));
            let mut p = [0.0, 0.0];
            for g in &self.galassie {
                let q = g.modello.accelerazione(x - g.x[0], y - g.x[1], self.fattore_alone);
                p[0] += q[0];
                p[1] += q[1];
            }
            s.v[0] = (f64::from(s.v[0]) + 0.5 * h * p[0]) as f32;
            s.v[1] = (f64::from(s.v[1]) + 0.5 * h * p[1]) as f32;
        }
        self.t += h;
    }

    /// Avanza di `durata` (unita' interne), con un massimo di passi.
    pub fn avanza(&mut self, durata: f64, massimo: usize) {
        let n = ((durata / PASSO).ceil() as usize).clamp(1, massimo);
        let h = (durata / n as f64).min(PASSO);
        for _ in 0..n {
            self.passo(h);
        }
    }

    pub fn separazione(&self) -> f64 {
        if self.galassie.len() < 2 {
            return 0.0;
        }
        let (a, b) = (self.galassie[0].x, self.galassie[1].x);
        (a[0] - b[0]).hypot(a[1] - b[1])
    }

    pub fn tempo_miliardi(&self) -> f64 {
        self.t * UNITA_IN_MILIARDI_ANNI
    }
}

/// Quante stelle si trovano lontano da tutti i centri.
pub fn stelle_lontane(s: &Simulazione, soglia: f64) -> usize {
    s.stelle
        .iter()
        .filter(|q| {
            s.galassie
                .iter()
                .all(|g| (f64::from(q.x[0]) - g.x[0]).hypot(f64::from(q.x[1]) - g.x[1]) > soglia)
        })
        .count()
}

/// Una sola galassia ferma al centro.
pub fn una_galassia(n: usize) -> Simulazione {
    let g = Galassia { modello: VIA_LATTEA, x: [0.0, 0.0], v: [0.0, 0.0] };
    Simulazione { stelle: disco(&g, 0, n, 1.0, 7), galassie: vec![g], t: 0.0, fattore_alone: 1.0, attrito: false }
}

/// Due galassie uguali: la seconda parte lontana e passa a distanza `passaggio`.
pub fn incontro(n: usize, passaggio: f64, velocita: f64, verso_seconda: f64) -> Simulazione {
    let distanza = 140.0;
    let a = Galassia { modello: VIA_LATTEA, x: [-distanza / 2.0, -passaggio / 2.0], v: [velocita / 2.0, 0.0] };
    let b = Galassia { modello: VIA_LATTEA, x: [distanza / 2.0, passaggio / 2.0], v: [-velocita / 2.0, 0.0] };
    let mut stelle = disco(&a, 0, n / 2, 1.0, 11);
    stelle.extend(disco(&b, 1, n / 2, verso_seconda, 23));
    Simulazione { galassie: vec![a, b], stelle, t: 0.0, fattore_alone: 1.0, attrito: true }
}

/// Via Lattea e Andromeda: 770 kpc, 110 km/s in avvicinamento, poco moto laterale.
pub fn andromeda(n: usize) -> Simulazione {
    let a = Galassia { modello: VIA_LATTEA, x: [0.0, 0.0], v: [0.0, 0.0] };
    let m31 = Modello { massa_stelle: 1e11, raggio_stelle: 5.0, ..VIA_LATTEA };
    let b = Galassia { modello: m31, x: [770.0, 0.0], v: [-110.0, 17.0] };
    let mut stelle = disco(&a, 0, n / 2, 1.0, 11);
    stelle.extend(disco(&b, 1, n / 2, 1.0, 23));
    Simulazione { galassie: vec![a, b], stelle, t: 0.0, fattore_alone: 1.0, attrito: true }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_sole_gira_a_circa_220_km_s() {
        let v = VIA_LATTEA.v_circolare(8.2, 1.0);
        assert!((v - 225.0).abs() < 20.0, "{v}");
    }

    #[test]
    fn con_l_alone_la_curva_resta_piatta_senza_cade() {
        let m = VIA_LATTEA;
        let (a, b) = (m.v_circolare(10.0, 1.0), m.v_circolare(40.0, 1.0));
        assert!((a / b - 1.0).abs() < 0.2, "{a} {b}");
        let (c, d) = (m.v_circolare(10.0, 0.0), m.v_circolare(40.0, 0.0));
        assert!(d < 0.6 * c, "{c} {d}");
    }

    #[test]
    fn il_disco_resta_in_piedi_con_l_alone_e_si_disperde_senza() {
        let mut s = una_galassia(400);
        s.avanza(2.0, 2000);
        assert!(stelle_lontane(&s, 30.0) < 8, "{}", stelle_lontane(&s, 30.0));

        let mut senza = una_galassia(400);
        senza.fattore_alone = 0.0;
        senza.avanza(2.0, 2000);
        assert!(stelle_lontane(&senza, 30.0) > 30, "{}", stelle_lontane(&senza, 30.0));
    }

    #[test]
    fn andromeda_arriva_tra_circa_quattro_miliardi_di_anni() {
        let mut s = andromeda(0);
        let mut primo = None;
        while s.t < 8.0 && primo.is_none() {
            s.avanza(0.01, 100);
            if s.separazione() < 80.0 {
                primo = Some(s.tempo_miliardi());
            }
        }
        let t = primo.expect("devono incontrarsi");
        assert!(t > 3.0 && t < 5.5, "{t}");
    }

    #[test]
    fn un_passaggio_nello_stesso_verso_strappa_piu_stelle() {
        let durata = 1.0;
        let mut diretto = incontro(800, 30.0, 400.0, 1.0);
        diretto.attrito = false;
        diretto.galassie[1].x[1] = 15.0;
        let mut contrario = incontro(800, 30.0, 400.0, -1.0);
        contrario.attrito = false;
        contrario.galassie[1].x[1] = 15.0;
        diretto.avanza(durata, 2000);
        contrario.avanza(durata, 2000);
        let conta = |s: &Simulazione| s.stelle.iter().filter(|q| q.madre == 1).filter(|q| {
            (f64::from(q.x[0]) - s.galassie[1].x[0]).hypot(f64::from(q.x[1]) - s.galassie[1].x[1]) > 25.0
        }).count();
        assert!(conta(&diretto) > conta(&contrario), "{} {}", conta(&diretto), conta(&contrario));
    }

    #[test]
    fn con_l_attrito_due_galassie_si_fondono() {
        let mut s = incontro(0, 20.0, 300.0, 1.0);
        s.avanza(6.0, 4000);
        assert!(s.separazione() < 30.0, "{}", s.separazione());
    }
}
