//! Traiettorie della luce in Schwarzschild.
//!
//! In coordinate cartesiane costruite su (r, theta, phi) la geodetica nulla
//! obbedisce a x'' = -3/2 h^2 x / r^5 (con rs = 1), dove h = |x x x'| si conserva.
//! Lo schema (RK4 con passo proporzionale a r) e' lo stesso di `nero.wgsl`.

/// Passo di integrazione in rapporto alla distanza dal centro.
pub const PASSO_RELATIVO: f64 = 0.06;
pub const PASSI_MASSIMI: usize = 20_000;

pub type V3 = [f64; 3];

fn piu(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn per(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn punto(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
pub fn vettoriale(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn norma(a: V3) -> f64 {
    punto(a, a).sqrt()
}

fn accelerazione(x: V3, h2: f64) -> V3 {
    let r2 = punto(x, x);
    per(x, -1.5 * h2 / (r2 * r2 * r2.sqrt()))
}

/// Un passo di RK4.
pub fn passo(x: V3, v: V3, h2: f64, d: f64) -> (V3, V3) {
    let k1x = v;
    let k1v = accelerazione(x, h2);
    let k2x = piu(v, per(k1v, d / 2.0));
    let k2v = accelerazione(piu(x, per(k1x, d / 2.0)), h2);
    let k3x = piu(v, per(k2v, d / 2.0));
    let k3v = accelerazione(piu(x, per(k2x, d / 2.0)), h2);
    let k4x = piu(v, per(k3v, d));
    let k4v = accelerazione(piu(x, per(k3x, d)), h2);
    let dx = piu(piu(k1x, per(piu(k2x, k3x), 2.0)), k4x);
    let dv = piu(piu(k1v, per(piu(k2v, k3v), 2.0)), k4v);
    (piu(x, per(dx, d / 6.0)), piu(v, per(dv, d / 6.0)))
}

/// Converte una direzione misurata da un osservatore fermo in `x` nella
/// direzione coordinata da integrare. Restituisce anche il parametro d'impatto.
pub fn raggio_da_osservatore(x: V3, locale: V3) -> (V3, f64) {
    let r = norma(x);
    let radiale = per(x, 1.0 / r);
    let vr = punto(locale, radiale);
    let tangenziale = piu(locale, per(radiale, -vr));
    let radice = (1.0 - 1.0 / r).sqrt();
    let v = piu(per(radiale, vr * radice), tangenziale);
    (v, r * norma(tangenziale) / radice)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Esito {
    Catturato,
    Fuggito(V3),
    Indeciso,
}

/// Segue un raggio finche' non cade nell'orizzonte o supera `r_fuga`.
pub fn segui(mut x: V3, mut v: V3, r_fuga: f64) -> Esito {
    let h2 = {
        let h = vettoriale(x, v);
        punto(h, h)
    };
    for _ in 0..PASSI_MASSIMI {
        let r = norma(x);
        if r <= 1.0 {
            return Esito::Catturato;
        }
        if r >= r_fuga && punto(x, v) > 0.0 {
            let n = norma(v);
            return Esito::Fuggito(per(v, 1.0 / n));
        }
        (x, v) = passo(x, v, h2, PASSO_RELATIVO * r / norma(v));
    }
    Esito::Indeciso
}

/// Angolo di deviazione totale di un raggio che arriva da lontano con parametro `b`.
pub fn deviazione(b: f64) -> Option<f64> {
    let partenza = b.max(1.0) * 1e5;
    match segui([-partenza, b, 0.0], [1.0, 0.0, 0.0], partenza * 2.0) {
        Esito::Fuggito(d) => Some(d[1].atan2(d[0]).abs()),
        _ => None,
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::buco_nero::impatto_critico;
    use crate::costanti::{C, G, MASSA_SOLE};

    #[test]
    fn deviazione_debole() {
        // Al primo ordine, con rs = 1, la deviazione vale 2 / b.
        let b = 2000.0;
        let d = deviazione(b).unwrap();
        let primo = 2.0 / b;
        assert!((d / primo - 1.0).abs() < 2e-3, "{d} contro {primo}");
    }

    #[test]
    fn sole_angolo_di_einstein() {
        // 1,75 secondi d'arco al bordo del Sole.
        let rs = 2.0 * G * MASSA_SOLE / (C * C);
        let d = 2.0 / (6.957e8 / rs);
        let arcosec = d.to_degrees() * 3600.0;
        assert!((arcosec - 1.75).abs() < 0.01, "{arcosec}");
    }

    #[test]
    fn soglia_di_cattura() {
        let c = impatto_critico();
        assert!(deviazione(c + 0.005).is_some(), "appena fuori deve sfuggire");
        assert!(deviazione(c - 0.005).is_none(), "appena dentro deve cadere");
    }

    #[test]
    fn h_si_conserva() {
        let mut x = [-30.0, 4.0, 1.0];
        let mut v = [1.0, 0.0, 0.0];
        let iniziale = vettoriale(x, v);
        let h2 = punto(iniziale, iniziale);
        for _ in 0..300 {
            let r = norma(x);
            if r < 1.1 {
                break;
            }
            (x, v) = passo(x, v, h2, PASSO_RELATIVO * r);
        }
        let finale = vettoriale(x, v);
        assert!((norma(finale) / h2.sqrt() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn integrale_primo_da_osservatore_fermo() {
        // (du/dphi)^2 + u^2 (1 - u) = 1 / b^2, con b calcolato per l'osservatore.
        let x = [8.0, 0.0, 0.0];
        let angolo: f64 = 2.2;
        let (v, b) = raggio_da_osservatore(x, [angolo.cos(), angolo.sin(), 0.0]);
        let h = norma(vettoriale(x, v));
        let r = norma(x);
        let u = 1.0 / r;
        let du = -(punto(x, v) / r) / h;
        let primo = du * du + u * u * (1.0 - u);
        assert!((primo * b * b - 1.0).abs() < 1e-12, "{}", primo * b * b);
    }

    #[test]
    fn ombra_vista_da_vicino() {
        // Angolo critico per un osservatore fermo a r: sin(psi) = b_c sqrt(1-1/r) / r.
        let r: f64 = 10.0;
        let seno = impatto_critico() * (1.0 - 1.0 / r).sqrt() / r;
        let psi = seno.asin();
        let prova = |delta: f64| {
            let a = psi + delta;
            // La direzione e' verso il centro, inclinata di a.
            let (v, _) = raggio_da_osservatore([r, 0.0, 0.0], [-a.cos(), a.sin(), 0.0]);
            segui([r, 0.0, 0.0], v, 1e4)
        };
        assert_eq!(prova(-0.003), Esito::Catturato);
        assert!(matches!(prova(0.003), Esito::Fuggito(_)));
    }
}
