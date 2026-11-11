//! Colossi: finestra interattiva, oppure `--scatta file.png` per un'immagine
//! senza finestra usata nelle verifiche.

mod ammasso;
mod audio;
mod banco;
mod benvenuto;
mod cadere;
mod curva;
mod fondo;
mod galassie;
mod formato;
mod gemelli;
mod giganti;
mod gpu;
mod guida;
mod lab;
mod missioni;
mod neutroni;
mod onde;
mod orizzonte;
mod osservatorio;
mod pannello;
mod scala;
mod scena_gemelli;
mod sfondo;
mod stato;
mod stelle;
mod strumenti;
mod trecorpi;
mod visite;

use egui_wgpu::wgpu;
use egui_winit::winit;
use gpu::Disegnatore;
use stato::Stato;
use std::sync::Arc;
use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

fn istanza() -> wgpu::Instance {
    let mut descrittore = wgpu::InstanceDescriptor::new_without_display_handle();
    descrittore.backends = wgpu::Backends::PRIMARY;
    wgpu::Instance::new(descrittore)
}

async fn dispositivo(adattatore: &wgpu::Adapter) -> (wgpu::Device, wgpu::Queue) {
    adattatore
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("colossi"),
            ..Default::default()
        })
        .await
        .expect("dispositivo grafico non disponibile")
}

fn stile(ctx: &egui::Context) {
    let mut visuali = egui::Visuals::dark();
    visuali.panel_fill = pannello::SUPERFICIE;
    visuali.selection.bg_fill = pannello::CIANO.gamma_multiply(0.3);
    visuali.selection.stroke = egui::Stroke::new(1.0, pannello::CIANO);
    ctx.set_visuals(visuali);
    ctx.global_style_mut(|s| {
        for (testo, font) in &mut s.text_styles {
            if *testo == egui::TextStyle::Body || *testo == egui::TextStyle::Button {
                font.size = 13.5;
            }
        }
    });
}

/// Rettangolo della scena in pixel: x, y, larghezza, altezza.
fn in_pixel(area: egui::Rect, punti_per_pixel: f32) -> [u32; 4] {
    [
        (area.left() * punti_per_pixel) as u32,
        (area.top() * punti_per_pixel) as u32,
        (area.width() * punti_per_pixel).max(2.0) as u32,
        (area.height() * punti_per_pixel).max(2.0) as u32,
    ]
}

struct Finestra {
    finestra: Arc<Window>,
    superficie: wgpu::Surface<'static>,
    configurazione: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    disegnatore: Disegnatore,
    egui: egui_winit::State,
}

struct App {
    finestra: Option<Finestra>,
    stato: Stato,
    ultimo: Instant,
    fps: f32,
    /// Nel browser il dispositivo grafico arriva in modo asincrono.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    proxy: EventLoopProxy<Finestra>,
    avviata: bool,
}

impl Finestra {
    async fn new(finestra: Arc<Window>) -> Self {
        let istanza = istanza();
        let superficie = istanza.create_surface(finestra.clone()).expect("superficie");
        let adattatore = istanza
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&superficie),
            })
            .await
            .expect("nessuna scheda grafica compatibile");
        let (device, queue) = dispositivo(&adattatore).await;

        let capacita = superficie.get_capabilities(&adattatore);
        // Lo shader finale applica gia' la curva sRGB.
        let formato = capacita
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(capacita.formats[0].remove_srgb_suffix());
        let misura = finestra.inner_size();
        let configurazione = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: formato,
            width: misura.width.max(1),
            height: misura.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: capacita.alpha_modes[0],
            view_formats: vec![],
        };
        superficie.configure(&device, &configurazione);

        let disegnatore = Disegnatore::new(&device, &queue, formato);
        let contesto = egui::Context::default();
        stile(&contesto);
        let egui = egui_winit::State::new(
            contesto,
            egui::ViewportId::ROOT,
            &finestra,
            Some(finestra.scale_factor() as f32),
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );

        Self { finestra, superficie, configurazione, device, queue, disegnatore, egui }
    }

    fn ridimensiona(&mut self, larghezza: u32, altezza: u32) {
        if larghezza > 0 && altezza > 0 {
            self.configurazione.width = larghezza;
            self.configurazione.height = altezza;
            self.superficie.configure(&self.device, &self.configurazione);
        }
    }

    fn disegna(&mut self, stato: &mut Stato, fps: f32) {
        let immagine = match self.superficie.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.superficie.configure(&self.device, &self.configurazione);
                return;
            }
            _ => return,
        };
        let vista = immagine.texture.create_view(&Default::default());

        let ingresso = self.egui.take_egui_input(&self.finestra);
        let contesto = self.egui.egui_ctx().clone();
        let mut area = None;
        let uscita = contesto.run_ui(ingresso, |ui| area = pannello::mostra(ui, stato, fps));
        self.egui.handle_platform_output(&self.finestra, uscita.platform_output);

        let primitive = contesto.tessellate(uscita.shapes, uscita.pixels_per_point);
        for (id, delta) in &uscita.textures_delta.set {
            self.disegnatore.egui.update_texture(&self.device, &self.queue, *id, delta);
        }
        self.disegnatore.aggiorna_scie(&self.queue, &mut stato.lab.mappa);
        let schermo = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.configurazione.width, self.configurazione.height],
            pixels_per_point: uscita.pixels_per_point,
        };
        self.disegnatore.disegna(
            &self.device,
            &self.queue,
            &vista,
            area.map(|a| in_pixel(a, uscita.pixels_per_point)),
            stato.qualita,
            stato.parametri(),
            Some((&primitive, &schermo)),
        );
        self.finestra.pre_present_notify();
        immagine.present();
        for id in &uscita.textures_delta.free {
            self.disegnatore.egui.free_texture(id);
        }
    }
}

impl ApplicationHandler<Finestra> for App {
    fn resumed(&mut self, evento: &ActiveEventLoop) {
        if self.avviata {
            return;
        }
        self.avviata = true;
        #[cfg(not(target_arch = "wasm32"))]
        {
            let attributi = Window::default_attributes().with_title("Colossi").with_maximized(true);
            let finestra = Arc::new(evento.create_window(attributi).expect("finestra"));
            self.finestra = Some(pollster::block_on(Finestra::new(finestra)));
            self.ultimo = Instant::now();
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            let attributi = Window::default_attributes().with_title("Colossi").with_append(true);
            let finestra = Arc::new(evento.create_window(attributi).expect("finestra"));
            let proxy = self.proxy.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let pronta = Finestra::new(finestra).await;
                let _ = proxy.send_event(pronta);
            });
        }
    }

    fn user_event(&mut self, _evento: &ActiveEventLoop, pronta: Finestra) {
        pronta.finestra.request_redraw();
        self.finestra = Some(pronta);
        self.ultimo = Instant::now();
    }

    fn window_event(&mut self, evento: &ActiveEventLoop, _id: WindowId, messaggio: WindowEvent) {
        let Some(f) = self.finestra.as_mut() else { return };
        let _ = f.egui.on_window_event(&f.finestra, &messaggio);
        match messaggio {
            WindowEvent::CloseRequested => evento.exit(),
            WindowEvent::Resized(misura) => f.ridimensiona(misura.width, misura.height),
            WindowEvent::RedrawRequested => {
                let adesso = Instant::now();
                let dt = (adesso - self.ultimo).as_secs_f64().min(0.1);
                self.ultimo = adesso;
                self.fps += (1.0 / dt.max(1e-4) as f32 - self.fps) * 0.05;
                self.stato.avanza(dt);
                f.disegna(&mut self.stato, self.fps);
                f.finestra.request_redraw();
            }
            _ => {}
        }
    }
}

/// Opzioni di `--scatta`.
#[cfg(not(target_arch = "wasm32"))]
struct Scatto {
    file: String,
    larghezza: u32,
    altezza: u32,
    interfaccia: bool,
}

#[cfg(not(target_arch = "wasm32"))]
fn scatta(opzioni: Scatto, mut stato: Stato) {
    pollster::block_on(async {
        let istanza = istanza();
        let adattatore = istanza
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .expect("nessuna scheda grafica");
        println!("scheda: {}", adattatore.get_info().name);
        let (device, queue) = dispositivo(&adattatore).await;

        let formato = wgpu::TextureFormat::Rgba8Unorm;
        let mut disegnatore = Disegnatore::new(&device, &queue, formato);
        let (l, a) = (opzioni.larghezza, opzioni.altezza);
        let bersaglio = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scatto"),
            size: wgpu::Extent3d { width: l, height: a, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formato,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let vista = bersaglio.create_view(&Default::default());

        let inizio = Instant::now();
        disegnatore.aggiorna_scie(&queue, &mut stato.lab.mappa);
        if opzioni.interfaccia {
            let contesto = egui::Context::default();
            stile(&contesto);
            let mut uscita = None;
            let mut area = None;
            // Il primo giro misura i testi, gli altri li dispongono.
            for _ in 0..3 {
                let ingresso = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(l as f32, a as f32),
                    )),
                    ..Default::default()
                };
                let u = contesto.run_ui(ingresso, |ui| area = pannello::mostra(ui, &mut stato, 0.0));
                for (id, delta) in &u.textures_delta.set {
                    disegnatore.egui.update_texture(&device, &queue, *id, delta);
                }
                uscita = Some(u);
            }
            let uscita = uscita.expect("almeno un giro");
            let primitive = contesto.tessellate(uscita.shapes, uscita.pixels_per_point);
            let schermo = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [l, a],
                pixels_per_point: uscita.pixels_per_point,
            };
            disegnatore.disegna(&device, &queue, &vista, area.map(|r| in_pixel(r, 1.0)), 1.0, stato.parametri(), Some((&primitive, &schermo)));
        } else {
            disegnatore.disegna(&device, &queue, &vista, Some([0, 0, l, a]), 1.0, stato.parametri(), None);
        }

        let riga = (l * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let lettura = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("lettura"),
            size: u64::from(riga * a),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            bersaglio.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &lettura,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(riga),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d { width: l, height: a, depth_or_array_layers: 1 },
        );
        queue.submit([encoder.finish()]);
        lettura.map_async(wgpu::MapMode::Read, .., |esito| esito.expect("lettura"));
        device
            .poll(wgpu::PollType::Wait { submission_index: None, timeout: None })
            .expect("attesa della scheda");
        println!("fotogramma: {:.1} ms", inizio.elapsed().as_secs_f64() * 1000.0);

        let dati = lettura.get_mapped_range(..);
        let pixel: Vec<u8> = dati
            .chunks(riga as usize)
            .flat_map(|r| r[..(l * 4) as usize].iter().copied())
            .collect();
        let file = std::fs::File::create(&opzioni.file).expect("file di uscita");
        let mut codifica = png::Encoder::new(std::io::BufWriter::new(file), l, a);
        codifica.set_color(png::ColorType::Rgba);
        codifica.set_depth(png::BitDepth::Eight);
        codifica
            .write_header()
            .and_then(|mut w| w.write_image_data(&pixel))
            .expect("scrittura png");
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn valore<T: std::str::FromStr>(argomenti: &[String], nome: &str) -> Option<T> {
    let i = argomenti.iter().position(|a| a == nome)?;
    argomenti.get(i + 1)?.parse().ok()
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use winit::platform::web::EventLoopExtWebSys;
    console_error_panic_hook::set_once();
    let ciclo = EventLoop::<Finestra>::with_user_event().build().expect("ciclo eventi");
    let proxy = ciclo.create_proxy();
    ciclo.spawn_app(App {
        finestra: None,
        stato: Stato::default(),
        ultimo: Instant::now(),
        fps: 60.0,
        proxy,
        avviata: false,
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let argomenti: Vec<String> = std::env::args().collect();

    if argomenti.iter().any(|a| a == "--prova-audio") {
        // Solo silenzio: controlla che l'uscita audio si apra.
        let mut uscita = audio::Audio::default();
        println!("audio: {:?}", uscita.suona(vec![0.0; 4410], 44_100));
        std::thread::sleep(std::time::Duration::from_millis(150));
        return;
    }

    if let Some(file) = valore::<String>(&argomenti, "--scatta") {
        let mut stato = Stato::default();
        if let Some(m) = valore(&argomenti, "--massa") {
            stato.masse = m;
        }
        if let Some(r) = valore(&argomenti, "--distanza") {
            stato.r_cam = r;
        }
        if let Some(e) = valore::<f64>(&argomenti, "--elevazione") {
            stato.elevazione = e.to_radians();
        }
        if let Some(a) = valore::<f64>(&argomenti, "--azimut") {
            stato.azimut = a.to_radians();
        }
        stato.confronti = argomenti.iter().any(|a| a == "--confronti");
        stato.doppler = !argomenti.iter().any(|a| a == "--senza-doppler");
        stato.disco = !argomenti.iter().any(|a| a == "--senza-disco");
        stato.telescopio_grande = argomenti.iter().any(|a| a == "--telescopio-grande");
        if let Some(i) = valore::<usize>(&argomenti, "--esperimento") {
            stato.lab.carica(i.min(lab::ESPERIMENTI.len() - 1));
        }
        stato.stanza = match valore::<u32>(&argomenti, "--banco") {
            Some(1) => stato::Stanza::Gemelli,
            Some(2) => stato::Stanza::Cadere,
            Some(3) => stato::Stanza::Curva,
            Some(4) => stato::Stanza::Neutroni,
            Some(5) => stato::Stanza::Giganti,
            Some(6) => stato::Stanza::Osservatorio,
            Some(7) => stato::Stanza::TreCorpi,
            Some(8) => stato::Stanza::Galassie,
            Some(9) => stato::Stanza::Onde,
            Some(10) => stato::Stanza::Fondo,
            Some(11) => stato::Stanza::Scala,
            Some(12) => stato::Stanza::Stelle,
            Some(13) => stato::Stanza::Orizzonte,
            Some(14) => stato::Stanza::Benvenuto,
            _ => stato::Stanza::BucoNero,
        };
        if let Some(l) = valore::<f64>(&argomenti, "--vista") {
            stato.scala.log_vista = l;
        }
        if let Some(i) = valore::<usize>(&argomenti, "--visita") {
            visite::inizia(&mut stato, i);
        }
        if let Some(i) = valore::<usize>(&argomenti, "--galassie") {
            use galassie::Scena::{Andromeda, Incontro, Una};
            stato.galassie.costruisci([Una, Incontro, Andromeda][i.min(2)]);
        }
        if argomenti.iter().any(|a| a == "--cielo") {
            stato.fondo.pagina = fondo::Pagina::Cielo;
            stato.fondo.attendi_mappa(60.0);
        }
        if argomenti.iter().any(|a| a == "--solo-macchie") {
            stato.fondo.con_dipolo = false;
            stato.fondo.con_galassia = false;
        }
        if argomenti.iter().any(|a| a == "--da-dentro") {
            stato.fondo.dall_interno = true;
        }
        if argomenti.iter().any(|a| a == "--macchie") {
            stato.fondo.pagina = fondo::Pagina::Macchie;
        }
        if let Some(z) = valore::<f64>(&argomenti, "--z") {
            stato.fondo.z = z;
        }
        if let Some(i) = valore::<usize>(&argomenti, "--scena") {
            stato.trecorpi.carica(i);
        }
        if argomenti.iter().any(|a| a == "--gemello") {
            stato.trecorpi.caos = true;
            stato.trecorpi.ricomincia();
        }
        if let Some(secondi) = valore::<f64>(&argomenti, "--mappa") {
            stato.trecorpi.mappa_pronta(secondi);
        }
        if let Some(i) = valore::<usize>(&argomenti, "--missione") {
            stato.osservatorio.sezione = osservatorio::Sezione::Altra(i);
            if argomenti.iter().any(|a| a == "--calcola") {
                stato.osservatorio.missioni[i].calcola();
            }
        } else if argomenti.iter().any(|a| a == "--ammasso") {
            stato.osservatorio.sezione = osservatorio::Sezione::Ammasso;
        } else if argomenti.iter().any(|a| a == "--calcola") {
            stato.osservatorio.calcola();
        }
        if let Some(t) = valore::<f64>(&argomenti, "--avanza") {
            stato.lab.velocita = 1.0;
            // Passi piccoli: i banchi a particelle limitano il lavoro per chiamata.
            let n = (t / 0.02).ceil().clamp(1.0, 2000.0);
            if stato.stanza == stato::Stanza::Galassie {
                for _ in 0..n as u32 {
                    stato.avanza(t / n);
                }
            } else {
                stato.avanza(t);
            }
        }
        scatta(
            Scatto {
                file,
                larghezza: valore(&argomenti, "--larghezza").unwrap_or(1440),
                altezza: valore(&argomenti, "--altezza").unwrap_or(860),
                interfaccia: !argomenti.iter().any(|a| a == "--senza-interfaccia"),
            },
            stato,
        );
        return;
    }

    let ciclo = EventLoop::<Finestra>::with_user_event().build().expect("ciclo eventi");
    let proxy = ciclo.create_proxy();
    let mut app = App {
        finestra: None,
        stato: Stato::default(),
        ultimo: Instant::now(),
        fps: 60.0,
        proxy,
        avviata: false,
    };
    ciclo.run_app(&mut app).expect("esecuzione");
}
