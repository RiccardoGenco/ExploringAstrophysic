//! Vita di una stella in forma semplificata: relazioni di scala, non un modello
//! stellare completo. Serve a percepire tempi, grandezze e finali.

/// Massa minima per finire in supernova, in masse solari.
pub const MASSA_SUPERNOVA: f64 = 8.0;
/// Oltre questa massa iniziale il resto e' un buco nero (valore indicativo).
pub const MASSA_BUCO_NERO: f64 = 25.0;
const VITA_SOLE: f64 = 1.0e10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fine {
    NanaBianca,
    StellaDiNeutroni,
    BucoNero,
    /// Stelle cosi' piccole durano piu' dell'eta' dell'universo.
    AncoraViva,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Istante {
    pub fase: &'static str,
    /// Luminosita' e raggio in unita' solari, temperatura in kelvin.
    pub luminosita: f64,
    pub raggio: f64,
    pub temperatura: f64,
}

fn luminosita_principale(m: f64) -> f64 {
    if m < 0.43 {
        0.23 * m.powf(2.3)
    } else if m < 2.0 {
        m.powi(4)
    } else if m < 55.0 {
        1.4 * m.powf(3.5)
    } else {
        32_000.0 * m
    }
}

fn raggio_principale(m: f64) -> f64 {
    if m < 1.0 {
        m.powf(0.8)
    } else {
        m.powf(0.57)
    }
}

fn temperatura_da(l: f64, r: f64) -> f64 {
    5772.0 * (l / (r * r)).powf(0.25)
}

/// Durata della sequenza principale, in anni.
pub fn durata_principale(m: f64) -> f64 {
    VITA_SOLE * m / luminosita_principale(m)
}

/// Durata complessiva fino alla fine, in anni.
pub fn durata_totale(m: f64) -> f64 {
    durata_principale(m) * 1.13
}

/// Il finale di una stella di massa iniziale `m`.
pub fn fine(m: f64) -> Fine {
    if durata_totale(m) > 1.38e10 {
        Fine::AncoraViva
    } else if m < MASSA_SUPERNOVA {
        Fine::NanaBianca
    } else if m < MASSA_BUCO_NERO {
        Fine::StellaDiNeutroni
    } else {
        Fine::BucoNero
    }
}

/// Massa del resto, in masse solari. Relazione empirica per le nane bianche.
pub fn massa_resto(m: f64) -> f64 {
    match fine(m) {
        Fine::NanaBianca => 0.109 * m + 0.394,
        Fine::StellaDiNeutroni => 1.4,
        Fine::BucoNero => 0.25 * m,
        Fine::AncoraViva => m,
    }
}

/// Stato della stella alla frazione `k` (da 0 a 1) della sua vita.
pub fn istante(m: f64, k: f64) -> Istante {
    let k = k.clamp(0.0, 1.0);
    let (l0, r0) = (luminosita_principale(m), raggio_principale(m));
    let massiccia = m >= MASSA_SUPERNOVA;
    let k_fine_principale = 1.0 / 1.13;
    let (fase, l, r) = if k < k_fine_principale {
        let u = k / k_fine_principale;
        ("Sequenza principale: brucia idrogeno", l0 * (1.0 + 0.8 * u), r0 * (1.0 + 0.5 * u))
    } else {
        let u = (k - k_fine_principale) / (1.0 - k_fine_principale);
        if massiccia {
            // Supergigante: la luminosita' cambia poco, il raggio moltissimo.
            let raggio = r0 * (1.5 + 90.0 * u.powf(0.6));
            ("Supergigante: brucia elementi via via piu' pesanti", l0 * (1.8 + 0.8 * u), raggio)
        } else {
            let esponente = u.powf(1.5);
            let raggio = r0 * 1.5 * 150f64.powf(esponente);
            ("Gigante rossa", l0 * 1.8 * 1500f64.powf(esponente), raggio)
        }
    };
    Istante { fase, luminosita: l, raggio: r, temperatura: temperatura_da(l, r) }
}

/// Stima del colore BP-RP dalla temperatura (relazione di corpo nero tarata sul Sole).
pub fn colore_gaia(t: f64) -> f64 {
    // Inverte in modo semplice la formula di Ballesteros per B-V.
    let mut c = 0.8;
    for _ in 0..60 {
        let stima = 4600.0 * (1.0 / (0.92 * c + 1.7) + 1.0 / (0.92 * c + 0.62));
        c += (stima - t) * 5e-5;
        c = c.clamp(-0.4, 4.5);
    }
    c * 1.25
}

/// Magnitudine assoluta approssimata dalla luminosita'.
pub fn magnitudine(l: f64) -> f64 {
    4.67 - 2.5 * l.log10()
}

/// Stelle note, per il confronto delle grandezze. Raggi in unita' solari, arrotondati.
pub const STELLE_NOTE: [(&str, f64); 9] = [
    ("Proxima Centauri", 0.15),
    ("Il Sole", 1.0),
    ("Sirio A", 1.71),
    ("Vega", 2.5),
    ("Arturo", 25.0),
    ("Aldebaran", 45.0),
    ("Rigel", 79.0),
    ("Betelgeuse", 760.0),
    ("VY Canis Majoris", 1400.0),
];

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_sole_vive_dieci_miliardi_di_anni() {
        assert!((durata_principale(1.0) / 1e10 - 1.0).abs() < 1e-9);
        let oggi = istante(1.0, 0.0);
        assert!((oggi.temperatura - 5772.0).abs() < 1.0);
    }

    #[test]
    fn piu_massa_meno_vita() {
        let mut prima = f64::MAX;
        for m in [0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 40.0] {
            let d = durata_principale(m);
            assert!(d < prima, "{m}");
            prima = d;
        }
    }

    #[test]
    fn i_finali_dipendono_dalla_massa() {
        assert_eq!(fine(0.3), Fine::AncoraViva);
        assert_eq!(fine(1.0), Fine::NanaBianca);
        assert_eq!(fine(12.0), Fine::StellaDiNeutroni);
        assert_eq!(fine(40.0), Fine::BucoNero);
        assert!((massa_resto(1.0) - 0.503).abs() < 1e-9);
    }

    #[test]
    fn il_sole_diventa_una_gigante_fredda_e_grande() {
        let g = istante(1.0, 0.999);
        assert!(g.raggio > 100.0 && g.temperatura < 4500.0, "{g:?}");
    }

    #[test]
    fn il_colore_del_sole() {
        let c = colore_gaia(5772.0);
        assert!((c - 0.82).abs() < 0.12, "{c}");
    }
}
