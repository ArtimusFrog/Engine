use std::path::Path;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::camera::rh;
use glam::{Mat3, Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::app::Context;
use crate::mesh::{MeshData, Vertex};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const SHADOW_MAP_SIZE: u32 = 2048;

const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
const INSTANCE_ATTRIBUTES: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
    2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4,
    6 => Float32x3, 7 => Float32x3, 8 => Float32x3,
    9 => Float32x4,
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Globals {
    view_proj: [[f32; 4]; 4],
    light_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    sun_dir: [f32; 4],
    sun_color: [f32; 4],
    sky_ambient: [f32; 4],
    ground_ambient: [f32; 4],
    fog: [f32; 4],
    shadow_params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    model: [[f32; 4]; 4],
    normal: [[f32; 3]; 3],
    color: [f32; 4],
}

struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

pub(crate) struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    depth: wgpu::TextureView,
    pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    globals_buffer: wgpu::Buffer,
    globals_bind_group: wgpu::BindGroup,
    shadow_bind_group: wgpu::BindGroup,
    shadow_map: wgpu::TextureView,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    meshes: Vec<GpuMesh>,
    adapter_name: String,
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

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor { label: Some("engine"), ..Default::default() })
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("basic"),
            bind_group_layouts: &[Some(&globals_layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("basic"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &vertex_buffers,
            },
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

        let instance_capacity = 256;
        let instance_buffer = Self::create_instance_buffer(&device, instance_capacity);
        let depth = Self::create_depth(&device, config.width, config.height);

        Renderer {
            surface,
            device,
            queue,
            config,
            depth,
            pipeline,
            shadow_pipeline,
            globals_buffer,
            globals_bind_group,
            shadow_bind_group,
            shadow_map,
            instance_buffer,
            instance_capacity,
            meshes: Vec::new(),
            adapter_name,
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
        self.depth = Self::create_depth(&self.device, width, height);
    }

    /// Zeichnet einen Frame ins Fenster.
    pub fn render(&mut self, ctx: &Context) {
        let frame = match self.surface.get_current_texture() {
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
        self.draw(ctx, &view);
        self.queue.present(frame);
    }

    /// Zeichnet einen Frame in eine unsichtbare Textur und speichert ihn als PNG.
    pub fn screenshot(&mut self, ctx: &Context, path: &Path) -> Result<(), String> {
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
        self.draw(ctx, &texture.create_view(&Default::default()));

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

    fn draw(&mut self, ctx: &Context, target: &wgpu::TextureView) {
        self.upload_new_meshes(ctx.assets.meshes());

        let env = &ctx.env;
        let aspect = self.config.width as f32 / self.config.height as f32;
        let texel_world = 2.0 * env.shadow_range / SHADOW_MAP_SIZE as f32;
        let globals = Globals {
            view_proj: ctx.camera.view_projection(aspect).to_cols_array_2d(),
            light_view_proj: sun_view_projection(ctx).to_cols_array_2d(),
            camera_pos: ctx.camera.position.extend(1.0).into(),
            sun_dir: env.sun_direction.normalize().extend(0.0).into(),
            sun_color: env.sun_color.extend(0.0).into(),
            sky_ambient: env.sky_ambient.extend(0.0).into(),
            ground_ambient: env.ground_ambient.extend(0.0).into(),
            fog: env.sky_color.extend(env.fog_density).into(),
            shadow_params: [1.0 / SHADOW_MAP_SIZE as f32, texel_world * 1.5, 0.0, 0.0],
        };
        self.queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));

        // Objekte nach Mesh sortieren, damit jedes Mesh mit einem einzigen Draw-Call
        // (Instancing) gezeichnet wird.
        let mut visible: Vec<_> = ctx.scene.iter_world().filter(|(e, _)| e.visible).collect();
        visible.sort_by_key(|(e, _)| e.mesh);
        let instances: Vec<Instance> = visible
            .iter()
            .map(|&(e, model)| {
                Instance {
                    model: model.to_cols_array_2d(),
                    normal: Mat3::from_mat4(model).inverse().transpose().to_cols_array_2d(),
                    color: e.color.into(),
                }
            })
            .collect();
        if instances.len() > self.instance_capacity {
            self.instance_capacity = instances.len().next_power_of_two();
            self.instance_buffer = Self::create_instance_buffer(&self.device, self.instance_capacity);
        }
        self.queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instances));

        // Zusammenhängende Bereiche mit gleichem Mesh
        let mut batches = Vec::new();
        let mut start = 0;
        while start < visible.len() {
            let mesh_id = visible[start].0.mesh;
            let end = start + visible[start..].iter().take_while(|(e, _)| e.mesh == mesh_id).count();
            batches.push((mesh_id.0 as usize, start as u32..end as u32));
            start = end;
        }

        let sky = env.sky_color;
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_map,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_bind_group, &[]);
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            self.draw_batches(&mut pass, &batches);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
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
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.globals_bind_group, &[]);
            pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            self.draw_batches(&mut pass, &batches);
        }
        self.queue.submit([encoder.finish()]);
    }

    fn draw_batches(&self, pass: &mut wgpu::RenderPass<'_>, batches: &[(usize, std::ops::Range<u32>)]) {
        for (mesh, instances) in batches {
            let mesh = &self.meshes[*mesh];
            pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.index_count, 0, instances.clone());
        }
    }

    fn upload_new_meshes(&mut self, meshes: &[MeshData]) {
        for mesh in &meshes[self.meshes.len()..] {
            self.meshes.push(GpuMesh {
                vertices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh vertices"),
                    contents: bytemuck::cast_slice(&mesh.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
                indices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mesh indices"),
                    contents: bytemuck::cast_slice(&mesh.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
                index_count: mesh.indices.len() as u32,
            });
        }
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
