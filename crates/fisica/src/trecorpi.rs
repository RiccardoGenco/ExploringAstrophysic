//! Problema a piu' corpi nel piano, con G = 1. Si integra a 64 bit con un metodo
//! di Dormand-Prince a passo adattivo: gli incontri ravvicinati decidono il passo.

use std::f64::consts::TAU;

pub const MAX_CORPI: usize = 6;
const MAX_Y: usize = MAX_CORPI * 4;
const H_MASSIMO: f64 = 0.5;

pub type V2 = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corpo {
    pub massa: f64,
    pub x: V2,
    pub v: V2,
}

fn corpo(massa: f64, x: V2, v: V2) -> Corpo {
    Corpo { massa, x, v }
}

#[derive(Clone, Debug)]
pub struct Sistema {
    masse: [f64; MAX_CORPI],
    n: usize,
    y: [f64; MAX_Y],
    pub t: f64,
    pub tolleranza: f64,
    pub h: f64,
    pub passi: u64,
    /// Coppia finita in uno scontro: il passo e' crollato.
    pub scontro: Option<(usize, usize)>,
    energia0: f64,
    momento0: f64,
}

impl Sistema {
    pub fn nuovo(corpi: &[Corpo], tolleranza: f64) -> Self {
        let n = corpi.len().min(MAX_CORPI);
        let mut masse = [0.0; MAX_CORPI];
        let mut y = [0.0; MAX_Y];
        for (i, c) in corpi.iter().take(n).enumerate() {
            masse[i] = c.massa;
            y[4 * i] = c.x[0];
            y[4 * i + 1] = c.x[1];
            y[4 * i + 2] = c.v[0];
            y[4 * i + 3] = c.v[1];
        }
        let mut s = Self {
            masse,
            n,
            y,
            t: 0.0,
            tolleranza,
            h: 1e-3,
            passi: 0,
            scontro: None,
            energia0: 0.0,
            momento0: 0.0,
        };
        s.energia0 = s.energia();
        s.momento0 = s.momento_angolare();
        s
    }

    pub fn numero(&self) -> usize {
        self.n
    }

    pub fn massa(&self, i: usize) -> f64 {
        self.masse[i]
    }

    pub fn posizione(&self, i: usize) -> V2 {
        [self.y[4 * i], self.y[4 * i + 1]]
    }

    pub fn velocita(&self, i: usize) -> V2 {
        [self.y[4 * i + 2], self.y[4 * i + 3]]
    }

    pub fn energia(&self) -> f64 {
        let mut e = 0.0;
        for i in 0..self.n {
            let v = self.velocita(i);
            e += 0.5 * self.masse[i] * (v[0] * v[0] + v[1] * v[1]);
            for j in i + 1..self.n {
                let (a, b) = (self.posizione(i), self.posizione(j));
                e -= self.masse[i] * self.masse[j] / (a[0] - b[0]).hypot(a[1] - b[1]);
            }
        }
        e
    }

    pub fn momento_angolare(&self) -> f64 {
        (0..self.n)
            .map(|i| {
                let (x, v) = (self.posizione(i), self.velocita(i));
                self.masse[i] * (x[0] * v[1] - x[1] * v[0])
            })
            .sum()
    }

    /// Errore relativo sull'energia accumulato dall'integrazione.
    pub fn errore_energia(&self) -> f64 {
        ((self.energia() - self.energia0) / self.energia0.abs().max(1e-30)).abs()
    }

    pub fn errore_momento(&self) -> f64 {
        // Se il momento totale e' nullo, il confronto va fatto con i singoli contributi.
        let scala: f64 = (0..self.n)
            .map(|i| {
                let (x, v) = (self.posizione(i), self.velocita(i));
                (self.masse[i] * (x[0] * v[1] - x[1] * v[0])).abs()
            })
            .sum();
        (self.momento_angolare() - self.momento0).abs() / scala.max(self.momento0.abs()).max(1e-30)
    }

    /// Scarto massimo tra due sistemi con gli stessi corpi.
    pub fn distanza_da(&self, altro: &Sistema) -> f64 {
        (0..self.n)
            .map(|i| {
                let (a, b) = (self.posizione(i), altro.posizione(i));
                (a[0] - b[0]).hypot(a[1] - b[1])
            })
            .fold(0.0, f64::max)
    }

    /// Inverte tutte le velocita': il moto deve ripercorrersi all'indietro.
    pub fn inverti(&mut self) {
        for i in 0..self.n {
            self.y[4 * i + 2] = -self.y[4 * i + 2];
            self.y[4 * i + 3] = -self.y[4 * i + 3];
        }
    }

    pub fn corpi(&self) -> Vec<Corpo> {
        (0..self.n)
            .map(|i| corpo(self.masse[i], self.posizione(i), self.velocita(i)))
            .collect()
    }

    fn derivate(&self, y: &[f64; MAX_Y]) -> [f64; MAX_Y] {
        let mut d = [0.0; MAX_Y];
        for i in 0..self.n {
            d[4 * i] = y[4 * i + 2];
            d[4 * i + 1] = y[4 * i + 3];
        }
        for i in 0..self.n {
            for j in i + 1..self.n {
                let dx = y[4 * j] - y[4 * i];
                let dy = y[4 * j + 1] - y[4 * i + 1];
                let r2 = dx * dx + dy * dy;
                let terzo = 1.0 / (r2 * r2.sqrt());
                d[4 * i + 2] += self.masse[j] * dx * terzo;
                d[4 * i + 3] += self.masse[j] * dy * terzo;
                d[4 * j + 2] -= self.masse[i] * dx * terzo;
                d[4 * j + 3] -= self.masse[i] * dy * terzo;
            }
        }
        d
    }

    fn coppia_piu_vicina(&self) -> (usize, usize) {
        let mut migliore = (f64::MAX, (0, 1));
        for i in 0..self.n {
            for j in i + 1..self.n {
                let (a, b) = (self.posizione(i), self.posizione(j));
                let d = (a[0] - b[0]).hypot(a[1] - b[1]);
                if d < migliore.0 {
                    migliore = (d, (i, j));
                }
            }
        }
        migliore.1
    }

    /// Un passo di Dormand-Prince 5(4). Vero se e' stato accettato.
    fn tenta(&mut self, h: f64) -> bool {
        const A: [[f64; 6]; 6] = [
            [1.0 / 5.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [3.0 / 40.0, 9.0 / 40.0, 0.0, 0.0, 0.0, 0.0],
            [44.0 / 45.0, -56.0 / 15.0, 32.0 / 9.0, 0.0, 0.0, 0.0],
            [19372.0 / 6561.0, -25360.0 / 2187.0, 64448.0 / 6561.0, -212.0 / 729.0, 0.0, 0.0],
            [9017.0 / 3168.0, -355.0 / 33.0, 46732.0 / 5247.0, 49.0 / 176.0, -5103.0 / 18656.0, 0.0],
            [35.0 / 384.0, 0.0, 500.0 / 1113.0, 125.0 / 192.0, -2187.0 / 6784.0, 11.0 / 84.0],
        ];
        const E: [f64; 7] = [
            71.0 / 57600.0,
            0.0,
            -71.0 / 16695.0,
            71.0 / 1920.0,
            -17253.0 / 339200.0,
            22.0 / 525.0,
            -1.0 / 40.0,
        ];
        let m = 4 * self.n;
        let mut k = [[0.0; MAX_Y]; 7];
        k[0] = self.derivate(&self.y);
        let mut prova = self.y;
        for s in 0..6 {
            prova = self.y;
            for q in 0..=s {
                if A[s][q] != 0.0 {
                    for c in 0..m {
                        prova[c] += h * A[s][q] * k[q][c];
                    }
                }
            }
            k[s + 1] = self.derivate(&prova);
        }
        let mut errore: f64 = 0.0;
        for c in 0..m {
            let mut e = 0.0;
            for s in 0..7 {
                e += E[s] * k[s][c];
            }
            let scala = self.tolleranza * (1e-3 + self.y[c].abs().max(prova[c].abs()));
            errore = errore.max((h * e / scala).abs());
        }
        if !errore.is_finite() {
            self.h = h * 0.2;
            return false;
        }
        let fattore = (0.9 * errore.max(1e-12).powf(-0.2)).clamp(0.2, 5.0);
        self.h = (h * fattore).min(H_MASSIMO);
        if errore <= 1.0 {
            self.y = prova;
            self.t += h;
            self.passi += 1;
            true
        } else {
            false
        }
    }

    /// Avanza fino a `bersaglio`, per al piu' `budget` tentativi.
    pub fn avanza(&mut self, bersaglio: f64, budget: usize, mut ogni_passo: impl FnMut(&Sistema)) {
        for _ in 0..budget {
            if self.scontro.is_some() {
                return;
            }
            let resto = bersaglio - self.t;
            if resto <= 1e-12 {
                return;
            }
            if self.h < 1e-13 * (1.0 + self.t.abs()) {
                self.scontro = Some(self.coppia_piu_vicina());
                return;
            }
            let h_precedente = self.h;
            let h = self.h.min(resto);
            let accorciato = h < h_precedente;
            if self.tenta(h) {
                if accorciato {
                    // Un passo tagliato per il bersaglio non deve frenare i successivi.
                    self.h = self.h.max(h_precedente);
                }
                ogni_passo(self);
            }
        }
    }

    /// Indice del corpo che si sta allontanando per sempre dagli altri, se c'e'.
    pub fn espulso(&self, distanza_minima: f64) -> Option<usize> {
        if self.n < 3 {
            return None;
        }
        for i in 0..self.n {
            let m = self.masse[i];
            if m == 0.0 {
                continue;
            }
            let (mut massa, mut cx, mut cy, mut wx, mut wy) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for j in (0..self.n).filter(|&j| j != i) {
                let (p, v, q) = (self.posizione(j), self.velocita(j), self.masse[j]);
                massa += q;
                cx += q * p[0];
                cy += q * p[1];
                wx += q * v[0];
                wy += q * v[1];
            }
            if massa == 0.0 {
                continue;
            }
            let (p, v) = (self.posizione(i), self.velocita(i));
            let (rx, ry) = (p[0] - cx / massa, p[1] - cy / massa);
            let (ux, uy) = (v[0] - wx / massa, v[1] - wy / massa);
            let r = rx.hypot(ry);
            let energia = 0.5 * (ux * ux + uy * uy) - (m + massa) / r;
            if r > distanza_minima && rx * ux + ry * uy > 0.0 && energia > 0.0 {
                return Some(i);
            }
        }
        None
    }
}

/// Elementi orbitali istantanei di un corpo attorno a un altro: (semiasse, eccentricita').
/// Il semiasse e' negativo se l'orbita e' aperta.
pub fn elementi(s: &Sistema, corpo: usize, centro: usize) -> (f64, f64) {
    let (a, b) = (s.posizione(corpo), s.posizione(centro));
    let (va, vb) = (s.velocita(corpo), s.velocita(centro));
    let (x, y, u, v) = (a[0] - b[0], a[1] - b[1], va[0] - vb[0], va[1] - vb[1]);
    let mu = s.massa(centro) + s.massa(corpo);
    let r = x.hypot(y);
    let energia = 0.5 * (u * u + v * v) - mu / r;
    let h = x * v - y * u;
    let semiasse = -mu / (2.0 * energia);
    let e2 = 1.0 + 2.0 * energia * h * h / (mu * mu);
    (semiasse, e2.max(0.0).sqrt())
}

/// I cinque punti di Lagrange con le primarie 1 - mu in (-mu, 0) e mu in (1 - mu, 0).
pub fn punti_lagrange(mu: f64) -> [V2; 5] {
    let forza = |x: f64| {
        let (r1, r2) = ((x + mu).abs(), (x - 1.0 + mu).abs());
        x - (1.0 - mu) * (x + mu) / (r1 * r1 * r1) - mu * (x - 1.0 + mu) / (r2 * r2 * r2)
    };
    let radice = |mut a: f64, mut b: f64| {
        let fa = forza(a);
        for _ in 0..200 {
            let m = 0.5 * (a + b);
            if forza(m).signum() == fa.signum() {
                a = m;
            } else {
                b = m;
            }
        }
        0.5 * (a + b)
    };
    let eps = 1e-6;
    let l1 = radice(-mu + eps, 1.0 - mu - eps);
    let l2 = radice(1.0 - mu + eps, 3.0);
    let l3 = radice(-3.0, -mu - eps);
    let alto = 3.0_f64.sqrt() / 2.0;
    [[l1, 0.0], [l2, 0.0], [l3, 0.0], [0.5 - mu, alto], [0.5 - mu, -alto]]
}

/// Limite di Routh: sopra questo rapporto di massa L4 e L5 non sono stabili.
pub const LIMITE_ROUTH: f64 = 0.038_520_9;

fn orbite_a_coppia(p1: f64, p2: f64) -> Vec<Corpo> {
    vec![
        corpo(1.0, [-1.0, 0.0], [p1, p2]),
        corpo(1.0, [1.0, 0.0], [p1, p2]),
        corpo(1.0, [0.0, 0.0], [-2.0 * p1, -2.0 * p2]),
    ]
}

/// Primarie su orbita circolare, piu' un corpo senza massa vicino a L4.
pub fn troiani(mu: f64, scarto: f64) -> Vec<Corpo> {
    let l4 = punti_lagrange(mu)[3];
    let (x, y) = (l4[0] + scarto, l4[1]);
    vec![
        corpo(1.0 - mu, [-mu, 0.0], [0.0, -mu]),
        corpo(mu, [1.0 - mu, 0.0], [0.0, 1.0 - mu]),
        corpo(0.0, [x, y], [-y, x]),
    ]
}

fn figura_otto() -> Vec<Corpo> {
    let v3 = [-0.932_407_37, -0.864_731_46];
    vec![
        corpo(1.0, [0.970_004_36, -0.243_087_53], [-v3[0] / 2.0, -v3[1] / 2.0]),
        corpo(1.0, [-0.970_004_36, 0.243_087_53], [-v3[0] / 2.0, -v3[1] / 2.0]),
        corpo(1.0, [0.0, 0.0], v3),
    ]
}

fn triangolo() -> Vec<Corpo> {
    let v = (1.0 / 3.0_f64.sqrt()).sqrt();
    (0..3)
        .map(|i| {
            let a = TAU * f64::from(i) / 3.0;
            corpo(1.0, [a.cos(), a.sin()], [-v * a.sin(), v * a.cos()])
        })
        .collect()
}

fn allineati() -> Vec<Corpo> {
    let v = 1.25_f64.sqrt();
    vec![
        corpo(1.0, [-1.0, 0.0], [0.0, -v]),
        corpo(1.0, [0.0, 0.0], [0.0, 0.0]),
        corpo(1.0, [1.0, 0.0], [0.0, v]),
    ]
}

fn due_corpi() -> Vec<Corpo> {
    vec![corpo(1.0, [0.0, 0.0], [0.0, -0.1]), corpo(0.1, [1.5, 0.0], [0.0, 1.0])]
}

fn pitagorico() -> Vec<Corpo> {
    vec![
        corpo(3.0, [1.0, 3.0], [0.0, 0.0]),
        corpo(4.0, [-2.0, -1.0], [0.0, 0.0]),
        corpo(5.0, [1.0, -1.0], [0.0, 0.0]),
    ]
}

/// Una coppia di stelle passa vicino a un corpo enorme.
fn fionda() -> Vec<Corpo> {
    let (grande, stella, a): (f64, f64, f64) = (1000.0, 1.0, 0.1);
    let periastro = 0.55;
    let r0: f64 = 8.0;
    let v = (2.0 * grande / r0).sqrt();
    let vt = (2.0 * grande * periastro).sqrt() / r0;
    let vr = -(v * v - vt * vt).sqrt();
    let centro = [r0, 0.0];
    let velocita = [vr, vt];
    let interna = (2.0 * stella / a).sqrt() / 2.0;
    vec![
        corpo(grande, [0.0, 0.0], [0.0, 0.0]),
        corpo(stella, [centro[0] + a / 2.0, centro[1]], [velocita[0], velocita[1] + interna]),
        corpo(stella, [centro[0] - a / 2.0, centro[1]], [velocita[0], velocita[1] - interna]),
    ]
}

/// Unita': masse solari, unita' astronomiche, anni / 2 pi. Orbite circolari nel piano.
fn sistema_solare() -> Vec<Corpo> {
    let pianeta = |m: f64, a: f64| corpo(m, [a, 0.0], [0.0, (1.0 / a).sqrt()]);
    vec![
        corpo(1.0, [0.0, 0.0], [0.0, 0.0]),
        pianeta(3.003e-6, 1.0),
        pianeta(9.546e-4, 5.203),
        pianeta(2.858e-4, 9.537),
    ]
}

/// Tre soli e un pianeta senza massa, unita' come sopra.
fn tre_soli() -> Vec<Corpo> {
    let (m, r): (f64, f64) = (1.0, 6.0);
    let v = (m / (3.0_f64.sqrt() * r)).sqrt() * 0.86;
    let mut c: Vec<Corpo> = (0..3)
        .map(|i| {
            let a = TAU * f64::from(i) / 3.0 + 0.15 * f64::from(i);
            corpo(m, [r * a.cos(), r * a.sin()], [-v * a.sin(), v * a.cos()])
        })
        .collect();
    let (p, q) = (c[0].x, c[0].v);
    c.push(corpo(0.0, [p[0] + 1.3, p[1]], [q[0], q[1] + (1.0_f64 / 1.3).sqrt()]));
    c
}

pub struct Voce {
    pub nome: &'static str,
    pub gruppo: &'static str,
    pub nota: &'static str,
    /// Periodo noto, per le orbite che si richiudono.
    pub periodo: Option<f64>,
    pub unita_reali: bool,
    pub corpi: fn() -> Vec<Corpo>,
}

pub const GALLERIA: [Voce; 14] = [
    Voce { nome: "Due corpi", gruppo: "Si parte da qui", nota: "Con due corpi tutto si ripete, per sempre. Aggiungine un terzo.", periodo: None, unita_reali: false, corpi: due_corpi },
    Voce { nome: "Figura a otto", gruppo: "Soluzioni rare", nota: "Tre masse uguali sullo stesso otto. Trovata al calcolatore nel 1993.", periodo: Some(6.324_9), unita_reali: false, corpi: figura_otto },
    Voce { nome: "Farfalla", gruppo: "Soluzioni rare", nota: "Una delle famiglie trovate nel 2013. Basta un soffio per perderla.", periodo: Some(6.235_641), unita_reali: false, corpi: || orbite_a_coppia(0.306_893, 0.125_507) },
    Voce { nome: "Falena", gruppo: "Soluzioni rare", nota: "Una delle famiglie trovate nel 2013.", periodo: Some(14.893_911), unita_reali: false, corpi: || orbite_a_coppia(0.464_445, 0.396_060) },
    Voce { nome: "Yin-yang", gruppo: "Soluzioni rare", nota: "Una delle famiglie trovate nel 2013.", periodo: Some(17.328_370), unita_reali: false, corpi: || orbite_a_coppia(0.513_938, 0.304_736) },
    Voce { nome: "Triangolo", gruppo: "Soluzioni di Lagrange ed Eulero", nota: "Perfetta sulla carta. Con masse uguali basta l'errore di calcolo a distruggerla.", periodo: None, unita_reali: false, corpi: triangolo },
    Voce { nome: "In fila", gruppo: "Soluzioni di Lagrange ed Eulero", nota: "Tre corpi sempre allineati: una soluzione di Eulero, ancora più instabile.", periodo: None, unita_reali: false, corpi: allineati },
    Voce { nome: "Troiani stabili", gruppo: "Punti di Lagrange", nota: "Un corpo leggero a 60 gradi dal pianeta resta lì. Guarda con la vista che ruota.", periodo: None, unita_reali: false, corpi: || troiani(0.001, 0.01) },
    Voce { nome: "Troiani instabili", gruppo: "Punti di Lagrange", nota: "Sopra il limite di Routh il punto a 60 gradi non trattiene più niente.", periodo: None, unita_reali: false, corpi: || troiani(0.1, 0.01) },
    Voce { nome: "Problema pitagorico", gruppo: "Il finale tipico", nota: "Masse 3, 4, 5 ferme ai vertici di un triangolo. Il finale è quasi sempre lo stesso: una coppia e uno che se ne va.", periodo: None, unita_reali: false, corpi: pitagorico },
    Voce { nome: "Stella fionda", gruppo: "Il finale tipico", nota: "Una coppia passa vicino a un corpo enorme: uno resta catturato, l'altro viene sparato via.", periodo: None, unita_reali: false, corpi: fionda },
    Voce { nome: "Sistema Solare", gruppo: "Noi", nota: "Sole, Terra, Giove e Saturno. Regge. Prova a far pesare Giove trenta volte tanto.", periodo: None, unita_reali: true, corpi: sistema_solare },
    Voce { nome: "Pianeta fra tre soli", gruppo: "Noi", nota: "Un pianeta in un sistema triplo: guarda il suo cielo e il suo clima.", periodo: None, unita_reali: true, corpi: tre_soli },
    Voce { nome: "Vuoto", gruppo: "Si parte da qui", nota: "Tre corpi fermi: spostali e dai loro una spinta.", periodo: None, unita_reali: false, corpi: || vec![corpo(1.0, [-1.0, 0.0], [0.0, -0.3]), corpo(1.0, [1.0, 0.0], [0.0, 0.3]), corpo(1.0, [0.0, 1.2], [0.3, 0.0])] },
];

/// Esito di una casella della mappa del caos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EsitoMappa {
    /// Chi se ne va: 0 e 1 sono i corpi alla base, 2 quello che si sposta.
    pub espulso: Option<u8>,
    pub tempo: f32,
    pub scontro: bool,
}

pub const TEMPO_MAPPA: f64 = 120.0;
const DISTANZA_ESPULSIONE: f64 = 8.0;

/// Tre corpi fermi: due alla base, il terzo nel punto (alfa, beta).
pub fn corpi_mappa(alfa: f64, beta: f64, massa_terzo: f64) -> Vec<Corpo> {
    vec![
        corpo(1.0, [-0.5, 0.0], [0.0, 0.0]),
        corpo(1.0, [0.5, 0.0], [0.0, 0.0]),
        corpo(massa_terzo, [alfa, beta], [0.0, 0.0]),
    ]
}

pub fn esito_mappa(alfa: f64, beta: f64, massa_terzo: f64) -> EsitoMappa {
    let mut s = Sistema::nuovo(&corpi_mappa(alfa, beta, massa_terzo), 1e-8);
    let mut bersaglio = 0.5;
    while bersaglio <= TEMPO_MAPPA {
        s.avanza(bersaglio, 400_000, |_| {});
        if s.scontro.is_some() {
            return EsitoMappa { espulso: None, tempo: s.t as f32, scontro: true };
        }
        if let Some(i) = s.espulso(DISTANZA_ESPULSIONE) {
            return EsitoMappa { espulso: Some(i as u8), tempo: s.t as f32, scontro: false };
        }
        bersaglio += 0.5;
    }
    EsitoMappa { espulso: None, tempo: TEMPO_MAPPA as f32, scontro: false }
}

#[cfg(test)]
mod test {
    use super::*;

    const PRECISO: f64 = 1e-12;
    const MOLTI: usize = 5_000_000;

    fn dopo(corpi: &[Corpo], t: f64) -> Sistema {
        let mut s = Sistema::nuovo(corpi, PRECISO);
        s.avanza(t, MOLTI, |_| {});
        s
    }

    fn voce(nome: &str) -> &'static Voce {
        GALLERIA.iter().find(|v| v.nome == nome).unwrap()
    }

    #[test]
    fn le_orbite_rare_si_richiudono_dopo_il_loro_periodo() {
        for v in GALLERIA.iter().filter(|v| v.periodo.is_some()) {
            let iniziali = (v.corpi)();
            let s = dopo(&iniziali, v.periodo.unwrap());
            let scarto = s.distanza_da(&Sistema::nuovo(&iniziali, PRECISO));
            assert!(scarto < 0.02, "{}: {scarto}", v.nome);
        }
    }

    #[test]
    fn due_corpi_seguono_keplero_e_conservano_tutto() {
        let c = [corpo(1.0, [0.0, 0.0], [0.0, 0.0]), corpo(0.0, [1.0, 0.0], [0.0, 1.0])];
        let s = dopo(&c, TAU);
        assert!((s.posizione(1)[0] - 1.0).abs() < 1e-8 && s.posizione(1)[1].abs() < 1e-8);
        let d = dopo(&due_corpi(), 200.0);
        assert!(d.errore_energia() < 1e-9 && d.errore_momento() < 1e-9, "{}", d.errore_energia());
    }

    #[test]
    fn elementi_di_un_orbita_nota() {
        let c = [corpo(1.0, [0.0, 0.0], [0.0, 0.0]), corpo(0.0, [0.5, 0.0], [0.0, 3.0_f64.sqrt()])];
        let (a, e) = elementi(&Sistema::nuovo(&c, PRECISO), 1, 0);
        assert!((a - 1.0).abs() < 1e-9 && (e - 0.5).abs() < 1e-9, "{a} {e}");
    }

    #[test]
    fn il_problema_pitagorico_finisce_come_nel_1967() {
        // Szebehely e Peters: resta la coppia 4-5, se ne va la massa 3.
        let mut s = Sistema::nuovo(&pitagorico(), PRECISO);
        s.avanza(80.0, MOLTI, |_| {});
        assert_eq!(s.espulso(10.0), Some(0));
        assert!(s.errore_energia() < 1e-6, "{}", s.errore_energia());
    }

    #[test]
    fn tornare_indietro_riporta_all_inizio_solo_se_il_caos_lo_permette() {
        let iniziali = voce("Figura a otto");
        let mut s = dopo(&(iniziali.corpi)(), 20.0);
        s.inverti();
        let fine = s.t + 20.0;
        s.avanza(fine, MOLTI, |_| {});
        let mut riferimento = Sistema::nuovo(&(iniziali.corpi)(), PRECISO);
        riferimento.inverti();
        assert!(s.distanza_da(&riferimento) < 1e-6, "{}", s.distanza_da(&riferimento));
    }

    #[test]
    fn un_soffio_cresce_nel_problema_pitagorico() {
        let mut c = pitagorico();
        let a = Sistema::nuovo(&c, PRECISO);
        c[0].x[0] += 1e-9;
        let b = Sistema::nuovo(&c, PRECISO);
        let (mut a, mut b) = (a, b);
        a.avanza(45.0, MOLTI, |_| {});
        b.avanza(45.0, MOLTI, |_| {});
        assert!(a.distanza_da(&b) > 1e-6, "{}", a.distanza_da(&b));
    }

    #[test]
    fn i_punti_di_lagrange_sono_in_equilibrio() {
        let mu = 0.01;
        for (n, p) in punti_lagrange(mu).iter().enumerate() {
            let (r1, r2) = ((p[0] + mu).hypot(p[1]), (p[0] - 1.0 + mu).hypot(p[1]));
            let ax = p[0] - (1.0 - mu) * (p[0] + mu) / r1.powi(3) - mu * (p[0] - 1.0 + mu) / r2.powi(3);
            let ay = p[1] - (1.0 - mu) * p[1] / r1.powi(3) - mu * p[1] / r2.powi(3);
            assert!(ax.hypot(ay) < 1e-6, "L{}: {}", n + 1, ax.hypot(ay));
        }
    }

    /// Massima distanza dal punto L4, seguito nel sistema che ruota.
    fn scarto_da_l4(mu: f64, t: f64) -> f64 {
        let mut s = Sistema::nuovo(&troiani(mu, 0.01), 1e-11);
        let l4 = punti_lagrange(mu)[3];
        let mut massimo: f64 = 0.0;
        let n = 400;
        for k in 1..=n {
            s.avanza(t * f64::from(k) / f64::from(n), MOLTI, |_| {});
            let (a, b, p) = (s.posizione(0), s.posizione(1), s.posizione(2));
            let angolo = (b[1] - a[1]).atan2(b[0] - a[0]);
            let (sa, ca) = angolo.sin_cos();
            let (cx, cy) = (a[0] * (1.0 - mu) + b[0] * mu, a[1] * (1.0 - mu) + b[1] * mu);
            let (dx, dy) = (p[0] - cx, p[1] - cy);
            let (xr, yr) = (ca * dx + sa * dy, -sa * dx + ca * dy);
            massimo = massimo.max((xr - l4[0]).hypot(yr - l4[1]));
        }
        massimo
    }

    #[test]
    fn sotto_il_limite_di_routh_si_resta_sopra_si_fugge() {
        let (basso, alto) = (scarto_da_l4(0.001, 400.0), scarto_da_l4(0.1, 400.0));
        assert!(basso < 1.0 && alto > 5.0, "{basso} {alto}");
    }

    #[test]
    fn la_fionda_spara_via_una_stella() {
        let mut s = Sistema::nuovo(&fionda(), 1e-11);
        s.avanza(25.0, MOLTI, |_| {});
        let (a1, _) = elementi(&s, 1, 0);
        let (a2, _) = elementi(&s, 2, 0);
        assert!((a1 > 0.0) != (a2 > 0.0), "una legata e una libera: {a1} {a2}");
    }

    #[test]
    fn il_sistema_solare_regge_e_giove_pesante_no() {
        let mut c = sistema_solare();
        let mut s = Sistema::nuovo(&c, 1e-10);
        s.avanza(5000.0, MOLTI, |_| {});
        let (a, e) = elementi(&s, 1, 0);
        assert!((a - 1.0).abs() < 0.02 && e < 0.1, "{a} {e}");

        c[2].massa *= 30.0;
        c[3].massa *= 30.0;
        let mut p = Sistema::nuovo(&c, 1e-10);
        p.avanza(20_000.0, MOLTI, |_| {});
        let saturno = elementi(&p, 3, 0);
        assert!(saturno.0 < 0.0 || saturno.1 > 0.2, "{saturno:?}");
    }

    #[test]
    fn la_mappa_distingue_gli_esiti() {
        let e = esito_mappa(0.1, 0.8, 1.0);
        assert!(e.tempo > 0.0);
        assert!(e.espulso.is_some() || e.scontro || e.tempo >= TEMPO_MAPPA as f32);
    }

    #[test]
    fn tutta_la_galleria_parte_senza_errori() {
        for v in &GALLERIA {
            let s = dopo(&(v.corpi)(), 1.0);
            assert!(s.energia().is_finite(), "{}", v.nome);
        }
    }
}
