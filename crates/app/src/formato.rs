//! Numeri scritti come li direbbe una persona.

use colossi_fisica::costanti::{ANNO, ANNO_LUCE, DENSITA_ACQUA, DENSITA_ARIA, UA};

pub fn decimale(x: f64) -> String {
    let cifre = if x >= 100.0 {
        0
    } else if x >= 10.0 {
        1
    } else {
        2
    };
    format!("{x:.cifre$}").replace('.', ",")
}

pub fn grande(x: f64) -> String {
    if x >= 1e15 {
        let esponente = x.log10().floor();
        format!("{} × 10^{}", decimale(x / 10f64.powf(esponente)), esponente as i32)
    } else if x >= 1e12 {
        format!("{} mila miliardi", decimale(x / 1e12))
    } else if x >= 1e9 {
        format!("{} miliardi", decimale(x / 1e9))
    } else if x >= 1e6 {
        format!("{} milioni", decimale(x / 1e6))
    } else if x >= 1e3 {
        format!("{} mila", decimale(x / 1e3))
    } else {
        decimale(x)
    }
}

pub fn lunghezza(m: f64) -> String {
    if m < 1e3 {
        format!("{} m", decimale(m))
    } else if m < 0.5 * UA {
        format!("{} km", grande(m / 1e3))
    } else if m < 0.1 * ANNO_LUCE {
        format!("{} UA", grande(m / UA))
    } else {
        format!("{} anni luce", grande(m / ANNO_LUCE))
    }
}

pub fn tempo(s: f64) -> String {
    if s < 1e-3 {
        format!("{} µs", decimale(s * 1e6))
    } else if s < 1.0 {
        format!("{} ms", decimale(s * 1e3))
    } else if s < 120.0 {
        format!("{} s", decimale(s))
    } else if s < 7200.0 {
        format!("{} min", decimale(s / 60.0))
    } else if s < 172_800.0 {
        format!("{} ore", decimale(s / 3600.0))
    } else if s < ANNO {
        format!("{} giorni", decimale(s / 86_400.0))
    } else {
        format!("{} anni", grande(s / ANNO))
    }
}

pub fn densita(rho: f64) -> String {
    if rho >= DENSITA_ACQUA {
        format!("{} volte l'acqua", grande(rho / DENSITA_ACQUA))
    } else if rho >= DENSITA_ARIA {
        "tra l'aria e l'acqua".to_owned()
    } else {
        format!("{} volte meno dell'aria", grande(DENSITA_ARIA / rho))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn esempi() {
        assert_eq!(lunghezza(29_532.0), "29,5 km");
        assert_eq!(grande(4.3e6), "4,30 milioni");
        assert_eq!(tempo(66.0), "66,0 s");
        assert_eq!(densita(0.011), "111 volte meno dell'aria");
    }
}
