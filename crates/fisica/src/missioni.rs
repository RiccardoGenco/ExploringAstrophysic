//! Missioni dell'Osservatorio: modelli semplici adattati a misure pubblicate.

use crate::costanti::{C, G, MASSA_SOLE};
use std::f64::consts::PI;

const TEMPO_SOLE: f64 = G * MASSA_SOLE / (C * C * C);
const ITERAZIONI: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Misura {
    pub x: f64,
    pub y: f64,
    pub err: f64,
    pub attiva: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parametro {
    pub nome: &'static str,
    pub min: f64,
    pub max: f64,
    pub valore: f64,
    pub libero: bool,
}

pub fn chi2(m: &dyn Fn(&[f64], f64) -> f64, dati: &[Misura], p: &[f64]) -> f64 {
    dati.iter()
        .filter(|d| d.attiva)
        .map(|d| ((d.y - m(p, d.x)) / d.err).powi(2))
        .sum()
}

fn risolvi(a: &mut [f64], b: &[f64], n: usize) -> Option<Vec<f64>> {
    let mut x = b.to_vec();
    for c in 0..n {
        let pivot = (c..n).max_by(|&p, &q| a[p * n + c].abs().total_cmp(&a[q * n + c].abs()))?;
        if a[pivot * n + c].abs() < 1e-300 {
            return None;
        }
        if pivot != c {
            for k in 0..n {
                a.swap(pivot * n + k, c * n + k);
            }
            x.swap(pivot, c);
        }
        for r in c + 1..n {
            let f = a[r * n + c] / a[c * n + c];
            for k in c..n {
                a[r * n + k] -= f * a[c * n + k];
            }
            x[r] -= f * x[c];
        }
    }
    for c in (0..n).rev() {
        let s: f64 = (c + 1..n).map(|k| a[c * n + k] * x[k]).sum();
        x[c] = (x[c] - s) / a[c * n + c];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

fn normali(m: &dyn Fn(&[f64], f64) -> f64, dati: &[Misura], par: &[Parametro], v: &[f64], lib: &[usize]) -> (Vec<f64>, Vec<f64>) {
    let n = lib.len();
    let mut jtj = vec![0.0; n * n];
    let mut jtr = vec![0.0; n];
    let mut riga = vec![0.0; n];
    let mut prova = v.to_vec();
    for d in dati.iter().filter(|d| d.attiva) {
        let r = (d.y - m(v, d.x)) / d.err;
        for (c, &k) in lib.iter().enumerate() {
            let passo = (par[k].max - par[k].min) * 1e-6;
            prova[k] = v[k] + passo;
            let su = m(&prova, d.x);
            prova[k] = v[k] - passo;
            let giu = m(&prova, d.x);
            prova[k] = v[k];
            riga[c] = (su - giu) / (2.0 * passo * d.err);
        }
        for a in 0..n {
            jtr[a] += riga[a] * r;
            for b in 0..n {
                jtj[a * n + b] += riga[a] * riga[b];
            }
        }
    }
    (jtj, jtr)
}

/// Levenberg-Marquardt entro i limiti dei parametri. Modifica i valori liberi.
pub fn adatta(m: &dyn Fn(&[f64], f64) -> f64, dati: &[Misura], par: &mut [Parametro]) {
    let lib: Vec<usize> = (0..par.len()).filter(|&k| par[k].libero).collect();
    let n = lib.len();
    if n == 0 {
        return;
    }
    let mut v: Vec<f64> = par.iter().map(|p| p.valore).collect();
    let mut chi = chi2(m, dati, &v);
    let mut lambda = 1e-3;
    for _ in 0..ITERAZIONI {
        let (jtj, jtr) = normali(m, dati, par, &v, &lib);
        let mut migliorato = false;
        for _ in 0..30 {
            let mut a = jtj.clone();
            for k in 0..n {
                a[k * n + k] = jtj[k * n + k] * (1.0 + lambda) + 1e-12;
            }
            if let Some(d) = risolvi(&mut a, &jtr, n) {
                let mut prova = v.clone();
                for (c, &k) in lib.iter().enumerate() {
                    prova[k] = (v[k] + d[c]).clamp(par[k].min, par[k].max);
                }
                let c2 = chi2(m, dati, &prova);
                if c2 < chi {
                    let guadagno = chi - c2;
                    v = prova;
                    chi = c2;
                    lambda = (lambda * 0.4).max(1e-10);
                    migliorato = guadagno > 1e-8 * chi.max(1.0);
                    break;
                }
            }
            lambda *= 3.0;
        }
        if !migliorato {
            break;
        }
    }
    for (p, x) in par.iter_mut().zip(v) {
        p.valore = x;
    }
}

/// Incertezze a una deviazione standard dei parametri liberi (zero per i fissi).
pub fn incertezze(m: &dyn Fn(&[f64], f64) -> f64, dati: &[Misura], par: &[Parametro]) -> Option<Vec<f64>> {
    let lib: Vec<usize> = (0..par.len()).filter(|&k| par[k].libero).collect();
    let n = lib.len();
    let v: Vec<f64> = par.iter().map(|p| p.valore).collect();
    let (jtj, _) = normali(m, dati, par, &v, &lib);
    let mut uscita = vec![0.0; par.len()];
    for c in 0..n {
        let mut unita = vec![0.0; n];
        unita[c] = 1.0;
        let colonna = risolvi(&mut jtj.clone(), &unita, n)?;
        uscita[lib[c]] = colonna[c].max(0.0).sqrt();
    }
    Some(uscita)
}

/// Onda gravitazionale della spirale. p = massa chirp, tempo di fusione, a, b.
pub fn ligo(p: &[f64], t: f64) -> f64 {
    let tc = p[0] * TEMPO_SOLE;
    let tau = p[1] - t;
    if tau < 0.0015 {
        // Il modello descrive solo la spirale: dopo la fusione non dice nulla.
        return 0.0;
    }
    let f = (5.0 / (256.0 * tau)).powf(3.0 / 8.0) * tc.powf(-5.0 / 8.0) / PI;
    let fase = -2.0 * (tau / (5.0 * tc)).powf(5.0 / 8.0);
    (f / 100.0).powf(2.0 / 3.0) * (p[2] * fase.cos() + p[3] * fase.sin())
}

/// Dato chirp e fusione, sceglie le ampiezze migliori (il problema e' lineare).
fn ampiezze(dati: &[Misura], mc: f64, t0: f64) -> (f64, f64, f64) {
    let (mut aa, mut ab, mut bb, mut ya, mut yb, mut yy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for d in dati.iter().filter(|d| d.attiva) {
        let c = ligo(&[mc, t0, 1.0, 0.0], d.x);
        let s = ligo(&[mc, t0, 0.0, 1.0], d.x);
        aa += c * c;
        ab += c * s;
        bb += s * s;
        ya += d.y * c;
        yb += d.y * s;
        yy += d.y * d.y;
    }
    let det = aa * bb - ab * ab;
    if det.abs() < 1e-30 {
        return (0.0, 0.0, yy);
    }
    let (a, b) = ((ya * bb - yb * ab) / det, (yb * aa - ya * ab) / det);
    (a, b, yy - a * ya - b * yb)
}

/// Cerca a tappeto chirp e istante della fusione, prima dell'adattamento fine.
pub fn cerca_ligo(dati: &[Misura], par: &mut [Parametro]) {
    let mut migliore = (f64::MAX, par[0].valore, par[1].valore, 0.0, 0.0);
    for i in 0..=70 {
        let mc = 15.0 + f64::from(i) * 0.5;
        for j in 0..=240 {
            let t0 = 0.40 + f64::from(j) * 0.00025;
            let (a, b, resto) = ampiezze(dati, mc, t0);
            if resto < migliore.0 {
                migliore = (resto, mc, t0, a, b);
            }
        }
    }
    par[0].valore = migliore.1;
    par[1].valore = migliore.2;
    par[2].valore = migliore.3.clamp(par[2].min, par[2].max);
    par[3].valore = migliore.4.clamp(par[3].min, par[3].max);
}

/// Distanza di luminosita' in Mpc. p = Omega materia, Omega energia oscura, H0.
fn distanza_luminosita(om: f64, ol: f64, h0: f64, z: f64) -> f64 {
    let ok = 1.0 - om - ol;
    let e = |zz: f64| {
        let a = 1.0 + zz;
        (om * a * a * a + ok * a * a + ol).max(1e-9).sqrt()
    };
    let n = 40;
    let h = z / f64::from(n);
    let mut somma = 1.0 / e(0.0) + 1.0 / e(z);
    for i in 1..n {
        somma += (if i % 2 == 1 { 4.0 } else { 2.0 }) / e(f64::from(i) * h);
    }
    let chi = somma * h / 3.0;
    let curvo = if ok > 1e-8 {
        (ok.sqrt() * chi).sinh() / ok.sqrt()
    } else if ok < -1e-8 {
        ((-ok).sqrt() * chi).sin() / (-ok).sqrt()
    } else {
        chi
    };
    (1.0 + z) * curvo * (C / 1e3) / h0
}

/// Modulo di distanza di una supernova a redshift `z`.
pub fn supernova(p: &[f64], z: f64) -> f64 {
    5.0 * distanza_luminosita(p[0], p[1], p[2], z).log10() + 25.0
}

/// Decadimento dell'orbita da onde gravitazionali (Peters), adimensionale.
pub fn dp_dt_relativita(m1: f64, m2: f64, periodo_s: f64, e: f64) -> f64 {
    let m = m1 + m2;
    let f = (1.0 + 73.0 / 24.0 * e * e + 37.0 / 96.0 * e.powi(4)) / (1.0 - e * e).powf(3.5);
    -192.0 * PI / 5.0 * (2.0 * PI * TEMPO_SOLE / periodo_s).powf(5.0 / 3.0) * m1 * m2 / m.powf(1.0 / 3.0) * f
}

pub const PERIODO_HT: f64 = 0.322_997_448_918 * 86_400.0;
pub const ECCENTRICITA_HT: f64 = 0.617_134;
const SECONDI_ANNO: f64 = 3.155_76e7;
pub const ANNO_INIZIO_HT: f64 = 1975.0;
/// Intrinseco, Weisberg e Huang 2016.
pub const DP_HT: f64 = -2.398e-12;
pub const DP_HT_ERRORE: f64 = 0.004e-12;
pub const MASSA_P_HT: f64 = 1.438;
pub const MASSA_C_HT: f64 = 1.390;

/// Scarto accumulato nel passaggio al periastro, in secondi. p = massa della pulsar.
pub fn hulse_taylor(p: &[f64], anno: f64) -> f64 {
    let dp = dp_dt_relativita(p[0], MASSA_C_HT, PERIODO_HT, ECCENTRICITA_HT);
    let t = (anno - ANNO_INIZIO_HT) * SECONDI_ANNO;
    0.5 * dp / PERIODO_HT * t * t
}

/// Parabola attorno al primo picco. p = altezza, posizione, larghezza.
pub fn parabola(p: &[f64], x: f64) -> f64 {
    p[0] * (1.0 - ((x - p[1]) / p[2]).powi(2))
}

/// Sequenza principale nel diagramma colore-magnitudine.
pub fn sequenza(p: &[f64], c: f64) -> f64 {
    p[0] + p[1] * c + p[2] * c * c
}

/// Una riga di una curva di rotazione, con i contributi visibili.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RigaRotazione {
    pub r: f64,
    pub gas: f64,
    pub disco: f64,
    pub bulbo: f64,
}

/// p = massa per luce del disco, del bulbo, velocita' asintotica, raggio del nucleo.
pub fn rotazione(righe: &[RigaRotazione], p: &[f64], r: f64) -> f64 {
    let r = r.max(righe[0].r);
    let i = righe.partition_point(|q| q.r < r).clamp(1, righe.len().max(2) - 1);
    let (a, b) = (righe[i - 1], righe[i.min(righe.len() - 1)]);
    let k = if b.r > a.r { ((r - a.r) / (b.r - a.r)).clamp(0.0, 1.0) } else { 0.0 };
    let q = RigaRotazione {
        r,
        gas: a.gas + (b.gas - a.gas) * k,
        disco: a.disco + (b.disco - a.disco) * k,
        bulbo: a.bulbo + (b.bulbo - a.bulbo) * k,
    };
    let alone = p[2] * p[2] * (1.0 - p[3] / r * (r / p[3]).atan());
    (q.gas * q.gas.abs() + p[0] * q.disco * q.disco + p[1] * q.bulbo * q.bulbo + alone).max(0.0).sqrt()
}

#[cfg(test)]
mod test {
    use super::*;

    fn dati(x: &[f64], m: &dyn Fn(f64) -> f64) -> Vec<Misura> {
        x.iter().map(|&q| Misura { x: q, y: m(q), err: 1.0, attiva: true }).collect()
    }

    fn par(nome: &'static str, valore: f64, min: f64, max: f64) -> Parametro {
        Parametro { nome, min, max, valore, libero: true }
    }

    #[test]
    fn l_adattamento_ritrova_una_parabola() {
        let x: Vec<f64> = (0..50).map(f64::from).collect();
        let d = dati(&x, &|q| parabola(&[5.0, 20.0, 15.0], q));
        let mut p = [par("h", 3.0, 0.0, 10.0), par("x", 24.0, 0.0, 50.0), par("w", 10.0, 1.0, 40.0)];
        adatta(&parabola, &d, &mut p);
        assert!((p[1].valore - 20.0).abs() < 1e-3, "{:?}", p[1]);
        assert!(incertezze(&parabola, &d, &p).is_some());
    }

    #[test]
    fn hulse_taylor_come_la_relativita_generale() {
        let d = dp_dt_relativita(MASSA_P_HT, MASSA_C_HT, PERIODO_HT, ECCENTRICITA_HT);
        assert!((d / -2.402_63e-12 - 1.0).abs() < 1e-3, "{d}");
        assert!((DP_HT / d - 0.9983).abs() < 0.002);
    }

    #[test]
    fn le_distanze_sono_quelle_di_un_universo_noto() {
        // Einstein-de Sitter: d_L = 2 c / H0 (1 + z) (1 - 1 / sqrt(1 + z)).
        let z: f64 = 1.0;
        let d = distanza_luminosita(1.0, 0.0, 70.0, z);
        let attesa = 2.0 * (C / 1e3) / 70.0 * (1.0 + z) * (1.0 - 1.0 / (1.0 + z).sqrt());
        assert!((d / attesa - 1.0).abs() < 1e-5, "{d} {attesa}");
    }

    #[test]
    fn la_curva_di_rotazione_senza_alone_e_solo_il_visibile() {
        let r = [RigaRotazione { r: 5.0, gas: 10.0, disco: 50.0, bulbo: 0.0 }, RigaRotazione { r: 6.0, gas: 10.0, disco: 50.0, bulbo: 0.0 }];
        let v = rotazione(&r, &[1.0, 0.0, 0.0, 1.0], 5.0);
        assert!((v - (100.0_f64 + 2500.0).sqrt()).abs() < 1e-9);
    }

    #[test]
    fn la_ricerca_ligo_trova_un_segnale_sintetico() {
        let x: Vec<f64> = (0..600).map(|i| 0.30 + f64::from(i) * 0.00016).collect();
        let vero = [31.0, 0.423, 0.3, -0.2];
        let d = dati(&x, &|q| ligo(&vero, q));
        let mut p = [par("m", 20.0, 10.0, 60.0), par("t", 0.41, 0.4, 0.46), par("a", 0.0, -3.0, 3.0), par("b", 0.0, -3.0, 3.0)];
        cerca_ligo(&d, &mut p);
        adatta(&ligo, &d, &mut p);
        assert!((p[0].valore - 31.0).abs() < 0.3, "{:?}", p[0]);
    }
}
