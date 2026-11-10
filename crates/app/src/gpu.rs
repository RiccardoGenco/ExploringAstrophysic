//! Rendering: buco nero in un'immagine HDR, bagliore, conversione e interfaccia.

use crate::lab::{Mappa, MAPPA_A, MAPPA_L};
use crate::sfondo;
use crate::stato::Parametri;
use egui_wgpu::wgpu;

const FORMATO_HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const LIVELLI_BAGLIORE: u32 = 5;
const LARGHEZZA_CIELO: u32 = 4096;
const ALTEZZA_CIELO: u32 = 2048;

struct Bersagli {
    dimensione: [u32; 2],
    hdr: wgpu::TextureView,
    bagliore: Vec<wgpu::TextureView>,
    gruppi_giu: Vec<wgpu::BindGroup>,
    gruppi_su: Vec<wgpu::BindGroup>,
    gruppo_finale: wgpu::BindGroup,
}

pub struct Disegnatore {
    nero: wgpu::RenderPipeline,
    giu: wgpu::RenderPipeline,
    su: wgpu::RenderPipeline,
    finale: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    gruppo_nero: wgpu::BindGroup,
    layout_post: wgpu::BindGroupLayout,
    campionatore: wgpu::Sampler,
    scie: wgpu::Texture,
    bersagli: Option<Bersagli>,
    pub egui: egui_wgpu::Renderer,
}

fn voce_texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn voce_campionatore(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

struct Pipeline<'a> {
    device: &'a wgpu::Device,
    layout: &'a wgpu::PipelineLayout,
    modulo: &'a wgpu::ShaderModule,
}

impl Pipeline<'_> {
    fn crea(
        &self,
        ingresso: &str,
        formato: wgpu::TextureFormat,
        fusione: Option<wgpu::BlendState>,
    ) -> wgpu::RenderPipeline {
        self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(ingresso),
            layout: Some(self.layout),
            vertex: wgpu::VertexState {
                module: self.modulo,
                entry_point: Some("vs_schermo"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: self.modulo,
                entry_point: Some(ingresso),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: formato,
                    blend: fusione,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }
}

impl Disegnatore {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, formato: wgpu::TextureFormat) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("parametri"),
            size: std::mem::size_of::<Parametri>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let campionatore = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let campionatore_post = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let dimensione_cielo = wgpu::Extent3d {
            width: LARGHEZZA_CIELO,
            height: ALTEZZA_CIELO,
            depth_or_array_layers: 1,
        };
        let cielo = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("cielo"),
            size: dimensione_cielo,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &cielo,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &sfondo::genera(LARGHEZZA_CIELO as usize, ALTEZZA_CIELO as usize),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(LARGHEZZA_CIELO * 4),
                rows_per_image: None,
            },
            dimensione_cielo,
        );
        let vista_cielo = cielo.create_view(&Default::default());

        let scie = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("scie"),
            size: wgpu::Extent3d {
                width: MAPPA_L as u32,
                height: MAPPA_A as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let vista_scie = scie.create_view(&Default::default());

        let layout_nero = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nero"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                voce_texture(1),
                voce_campionatore(2),
                voce_texture(3),
            ],
        });
        let gruppo_nero = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nero"),
            layout: &layout_nero,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&vista_cielo),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&campionatore),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&vista_scie),
                },
            ],
        });
        let layout_post = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post"),
            entries: &[voce_texture(0), voce_texture(1), voce_campionatore(2)],
        });

        let modulo_nero = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("nero"),
            source: wgpu::ShaderSource::Wgsl(include_str!("nero.wgsl").into()),
        });
        let modulo_post = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post"),
            source: wgpu::ShaderSource::Wgsl(include_str!("post.wgsl").into()),
        });
        let pipeline_layout_nero = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("nero"),
            bind_group_layouts: &[Some(&layout_nero)],
            immediate_size: 0,
        });
        let pipeline_layout_post = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post"),
            bind_group_layouts: &[Some(&layout_post)],
            immediate_size: 0,
        });

        let nero = Pipeline { device, layout: &pipeline_layout_nero, modulo: &modulo_nero }
            .crea("fs_nero", FORMATO_HDR, None);
        let post = Pipeline { device, layout: &pipeline_layout_post, modulo: &modulo_post };
        let somma = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let giu = post.crea("fs_giu", FORMATO_HDR, None);
        let su = post.crea(
            "fs_su",
            FORMATO_HDR,
            Some(wgpu::BlendState { color: somma, alpha: somma }),
        );
        let finale = post.crea("fs_finale", formato, None);

        let egui = egui_wgpu::Renderer::new(
            device,
            formato,
            egui_wgpu::RendererOptions { msaa_samples: 1, ..Default::default() },
        );

        Self {
            nero,
            giu,
            su,
            finale,
            buffer,
            gruppo_nero,
            layout_post,
            campionatore: campionatore_post,
            scie,
            bersagli: None,
            egui,
        }
    }

    fn gruppo(&self, device: &wgpu::Device, a: &wgpu::TextureView, b: &wgpu::TextureView) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post"),
            layout: &self.layout_post,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(a) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(b) },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.campionatore),
                },
            ],
        })
    }

    fn prepara_bersagli(&mut self, device: &wgpu::Device, dimensione: [u32; 2]) {
        if self.bersagli.as_ref().is_some_and(|b| b.dimensione == dimensione) {
            return;
        }
        let crea_vista = |d: [u32; 2]| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("hdr"),
                    size: wgpu::Extent3d {
                        width: d[0].max(1),
                        height: d[1].max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: FORMATO_HDR,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let hdr = crea_vista(dimensione);
        let bagliore: Vec<_> = (1..=LIVELLI_BAGLIORE)
            .map(|livello| crea_vista([dimensione[0] >> livello, dimensione[1] >> livello]))
            .collect();

        let gruppi_giu = (0..bagliore.len())
            .map(|i| {
                let sorgente = if i == 0 { &hdr } else { &bagliore[i - 1] };
                self.gruppo(device, sorgente, sorgente)
            })
            .collect();
        let gruppi_su = (0..bagliore.len() - 1)
            .map(|i| self.gruppo(device, &bagliore[i + 1], &bagliore[i + 1]))
            .collect();
        let gruppo_finale = self.gruppo(device, &hdr, &bagliore[0]);

        self.bersagli = Some(Bersagli { dimensione, hdr, bagliore, gruppi_giu, gruppi_su, gruppo_finale });
    }

    /// Carica la mappa delle scie del banco, se e' cambiata.
    pub fn aggiorna_scie(&self, queue: &wgpu::Queue, mappa: &mut Mappa) {
        if !mappa.sporca {
            return;
        }
        mappa.sporca = false;
        queue.write_texture(
            self.scie.as_image_copy(),
            &mappa.pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(MAPPA_L as u32 * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: MAPPA_L as u32,
                height: MAPPA_A as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    /// Disegna la scena nel rettangolo `area` (x, y, larghezza, altezza) di `vista`.
    pub fn disegna(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vista: &wgpu::TextureView,
        area: Option<[u32; 4]>,
        qualita: f32,
        mut parametri: Parametri,
        interfaccia: Option<(&[egui::ClippedPrimitive], &egui_wgpu::ScreenDescriptor)>,
    ) -> wgpu::SubmissionIndex {
        let mut encoder = device.create_command_encoder(&Default::default());
        // Senza telescopio il buco nero non si calcola affatto.
        let carico = if let Some(area) = area {
            let dimensione = [
                ((area[2] as f32 * qualita) as u32).max(2),
                ((area[3] as f32 * qualita) as u32).max(2),
            ];
            self.prepara_bersagli(device, dimensione);
            parametri.schermo[0] = dimensione[0] as f32;
            parametri.schermo[1] = dimensione[1] as f32;
            queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&parametri));
            let bersagli = self.bersagli.as_ref().expect("appena preparati");

            passaggio(&mut encoder, &bersagli.hdr, wgpu::LoadOp::Clear(wgpu::Color::BLACK), &self.nero, &self.gruppo_nero, None);
            for (i, bersaglio) in bersagli.bagliore.iter().enumerate() {
                passaggio(&mut encoder, bersaglio, wgpu::LoadOp::Clear(wgpu::Color::BLACK), &self.giu, &bersagli.gruppi_giu[i], None);
            }
            for i in (0..bersagli.gruppi_su.len()).rev() {
                passaggio(&mut encoder, &bersagli.bagliore[i], wgpu::LoadOp::Load, &self.su, &bersagli.gruppi_su[i], None);
            }
            passaggio(&mut encoder, vista, wgpu::LoadOp::Clear(wgpu::Color::BLACK), &self.finale, &bersagli.gruppo_finale, Some(area));
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        };

        let mut comandi = Vec::new();
        if let Some((primitive, schermo)) = interfaccia {
            comandi = self.egui.update_buffers(device, queue, &mut encoder, primitive, schermo);
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("interfaccia"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: vista,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations { load: carico, store: wgpu::StoreOp::Store },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui.render(&mut pass, primitive, schermo);
        }
        comandi.push(encoder.finish());
        queue.submit(comandi)
    }
}

fn passaggio(
    encoder: &mut wgpu::CommandEncoder,
    vista: &wgpu::TextureView,
    carica: wgpu::LoadOp<wgpu::Color>,
    pipeline: &wgpu::RenderPipeline,
    gruppo: &wgpu::BindGroup,
    area: Option<[u32; 4]>,
) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: None,
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: vista,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations { load: carica, store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some([x, y, l, a]) = area {
        pass.set_viewport(x as f32, y as f32, l as f32, a as f32, 0.0, 1.0);
    }
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, gruppo, &[]);
    pass.draw(0..3, 0..1);
}
