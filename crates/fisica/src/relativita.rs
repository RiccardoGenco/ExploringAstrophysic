//! Relativita' sul banco: linee d'universo, il percorso col massimo tempo, e che
//! cosa fa la materia a una palla di particelle in caduta libera.

use crate::costanti::C;
use crate::luce::{norma, punto, V3};
use std::f64::consts::PI;

/// Velocita' massima consentita a chi disegna una linea d'universo.
pub const VELOCITA_MASSIMA: f64 = 0.995;
const SCARTO_MINIMO: f64 = 0.02;

/// Evento dello spazio-tempo piatto: tempo in anni, posizione in anni luce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Evento {
    pub t: f64,
    pub x: f64,
}

pub fn gamma(beta: f64) -> f64 {
    1.0 / (1.0 - beta * beta).sqrt()
}

/// Lo stesso evento visto da chi si muove a velocita' `beta`.
pub fn boost(e: Evento, beta: f64) -> Evento {
    let g = gamma(beta);
    Evento { t: g * (e.t - beta * e.x), x: g * (e.x - beta * e.t) }
}

fn durata_tratto(a: Evento, b: Evento) -> f64 {
    let (dt, dx) = (b.t - a.t, b.x - a.x);
    (dt * dt - dx * dx).max(0.0).sqrt()
}

/// Tempo segnato da un orologio che percorre la spezzata.
pub fn tempo_proprio(linea: &[Evento]) -> f64 {
    linea.windows(2).map(|c| durata_tratto(c[0], c[1])).sum()
}

/// L'evento della spezzata in cui il suo orologio segna `tau`.
pub fn evento_a_tempo_proprio(linea: &[Evento], tau: f64) -> Option<Evento> {
    let mut restante = tau;
    for c in linea.windows(2) {
        let durata = durata_tratto(c[0], c[1]);
        if restante <= durata && durata > 0.0 {
            let k = restante / durata;
            return Some(Evento {
                t: c[0].t + (c[1].t - c[0].t) * k,
                x: c[0].x + (c[1].x - c[0].x) * k,
            });
        }
        restante -= durata;
    }
    None
}

/// Posizione e tempo proprio trascorso quando la coordinata tempo vale `t`.
pub fn stato_a_tempo(linea: &[Evento], t: f64) -> (f64, f64) {
    let mut tau = 0.0;
    for c in linea.windows(2) {
        if t <= c[1].t {
            let k = ((t - c[0].t) / (c[1].t - c[0].t)).clamp(0.0, 1.0);
            return (
                c[0].x + (c[1].x - c[0].x) * k,
                tau + durata_tratto(c[0], c[1]) * k,
            );
        }
        tau += durata_tratto(c[0], c[1]);
    }
    (linea.last().map_or(0.0, |e| e.x), tau)
}

/// Istante in cui la luce partita da `e` raggiunge la posizione 0.
pub fn arrivo_a_casa(e: Evento) -> f64 {
    e.t + e.x.abs()
}

/// Dove la luce partita da x = 0 all'istante `t0` incontra la spezzata.
pub fn arrivo_sulla_linea(linea: &[Evento], t0: f64) -> Option<Evento> {
    let davanti = |e: Evento| e.t - t0 - e.x.abs();
    for c in linea.windows(2) {
        if davanti(c[0]) <= 0.0 && davanti(c[1]) >= 0.0 {
            let (mut a, mut b) = (0.0, 1.0);
            let punto_a = |k: f64| Evento {
                t: c[0].t + (c[1].t - c[0].t) * k,
                x: c[0].x + (c[1].x - c[0].x) * k,
            };
            for _ in 0..60 {
                let m = 0.5 * (a + b);
                if davanti(punto_a(m)) < 0.0 {
                    a = m;
                } else {
                    b = m;
                }
            }
            return Some(punto_a(b));
        }
    }
    None
}

/// Riporta `voluto` nella zona raggiungibile a meno della velocita' della luce
/// da `prima` e verso `dopo`. `None` se la zona non esiste a quell'istante.
pub fn vincola(prima: Evento, dopo: Option<Evento>, voluto: Evento) -> Option<Evento> {
    let v = VELOCITA_MASSIMA;
    let mut t = voluto.t.max(prima.t + SCARTO_MINIMO);
    if let Some(d) = dopo {
        t = t.min(d.t - SCARTO_MINIMO);
        if t <= prima.t {
            return None;
        }
    }
    let mut basso = prima.x - v * (t - prima.t);
    let mut alto = prima.x + v * (t - prima.t);
    if let Some(d) = dopo {
        basso = basso.max(d.x - v * (d.t - t));
        alto = alto.min(d.x + v * (d.t - t));
    }
    (basso <= alto).then(|| Evento { t, x: voluto.x.clamp(basso, alto) })
}

/// Campo debole vicino a una superficie: anticipo dell'orologio di chi percorre
/// le altezze `h` rispetto a chi resta a quota zero, in secondi.
/// Restituisce quanto guadagna per la quota e quanto perde per la velocita'.
pub fn scarto_orologio(h: &[f64], dt: f64, g: f64) -> (f64, f64) {
    let c2 = C * C;
    let (mut quota, mut velocita) = (0.0, 0.0);
    for c in h.windows(2) {
        let v = (c[1] - c[0]) / dt;
        quota += g * 0.5 * (c[0] + c[1]) * dt / c2;
        velocita += 0.5 * v * v * dt / c2;
    }
    (quota, velocita)
}

/// La caduta libera che collega i due estremi in `n` intervalli.
pub fn caduta_libera(h0: f64, h1: f64, n: usize, dt: f64, g: f64) -> Vec<f64> {
    let durata = n as f64 * dt;
    (0..=n)
        .map(|i| {
            let t = i as f64 * dt;
            let k = t / durata;
            h0 + (h1 - h0) * k + 0.5 * g * t * (durata - t)
        })
        .collect()
}

/// Sposta ogni punto verso il valore che rende massimo il tempo dell'orologio.
pub fn rilassa(h: &mut [f64], dt: f64, g: f64, giri: usize) {
    for _ in 0..giri {
        for i in 1..h.len() - 1 {
            h[i] = 0.5 * (h[i - 1] + h[i + 1]) + 0.5 * g * dt * dt;
        }
    }
}

/// Sfera di densita' uniforme, attraversabile. Unita' con G = 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sfera {
    pub massa: f64,
    pub raggio: f64,
}

impl Sfera {
    pub fn densita(&self) -> f64 {
        self.massa / (4.0 / 3.0 * PI * self.raggio.powi(3))
    }

    pub fn accelerazione(&self, x: V3) -> V3 {
        let r = norma(x);
        let k = if r >= self.raggio {
            -self.massa / (r * r * r)
        } else {
            -self.massa / self.raggio.powi(3)
        };
        [x[0] * k, x[1] * k, x[2] * k]
    }

    /// Tempo tipico di caduta, usato per scegliere il passo.
    pub fn tempo_scala(&self) -> f64 {
        (self.raggio.powi(3) / self.massa).sqrt()
    }

    /// V''/V all'inizio secondo Einstein in campo debole: -4 pi G rho.
    pub fn focalizzazione_teorica(&self, centro: [f64; 2]) -> f64 {
        if centro[0].hypot(centro[1]) < self.raggio {
            -4.0 * PI * self.densita()
        } else {
            0.0
        }
    }

    /// V''/V misurato lasciando cadere davvero una piccola palla.
    pub fn focalizzazione_misurata(&self, centro: [f64; 2]) -> f64 {
        let mut palla = Palla::nuova(*self, centro, self.raggio * 0.01);
        let t = self.tempo_scala() * 0.02;
        palla.avanza(t);
        2.0 * (palla.volume_relativo() - 1.0) / (t * t)
    }
}

const PUNTI_ANELLO: usize = 32;
const INDICE_ANELLO: usize = 7;

#[derive(Clone, Copy, Debug)]
struct Particella {
    x: V3,
    v: V3,
}

/// Palla di particelle lasciata cadere da ferma: centro, sei punti sugli assi
/// (lungo la caduta, di lato, fuori dal piano) e un anello nel piano.
#[derive(Clone, Debug)]
pub struct Palla {
    pub sfera: Sfera,
    pub raggio: f64,
    pub t: f64,
    pub asse_radiale: [f64; 2],
    particelle: Vec<Particella>,
}

fn somma(a: V3, b: V3, k: f64) -> V3 {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}

impl Palla {
    pub fn nuova(sfera: Sfera, centro: [f64; 2], raggio: f64) -> Self {
        let c = [centro[0], 0.0, centro[1]];
        let r = centro[0].hypot(centro[1]);
        let radiale = if r > 1e-9 { [centro[0] / r, 0.0, centro[1] / r] } else { [1.0, 0.0, 0.0] };
        let laterale = [-radiale[2], 0.0, radiale[0]];
        let verticale = [0.0, 1.0, 0.0];

        let mut posizioni = vec![c];
        for asse in [radiale, laterale, verticale] {
            posizioni.push(somma(c, asse, raggio));
            posizioni.push(somma(c, asse, -raggio));
        }
        for i in 0..PUNTI_ANELLO {
            let a = i as f64 / PUNTI_ANELLO as f64 * 2.0 * PI;
            posizioni.push(somma(somma(c, radiale, raggio * a.cos()), laterale, raggio * a.sin()));
        }
        Self {
            sfera,
            raggio,
            t: 0.0,
            asse_radiale: [radiale[0], radiale[2]],
            particelle: posizioni.into_iter().map(|x| Particella { x, v: [0.0; 3] }).collect(),
        }
    }

    fn passo(&mut self, h: f64) {
        let s = self.sfera;
        for p in &mut self.particelle {
            let (x, v) = (p.x, p.v);
            let k1v = s.accelerazione(x);
            let k2x = somma(v, k1v, h / 2.0);
            let k2v = s.accelerazione(somma(x, v, h / 2.0));
            let k3x = somma(v, k2v, h / 2.0);
            let k3v = s.accelerazione(somma(x, k2x, h / 2.0));
            let k4x = somma(v, k3v, h);
            let k4v = s.accelerazione(somma(x, k3x, h));
            for i in 0..3 {
                p.x[i] += h / 6.0 * (v[i] + 2.0 * k2x[i] + 2.0 * k3x[i] + k4x[i]);
                p.v[i] += h / 6.0 * (k1v[i] + 2.0 * k2v[i] + 2.0 * k3v[i] + k4v[i]);
            }
        }
        self.t += h;
    }

    pub fn avanza(&mut self, durata: f64) {
        let massimo = self.sfera.tempo_scala() * 0.004;
        let n = (durata / massimo).ceil().max(1.0);
        for _ in 0..n as usize {
            self.passo(durata / n);
        }
    }

    pub fn centro(&self) -> V3 {
        self.particelle[0].x
    }

    /// Meta' distanza tra i punti opposti: lungo la caduta, di lato, fuori piano.
    pub fn semiassi(&self) -> [f64; 3] {
        [1, 3, 5].map(|i| {
            let (a, b) = (self.particelle[i].x, self.particelle[i + 1].x);
            let d = somma(a, b, -1.0);
            punto(d, d).sqrt() / 2.0
        })
    }

    pub fn volume_relativo(&self) -> f64 {
        self.semiassi().iter().product::<f64>() / self.raggio.powi(3)
    }

    /// Posizioni dell'anello nel piano, come (x, z).
    pub fn anello(&self) -> impl Iterator<Item = [f64; 2]> + '_ {
        self.particelle[INDICE_ANELLO..].iter().map(|p| [p.x[0], p.x[2]])
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn viaggio() -> Vec<Evento> {
        vec![
            Evento { t: 0.0, x: 0.0 },
            Evento { t: 5.0, x: 4.0 },
            Evento { t: 10.0, x: 0.0 },
        ]
    }

    #[test]
    fn il_viaggio_classico_dura_sei_anni() {
        assert!((tempo_proprio(&viaggio()) - 6.0).abs() < 1e-12);
    }

    #[test]
    fn il_tempo_proprio_non_dipende_da_chi_guarda() {
        for beta in [-0.9, -0.3, 0.6, 0.8] {
            let vista: Vec<_> = viaggio().iter().map(|e| boost(*e, beta)).collect();
            assert!((tempo_proprio(&vista) - 6.0).abs() < 1e-9, "{beta}");
        }
    }

    #[test]
    fn i_lampi_arrivano_come_vuole_doppler() {
        // Andata: 3 anni di viaggio, ogni lampo arriva a casa ogni 3 anni.
        let l = viaggio();
        let e = evento_a_tempo_proprio(&l, 3.0).unwrap();
        assert!((arrivo_a_casa(e) - 9.0).abs() < 1e-9);
        // Il lampo partito da casa all'anno 1 raggiunge la svolta.
        let arrivo = arrivo_sulla_linea(&l, 1.0).unwrap();
        assert!((arrivo.t - 5.0).abs() < 1e-6 && (arrivo.x - 4.0).abs() < 1e-6);
    }

    #[test]
    fn stato_a_tempo_a_meta_andata() {
        let (x, tau) = stato_a_tempo(&viaggio(), 2.5);
        assert!((x - 2.0).abs() < 1e-12 && (tau - 1.5).abs() < 1e-12);
    }

    #[test]
    fn non_si_supera_la_luce() {
        let a = Evento { t: 0.0, x: 0.0 };
        let d = Evento { t: 10.0, x: 0.0 };
        let e = vincola(a, Some(d), Evento { t: 5.0, x: 50.0 }).unwrap();
        assert!(e.x <= VELOCITA_MASSIMA * 5.0 + 1e-12);
    }

    #[test]
    fn la_caduta_libera_e_il_massimo_tempo() {
        let (n, dt, g) = (48, 2.0 / 48.0, 9.81);
        let libera = caduta_libera(0.0, 0.0, n, dt, g);
        let totale = |h: &[f64]| {
            let (a, b) = scarto_orologio(h, dt, g);
            a - b
        };
        // g^2 T^3 / (24 c^2): circa 0,36 femtosecondi.
        let atteso = g * g * 8.0 / (24.0 * C * C);
        assert!((totale(&libera) / atteso - 1.0).abs() < 1e-3);

        for (ampiezza, onde) in [(0.5, 1.0), (-0.4, 2.0), (0.1, 5.0)] {
            let storta: Vec<f64> = libera
                .iter()
                .enumerate()
                .map(|(i, h)| h + ampiezza * (i as f64 / n as f64 * PI * onde).sin())
                .collect();
            assert!(totale(&storta) < totale(&libera));
        }
    }

    #[test]
    fn rilassare_porta_alla_caduta_libera() {
        let (n, dt, g) = (48, 2.0 / 48.0, 9.81);
        let mut h = vec![0.0; n + 1];
        h[n / 2] = 9.0;
        rilassa(&mut h, dt, g, 20_000);
        let libera = caduta_libera(0.0, 0.0, n, dt, g);
        assert!(h.iter().zip(&libera).all(|(a, b)| (a - b).abs() < 1e-6));
    }

    #[test]
    fn dentro_la_materia_il_volume_si_contrae_come_dice_einstein() {
        let s = Sfera { massa: 2.0, raggio: 1.5 };
        let c = [0.6, 0.2];
        let misurata = s.focalizzazione_misurata(c);
        let teorica = s.focalizzazione_teorica(c);
        assert!(teorica < 0.0);
        assert!((misurata / teorica - 1.0).abs() < 0.01, "{misurata} contro {teorica}");
    }

    #[test]
    fn nel_vuoto_il_volume_non_cambia() {
        let s = Sfera { massa: 2.0, raggio: 1.5 };
        let c = [2.5, 1.0];
        assert_eq!(s.focalizzazione_teorica(c), 0.0);
        let scala = 4.0 * PI * s.densita();
        assert!((s.focalizzazione_misurata(c) / scala).abs() < 0.01);
    }

    #[test]
    fn nel_vuoto_si_allunga_e_si_stringe() {
        let s = Sfera { massa: 1.0, raggio: 1.0 };
        let mut palla = Palla::nuova(s, [3.0, 0.0], 0.1);
        palla.avanza(2.0);
        let [lungo, largo, fuori] = palla.semiassi();
        assert!(lungo > 0.1 && largo < 0.1 && fuori < 0.1, "{lungo} {largo} {fuori}");
    }

    #[test]
    fn dentro_una_sfera_uniforme_tutti_arrivano_insieme() {
        let s = Sfera { massa: 1.0, raggio: 1.0 };
        let mut palla = Palla::nuova(s, [0.4, 0.0], 0.1);
        palla.avanza(PI / 2.0);
        assert!(palla.semiassi().iter().all(|a| *a < 1e-4));
    }
}
