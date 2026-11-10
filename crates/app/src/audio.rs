//! Riproduzione di un suono gia' calcolato, una volta sola.

#[cfg(not(target_arch = "wasm32"))]
pub use nativo::Audio;
#[cfg(target_arch = "wasm32")]
pub use web::Audio;

#[cfg(not(target_arch = "wasm32"))]
mod nativo {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::SampleFormat;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Default)]
    pub struct Audio {
        flusso: Option<cpal::Stream>,
    }

    impl Audio {
        /// Fa partire i campioni, sostituendo un eventuale suono in corso.
        pub fn suona(&mut self, campioni: Vec<f32>, frequenza: u32) -> Result<(), String> {
            self.flusso = None;
            let dispositivo = cpal::default_host()
                .default_output_device()
                .ok_or("nessuna uscita audio disponibile")?;
            let configurazione = dispositivo.default_output_config().map_err(|e| e.to_string())?;
            let formato = configurazione.sample_format();
            let config: cpal::StreamConfig = configurazione.into();
            let canali = usize::from(config.channels);
            let rapporto = f64::from(frequenza) / f64::from(config.sample_rate.0);
            let campioni = Arc::new(campioni);
            let posizione = Arc::new(AtomicUsize::new(0));
            let errore = |e| eprintln!("audio: {e}");

            let flusso = match formato {
                SampleFormat::F32 => {
                    let (c, p) = (campioni.clone(), posizione.clone());
                    dispositivo.build_output_stream(
                        &config,
                        move |uscita: &mut [f32], _| riempi(uscita, canali, rapporto, &c, &p, |v| v),
                        errore,
                        None,
                    )
                }
                SampleFormat::I16 => {
                    let (c, p) = (campioni.clone(), posizione.clone());
                    dispositivo.build_output_stream(
                        &config,
                        move |uscita: &mut [i16], _| {
                            riempi(uscita, canali, rapporto, &c, &p, |v| (v * f32::from(i16::MAX)) as i16);
                        },
                        errore,
                        None,
                    )
                }
                altro => return Err(format!("formato audio non gestito: {altro}")),
            }
            .map_err(|e| e.to_string())?;
            flusso.play().map_err(|e| e.to_string())?;
            self.flusso = Some(flusso);
            Ok(())
        }
    }

    fn riempi<T: Copy + Default>(
        uscita: &mut [T],
        canali: usize,
        rapporto: f64,
        campioni: &[f32],
        posizione: &AtomicUsize,
        converti: impl Fn(f32) -> T,
    ) {
        let mut p = posizione.load(Ordering::Relaxed);
        for gruppo in uscita.chunks_mut(canali) {
            let indice = (p as f64 * rapporto) as usize;
            let valore = campioni.get(indice).map_or_else(T::default, |v| converti(*v));
            gruppo.fill(valore);
            p += 1;
        }
        posizione.store(p, Ordering::Relaxed);
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use web_sys::AudioContext;

    #[derive(Default)]
    pub struct Audio {
        contesto: Option<AudioContext>,
    }

    impl Audio {
        pub fn suona(&mut self, campioni: Vec<f32>, frequenza: u32) -> Result<(), String> {
            let errore = |e: wasm_bindgen::JsValue| format!("{e:?}");
            if let Some(c) = self.contesto.take() {
                let _ = c.close();
            }
            let contesto = AudioContext::new().map_err(errore)?;
            let buffer = contesto
                .create_buffer(1, campioni.len() as u32, frequenza as f32)
                .map_err(errore)?;
            buffer.copy_to_channel(&campioni, 0).map_err(errore)?;
            let sorgente = contesto.create_buffer_source().map_err(errore)?;
            sorgente.set_buffer(Some(&buffer));
            sorgente
                .connect_with_audio_node(&contesto.destination())
                .map_err(errore)?;
            sorgente.start().map_err(errore)?;
            self.contesto = Some(contesto);
            Ok(())
        }
    }
}
