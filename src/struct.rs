use std::{ops::Range, time::{self, Duration, Instant}};

use interprocess::local_socket::{
    tokio::{prelude::*, RecvHalf, SendHalf, Stream}, GenericFilePath, GenericNamespaced, ToNsName
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, sync::mpsc::Sender, try_join
};
use wgpu::{util::DeviceExt, Color};
// lib.rs
use winit::{dpi::LogicalPosition, event::{KeyboardInput, VirtualKeyCode, WindowEvent}, window::Window};
use crate::{camera::{Camera, CameraStaging, CameraUniform}, snake::{DrawModel, Instance, InstanceRaw, Mesh}, SnakeInputs};
use crate::texture;
use cgmath::prelude::*;

const NUM_INSTANCES_PER_ROW: u32 = 2;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);


const SPEED: f32 = 0.1;


pub struct State {
    pub surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    // The window must be declared after the surface so
    // it gets dropped after it as the surface contains
    // unsafe references to the window's resources.
    pub window: Window,
    pub clear_color: Color,
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    //pub camera: Camera,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub diffuse_bind_group: wgpu::BindGroup,
    pub diffuse_texture: texture::Texture,
    pub camera_staging: CameraStaging,
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
    pub meshes: Vec<Mesh>,
    pub input_sender: Sender<SnakeInputs>,
    pub direction: Option<SnakeInputs>,
    pub last_direction: u32,
    pub first_direction: u32,
    pub directions: Vec<SnakeInputs>,
    pub last_updated: Instant,
    pub apple_vertex_buffer: wgpu::Buffer,
    pub apple_instances_buffer: wgpu::Buffer,
    pub apple_instances: Vec<Instance>
}

const VERTICES: &[Vertex] = &[
    // Changed
    Vertex { position: [-0.05, 0.05, 0.0], tex_coords: [0.0, 0.0], }, // A
    Vertex { position: [-0.05, -0.05, 0.0], tex_coords: [0.0, 0.0], }, // B
    Vertex { position: [0.05, -0.05, 0.0], tex_coords: [0.0, 0.0], }, // C
    Vertex { position: [0.05, 0.05, 0.0], tex_coords: [0.0, 0.0], }, // D
];

const APPLE_VERTICES: &[Vertex] = &[
    // Changed
    Vertex { position: [-0.05, 0.05, 0.0], tex_coords: [1.0, 1.0], }, // A
    Vertex { position: [-0.05, -0.05, 0.0], tex_coords: [1.0, 1.0], }, // B
    Vertex { position: [0.05, -0.05, 0.0], tex_coords: [1.0, 1.0], }, // C
    Vertex { position: [0.05, 0.05, 0.0], tex_coords: [1.0, 1.0], }, // D
];


const INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3,
];


const MESH_VERTICES: &[Vertex] = &[
    // Changed
    Vertex { position: [-0.1, 0.1, 0.0], tex_coords: [-1.0, 1.0], }, // A
    Vertex { position: [-0.1, -0.1, 0.0], tex_coords: [-1.0, -1.0], }, // B
    Vertex { position: [0.1, -0.1, 0.0], tex_coords: [1.0, -1.0], }, // C
    Vertex { position: [0.1, 0.1, 0.0], tex_coords: [1.0, 1.0], }, // D
];



const MESH_INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3,
];


#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}


impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: Window, send: Sender<SnakeInputs>) -> Self {
        let size = window.inner_size();
        let instances = vec![
            Instance {
                position: cgmath::Vector3 { x: 0.0, y: 0.0, z: 0.0 },
                rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            },
            Instance {
                position: cgmath::Vector3 { x: 0.1, y: 0.0, z: 0.0 },
                rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            },
            Instance {
                position: cgmath::Vector3 { x: 0.2, y: 0.0, z: 0.0 },
                rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            }

        ];
        
        let apple_instances = vec![
            Instance {
                position: cgmath::Vector3 { x: 0.0, y: 0.2, z: 0.0 },
                rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            }
        ];
        
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // # Safety
        //
        // The surface needs to live as long as the window that created it.
        // State owns the window, so this should be safe.
        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();


        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web, we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .copied().find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()) 
        }
        );


        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        let apple_instance_data = apple_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let apple_instances_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Apple instance Buffer"),
                contents: bytemuck::cast_slice(&apple_instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );


        let camera = Camera {
            // position the camera 1 unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 0.0, 2.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
            pitch: 0.0,
            yaw: 0.0
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });

        let diffuse_bytes = include_bytes!("happy-tree.png");
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        use image::GenericImageView;
        let dimensions = diffuse_image.dimensions();
        
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap();
        
        let diffuse_texture_view = diffuse_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );


        let render_pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                ],
                push_constant_ranges: &[],
            }
        );
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", // 1.
                buffers: &[
                    Vertex::desc(), InstanceRaw::desc()
                ], // 2.
            },
            fragment: Some(wgpu::FragmentState { // 3.
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState { // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let apple_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Apple vertex Buffer"),
                contents: bytemuck::cast_slice(APPLE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        let num_indices = INDICES.len() as u32;

        let mesh_vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Vertex Buffer"),
                contents: bytemuck::cast_slice(MESH_VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let mesh_index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Mesh Index Buffer"),
                contents: bytemuck::cast_slice(MESH_INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );
        
        let mesh = Mesh {
            name: "square".to_owned(),
            vertex_buffer: mesh_vertex_buffer,
            index_buffer: mesh_index_buffer,
            elements: 1,
        };








        queue.write_texture(
            wgpu::ImageCopyTextureBase { 
                texture: &diffuse_texture.texture, 
                mip_level: 0, 
                origin: wgpu::Origin3d::ZERO, 
                aspect: wgpu::TextureAspect::All 
            }
            , 
            &diffuse_rgba, 
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4*dimensions.0),
                rows_per_image: Some(dimensions.1),
            }, 
            texture_size
        );

        
        let camera_staging = CameraStaging::new(camera);

        let meshes = vec![mesh];
        let last_direction = (instances.len()-1) as u32;

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            clear_color: Color::BLACK,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            diffuse_bind_group,
            diffuse_texture,
            camera_staging,
            instances,
            instance_buffer,
            meshes,
            input_sender: send,
            direction: None,
            last_direction,
            first_direction: 0,
            directions: vec![SnakeInputs::Left, SnakeInputs::Left],
            last_updated: Instant::now(),
            apple_vertex_buffer,
            apple_instances,
            apple_instances_buffer
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            let new_size = winit::dpi::PhysicalSize::new(new_size.width, new_size.width);

        // Update the size
            self.size = new_size;

        // Update the configuration for the surface to match the new size
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        //let s = serde_json::to_string(&Signal::Input).unwrap() + "\n";
        
        //pollster::block_on(self.sender.write_all(s.as_bytes())).unwrap();
        match event {
            WindowEvent::KeyboardInput { input: KeyboardInput { 
                virtual_keycode, ..
            }, 
            .. 
        } => {
            match virtual_keycode {
                Some(k) => {
                    
                    match k {
                        VirtualKeyCode::W if self.direction != Some(SnakeInputs::Down) => {
                            self.direction = Some(SnakeInputs::Up);
                            
                            return false;
                        },
                        VirtualKeyCode::S if self.direction != Some(SnakeInputs::Up) => {
                            self.direction = Some(SnakeInputs::Down);
                            return false;
                        },
                        VirtualKeyCode::A if self.direction != Some(SnakeInputs::Right) => {
                            self.direction = Some(SnakeInputs::Left);
                            return false;
                        },
                        VirtualKeyCode::D if self.direction != Some(SnakeInputs::Left) => {
                            self.direction = Some(SnakeInputs::Right);
                            return false;
                        }
                        

                        _=> {
                            
                        }
                        
                    }
                    
                    

                    return false;
                    
                },
                None => {},
            }
            
            
            false
        },
            WindowEvent::AxisMotion { axis, value,.. }
            => {
                //eprintln!("Axis?: {}, value: {}", axis, value);
                match axis {
                    0 => {
                        //self.camera_staging.camera.pitch = 1.0;
                        
                        
                    },
                    1 => {
                        //self.camera_staging.camera.yaw = 0.001;
                        
                    },
                    _=> {}
                }
                
                
                false
            }
            _ => {
                false
            }
        }


        
        
    }

    pub fn update(&mut self) {
        
        //match &self.direction {
        //    Some(d) => {
        //        self.instances[self.last_direction as usize].position = self.instances[self.first_direction as usize].position;
        //        match d {
        //            SnakeInputs::Up => {
        //                
        //                self.instances[self.last_direction as usize].position.y += 0.05;
        //            },
        //            SnakeInputs::Down => {self.instances[self.last_direction as usize].position.y -= 0.05;},
        //            SnakeInputs::Left => {self.instances[self.last_direction as usize].position.x -= 0.05;},
        //            SnakeInputs::Right => {self.instances[self.last_direction as usize].position.x += 0.05;},
        //        }
        //        //let i =  (self.instances.len() - 1) as u32;
        //        //self.last_direction = if self.last_direction == 0 {
        //        //    i
        //        //} else {
        //        //    self.last_direction - 1
        //        //};
////
        //        //self.first_direction = if self.first_direction == i {
        //        //    0
        //        //} else {
        //        //    self.first_direction + 1
        //        //};
//
        //    },
        //    None => {},
        //}
        //eprintln!("Position: {:?}", self.instances[0].position);
        #[allow(clippy::single_match)]
        match self.direction {
            Some(d) => {
                
                
                if self.last_updated.elapsed() >= Duration::from_millis(64) {
                    self.directions.insert(0, d);
                    if self.directions.len() > self.instances.len() {
                        self.directions.pop();
                    }
                    let mut i = 0;
                    while i < self.directions.len() {
                        match self.directions[i] {
                            SnakeInputs::Up => {

                                self.instances[i].position.y += SPEED;
                                if self.instances[i].position.y > 1.2 {self.instances[i].position.y = -1.1}
                            },
                            SnakeInputs::Down => {
                                self.instances[i].position.y -= SPEED;
                                if self.instances[i].position.y < -1.2 {self.instances[i].position.y = 1.1}
                            },
                            SnakeInputs::Left => {
                                self.instances[i].position.x -= SPEED;
                                if self.instances[i].position.x < -1.2 {self.instances[i].position.x = 1.1}
                            },
                            SnakeInputs::Right => {
                                self.instances[i].position.x += SPEED;
                                if self.instances[i].position.x > 1.2 {self.instances[i].position.x = -1.1}
                            },
                        }
                        i += 1;

                    }
                    self.last_updated = Instant::now();
                }
            },
            None => {

            }
        }
        

        self.rebuild_instance_buffer();
        //self.camera_uniform.update_view_proj(&self.camera);
        self.camera_staging.update_camera(&mut self.camera_uniform);
        //self.camera_staging.update_camera_pitch(&mut self.camera_uniform);
        //self.camera_staging.update_camera_yaw(&mut self.camera_uniform);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));

        
    }

//    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
//    let output = self.surface.get_current_texture()?;
//    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
//
//    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
//        label: Some("Render Encoder"),
//    });
//
//    {
//        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//            label: Some("Render Pass"),
//            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                view: &view,
//                resolve_target: None,
//                ops: wgpu::Operations {
//                    load: wgpu::LoadOp::Clear(self.clear_color),
//                    store: wgpu::StoreOp::Store,
//                },
//            })],
//            depth_stencil_attachment: None,
//        });
//
//        render_pass.set_pipeline(&self.render_pipeline);
//
//        // Set bind groups
//        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
//        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
//
//        // Set vertex and index buffers
//        render_pass.set_vertex_buffer(0, self.meshes[0].vertex_buffer.slice(..));
//        render_pass.set_index_buffer(self.meshes[0].index_buffer.slice(..), wgpu::IndexFormat::Uint16);
//
//        // Draw the indexed vertices
//        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
//    }
//
//    // Submit the commands
//    self.queue.submit(std::iter::once(encoder.finish()));
//    output.present();
//
//    Ok(())
//}

pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
    let output = self.surface.get_current_texture()?;
    let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
        render_pass.set_bind_group(1, &self.camera_bind_group, &[]);

        // Snake rendering
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // UPDATED!
        render_pass.draw_indexed(0..self.num_indices, 0, 0..self.instances.len() as _);

        // Apple rendering
        render_pass.set_vertex_buffer(0, self.apple_vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.apple_instances_buffer.slice(..));
        render_pass.draw_indexed(0..self.num_indices, 0, 0..self.apple_instances.len() as _);

    }

    // Submit the commands
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    //std::thread::sleep(Duration::from_millis(100));
    Ok(())
}

    pub fn rebuild_instance_buffer(&mut self) {
        
        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        

        

        self.queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data));

    }

}


impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}




