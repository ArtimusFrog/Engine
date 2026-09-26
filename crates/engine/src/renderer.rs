use std::path::Path;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::camera::rh;
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::app::Context;
use crate::scene::{Entity, EntityId};
use crate::assets::MeshId;
use crate::mesh::{SkinVertex, Vertex};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SHADOW_MAP_SIZE: u32 = 2048;
const MAX_LIGHTS: usize = 8;

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 12 => Float32x2];
const SKIN_ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![13 => Uint32x4, 14 => Float32x4];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
    3 => Float32x4, 4 => Float32x4, 5 => Float32x4, 6 => Float32x4,
    7 => Float32x3, 8 => Float32x3, 9 => Float32x3,
    10 => Float32x4, 11 => Float32x4,
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky_ambient: [f32; 4],
    ground_ambient: [f32; 4],
    fog: [f32; 4],
    shadow_params: [f32; 4],
    zenith: [f32; 4],
    sky_sun: [f32; 4],
    sky_moon: [f32; 4],
    sky_misc: [f32; 4],
    /// Wetter: Wolken, Nässe, Polarlicht, Regenbogen
    weather: [f32; 4],
    /// Je Punktlicht zwei Einträge: (Position, Reichweite), (Farbe, –).
    lights: [[f32; 4]; MAX_LIGHTS * 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
    color: [f32; 4],
    material: [f32; 4],
}

/// Fertig vorbereitete Benutzeroberfläche für einen Frame.
pub(crate) struct UiFrame {
    pub primitives: Vec<egui::ClippedPrimitive>,
    pub textures_delta: egui::TexturesDelta,
    pub pixels_per_point: f32,
}

/// Was im letzten Frame gezeichnet wurde.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub instances: usize,
    pub draw_calls: usize,
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    /// Knochen und Gewichte je Eckpunkt (nur Figuren, die die Grafikkarte verformt)
    skin: Option<wgpu::Buffer>,
    indices: wgpu::Buffer,
    index_count: u32,
    version: u64,
    texture: Option<crate::assets::TextureId>,
    double_sided: bool,
    alpha_cutout: bool,
}

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    /// Die Welt in geringerer Auflösung (Renderauflösung unter 100 %) und die Bindung, mit der
    /// sie aufs Fenster hochskaliert wird. `None` = direkt ins Fenster.
    scene_target: Option<(wgpu::TextureView, wgpu::BindGroup)>,
    blit_pipeline: wgpu::RenderPipeline,
    blit_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    /// Gewünschte Renderauflösung in Prozent, 0 = automatisch
    scale_setting: u8,
    /// Aktuelle Renderauflösung (Anteil je Achse)
    scale: f32,
    gpu_timer: Option<GpuTimer>,
    /// Für die Automatik: Rechenzeit der Grafikkarte (Summe, Anzahl) seit der letzten Anpassung
    scale_samples: (f32, u32),
    last_frame: Option<std::time::Instant>,
    pipeline: wgpu::RenderPipeline,
    sky_pipeline: wgpu::RenderPipeline,
    double_sided_pipeline: wgpu::RenderPipeline,
    shadow_cutout_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    /// Ausgeschnittene Flächen (Blätter, Gräser): dürfen Pixel verwerfen
    cutout_pipeline: wgpu::RenderPipeline,
    double_sided_cutout_pipeline: wgpu::RenderPipeline,
    /// Verformte Figuren: Bild und Schatten, dazu die Knochenmatrizen aller Figuren des Bildes
    skinned_pipeline: wgpu::RenderPipeline,
    skinned_shadow_pipeline: wgpu::RenderPipeline,
    palette_layout: wgpu::BindGroupLayout,
    palette_buffer: wgpu::Buffer,
    palette_bind_group: wgpu::BindGroup,
    palette_capacity: usize,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    shadow_bind_group: wgpu::BindGroup,
    shadow_map: wgpu::TextureView,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    meshes: Vec<GpuMesh>,
    textures: Vec<wgpu::BindGroup>,
    white_texture: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    texture_sampler: wgpu::Sampler,
    adapter_name: String,
    ui: egui_wgpu::Renderer,
    stats: RenderStats,
    /// Wartezeit auf die nächste Fläche der Grafikkarte beim letzten Bild (ms)
    acquire_ms: f32,
}

impl Renderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let surface = instance
            .create_surface(window.clone())
            .expect("Fenster-Oberfläche konnte nicht erstellt werden");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .expect("Keine passende Grafikkarte gefunden");
        let info = adapter.get_info();
        let adapter_name = format!("{} ({:?})", info.name, info.backend);
        log::info!("Grafikkarte: {adapter_name}");

        // Zeitmessung auf der Grafikkarte (für die automatische Renderauflösung), wenn vorhanden
        let required_features = adapter.features() & wgpu::Features::TIMESTAMP_QUERY;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor { label: Some("engine"), required_features, ..Default::default() })
            .await
            .expect("Grafikkarte konnte nicht initialisiert werden");

        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .expect("Fenster-Oberfläche wird von der Grafikkarte nicht unterstützt");
        // Shader rechnen in linearem Licht; ein sRGB-Ziel übernimmt die Gammakorrektur.
        if let Some(format) = surface.get_capabilities(&adapter).formats.into_iter().find(|f| f.is_srgb()) {
            config.format = format;
        }
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basic"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/basic.wgsl").into()),
        });

        let globals_entry = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("globals"),
            entries: &[
                globals_entry,
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        // Der Schatten-Durchgang schreibt in die Schattenkarte und darf sie deshalb
        // nicht gleichzeitig gebunden haben.
        let shadow_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("shadow globals"),
            entries: &[globals_entry],
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals"),
            size: size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let shadow_map = device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("shadow map"),
                size: wgpu::Extent3d { width: SHADOW_MAP_SIZE, height: SHADOW_MAP_SIZE, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
            .create_view(&Default::default());
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("globals"),
            layout: &globals_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: globals_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&shadow_map) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
            ],
        });
        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow globals"),
            layout: &shadow_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: globals_buffer.as_entire_binding() }],
        });

        let vertex_buffers = [
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &VERTEX_ATTRIBUTES,
            }),
            Some(wgpu::VertexBufferLayout {
                array_stride: size_of::<Instance>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &INSTANCE_ATTRIBUTES,
            }),
        ];

        let shadow_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow"),
            bind_group_layouts: &[Some(&shadow_layout)],
            immediate_size: 0,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });

        // Gruppe 1: Textur des Meshs (Meshes ohne Textur bekommen ein weißes Pixel).
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("textur"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("textur"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            anisotropy_clamp: 8,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            ..Default::default()
        });
        let white = crate::assets::Image { width: 1, height: 1, rgba: vec![255; 4] };
        let white_texture = texture_bind_group(&device, &queue, &texture_layout, &texture_sampler, &white);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("basic"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        // Normal nur Vorderseiten; beidseitige Meshes (Blätter) ohne Rückseiten-Culling.
        // `cutout`: darf Pixel verwerfen (Blätter) – sonst bleibt der frühe Tiefentest an.
        let main_pipeline = |cull_mode: Option<wgpu::Face>, cutout: bool| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("basic"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &vertex_buffers,
                },
                primitive: wgpu::PrimitiveState { cull_mode, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if cutout { "fs_main" } else { "fs_opaque" }),
                    compilation_options: Default::default(),
                    targets: &[Some(config.format.into())],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = main_pipeline(Some(wgpu::Face::Back), false);
        let double_sided_pipeline = main_pipeline(None, false);
        let cutout_pipeline = main_pipeline(Some(wgpu::Face::Back), true);
        let double_sided_cutout_pipeline = main_pipeline(None, true);

        // Schatten ausgeschnittener Flächen: Textur lesen und durchsichtige Pixel weglassen.
        let shadow_cutout_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow cutout"),
            bind_group_layouts: &[Some(&shadow_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let shadow_cutout_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow cutout"),
            layout: Some(&shadow_cutout_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow_uv"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_shadow_cutout"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });

        // Verformte Figuren: zusätzlicher Eckpunktstrom (Knochen, Gewichte) und die Matrizen als
        // Speicherpuffer in Gruppe 2.
        let palette_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("knochen"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let skinned_buffers = [
            vertex_buffers[0].clone(),
            vertex_buffers[1].clone(),
            Some(wgpu::VertexBufferLayout { array_stride: size_of::<SkinVertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex, attributes: &SKIN_ATTRIBUTES }),
        ];
        let skinned_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("verformt"),
            bind_group_layouts: &[Some(&globals_layout), Some(&texture_layout), Some(&palette_layout)],
            immediate_size: 0,
        });
        let skinned_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("verformt"),
            layout: Some(&skinned_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_skinned"), compilation_options: Default::default(), buffers: &skinned_buffers },
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let skinned_shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("verformt schatten"),
            bind_group_layouts: &[Some(&shadow_layout), Some(&texture_layout), Some(&palette_layout)],
            immediate_size: 0,
        });
        let skinned_shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("verformt schatten"),
            layout: Some(&skinned_shadow_layout),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_shadow_skinned"), compilation_options: Default::default(), buffers: &skinned_buffers },
            primitive: wgpu::PrimitiveState { cull_mode: Some(wgpu::Face::Back), ..Default::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState { constant: 2, slope_scale: 2.0, clamp: 0.0 },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let palette_capacity = 256;
        let palette_buffer = Self::create_palette_buffer(&device, palette_capacity);
        let palette_bind_group = Self::create_palette_bind_group(&device, &palette_layout, &palette_buffer);

        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            // Der Himmel liegt hinter allem (Tiefe 1): er wird zuletzt gezeichnet und nur dort,
            // wo der Tiefenpuffer noch leer ist; nie in den Tiefenpuffer schreiben.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });

        // Hochskalieren der Welt aufs Fenster
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blit.wgsl").into()),
        });
        let blit_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit"),
            bind_group_layouts: &[Some(&blit_layout)],
            immediate_size: 0,
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState { module: &blit_shader, entry_point: Some("vs_blit"), compilation_options: Default::default(), buffers: &[] },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_blit"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let gpu_timer = device.features().contains(wgpu::Features::TIMESTAMP_QUERY).then(|| GpuTimer::new(&device, &queue));

        let ui = egui_wgpu::Renderer::new(&device, config.format, egui_wgpu::RendererOptions::default());

        let instance_capacity = 256;
        let instance_buffer = Self::create_instance_buffer(&device, instance_capacity);
        let depth = Self::create_depth(&device, config.width, config.height);

        Renderer {
            surface,
            device,
            queue,
            config,
            depth,
            scene_target: None,
            blit_pipeline,
            blit_layout,
            blit_sampler,
            scale_setting: 0,
            scale: 1.0,
            gpu_timer,
            scale_samples: (0.0, 0),
            last_frame: None,
            pipeline,
            sky_pipeline,
            double_sided_pipeline,
            shadow_cutout_pipeline,
            shadow_pipeline,
            cutout_pipeline,
            double_sided_cutout_pipeline,
            skinned_pipeline,
            skinned_shadow_pipeline,
            palette_layout,
            palette_buffer,
            palette_bind_group,
            palette_capacity,
            globals_buffer,
            globals_bind_group,
            shadow_bind_group,
            shadow_map,
            instance_buffer,
            instance_capacity,
            meshes: Vec::new(),
            textures: Vec::new(),
            white_texture,
            texture_layout,
            texture_sampler,
            adapter_name,
            ui,
            stats: RenderStats::default(),
            acquire_ms: 0.0,
        }
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return; // minimiert
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.rebuild_targets();
    }

    /// Renderauflösung in Prozent (50–100) oder 0 = automatisch: die Grafikkarte soll dann
    /// etwa 75 Bilder pro Sekunde schaffen, die Auflösung sinkt dafür bis auf die Hälfte.
    pub fn set_render_scale(&mut self, percent: u8) {
        if percent == self.scale_setting {
            return;
        }
        self.scale_setting = percent;
        self.scale = if percent == 0 { 1.0 } else { (percent as f32 / 100.0).clamp(0.5, 1.0) };
        self.scale_samples = (0.0, 0);
        self.rebuild_targets();
    }

    /// Aktuelle Renderauflösung (Anteil je Achse, 1 = volle Auflösung).
    pub fn render_scale(&self) -> f32 {
        self.scale
    }

    /// Tiefenpuffer und (bei verkleinerter Auflösung) die Farbfläche für die Welt neu anlegen.
    fn rebuild_targets(&mut self) {
        let (width, height) = self.scene_size();
        self.depth = Self::create_depth(&self.device, width, height);
        self.scene_target = (self.scale < 0.999).then(|| {
            let view = self
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("welt"),
                    size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.config.format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default());
            let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("blit"),
                layout: &self.blit_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.blit_sampler) },
                ],
            });
            (view, bind_group)
        });
    }

    /// Größe, in der die Welt gezeichnet wird.
    fn scene_size(&self) -> (u32, u32) {
        let scaled = |v: u32| ((v as f32 * self.scale).round() as u32).max(1);
        (scaled(self.config.width), scaled(self.config.height))
    }

    /// Automatik: je nach Rechenzeit der Grafikkarte die Auflösung senken oder wieder anheben.
    /// Ohne Zeitmessung auf der Grafikkarte dient die Bildrate als Maß.
    fn adapt_scale(&mut self) {
        let now = std::time::Instant::now();
        let frame_ms = self.last_frame.replace(now).map(|t| (now - t).as_secs_f32() * 1000.0);
        if self.scale_setting != 0 {
            return;
        }
        let sample = match &self.gpu_timer {
            Some(timer) => timer.last_ms,
            None => frame_ms,
        };
        let Some(ms) = sample else { return };
        self.scale_samples.0 += ms;
        self.scale_samples.1 += 1;
        if self.scale_samples.1 < 20 {
            return;
        }
        let average = self.scale_samples.0 / self.scale_samples.1 as f32;
        self.scale_samples = (0.0, 0);
        const TARGET_MS: f32 = 13.0;
        let wanted = if average > TARGET_MS * 1.15 {
            // Pixelzahl wächst mit dem Quadrat der Auflösung
            self.scale * (TARGET_MS / average).sqrt()
        } else if average < TARGET_MS * 0.7 {
            self.scale + 0.05
        } else {
            return;
        };
        let wanted = ((wanted * 20.0).round() / 20.0).clamp(0.5, 1.0);
        if (wanted - self.scale).abs() > 0.01 {
            self.scale = wanted;
            self.rebuild_targets();
        }
    }

    /// Rechenzeit der Grafikkarte für die Welt im letzten gemessenen Bild (ms), wenn messbar.
    pub fn gpu_ms(&self) -> Option<f32> {
        self.gpu_timer.as_ref().and_then(|t| t.last_ms)
    }

    /// Davon der Schattendurchgang (ms).
    pub fn gpu_shadow_ms(&self) -> Option<f32> {
        self.gpu_timer.as_ref().and_then(|t| t.shadow_ms)
    }

    /// Zeichnet einen Frame ins Fenster.
    pub fn render(&mut self, ctx: &Context, ui: Option<&UiFrame>) {
        // Neue UI-Texturen (Schrift!) sofort hochladen – egui schickt sie nur ein einziges Mal.
        // Fällt dieses Bild gleich aus (auf dem Mac beim Start häufig), wären sie sonst für
        // immer verloren und die ganze Oberfläche bliebe unsichtbar.
        if let Some(ui) = ui {
            self.upload_ui_textures(ui);
        }
        let acquire = std::time::Instant::now();
        let current = self.surface.get_current_texture();
        self.acquire_ms = acquire.elapsed().as_secs_f32() * 1000.0;
        let frame = match current {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // Dieses Bild noch zeigen, fürs nächste neu konfigurieren.
                self.surface.configure(&self.device, &self.config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            other => {
                log::error!("Fensterbild nicht verfügbar: {other:?}");
                return;
            }
        };
        let view = frame.texture.create_view(&Default::default());
        self.draw(ctx, &view, ui);
        self.queue.present(frame);
    }

    /// Zeichnet einen Frame in eine unsichtbare Textur und speichert ihn als PNG.
    pub fn screenshot(&mut self, ctx: &Context, ui: Option<&UiFrame>, path: &Path) -> Result<(), String> {
        let (width, height) = (self.config.width, self.config.height);
        let size = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("screenshot"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.draw(ctx, &texture.create_view(&Default::default()), ui);

        let unpadded = width * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot"),
            size: padded as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded), rows_per_image: Some(height) },
            },
            size,
        );
        self.queue.submit([encoder.finish()]);

        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |result| {
            if let Err(e) = result {
                log::error!("Screenshot-Puffer nicht lesbar: {e}");
            }
        });
        self.device.poll(wgpu::PollType::wait_indefinitely()).map_err(|e| e.to_string())?;

        let data = slice.get_mapped_range().map_err(|e| e.to_string())?;
        let swap_red_blue = matches!(
            self.config.format,
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
        );
        let mut pixels = Vec::with_capacity((unpadded * height) as usize);
        for row in data.chunks(padded as usize) {
            for px in row[..unpadded as usize].chunks_exact(4) {
                if swap_red_blue {
                    pixels.extend_from_slice(&[px[2], px[1], px[0], 255]);
                } else {
                    pixels.extend_from_slice(&[px[0], px[1], px[2], 255]);
                }
            }
        }
        drop(data);
        buffer.unmap();

        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        image::save_buffer(path, &pixels, width, height, image::ExtendedColorType::Rgba8).map_err(|e| e.to_string())
    }

    fn draw(&mut self, ctx: &Context, target: &wgpu::TextureView, ui: Option<&UiFrame>) {
        self.sync_assets(&ctx.assets);
        if let Some(timer) = &mut self.gpu_timer {
            timer.collect(&self.device);
        }
        self.adapt_scale();

        let env = &ctx.env;
        let aspect = self.config.width as f32 / self.config.height as f32;
        let texel_world = 2.0 * env.shadow_range / SHADOW_MAP_SIZE as f32;
        let view_proj = ctx.camera.view_projection(aspect);
        let light_view_proj = sun_view_projection(ctx);
        // Die nächsten Punktlichter zur Kamera
        let mut nearby: Vec<_> = ctx.lights.iter().collect();
        nearby.sort_by(|a, b| {
            let da = a.position.distance_squared(ctx.camera.position);
            let db = b.position.distance_squared(ctx.camera.position);
            da.total_cmp(&db)
        });
        let mut lights = [[0.0f32; 4]; MAX_LIGHTS * 2];
        let light_count = nearby.len().min(MAX_LIGHTS);
        for (i, light) in nearby.iter().take(MAX_LIGHTS).enumerate() {
            lights[i * 2] = light.position.extend(light.radius).into();
            lights[i * 2 + 1] = light.color.extend(0.0).into();
        }
        let globals = Globals {
            view_proj: view_proj.to_cols_array_2d(),
            light_view_proj: light_view_proj.to_cols_array_2d(),
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            camera_pos: ctx.camera.position.extend(1.0).into(),
            sun_dir: env.sun_direction.normalize().extend(0.0).into(),
            sun_color: env.sun_color.extend(0.0).into(),
            sky_ambient: env.sky_ambient.extend(0.0).into(),
            ground_ambient: env.ground_ambient.extend(0.0).into(),
            fog: env.sky_color.extend(env.fog_density).into(),
            shadow_params: [1.0 / SHADOW_MAP_SIZE as f32, texel_world * 1.5, ctx.time.elapsed, 0.0],
            zenith: env.zenith_color.extend(env.exposure).into(),
            sky_sun: env.sky.sun_direction.normalize().extend(env.sky.sun_visible).into(),
            sky_moon: env.sky.moon_direction.normalize().extend(env.sky.moon_visible).into(),
            sky_misc: [env.sky.stars, env.sky.glow, light_count as f32, env.sky.mist],
            weather: [env.sky.clouds, env.sky.rain, env.sky.aurora, env.sky.rainbow],
            lights,
        };
        self.queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Nur zeichnen, was im Blickfeld liegt (bzw. für den Schatten im Bereich der Sonne),
        // und zwar in der Detailstufe, die zur Entfernung passt.
        let camera_planes = frustum_planes(view_proj);
        let shadow_planes = frustum_planes(light_view_proj);
        // Über 100.000 Objekte: die Prüfung läuft auf allen Prozessorkernen, jeder Kern nimmt
        // sich einen Abschnitt der Szene vor.
        let skinnable: Vec<bool> = self.meshes.iter().map(|m| m.skin.is_some()).collect();
        let view = CullView { ctx, camera_planes: &camera_planes, shadow_planes: &shadow_planes, skinnable: &skinnable };
        let entities = ctx.scene.slots();
        let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, 8);
        let chunk = entities.len().div_ceil(threads).max(4096);
        let parts: Vec<Culled> = if entities.len() <= chunk {
            vec![view.cull(entities, 0)]
        } else {
            std::thread::scope(|s| {
                let view = &view;
                let handles: Vec<_> = entities.chunks(chunk).enumerate().map(|(k, part)| s.spawn(move || view.cull(part, k * chunk))).collect();
                handles.into_iter().map(|h| h.join().expect("Sichtprüfung")).collect()
            })
        };
        let mut main_list: Vec<(MeshId, Instance)> = Vec::with_capacity(parts.iter().map(|p| p.main.len()).sum());
        let mut shadow_list: Vec<(MeshId, Instance)> = Vec::with_capacity(parts.iter().map(|p| p.shadow.len()).sum());
        let mut skinned_main: Vec<(MeshId, Instance)> = Vec::new();
        let mut skinned_shadow: Vec<(MeshId, Instance)> = Vec::new();
        let mut palette: Vec<[[f32; 4]; 4]> = Vec::new();
        for mut part in parts {
            main_list.append(&mut part.main);
            shadow_list.append(&mut part.shadow);
            // Die Knochen jedes Abschnitts beginnen hinter denen der vorigen
            let base = palette.len() as f32;
            for (_, instance) in part.skinned_main.iter_mut().chain(part.skinned_shadow.iter_mut()) {
                instance.material[3] += base;
            }
            skinned_main.append(&mut part.skinned_main);
            skinned_shadow.append(&mut part.skinned_shadow);
            palette.append(&mut part.palette);
        }

        // Nach Mesh sortiert: jedes Mesh mit einem einzigen Draw-Call (Instancing).
        let mut instances: Vec<Instance> = Vec::with_capacity(main_list.len() + shadow_list.len());
        let mut shadow_batches = batch(&mut shadow_list, &mut instances);
        let batches = batch(&mut main_list, &mut instances);
        // Von vorn nach hinten, feste Flächen vor ausgeschnittenen (Blätter): Verdecktes
        // scheitert dann schon am frühen Tiefentest und kostet keine Beleuchtung.
        let eye = ctx.camera.position;
        let mut keyed: Vec<((bool, f32), (usize, std::ops::Range<u32>))> = batches
            .into_iter()
            .map(|(mesh, range)| {
                let nearest = instances[range.start as usize..range.end as usize]
                    .iter()
                    .map(|i| Vec3::new(i.model[3][0], i.model[3][1], i.model[3][2]).distance_squared(eye))
                    .fold(f32::MAX, f32::min);
                ((self.meshes[mesh].alpha_cutout, nearest), (mesh, range))
            })
            .collect();
        keyed.sort_by(|a, b| a.0 .0.cmp(&b.0 .0).then(a.0 .1.total_cmp(&b.0 .1)));
        let mut batches: Vec<(usize, std::ops::Range<u32>)> = keyed.into_iter().map(|(_, b)| b).collect();
        let skinned_shadow_batches = batch(&mut skinned_shadow, &mut instances);
        let skinned_batches = batch(&mut skinned_main, &mut instances);
        let drawn = main_list.len() + skinned_main.len();
        if !palette.is_empty() {
            if palette.len() > self.palette_capacity {
                self.palette_capacity = palette.len().next_power_of_two();
                self.palette_buffer = Self::create_palette_buffer(&self.device, self.palette_capacity);
                self.palette_bind_group = Self::create_palette_bind_group(&self.device, &self.palette_layout, &self.palette_buffer);
            }
            self.queue.write_buffer(&self.palette_buffer, 0, bytemuck::cast_slice(&palette));
        }

        // Partikel als zwei eigene Stapel: kleine Würfel und runde Puffs
        for (round, mesh) in [(false, ctx.assets.cube()), (true, ctx.assets.sphere())] {
            let first = instances.len() as u32;
            instances.extend(ctx.particles.instances().filter(|p| p.3 == round).map(|(model, color, glow, _)| {
                let material = if glow > 0.0 { crate::scene::Material::Emissive { glow } } else { crate::scene::Material::Standard };
                instance(model, color, material.shader_params())
            }));
            if instances.len() as u32 > first {
                batches.push((mesh.0 as usize, first..instances.len() as u32));
                shadow_batches.push((mesh.0 as usize, first..instances.len() as u32));
            }
        }
        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len().next_power_of_two();
            self.instance_buffer = Self::create_instance_buffer(&self.device, self.instance_capacity);
        }
        self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        let sky = env.sky_color;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let timing = self.gpu_timer.as_mut().and_then(|t| t.begin());
        let scene_view = self.scene_target.as_ref().map_or(target, |(view, _)| view);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: timing.as_ref().map(|query_set| wgpu::RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: Some(1),
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_bind_group, &[]);
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            self.draw_batches(&mut pass, &shadow_batches, true);
            self.draw_skinned(&mut pass, &skinned_shadow_batches, true);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: sky.x as f64, g: sky.y as f64, b: sky.z as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Discard }),
                    stencil_ops: None,
                }),
                timestamp_writes: timing.as_ref().map(|query_set| wgpu::RenderPassTimestampWrites {
                    query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(2),
                }),
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            self.draw_batches(&mut pass, &batches, false);
            self.draw_skinned(&mut pass, &skinned_batches, false);
            // Himmel zuletzt: er wird nur dort berechnet, wo noch nichts gezeichnet ist.
            pass.set_bind_group(1, &self.white_texture, &[]);
            pass.set_pipeline(&self.sky_pipeline);
            pass.draw(0..3, 0..1);
        }
        // Verkleinert gezeichnete Welt aufs Fenster hochskalieren
        if let Some((_, bind_group)) = &self.scene_target {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.blit_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        if let Some(timer) = &mut self.gpu_timer {
            timer.resolve(&mut encoder);
        }
        self.stats = RenderStats {
            instances: drawn,
            draw_calls: batches.len() + shadow_batches.len() + skinned_batches.len() + skinned_shadow_batches.len(),
        };

        // Benutzeroberfläche über die 3D-Szene legen.
        if let Some(ui) = ui {
            let screen = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: ui.pixels_per_point,
            };
            let extra = self.ui.update_buffers(&self.device, &self.queue, &mut encoder, &ui.primitives, &screen);
            {
                let mut pass = encoder
                    .begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("ui"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: target,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    })
                    .forget_lifetime();
                self.ui.render(&mut pass, &ui.primitives, &screen);
            }
            self.queue.submit(extra.into_iter().chain([encoder.finish()]));
            for id in &ui.textures_delta.free {
                self.ui.free_texture(id);
            }
        } else {
            self.queue.submit([encoder.finish()]);
        }
        if let Some(timer) = &mut self.gpu_timer {
            timer.request_readback();
        }
    }

    pub fn stats(&self) -> RenderStats {
        self.stats
    }

    /// Wie lange das letzte Bild auf eine freie Fläche der Grafikkarte gewartet hat (ms).
    pub fn last_acquire_ms(&self) -> f32 {
        self.acquire_ms
    }

    /// VSync an: Bildrate folgt dem Monitor, kein Tearing. Aus: so schnell wie möglich.
    pub fn set_vsync(&mut self, vsync: bool) {
        self.config.present_mode = if vsync { wgpu::PresentMode::AutoVsync } else { wgpu::PresentMode::AutoNoVsync };
        self.surface.configure(&self.device, &self.config);
    }

    /// Überträgt neue oder geänderte UI-Texturen (Schrift, Bilder) auf die Grafikkarte.
    fn upload_ui_textures(&mut self, ui: &UiFrame) {
        for (id, deltas) in &ui.textures_delta.set {
            for delta in deltas {
                self.ui.update_texture(&self.device, &self.queue, *id, delta);
            }
        }
    }

    /// Zeichnet die Stapel und wählt je Mesh die passende Pipeline: im Schatten-Durchgang
    /// brauchen nur Ausschnitt-Meshes (Blätter) ihre Textur, im Hauptdurchgang alle.
    fn draw_batches(&self, pass: &mut wgpu::RenderPass<'_>, batches: &[(usize, std::ops::Range<u32>)], shadow: bool) {
        let mut current: Option<*const wgpu::RenderPipeline> = None;
        for (mesh, instances) in batches {
            let mesh = &self.meshes[*mesh];
            if mesh.index_count == 0 {
                continue;
            }
            let (pipeline, textured) = match (shadow, mesh.alpha_cutout, mesh.double_sided) {
                (true, true, _) => (&self.shadow_cutout_pipeline, true),
                (true, false, _) => (&self.shadow_pipeline, false),
                (false, true, true) => (&self.double_sided_cutout_pipeline, true),
                (false, true, false) => (&self.cutout_pipeline, true),
                (false, false, true) => (&self.double_sided_pipeline, true),
                (false, false, false) => (&self.pipeline, true),
            };
            if current != Some(pipeline as *const _) {
                pass.set_pipeline(pipeline);
                current = Some(pipeline as *const _);
            }
            if textured {
                let texture = mesh.texture.and_then(|t| self.textures.get(t.0 as usize)).unwrap_or(&self.white_texture);
                pass.set_bind_group(1, texture, &[]);
            }
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, instances.clone());
        }
    }

    /// Zeichnet verformte Figuren (eigene Pipeline, Knochenmatrizen in Gruppe 2).
    fn draw_skinned(&self, pass: &mut wgpu::RenderPass<'_>, batches: &[(usize, std::ops::Range<u32>)], shadow: bool) {
        if batches.is_empty() {
            return;
        }
        pass.set_pipeline(if shadow { &self.skinned_shadow_pipeline } else { &self.skinned_pipeline });
        pass.set_bind_group(2, &self.palette_bind_group, &[]);
        pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        if shadow {
            pass.set_bind_group(1, &self.white_texture, &[]);
        }
        for (mesh, instances) in batches {
            let mesh = &self.meshes[*mesh];
            let Some(skin) = &mesh.skin else { continue };
            if mesh.index_count == 0 {
                continue;
            }
            if !shadow {
                let texture = mesh.texture.and_then(|t| self.textures.get(t.0 as usize)).unwrap_or(&self.white_texture);
                pass.set_bind_group(1, texture, &[]);
            }
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_vertex_buffer(2, skin.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, instances.clone());
        }
    }

    fn create_palette_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("knochen"),
            size: (capacity * 64) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_palette_bind_group(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, buffer: &wgpu::Buffer) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("knochen"),
            layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        })
    }

    /// Lädt neue und geänderte Meshes und Texturen auf die Grafikkarte.
    fn sync_assets(&mut self, assets: &crate::assets::Assets) {
        for image in &assets.textures()[self.textures.len()..] {
            let bind_group = self.create_texture(image);
            self.textures.push(bind_group);
        }
        for (index, slot) in assets.mesh_slots().iter().enumerate() {
            let mesh = &slot.data;
            let vertex_bytes: &[u8] = bytemuck::cast_slice(&mesh.vertices);
            let index_bytes: &[u8] = bytemuck::cast_slice(&mesh.indices);
            if let Some(gpu) = self.meshes.get_mut(index) {
                if gpu.version == slot.version {
                    continue;
                }
                // Passt die neue Geometrie in die alten Puffer, nur überschreiben.
                if vertex_bytes.len() as u64 <= gpu.vertices.size() && index_bytes.len() as u64 <= gpu.indices.size() {
                    self.queue.write_buffer(&gpu.vertices, 0, vertex_bytes);
                    self.queue.write_buffer(&gpu.indices, 0, index_bytes);
                    gpu.index_count = mesh.indices.len() as u32;
                    gpu.version = slot.version;
                    gpu.texture = mesh.texture;
                    gpu.double_sided = mesh.double_sided;
                    gpu.alpha_cutout = mesh.alpha_cutout;
                    continue;
                }
            }
            let buffer = |bytes: &[u8], usage: wgpu::BufferUsages| {
                // Etwas Reserve, damit wachsende Meshes nicht jedes Mal neu angelegt werden.
                let size = (bytes.len() as u64 * 5 / 4).max(256).next_multiple_of(4);
                let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("mesh"),
                    size,
                    usage: usage | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                self.queue.write_buffer(&buffer, 0, bytes);
                buffer
            };
            let skin_bytes: &[u8] = bytemuck::cast_slice(&mesh.skin);
            let gpu = GpuMesh {
                vertices: buffer(vertex_bytes, wgpu::BufferUsages::VERTEX),
                skin: (!mesh.skin.is_empty()).then(|| buffer(skin_bytes, wgpu::BufferUsages::VERTEX)),
                indices: buffer(index_bytes, wgpu::BufferUsages::INDEX),
                index_count: mesh.indices.len() as u32,
                version: slot.version,
                texture: mesh.texture,
                double_sided: mesh.double_sided,
                alpha_cutout: mesh.alpha_cutout,
            };
            if index < self.meshes.len() {
                self.meshes[index] = gpu;
            } else {
                self.meshes.push(gpu);
            }
        }
    }

    fn create_texture(&self, image: &crate::assets::Image) -> wgpu::BindGroup {
        texture_bind_group(&self.device, &self.queue, &self.texture_layout, &self.texture_sampler, image)
    }

    fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instances"),
            size: (capacity * size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn create_depth(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("depth"),
                size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
            .create_view(&Default::default())
    }
}

/// Orthografische Sicht der Sonne auf den Bereich vor der Kamera. Der Mittelpunkt rastet
/// auf Texel der Schattenkarte ein, sonst flimmern Schattenkanten beim Bewegen.
fn sun_view_projection(ctx: &Context) -> Mat4 {
    let range = ctx.env.shadow_range;
    let to_sun = ctx.env.sun_direction.normalize();
    let up = if to_sun.y.abs() > 0.99 { Vec3::Z } else { Vec3::Y };
    let view = rh::view::look_to_mat4(Vec3::ZERO, -to_sun, up);

    let focus = ctx.camera.position + ctx.camera.forward() * range * 0.5;
    let center = view.transform_point3(focus);
    let texel = 2.0 * range / SHADOW_MAP_SIZE as f32;
    let (x, y) = ((center.x / texel).round() * texel, (center.y / texel).round() * texel);
    let depth = -center.z;
    let proj = rh::proj::directx::orthographic(x - range, x + range, y - range, y + range, depth - 200.0, depth + 200.0);
    proj * view
}

fn instance(model: Mat4, color: glam::Vec4, material: [f32; 4]) -> Instance {
    Instance {
        model: model.to_cols_array_2d(),
        normal: Mat3::from_mat4(model).inverse().transpose().to_cols_array_2d(),
        color: color.into(),
        material,
    }
}

fn texture_bind_group(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    sampler: &wgpu::Sampler,
    image: &crate::assets::Image,
) -> wgpu::BindGroup {
    let levels = image.mip_chain();
    let data: Vec<u8> = levels.iter().flat_map(|l| l.rgba.iter().copied()).collect();
    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            label: Some("textur"),
            size: wgpu::Extent3d { width: image.width, height: image.height, depth_or_array_layers: 1 },
            mip_level_count: levels.len() as u32,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // PNG-Farben sind sRGB; so liefert der Shader beim Lesen lineare Werte.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::LayerMajor,
        &data,
    );
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("textur"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&texture.create_view(&Default::default())) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

/// Die sechs Ebenen des Sichtkörpers einer Projektion (Normalen zeigen nach innen).
/// Tiefe wie bei wgpu von 0 bis 1.
fn frustum_planes(view_proj: Mat4) -> [glam::Vec4; 6] {
    let (r0, r1, r2, r3) = (view_proj.row(0), view_proj.row(1), view_proj.row(2), view_proj.row(3));
    [r3 + r0, r3 - r0, r3 + r1, r3 - r1, r2, r3 - r2].map(|p| p / p.truncate().length().max(1e-6))
}

/// Liegt die Kugel zumindest teilweise im Sichtkörper?
fn sphere_visible(planes: &[glam::Vec4; 6], center: glam::Vec3, radius: f32) -> bool {
    planes.iter().all(|p| p.truncate().dot(center) + p.w >= -radius)
}

/// Sortiert nach Mesh, hängt die Instanzen an und liefert die Stapel (Mesh, Instanzbereich).
/// Was ein Abschnitt der Szene zum Bild beiträgt (siehe `CullView::cull`).
#[derive(Default)]
struct Culled {
    main: Vec<(MeshId, Instance)>,
    shadow: Vec<(MeshId, Instance)>,
    /// Verformte Figuren; ihre Knochenmatrizen in `palette` (Beginn in `material[3]`)
    skinned_main: Vec<(MeshId, Instance)>,
    skinned_shadow: Vec<(MeshId, Instance)>,
    palette: Vec<[[f32; 4]; 4]>,
}

/// Alles, was die Sichtprüfung eines Bildes braucht (wird von mehreren Kernen gleichzeitig gelesen).
struct CullView<'a> {
    ctx: &'a Context,
    camera_planes: &'a [glam::Vec4; 6],
    shadow_planes: &'a [glam::Vec4; 6],
    /// Je Mesh: hat es Knochendaten auf der Grafikkarte?
    skinnable: &'a [bool],
}

impl CullView<'_> {
    /// Prüft einen Abschnitt der Szene (`first` = Nummer des ersten Objekts): nur was im
    /// Blickfeld liegt (bzw. für den Schatten im Bereich der Sonne), in passender Detailstufe.
    fn cull(&self, entities: &[Option<Entity>], first: usize) -> Culled {
        let ctx = self.ctx;
        let slots = ctx.assets.mesh_slots();
        let mut out = Culled::default();
        for (i, entity) in entities.iter().enumerate() {
            let Some(entity) = entity else { continue };
            if !entity.visible {
                continue;
            }
            let (center, radius) = slots[entity.mesh.0 as usize].bounds;
            // Objekte ohne Eltern (fast alles: Bäume, Gras, Felsen) werden erst geprüft und nur
            // bei Bedarf in eine Matrix gerechnet.
            let (world_center, scale, model) = match entity.parent {
                None => {
                    let t = &entity.transform;
                    (t.position + t.rotation * (center * t.scale), t.scale.abs().max_element(), None)
                }
                Some(_) => {
                    let model = ctx.scene.world_matrix(EntityId(first + i));
                    let scale = model.x_axis.truncate().length().max(model.y_axis.truncate().length()).max(model.z_axis.truncate().length());
                    (model.transform_point3(center), scale, Some(model))
                }
            };
            // Etwas Luft für Wind und Wellen, die der Shader noch verschiebt.
            let world_radius = radius * scale + 1.0;
            let Some(mesh) = ctx.assets.mesh_at_distance(entity.mesh, world_center.distance(ctx.camera.position)) else { continue };
            let seen = sphere_visible(self.camera_planes, world_center, world_radius);
            let casts_shadow = sphere_visible(self.shadow_planes, world_center, world_radius);
            if !seen && !casts_shadow {
                continue;
            }
            let model = model.unwrap_or_else(|| entity.transform.matrix());
            let gpu_skin = !entity.joints.is_empty() && self.skinnable.get(mesh.0 as usize).copied().unwrap_or(false);
            let mut params = entity.material.shader_params();
            if gpu_skin {
                params[3] = out.palette.len() as f32;
                out.palette.extend(entity.joints.iter().map(|m| m.to_cols_array_2d()));
            }
            let item = (mesh, instance(model, entity.color, params));
            let (main, shadow) = if gpu_skin { (&mut out.skinned_main, &mut out.skinned_shadow) } else { (&mut out.main, &mut out.shadow) };
            if seen {
                main.push(item);
            }
            if casts_shadow {
                shadow.push(item);
            }
        }
        out
    }
}

fn batch(list: &mut [(MeshId, Instance)], instances: &mut Vec<Instance>) -> Vec<(usize, std::ops::Range<u32>)> {
    list.sort_unstable_by_key(|(mesh, _)| *mesh);
    let mut batches: Vec<(usize, std::ops::Range<u32>)> = Vec::new();
    for (mesh, instance) in list.iter() {
        let index = instances.len() as u32;
        instances.push(*instance);
        match batches.last_mut() {
            Some((last, range)) if *last == mesh.0 as usize => range.end = index + 1,
            _ => batches.push((mesh.0 as usize, index..index + 1)),
        }
    }
    batches
}

/// Misst, wie lange die Grafikkarte für Schatten und Welt eines Bildes braucht (Zeitstempel am
/// Anfang des Schatten- und am Ende des Hauptdurchgangs). Das Ergebnis kommt ein paar Bilder
/// später an; mehrere Auslesepuffer, damit nie auf die Grafikkarte gewartet wird.
struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<(wgpu::Buffer, Arc<std::sync::atomic::AtomicU8>)>,
    /// Platz, in den dieses Bild gemessen wird (None = alle belegt, dieses Bild nicht messen)
    current: Option<usize>,
    period_ns: f32,
    last_ms: Option<f32>,
    /// Davon der Schattendurchgang
    shadow_ms: Option<f32>,
}

const SLOT_FREE: u8 = 0;
const SLOT_PENDING: u8 = 1;
const SLOT_READY: u8 = 2;

impl GpuTimer {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> GpuTimer {
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor { label: Some("zeit"), ty: wgpu::QueryType::Timestamp, count: 3 });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("zeit"),
            size: 24,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..4)
            .map(|_| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("zeit lesen"),
                    size: 24,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                (buffer, Arc::new(std::sync::atomic::AtomicU8::new(SLOT_FREE)))
            })
            .collect();
        GpuTimer { query_set, resolve, slots, current: None, period_ns: queue.get_timestamp_period(), last_ms: None, shadow_ms: None }
    }

    /// Fertige Messungen abholen.
    fn collect(&mut self, device: &wgpu::Device) {
        use std::sync::atomic::Ordering;
        let _ = device.poll(wgpu::PollType::Poll);
        for (buffer, state) in &self.slots {
            if state.load(Ordering::Acquire) != SLOT_READY {
                continue;
            }
            if let Ok(data) = buffer.slice(..).get_mapped_range() {
                let stamps: &[u64] = bytemuck::cast_slice(&data);
                let ms = |a: u64, b: u64| (b.saturating_sub(a) as f64 * self.period_ns as f64 / 1_000_000.0) as f32;
                let total = ms(stamps[0], stamps[2]);
                // Unsinnige Werte (Zähler übergelaufen, Treiber-Eigenheiten) verwerfen
                if stamps[2] > stamps[0] && total < 1000.0 {
                    self.last_ms = Some(total);
                    self.shadow_ms = Some(ms(stamps[0], stamps[1]));
                }
            }
            buffer.unmap();
            state.store(SLOT_FREE, Ordering::Release);
        }
    }

    /// Einen freien Auslesepuffer für dieses Bild wählen; gibt die Messpunkte zurück.
    fn begin(&mut self) -> Option<wgpu::QuerySet> {
        use std::sync::atomic::Ordering;
        self.current = self.slots.iter().position(|(_, state)| state.load(Ordering::Acquire) == SLOT_FREE);
        self.current.map(|_| self.query_set.clone())
    }

    fn resolve(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(slot) = self.current else { return };
        encoder.resolve_query_set(&self.query_set, 0..3, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.slots[slot].0, 0, 24);
    }

    fn request_readback(&mut self) {
        use std::sync::atomic::Ordering;
        let Some(slot) = self.current.take() else { return };
        let (buffer, state) = &self.slots[slot];
        state.store(SLOT_PENDING, Ordering::Release);
        let state = state.clone();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |result| {
            state.store(if result.is_ok() { SLOT_READY } else { SLOT_FREE }, Ordering::Release);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sichtpruefung_mit_kamera() {
        let camera = crate::camera::Camera { position: glam::Vec3::ZERO, ..Default::default() };
        let planes = frustum_planes(camera.view_projection(16.0 / 9.0));
        assert!(sphere_visible(&planes, glam::Vec3::new(0.0, 0.0, -10.0), 1.0), "vor der Kamera");
        assert!(!sphere_visible(&planes, glam::Vec3::new(0.0, 0.0, 10.0), 1.0), "hinter der Kamera");
        assert!(!sphere_visible(&planes, glam::Vec3::new(100.0, 0.0, -10.0), 1.0), "weit rechts daneben");
        assert!(sphere_visible(&planes, glam::Vec3::new(0.0, 0.0, 1.0), 2.0), "Kugel um die Kamera herum");
        assert!(!sphere_visible(&planes, glam::Vec3::new(0.0, 0.0, -5000.0), 1.0), "hinter der Sichtweite");
    }
}
