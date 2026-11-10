//! Osservatorio: orbite kepleriane adattate a misure reali di posizione e velocita'.

use std::f64::consts::{PI, TAU};
use std::fmt::Write as _;

/// Da unita' astronomiche all'anno a km/s.
const UA_ANNO_IN_KMS: f64 = 4.740_47;
/// Epoca di riferimento per la deriva del sistema di coordinate.
const EPOCA: f64 = 2000.0;
const N_PARAMETRI: usize = 13;
const ITERAZIONI: usize = 200;

/// Un'orbita vista dalla Terra. Angoli in gradi, a in millesimi d'arco, tempi in
/// anni, distanza in kiloparsec, offset in mas e mas/anno, vz in km/s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Orbita {
    pub a: f64,
    pub e: f64,
    pub i: f64,
    pub nodo: f64,
    pub periastro: f64,
    pub t0: f64,
    pub p: f64,
    pub r0: f64,
    pub x0: f64,
    pub y0: f64,
    pub vx: f64,
    pub vy: f64,
    pub vz: f64,
}

impl Orbita {
    fn in_vettore(&self) -> [f64; N_PARAMETRI] {
        [
            self.a, self.e, self.i, self.nodo, self.periastro, self.t0, self.p, self.r0, self.x0,
            self.y0, self.vx, self.vy, self.vz,
        ]
    }

    fn da_vettore(v: [f64; N_PARAMETRI]) -> Self {
        Self {
            a: v[0],
            e: v[1],
            i: v[2],
            nodo: v[3],
            periastro: v[4],
            t0: v[5],
            p: v[6],
            r0: v[7],
            x0: v[8],
            y0: v[9],
            vx: v[10],
            vy: v[11],
            vz: v[12],
        }
    }

    /// Semiasse maggiore in unita' astronomiche.
    pub fn a_ua(&self) -> f64 {
        self.a * self.r0
    }

    /// Massa centrale, in masse solari, dalla terza legge di Keplero.
    pub fn massa(&self) -> f64 {
        self.a_ua().powi(3) / (self.p * self.p)
    }

    /// Distanza minima dal centro, in unita' astronomiche.
    pub fn distanza_minima(&self) -> f64 {
        self.a_ua() * (1.0 - self.e)
    }

    /// Velocita' nel punto piu' vicino, in km/s.
    pub fn velocita_massima(&self) -> f64 {
        TAU * self.a_ua() / self.p * ((1.0 + self.e) / (1.0 - self.e)).sqrt() * UA_ANNO_IN_KMS
    }

    fn anomalia_eccentrica(&self, t: f64) -> f64 {
        let media = (TAU * (t - self.t0) / self.p + PI).rem_euclid(TAU) - PI;
        let mut e_a = if self.e > 0.8 { PI * media.signum() } else { media };
        for _ in 0..60 {
            let delta = (e_a - self.e * e_a.sin() - media) / (1.0 - self.e * e_a.cos());
            e_a -= delta;
            if delta.abs() < 1e-13 {
                break;
            }
        }
        e_a
    }

    /// Posizione apparente: (est, nord), in mas.
    pub fn posizione(&self, t: f64) -> (f64, f64) {
        let e_a = self.anomalia_eccentrica(t);
        let (x, y) = (e_a.cos() - self.e, (1.0 - self.e * self.e).sqrt() * e_a.sin());
        let (som, com) = self.periastro.to_radians().sin_cos();
        let (son, con) = self.nodo.to_radians().sin_cos();
        let ci = self.i.to_radians().cos();
        let a = self.a * (com * con - som * son * ci);
        let b = self.a * (com * son + som * con * ci);
        let f = self.a * (-som * con - com * son * ci);
        let g = self.a * (-som * son + com * con * ci);
        let dt = t - EPOCA;
        (
            // Segno scelto per riprodurre gli angoli pubblicati per S2 (i ~ 135, nodo ~ 226).
            -(b * x + g * y) + self.x0 + self.vx * dt,
            a * x + f * y + self.y0 + self.vy * dt,
        )
    }

    /// Velocita' lungo la linea di vista, positiva se si allontana, in km/s.
    pub fn velocita_radiale(&self, t: f64) -> f64 {
        let e_a = self.anomalia_eccentrica(t);
        let vera = 2.0 * ((1.0 + self.e).sqrt() * (e_a / 2.0).sin())
            .atan2((1.0 - self.e).sqrt() * (e_a / 2.0).cos());
        let w = self.periastro.to_radians();
        let k = TAU * self.a_ua() * self.i.to_radians().sin()
            / (self.p * (1.0 - self.e * self.e).sqrt())
            * UA_ANNO_IN_KMS;
        k * ((vera + w).cos() + self.e * w.cos()) + self.vz
    }
}

/// Punto di partenza volutamente sbagliato: tocca a chi usa il banco migliorarlo.
pub const INIZIO: Orbita = Orbita {
    a: 90.0,
    e: 0.5,
    i: 110.0,
    nodo: 200.0,
    periastro: 40.0,
    t0: 2003.0,
    p: 20.0,
    r0: 8.0,
    x0: 0.0,
    y0: 0.0,
    vx: 0.0,
    vy: 0.0,
    vz: 0.0,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Posizione {
    pub t: f64,
    pub est: f64,
    pub err_est: f64,
    pub nord: f64,
    pub err_nord: f64,
    pub attiva: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Velocita {
    pub t: f64,
    pub v: f64,
    pub err: f64,
    pub attiva: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dati {
    pub posizioni: Vec<Posizione>,
    pub velocita: Vec<Velocita>,
}

/// Errori combinati in quadratura, come chiede l'articolo.
fn pos(t: f64, est: f64, e1: f64, e2: f64, nord: f64, n1: f64, n2: f64) -> Posizione {
    Posizione {
        t,
        est,
        err_est: e1.hypot(e2),
        nord,
        err_nord: n1.hypot(n2),
        attiva: true,
    }
}

fn vel(t: f64, v: f64, err: f64) -> Velocita {
    Velocita { t, v, err, attiva: true }
}

/// Stella S0-2 (S2): misure Keck pubblicate in Ghez et al. 2008, ApJ 689, 1044,
/// tabelle 3 e 4, piu' quattro velocita' VLT citate da quell'articolo. La quinta non
/// e' pubblicata in forma numerica. X verso est, Y verso nord, velocita' nel sistema
/// locale.
pub fn s2_ghez_2008() -> Dati {
    Dati {
        posizioni: vec![
            pos(1995.439, 42.6, 1.0, 0.0, 164.10, 0.98, 0.0),
            pos(1996.485, 53.0, 9.5, 0.0, 155.4, 9.5, 0.0),
            pos(1997.367, 56.5, 1.7, 0.0, 137.0, 1.7, 0.0),
            pos(1999.333, 66.6, 3.1, 0.0, 91.5, 3.1, 0.0),
            pos(1999.559, 67.4, 1.4, 0.0, 88.3, 1.4, 0.0),
            pos(2000.305, 64.3, 3.0, 0.0, 65.8, 3.1, 0.0),
            pos(2000.381, 66.7, 1.1, 0.0, 63.0, 1.1, 0.0),
            pos(2000.548, 64.84, 0.78, 0.0, 57.94, 0.80, 0.0),
            pos(2000.797, 65.4, 4.8, 0.0, 46.8, 4.9, 0.0),
            pos(2001.351, 56.7, 1.6, 0.0, 26.5, 1.6, 0.0),
            pos(2001.572, 53.0, 1.4, 0.0, 14.2, 1.3, 0.0),
            pos(2003.303, -34.9, 1.5, 0.0, 69.5, 1.6, 0.0),
            pos(2003.554, -35.45, 0.90, 0.0, 81.04, 0.90, 0.0),
            pos(2003.682, -34.5, 2.3, 0.0, 87.4, 2.3, 0.0),
            pos(2004.327, -32.15, 0.84, 0.0, 113.95, 0.86, 0.0),
            pos(2004.564, -28.7, 1.4, 0.0, 121.3, 1.5, 0.0),
            pos(2004.567, -28.4, 1.4, 0.0, 122.9, 1.4, 0.0),
            pos(2004.660, -26.8, 1.1, 0.0, 125.5, 1.1, 0.0),
            pos(2005.312, -18.58, 0.88, 0.0, 142.43, 0.92, 0.0),
            pos(2005.495, -18.6, 1.0, 1.1, 145.3, 1.0, 2.5),
            pos(2005.566, -15.3, 1.7, 0.0, 148.9, 1.8, 0.0),
            pos(2005.580, -16.9, 0.23, 1.0, 146.8, 0.23, 1.5),
            pos(2006.336, -7.97, 0.13, 0.77, 159.82, 0.13, 0.66),
            pos(2006.470, -6.01, 0.14, 0.77, 161.57, 0.14, 0.66),
            pos(2006.541, -4.89, 0.17, 0.77, 162.26, 0.17, 0.66),
            pos(2007.612, 6.88, 0.20, 0.77, 173.47, 0.20, 0.66),
        ],
        velocita: vec![
            vel(2000.487, 1199.0, 100.0),
            vel(2003.438, -1550.0, 22.0),
            vel(2004.474, -1143.0, 57.0),
            vel(2005.410, -926.0, 16.0),
            vel(2005.504, -850.0, 31.0),
            vel(2006.391, -692.0, 21.0),
            vel(2006.461, -718.0, 17.0),
            vel(2006.495, -695.0, 36.0),
            vel(2006.497, -713.0, 26.0),
            vel(2007.385, -483.0, 50.0),
            vel(2007.548, -506.0, 50.0),
            // Quattro delle cinque epoche aggiunte dall'articolo, dalle fonti originali.
            vel(2003.27, -1545.0, 25.0), // Eisenhauer et al. 2003, SPIFFI
            vel(2003.35, -1512.0, 35.0), // Eisenhauer et al. 2003, NACO
            vel(2004.537, -1070.0, 25.0), // Eisenhauer et al. 2005, SINFONI
            vel(2004.632, -1080.0, 20.0), // Eisenhauer et al. 2005, SINFONI
        ],
    }
}

/// Incertezze di un adattamento, a una deviazione standard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incertezze {
    pub massa: f64,
    pub r0: f64,
    pub a: f64,
    pub e: f64,
    pub p: f64,
}

/// Il risultato pubblicato, per il confronto finale.
pub struct Pubblicato {
    pub descrizione: &'static str,
    pub massa: f64,
    pub massa_errore: f64,
    pub r0: f64,
    pub r0_errore: f64,
    pub a: f64,
    pub e: f64,
    pub p: f64,
}

/// Ghez et al. 2008: con il buco nero fermo e con velocita' lungo la vista libera.
pub const GHEZ_VZ_FISSA: Pubblicato = Pubblicato {
    descrizione: "Ghez et al. 2008, buco nero fermo",
    massa: 4.5e6,
    massa_errore: 0.4e6,
    r0: 8.4,
    r0_errore: 0.4,
    a: 124.4,
    e: 0.8866,
    p: 15.78,
};

pub const GHEZ_VZ_LIBERA: Pubblicato = Pubblicato {
    descrizione: "Ghez et al. 2008, velocità del buco nero libera",
    massa: 4.1e6,
    massa_errore: 0.6e6,
    r0: 8.0,
    r0_errore: 0.6,
    a: 126.5,
    e: 0.8904,
    p: 15.86,
};

impl Dati {
    /// Numero di misure attive (ogni posizione vale due).
    pub fn numero(&self) -> usize {
        2 * self.posizioni.iter().filter(|p| p.attiva).count()
            + self.velocita.iter().filter(|v| v.attiva).count()
    }

    fn residui(&self, o: &Orbita) -> Vec<f64> {
        let mut r = Vec::with_capacity(self.numero());
        for p in self.posizioni.iter().filter(|p| p.attiva) {
            let (est, nord) = o.posizione(p.t);
            r.push((p.est - est) / p.err_est);
            r.push((p.nord - nord) / p.err_nord);
        }
        for v in self.velocita.iter().filter(|v| v.attiva) {
            r.push((v.v - o.velocita_radiale(v.t)) / v.err);
        }
        r
    }

    pub fn chi2(&self, o: &Orbita) -> f64 {
        self.residui(o).iter().map(|r| r * r).sum()
    }

    /// Chi quadro per grado di liberta': attorno a 1 l'orbita spiega le misure.
    pub fn chi2_ridotto(&self, o: &Orbita) -> f64 {
        self.chi2(o) / (self.numero() as f64 - N_PARAMETRI as f64).max(1.0)
    }

    /// Cerca l'orbita migliore partendo da `inizio` (Levenberg-Marquardt).
    /// Con `vz_fissa` il buco nero non si muove lungo la linea di vista.
    pub fn adatta(&self, inizio: &Orbita, vz_fissa: bool) -> Orbita {
        let mut attuale = *inizio;
        if vz_fissa {
            attuale.vz = 0.0;
        }
        // Prima solo la forma dell'orbita, poi tutto: dal lontano e' piu' stabile.
        let forma: Vec<usize> = (0..8).collect();
        let liberi: Vec<usize> = (0..N_PARAMETRI).filter(|&k| !(vz_fissa && k == 12)).collect();
        attuale = self.minimizza(attuale, &forma);
        self.minimizza(attuale, &liberi)
    }

    fn minimizza(&self, inizio: Orbita, liberi: &[usize]) -> Orbita {
        let n = liberi.len();
        let mut v = inizio.in_vettore();
        let mut lambda = 1e-2;
        let mut chi = self.chi2(&inizio);

        for _ in 0..ITERAZIONI {
            let r0 = self.residui(&Orbita::da_vettore(v));
            let m = r0.len();
            if m < n {
                break;
            }
            // Jacobiano numerico.
            let mut j = vec![0.0; m * n];
            for (c, &k) in liberi.iter().enumerate() {
                let passo = (v[k].abs() * 1e-6).max(1e-6);
                let (mut piu, mut meno) = (v, v);
                piu[k] += passo;
                meno[k] -= passo;
                let rp = self.residui(&Orbita::da_vettore(piu));
                let rm = self.residui(&Orbita::da_vettore(meno));
                for riga in 0..m {
                    j[riga * n + c] = (rp[riga] - rm[riga]) / (2.0 * passo);
                }
            }
            let mut jtj = vec![0.0; n * n];
            let mut jtr = vec![0.0; n];
            for riga in 0..m {
                for a in 0..n {
                    jtr[a] += j[riga * n + a] * r0[riga];
                    for b in 0..n {
                        jtj[a * n + b] += j[riga * n + a] * j[riga * n + b];
                    }
                }
            }

            let mut migliorato = false;
            for _ in 0..24 {
                let mut matrice = jtj.clone();
                for a in 0..n {
                    // Il piccolo termine fisso regge i parametri che i dati non vincolano.
                    matrice[a * n + a] = jtj[a * n + a] * (1.0 + lambda) + 1e-9;
                }
                let Some(delta) = risolvi(&mut matrice, &jtr, n) else {
                    lambda *= 4.0;
                    continue;
                };
                let mut prova = v;
                for (c, &k) in liberi.iter().enumerate() {
                    // Passo di Gauss-Newton: J^T J d = J^T r, e si va verso -d.
                    prova[k] -= delta[c];
                }
                let valida = prova[1] >= 0.0 && prova[1] < 0.995 && prova[0] > 1.0 && prova[6] > 0.5 && prova[7] > 0.1;
                let nuova = valida.then(|| self.chi2(&Orbita::da_vettore(prova)));
                if let Some(c2) = nuova.filter(|c| *c < chi) {
                    let guadagno = chi - c2;
                    v = prova;
                    chi = c2;
                    lambda = (lambda * 0.35).max(1e-9);
                    migliorato = guadagno > 1e-9 * chi.max(1.0);
                    break;
                }
                lambda *= 4.0;
            }
            if !migliorato {
                break;
            }
        }
        Orbita::da_vettore(v)
    }

    /// Incertezze a una deviazione standard, dalla matrice di covarianza dell'adattamento.
    pub fn incertezze(&self, o: &Orbita, vz_fissa: bool) -> Option<Incertezze> {
        let liberi: Vec<usize> = (0..N_PARAMETRI).filter(|&k| !(vz_fissa && k == 12)).collect();
        let n = liberi.len();
        let v = o.in_vettore();
        let m = self.numero();
        if m <= n {
            return None;
        }
        let mut j = vec![0.0; m * n];
        for (c, &k) in liberi.iter().enumerate() {
            let passo = (v[k].abs() * 1e-6).max(1e-6);
            let (mut piu, mut meno) = (v, v);
            piu[k] += passo;
            meno[k] -= passo;
            let rp = self.residui(&Orbita::da_vettore(piu));
            let rm = self.residui(&Orbita::da_vettore(meno));
            for riga in 0..m {
                j[riga * n + c] = (rp[riga] - rm[riga]) / (2.0 * passo);
            }
        }
        let mut jtj = vec![0.0; n * n];
        for riga in 0..m {
            for a in 0..n {
                for b in 0..n {
                    jtj[a * n + b] += j[riga * n + a] * j[riga * n + b];
                }
            }
        }
        let mut covarianza = vec![0.0; n * n];
        for c in 0..n {
            let mut unita = vec![0.0; n];
            unita[c] = 1.0;
            let colonna = risolvi(&mut jtj.clone(), &unita, n)?;
            for r in 0..n {
                covarianza[r * n + c] = colonna[r];
            }
        }
        let indice = |k: usize| liberi.iter().position(|&x| x == k);
        let sigma = |k: usize| indice(k).map_or(0.0, |c| covarianza[c * n + c].max(0.0).sqrt());
        // Propagazione alla massa: M = (a r0)^3 / P^2.
        let massa = o.massa();
        let gradiente = [
            (0, 3.0 * massa / o.a),
            (6, -2.0 * massa / o.p),
            (7, 3.0 * massa / o.r0),
        ];
        let mut varianza = 0.0;
        for (ka, ga) in gradiente {
            for (kb, gb) in gradiente {
                if let (Some(a), Some(b)) = (indice(ka), indice(kb)) {
                    varianza += ga * gb * covarianza[a * n + b];
                }
            }
        }
        Some(Incertezze {
            massa: varianza.max(0.0).sqrt(),
            r0: sigma(7),
            a: sigma(0),
            e: sigma(1),
            p: sigma(6),
        })
    }

    /// Il testo CSV di queste misure, nel formato accettato da `leggi_csv`.
    pub fn in_csv(&self) -> String {
        let mut s = String::from(INTESTAZIONE);
        s.push('\n');
        for p in &self.posizioni {
            let _ = writeln!(s, "pos,{},{},{},{},{},,", p.t, p.est, p.err_est, p.nord, p.err_nord);
        }
        for v in &self.velocita {
            let _ = writeln!(s, "vel,{},,,,,{},{}", v.t, v.v, v.err);
        }
        s
    }
}

/// Eliminazione di Gauss con pivot parziale.
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
        let somma: f64 = (c + 1..n).map(|k| a[c * n + k] * x[k]).sum();
        x[c] = (x[c] - somma) / a[c * n + c];
    }
    x.iter().all(|v| v.is_finite()).then_some(x)
}

/// Intestazione obbligatoria: i nomi delle colonne dichiarano le unita'.
pub const INTESTAZIONE: &str =
    "tipo,tempo_anni,est_mas,errore_est_mas,nord_mas,errore_nord_mas,velocita_kms,errore_velocita_kms";

/// Legge misure da CSV. Unita' e incertezze sono obbligatorie: senza, si rifiuta.
pub fn leggi_csv(testo: &str) -> Result<Dati, String> {
    let mut righe = testo.lines().enumerate().filter(|(_, r)| !r.trim().is_empty());
    let (_, prima) = righe.next().ok_or("il file è vuoto")?;
    if prima.trim().trim_start_matches('\u{feff}') != INTESTAZIONE {
        return Err(format!("la prima riga deve essere esattamente: {INTESTAZIONE}"));
    }
    let mut dati = Dati::default();
    for (n, riga) in righe {
        let campi: Vec<&str> = riga.split(',').map(str::trim).collect();
        let riga_n = n + 1;
        if campi.len() != 8 {
            return Err(format!("riga {riga_n}: servono 8 colonne, ne ho trovate {}", campi.len()));
        }
        let numero = |i: usize| -> Result<f64, String> {
            campi[i]
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("riga {riga_n}: colonna {} non è un numero: '{}'", i + 1, campi[i]))
        };
        let errore = |i: usize| -> Result<f64, String> {
            let e = numero(i)?;
            if e > 0.0 {
                Ok(e)
            } else {
                Err(format!("riga {riga_n}: l'incertezza deve essere maggiore di zero"))
            }
        };
        match campi[0] {
            "pos" => dati.posizioni.push(Posizione {
                t: numero(1)?,
                est: numero(2)?,
                err_est: errore(3)?,
                nord: numero(4)?,
                err_nord: errore(5)?,
                attiva: true,
            }),
            "vel" => dati.velocita.push(Velocita {
                t: numero(1)?,
                v: numero(6)?,
                err: errore(7)?,
                attiva: true,
            }),
            altro => return Err(format!("riga {riga_n}: tipo sconosciuto '{altro}', uso pos o vel")),
        }
    }
    if dati.numero() <= N_PARAMETRI {
        return Err(format!(
            "troppo poche misure: ne servono più di {N_PARAMETRI}, ce ne sono {}",
            dati.numero()
        ));
    }
    Ok(dati)
}

#[cfg(test)]
mod test {
    use super::*;

    const VERA: Orbita = Orbita {
        a: 125.0,
        e: 0.88,
        i: 135.0,
        nodo: 226.0,
        periastro: 65.0,
        t0: 2002.33,
        p: 15.8,
        r0: 8.2,
        x0: 1.0,
        y0: -3.0,
        vx: -0.2,
        vy: 0.3,
        vz: 15.0,
    };

    fn sintetici() -> Dati {
        let mut d = s2_ghez_2008();
        for p in &mut d.posizioni {
            let (est, nord) = VERA.posizione(p.t);
            p.est = est;
            p.nord = nord;
        }
        for v in &mut d.velocita {
            v.v = VERA.velocita_radiale(v.t);
        }
        d
    }

    #[test]
    fn keplero_torna_dopo_un_periodo() {
        let (a, b) = (VERA.posizione(2010.0), VERA.posizione(2010.0 + VERA.p));
        assert!((a.0 + VERA.vx * VERA.p - b.0).abs() < 1e-8);
        assert!((a.1 + VERA.vy * VERA.p - b.1).abs() < 1e-8);
    }

    #[test]
    fn massa_dalla_terza_legge() {
        // La Terra attorno al Sole: 1 UA, 1 anno, 1 massa solare.
        let o = Orbita { a: 1.0, p: 1.0, r0: 1.0, ..VERA };
        assert!((o.massa() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn la_velocita_e_la_derivata_della_distanza_kepleriana() {
        // Velocita' radiale coerente con la velocita' orbitale al periastro.
        let o = Orbita { i: 90.0, periastro: 0.0, vz: 0.0, ..VERA };
        let k = o.velocita_radiale(o.t0);
        assert!((k.abs() - o.velocita_massima()).abs() / o.velocita_massima() < 1e-3, "{k}");
    }

    #[test]
    fn l_adattamento_ritrova_un_orbita_nota() {
        let d = sintetici();
        let fit = d.adatta(&INIZIO, false);
        assert!(d.chi2_ridotto(&fit) < 1e-4, "{}", d.chi2_ridotto(&fit));
        assert!((fit.massa() / VERA.massa() - 1.0).abs() < 1e-3);
        assert!((fit.r0 / VERA.r0 - 1.0).abs() < 1e-3);
    }

    #[test]
    fn le_misure_vere_di_s2_si_accordano_col_risultato_pubblicato() {
        let d = s2_ghez_2008();
        for (vz_fissa, p) in [(true, GHEZ_VZ_FISSA), (false, GHEZ_VZ_LIBERA)] {
            let fit = d.adatta(&INIZIO, vz_fissa);
            let e = d.incertezze(&fit, vz_fissa).unwrap();
            println!(
                "vz fissa {vz_fissa}: a {:.2}+-{:.2} e {:.4} i {:.1} nodo {:.1} w {:.1} P {:.2}+-{:.2} R0 {:.2}+-{:.2} M {:.3e}+-{:.2e} chi2r {:.2}",
                fit.a, e.a, fit.e, fit.i, fit.nodo, fit.periastro, fit.p, e.p, fit.r0, e.r0, fit.massa(), e.massa, d.chi2_ridotto(&fit)
            );
            assert!((fit.a - p.a).abs() < 6.0, "{}", fit.a);
            assert!((fit.e - p.e).abs() < 0.02, "{}", fit.e);
            assert!((fit.p - p.p).abs() < 0.6, "{}", fit.p);
            assert!((fit.i - 135.0).abs() < 6.0 && (fit.nodo - 226.0).abs() < 8.0);
            let scarto = (fit.massa() - p.massa).abs() / e.massa.hypot(p.massa_errore);
            assert!(scarto < 2.5, "{scarto} deviazioni: {}", fit.massa());
            assert!(d.chi2_ridotto(&fit) < 4.0, "{}", d.chi2_ridotto(&fit));
        }
    }

    #[test]
    fn togliere_dati_cambia_il_risultato() {
        let mut d = s2_ghez_2008();
        let prima = d.adatta(&INIZIO, true);
        for v in &mut d.velocita {
            v.attiva = false;
        }
        let dopo = d.adatta(&prima, true);
        assert_ne!(prima, dopo);
    }

    #[test]
    fn il_csv_va_e_torna() {
        let d = s2_ghez_2008();
        assert_eq!(leggi_csv(&d.in_csv()).unwrap(), d);
    }

    #[test]
    fn il_csv_senza_incertezze_viene_rifiutato() {
        let cattivo = format!("{INTESTAZIONE}\npos,2000,1,0,2,1,,");
        assert!(leggi_csv(&cattivo).unwrap_err().contains("incertezza"));
        assert!(leggi_csv("tempo,x,y\n1,2,3").unwrap_err().contains("prima riga"));
    }
}
