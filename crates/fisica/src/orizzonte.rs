//! Dentro l'orizzonte: coni di luce e trascinamento dello spazio. Unita': rs = 1.

/// Pendenze dt/dr dei raggi di luce in coordinate di Eddington-Finkelstein
/// entranti: il raggio che entra ha -1, quello che esce (r + 1) / (r - 1).
pub fn pendenze_cono(r: f64) -> (f64, f64) {
    (-1.0, (r + 1.0) / (r - 1.0))
}

/// Vero se a distanza `r` anche la luce che punta in fuori va verso il centro.
pub fn solo_verso_il_centro(r: f64) -> bool {
    r < 1.0
}

/// Buco nero rotante nel piano equatoriale. `a` e' la rotazione, da 0 a 1 di quella massima.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kerr {
    pub a: f64,
}

impl Kerr {
    const M: f64 = 0.5;

    fn spin(&self) -> f64 {
        self.a * Self::M
    }

    pub fn orizzonte(&self) -> f64 {
        Self::M + (Self::M * Self::M - self.spin().powi(2)).max(0.0).sqrt()
    }

    /// Sul piano equatoriale l'ergosfera arriva sempre a 2 M.
    pub fn ergosfera(&self) -> f64 {
        2.0 * Self::M
    }

    /// Velocita' angolare a cui lo spazio trascina chi non ha momento angolare.
    pub fn trascinamento(&self, r: f64) -> f64 {
        let a = self.spin();
        let delta = r * r - 2.0 * Self::M * r + a * a;
        let big = (r * r + a * a).powi(2) - a * a * delta;
        2.0 * Self::M * a * r / big
    }

    /// Velocita' angolare dell'orizzonte.
    pub fn omega_orizzonte(&self) -> f64 {
        let a = self.spin();
        let r = self.orizzonte();
        a / (r * r + a * a)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn fuori_la_luce_puo_uscire_dentro_no() {
        assert!(pendenze_cono(3.0).1 > 0.0);
        assert!(pendenze_cono(0.5).1 < 0.0);
        assert!(solo_verso_il_centro(0.9) && !solo_verso_il_centro(1.1));
    }

    #[test]
    fn senza_rotazione_ritrova_schwarzschild() {
        let k = Kerr { a: 0.0 };
        assert!((k.orizzonte() - 1.0).abs() < 1e-12);
        assert_eq!(k.trascinamento(3.0), 0.0);
    }

    #[test]
    fn l_orizzonte_si_stringe_ruotando() {
        assert!((Kerr { a: 1.0 }.orizzonte() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn sull_orizzonte_tutto_gira_con_lui() {
        let k = Kerr { a: 0.7 };
        let r = k.orizzonte();
        assert!((k.trascinamento(r) / k.omega_orizzonte() - 1.0).abs() < 1e-9);
        assert!(k.trascinamento(2.0) > k.trascinamento(5.0));
    }
}
