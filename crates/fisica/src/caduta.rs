//! Caduta radiale da fermo in Schwarzschild, in forma esatta (cicloide).
//!
//! Unita': lunghezze in raggi di Schwarzschild, tempi in rs/c.

use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caduta {
    /// Distanza di partenza, maggiore di 1.
    pub r0: f64,
}

impl Caduta {
    pub fn new(r0: f64) -> Self {
        assert!(r0 > 1.0, "la caduta deve partire fuori dall'orizzonte");
        Self { r0 }
    }

    /// Distanza dal centro al parametro `eta`, tra 0 (partenza) e pi greco (centro).
    pub fn raggio(&self, eta: f64) -> f64 {
        self.r0 / 2.0 * (1.0 + eta.cos())
    }

    /// Tempo sull'orologio di chi cade.
    pub fn tempo_proprio(&self, eta: f64) -> f64 {
        self.r0 / 2.0 * self.r0.sqrt() * (eta + eta.sin())
    }

    /// Valore di `eta` all'orizzonte.
    pub fn eta_orizzonte(&self) -> f64 {
        (2.0 / self.r0 - 1.0).acos()
    }

    /// Tempo sugli orologi lontani. Diverge avvicinandosi all'orizzonte.
    pub fn tempo_lontano(&self, eta: f64) -> f64 {
        debug_assert!(eta < self.eta_orizzonte());
        let s = (self.r0 - 1.0).sqrt();
        let tan = (eta / 2.0).tan();
        ((s + tan) / (s - tan)).ln() + s * (eta + self.r0 / 2.0 * (eta + eta.sin()))
    }

    /// Inverso di `tempo_lontano`. Il risultato resta sempre fuori dall'orizzonte.
    pub fn eta_da_tempo_lontano(&self, t: f64) -> f64 {
        let mut basso = 0.0;
        let mut alto = self.eta_orizzonte() * (1.0 - 1e-15);
        for _ in 0..200 {
            let medio = 0.5 * (basso + alto);
            if self.tempo_lontano(medio) < t {
                basso = medio;
            } else {
                alto = medio;
            }
        }
        basso
    }

    /// Tempo proprio totale fino al centro.
    pub fn tempo_proprio_totale(&self) -> f64 {
        self.tempo_proprio(PI)
    }

    /// Fattore con cui un osservatore lontano riceve la luce di chi cade
    /// verso l'esterno (1 = invariata, 0 = spenta).
    pub fn spostamento_verso_lontano(&self, eta: f64) -> f64 {
        let r = self.raggio(eta);
        let f = 1.0 - 1.0 / r;
        if f <= 0.0 {
            return 0.0;
        }
        let e = (1.0 - 1.0 / self.r0).sqrt();
        let velocita_radiale = (e * e - f).max(0.0).sqrt();
        // u^t e u^r per un fotone radiale uscente.
        f / (e + velocita_radiale)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const R0: f64 = 12.0;

    fn derivata(f: impl Fn(f64) -> f64, x: f64) -> f64 {
        let d = 1e-6;
        (f(x + d) - f(x - d)) / (2.0 * d)
    }

    #[test]
    fn parte_da_fermo_e_finisce_al_centro() {
        let c = Caduta::new(R0);
        assert!((c.raggio(0.0) - R0).abs() < 1e-12);
        assert!(c.raggio(PI).abs() < 1e-12);
        assert!((c.raggio(c.eta_orizzonte()) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn rispetta_l_equazione_del_moto() {
        let c = Caduta::new(R0);
        let e2 = 1.0 - 1.0 / R0;
        for eta in [0.3, 1.0, 1.7, 2.3, 2.9] {
            let r = c.raggio(eta);
            let dr = derivata(|x| c.raggio(x), eta) / derivata(|x| c.tempo_proprio(x), eta);
            let atteso = -(e2 - (1.0 - 1.0 / r)).sqrt();
            assert!((dr - atteso).abs() < 1e-6, "eta {eta}: {dr} contro {atteso}");
        }
    }

    #[test]
    fn la_dilatazione_del_tempo_e_corretta() {
        let c = Caduta::new(R0);
        let e = (1.0 - 1.0 / R0).sqrt();
        for eta in [0.3, 1.0, 1.5, 1.9] {
            assert!(eta < c.eta_orizzonte());
            let r = c.raggio(eta);
            let dt = derivata(|x| c.tempo_lontano(x), eta) / derivata(|x| c.tempo_proprio(x), eta);
            let atteso = e / (1.0 - 1.0 / r);
            assert!((dt / atteso - 1.0).abs() < 1e-6, "eta {eta}: {dt} contro {atteso}");
        }
    }

    #[test]
    fn da_lontano_non_si_attraversa_mai() {
        let c = Caduta::new(R0);
        let eta = c.eta_da_tempo_lontano(1e6);
        assert!(eta < c.eta_orizzonte());
        assert!(c.raggio(eta) >= 1.0);
    }

    #[test]
    fn inverso_del_tempo_lontano() {
        let c = Caduta::new(R0);
        let eta = 1.2;
        let t = c.tempo_lontano(eta);
        assert!((c.eta_da_tempo_lontano(t) - eta).abs() < 1e-9);
    }

    #[test]
    fn caduta_dall_orizzonte_dura_il_massimo_teorico() {
        // Da fermo appena sopra l'orizzonte: tempo proprio pi/2 in unita' rs/c.
        let c = Caduta::new(1.0 + 1e-12);
        assert!((c.tempo_proprio_totale() - PI / 2.0).abs() < 1e-6);
    }

    #[test]
    fn la_luce_si_spegne_all_orizzonte() {
        let c = Caduta::new(R0);
        assert!((c.spostamento_verso_lontano(0.0) - (1.0 - 1.0 / R0).sqrt()).abs() < 1e-12);
        assert!(c.spostamento_verso_lontano(c.eta_orizzonte() * 0.999999) < 1e-3);
    }
}
