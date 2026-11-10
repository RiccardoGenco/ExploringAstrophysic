//! Cielo stellato inventato, in proiezione equirettangolare.

use std::f64::consts::PI;

struct Casuale(u64);

impl Casuale {
    fn prossimo(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

const COLORI: [[f64; 3]; 5] = [
    [1.0, 0.72, 0.5],
    [1.0, 0.86, 0.72],
    [1.0, 0.97, 0.92],
    [0.85, 0.9, 1.0],
    [0.68, 0.78, 1.0],
];

/// Pixel RGBA8 in spazio sRGB, riga per riga.
pub fn genera(larghezza: usize, altezza: usize) -> Vec<u8> {
    let mut lineare = vec![[0.0_f64; 3]; larghezza * altezza];
    let mut rng = Casuale(0x9E37_79B9_7F4A_7C15);

    for _ in 0..42_000 {
        let z = 2.0 * rng.prossimo() - 1.0;
        let longitudine = rng.prossimo();
        let colatitudine = z.acos();
        // Legge di potenza: poche stelle brillanti, moltissime deboli.
        let luce = 0.02 * (1.0 - rng.prossimo()).powf(-0.6);
        let colore = COLORI[(rng.prossimo() * COLORI.len() as f64) as usize];
        let raggio = (0.7 + 0.25 * luce.sqrt()).min(1.6);

        let cy = colatitudine / PI * altezza as f64;
        let cx = longitudine * larghezza as f64;
        // Vicino ai poli la proiezione stira le righe in orizzontale.
        let stiramento = (1.0 / colatitudine.sin().abs().max(0.02)).min(40.0);
        let rx = (raggio * stiramento).ceil() as isize + 1;
        let ry = raggio.ceil() as isize + 1;
        for dy in -ry..=ry {
            let y = cy as isize + dy;
            if y < 0 || y >= altezza as isize {
                continue;
            }
            for dx in -rx..=rx {
                let x = (cx as isize + dx).rem_euclid(larghezza as isize) as usize;
                let ex = dx as f64 / stiramento;
                let ey = dy as f64;
                let peso = (-(ex * ex + ey * ey) / (raggio * raggio)).exp() * luce;
                let pixel = &mut lineare[y as usize * larghezza + x];
                for canale in 0..3 {
                    pixel[canale] += colore[canale] * peso;
                }
            }
        }
    }

    let mut uscita = Vec::with_capacity(larghezza * altezza * 4);
    for pixel in lineare {
        for canale in pixel {
            uscita.push((srgb(canale.min(1.0)) * 255.0 + 0.5) as u8);
        }
        uscita.push(255);
    }
    uscita
}

fn srgb(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}
