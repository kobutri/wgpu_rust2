#![feature(destructuring_assignment)]

use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Rad, Vector3, Zero};
use crevice::std140::{AsStd140, Std140};
use wgpu::util::DeviceExt;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

mod experiments;

struct Octree {
    data: Vec<Node>,
    depth: u32,
    size: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Node {
    material_id: u32,
    sub_voxels: [u32; 8],
}

impl Octree {
    fn new_random(depth: u32, size: f32) -> Self {
        let mut data = vec![];
        Self::new_random_internal(depth, &mut data);
        Self { data, depth, size }
    }

    fn new_random_internal(depth: u32, data: &mut Vec<Node>) {
        data.push(Node {
            material_id: if depth == 0{
                Self::SOLID
            } else {
                Self::EMPTY
            },
            ..Default::default()
        });
        let first_address = data.len();
        if data[first_address - 1].material_id & Self::SOLID == 0 && depth != 0 {
            for i in 0..8 {
                if rand::random() || depth != 1 {
                    let new_address = data.len();
                    data[first_address - 1].sub_voxels[i] = new_address as u32;
                    Self::new_random_internal(depth - 1, data);
                } else {
                    data[first_address - 1].sub_voxels[i] = 0;
                }
            }
        }
    }

    const SOLID: u32 = 1;
    const EMPTY: u32 = 0;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, AsStd140)]
struct Uniforms {
    view_pos: mint::Vector3<f32>,
    view_dir: mint::Vector3<f32>,
    view_up: mint::Vector3<f32>,
    view_right: mint::Vector3<f32>,
    fov: f32,
    width: u32,
    height: u32,
    octree_size: f32,
    octree_depth: u32,
}

fn compile_shader_alternative(old_shader: Option<Arc<shaderc::CompilationArtifact>>, dir: &std::path::PathBuf, name: &str, kind: shaderc::ShaderKind) -> Arc<shaderc::CompilationArtifact> {
    let mut compiler = shaderc::Compiler::new().unwrap();
    let code = std::fs::read_to_string(dir.join(name).as_path())
        .unwrap();
    let artifact = compiler
        .compile_into_spirv(
            code.as_str(),
            kind,
            name,
            "main",
            None,
        ).map_err(|e| { println!("{}", e); })
        .ok().map(|artifact| { Arc::new(artifact) }).or(old_shader).unwrap();
    ();
    if artifact.get_num_warnings() != 0 {
        eprintln!("{} shader:\n {}", match kind {
            shaderc::ShaderKind::Vertex => "vertex",
            shaderc::ShaderKind::Fragment => "fragment",
            shaderc::ShaderKind::Compute => "compute",
            _ => "misc",
        }, artifact.get_warning_messages());
    }
    artifact
}

fn create_octree(device: &wgpu::Device, layout: &wgpu::BindGroupLayout) -> (Octree, wgpu::Buffer, wgpu::BindGroup) {
    let octree = Octree {
        data: vec![
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [1, 2, 3, 4, 5, 6, 7, 8],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [9, 0, 0, 0, 0, 0, 0, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 9, 0, 0, 0, 0, 0, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 0, 9, 0, 0, 0, 0, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 0, 0, 9, 0, 0, 0, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 0, 0, 0, 9, 0, 0, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 0, 0, 0, 0, 9, 0, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 0, 0, 0, 0, 0, 9, 0],
            },
            Node {
                material_id: Octree::EMPTY,
                sub_voxels: [0, 0, 0, 0, 0, 0, 0, 9],
            },
            Node {
                material_id: Octree::SOLID,
                sub_voxels: [0;8],
            },


        ],
        depth: 2,
        size: 5.0
    };
   //let octree = Octree::new_random(2, 5.0);
    let octree_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &bytemuck::cast_slice(octree.data.as_slice()),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let octree_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: octree_buffer.as_entire_binding(),
            },
        ],
    });
    (octree, octree_buffer, octree_bind_group)
}

fn create_uniforms(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, config: &wgpu::SurfaceConfiguration, octree: &Octree, angle: f32) -> (Uniforms, wgpu::Buffer, wgpu::BindGroup) {
    let uniforms = {
        let origin = Vector3::<f32>::new(20.0 * angle.cos(), 20.0 * angle.sin(), 0.0);
        let view_dir = (Vector3::zero() - origin).normalize();
        let global_up = Vector3::unit_z();
        let right = view_dir.cross(global_up);
        let up = right.cross(view_dir);
        let fov = std::f32::consts::PI * 90.0 / 180.0;

        Uniforms {
            view_pos: origin.into(),
            view_dir: view_dir.into(),
            view_up: up.into(),
            view_right: right.into(),
            width: config.width,
            height: config.height,
            fov,
            octree_size: octree.size,
            octree_depth: octree.depth,
        }
    };

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: uniforms.as_std140().as_bytes(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
        ],
    });

    (uniforms, uniform_buffer, uniform_bind_group)
}

fn reload_shaders(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    swapchain_format: wgpu::TextureFormat,
    vertex_code: Option<Arc<shaderc::CompilationArtifact>>,
    fragment_code: Option<Arc<shaderc::CompilationArtifact>>,
) -> (wgpu::RenderPipeline, Arc<shaderc::CompilationArtifact>, Arc<shaderc::CompilationArtifact>) {
    let shader_dir = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join("shaders");
    let vertex_shader_code = compile_shader_alternative(vertex_code, &shader_dir, "shader.vert", shaderc::ShaderKind::Vertex);
    let fragment_shader_code = compile_shader_alternative(fragment_code, &shader_dir, "shader.frag", shaderc::ShaderKind::Fragment);
    let vertex_shader = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: None,
            source: wgpu::util::make_spirv_raw(vertex_shader_code.as_binary_u8()),
        })
    };
    let fragment_shader = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: None,
            source: wgpu::util::make_spirv_raw(fragment_shader_code.as_binary_u8()),
        })
    };
    (device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: &vertex_shader,
            entry_point: "main",
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &fragment_shader,
            entry_point: "main",
            targets: &[
                swapchain_format.into()
            ],
        }),
    }), vertex_shader_code, fragment_shader_code)
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find appropriate adapter");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
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
            }
        ],
    });

    let octree_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&uniform_bind_group_layout, &octree_bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

    let (mut render_pipeline, mut vertex_code, mut fragment_code) = reload_shaders(&device, &pipeline_layout, swapchain_format, None, None);

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    surface.configure(&device, &config);

    let (mut octree, mut octree_buffer, mut octree_bind_group) = create_octree(&device, &octree_bind_group_layout);

    let mut angle = std::f32::consts::PI /4.0;
    let (mut uniforms, mut uniform_buffer, mut uniform_bind_group) = create_uniforms(&device, &uniform_bind_group_layout, &config, &octree, angle);


    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &pipeline_layout);

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                if size.width != 0 && size.height != 0 {
                    config.width = size.width.max(1);
                    config.height = size.height.max(1);
                    surface.configure(&device, &config);
                }
            }
            Event::RedrawRequested(_) => {
                let output = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[
                            wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations::default(),
                            },
                        ],
                        depth_stencil_attachment: None,
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &uniform_bind_group, &[]);
                    rpass.set_bind_group(1, &octree_bind_group, &[]);
                    rpass.draw(0..6, 0..1);
                }
                queue.submit(Some(encoder.finish()));
                output.present();
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::DeviceEvent {
                event:
                DeviceEvent::Key(KeyboardInput {
                                     state: ElementState::Pressed,
                                     virtual_keycode: Some(keycode),
                                     ..
                                 }),
                ..
            } => {
                match keycode {
                    VirtualKeyCode::R => {
                        (render_pipeline, vertex_code, fragment_code) = reload_shaders(&device, &pipeline_layout, swapchain_format, Some(vertex_code.clone()), Some(fragment_code.clone()));
                    }
                    VirtualKeyCode::B => {
                        (octree, octree_buffer, octree_bind_group) = create_octree(&device, &octree_bind_group_layout);
                    }
                    VirtualKeyCode::A | VirtualKeyCode::Left => {
                        angle += 3.0 * std::f32::consts::PI / 180.0;
                        (uniforms, uniform_buffer, uniform_bind_group) = create_uniforms(&device, &uniform_bind_group_layout, &config, &octree, angle);
                    }
                    VirtualKeyCode::D | VirtualKeyCode::Right => {
                        angle -= 3.0 * std::f32::consts::PI / 180.0;
                        (uniforms, uniform_buffer, uniform_bind_group) = create_uniforms(&device, &uniform_bind_group_layout, &config, &octree, angle);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_resizable(false)
        .with_inner_size(winit::dpi::PhysicalSize::new(1000, 1000))
        .build(&event_loop)
        .unwrap();
    pollster::block_on(run(event_loop, window));
}
