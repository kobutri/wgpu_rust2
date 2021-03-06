extern crate core;

use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Vector3, Zero};
use crevice::std140::{AsStd140, Std140};
use rand::Rng;
use wgpu::util::DeviceExt;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

mod experiments;

struct Octree {
    data: Vec<Node>,
    depth: i32,
    size: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Node {
    material_id: i32,
    sub_voxels: [i32; 8],
    //ropes: [i32; 6],
}

impl Octree {
    fn new_random(depth: i32, size: f32, chance: f64) -> Self {
        let mut data = vec![];
        Self::new_random_internal(depth, &mut data, chance);
        Self { data, depth, size }
    }

    fn new_random_internal(depth: i32, data: &mut Vec<Node>, chance: f64) {
        data.push(Node {
            material_id: if depth == 0 && rand::thread_rng().gen_bool(chance) {
                Self::SOLID
            } else {
                Self::EMPTY
            },
            ..Default::default()
        });
        let first_address = data.len();
        if data[first_address - 1].material_id & Self::SOLID == 0 && depth != 0 {
            for i in 0..8 {
                let new_address = data.len();
                data[first_address - 1].sub_voxels[i] = new_address as i32;
                Self::new_random_internal(depth - 1, data, chance);
            }
        }
    }

    fn new_wall(depth: i32, size: f32) -> Self {
        let mut data = vec![];
        Self::new_wall_internal(depth, &mut data);
        Self { data, depth, size }
    }


    fn new_wall_internal(depth: i32, data: &mut Vec<Node>) {
        data.push(Node {
            material_id: if depth == 0 {
                Self::SOLID
            } else {
                Self::EMPTY
            },
            ..Default::default()
        });
        let first_address = data.len();
        if data[first_address - 1].material_id & Self::SOLID == 0 && depth != 0 {
            for i in 0..8 {
                if i & 1 == 0 {
                    let new_adress = data.len();
                    data[first_address-1].sub_voxels[i] = new_adress as i32;
                    Self::new_wall_internal(depth - 1, data);
                }
            }
        }
    }

    fn generate_ropes(data: &mut Vec<Node>) {
        
    }

    const SOLID: i32 = 1;
    const EMPTY: i32 = 0;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, AsStd140)]
struct Uniforms {
    view_pos: mint::Vector3<f32>,
    view_dir: mint::Vector3<f32>,
    view_up: mint::Vector3<f32>,
    view_right: mint::Vector3<f32>,
    fov: f32,
    width: i32,
    height: i32,
    octree_size: f32,
    octree_depth: i32,
}

fn compile_shader_alternative(
    dir: &std::path::PathBuf,
    name: &str,
    kind: shaderc::ShaderKind,
) -> Option<Box<[u8]>> {
    let code = std::fs::read_to_string(dir.join(name).as_path()).unwrap();
    let mut compiler = shaderc::Compiler::new().unwrap();
    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    match compiler.compile_into_spirv(code.as_str(), kind, name, "main", Some(&options)) {
        Ok(artifact) => {
            if artifact.get_num_warnings() != 0 {
                eprintln!("{}", artifact.get_warning_messages());
            }
            Some(Box::from(artifact.as_binary_u8()))
        }
        Err(err) => {
            eprintln!("{}", err);
            None
        }
    }
}

fn create_octree(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
) -> (Octree, wgpu::Buffer, wgpu::BindGroup) {
    let octree = Octree::new_random(8, 8.0, 0.005);
    //let octree = Octree::new_wall(12, 8.0);
    // let octree = Octree {
    //     data: vec![
    //         Node {material_id: 0, sub_voxels: [1, 2, 3, 4, 5, 6, 7, 8]},
    //         Node {material_id: 0, sub_voxels: [9, 0, 0, 0, 0, 0, 0, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 9, 0, 0, 0, 0, 0, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 0, 9, 0, 0, 0, 0, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 0, 0, 9, 0, 0, 0, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 0, 0, 0, 9, 0, 0, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 0, 0, 0, 0, 9, 0, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 0, 0, 0, 0, 0, 9, 0]},
    //         Node {material_id: 0, sub_voxels: [0, 0, 0, 0, 0, 0, 0, 9]},
    //         Node {material_id: 1, sub_voxels: [0, 0, 0, 0, 0, 0, 0, 0]},
    //     ],
    //     depth: 2,
    //     size: 8.0,
    // };
    println!("{}", octree.data.len());
    let octree_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &bytemuck::cast_slice(octree.data.as_slice()),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let octree_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: octree_buffer.as_entire_binding(),
        }],
    });
    (octree, octree_buffer, octree_bind_group)
}

fn create_uniforms(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    config: &wgpu::SurfaceConfiguration,
    octree: &Octree,
    angle: f32,
) -> (Uniforms, wgpu::Buffer, wgpu::BindGroup) {
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
            width: config.width as i32,
            height: config.height as i32,
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
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    (uniforms, uniform_buffer, uniform_bind_group)
}

fn reload_shaders(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    swapchain_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader_dir = std::path::PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .join("res")
        .join("shaders");
    let vertex_shader_code = compile_shader_alternative(
        &shader_dir,
        "shader.vert",
        shaderc::ShaderKind::Vertex,
    ).unwrap_or(Box::from(include_bytes!(concat!(env!("OUT_DIR"), "/fallback_vert.spv")).as_slice()));
    let fragment_shader_code = compile_shader_alternative(
        &shader_dir,
        "shader.frag",
        shaderc::ShaderKind::Fragment,
    ).unwrap_or(Box::from(include_bytes!(concat!(env!("OUT_DIR"), "/fallback_frag.spv")).as_slice()));
    let vertex_shader = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: None,
            source: bytemuck::cast_slice(vertex_shader_code.as_ref()).into(),
        })
    };
    let fragment_shader = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: None,
            source: bytemuck::cast_slice(fragment_shader_code.as_ref()).into(),
        })
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        multiview: None,
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
            targets: &[swapchain_format.into()],
        }),
    })
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::Backends::VULKAN);
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("Failed to find appropriate adapter");

    let mut limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
    limits.max_storage_buffer_binding_size = 2147483648;
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::SPIRV_SHADER_PASSTHROUGH,
                limits,
            },
            None,
        )
        .await
        .expect("Failed to create device");

    let uniform_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let octree_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&uniform_bind_group_layout, &octree_bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

    let mut render_pipeline =
        reload_shaders(&device, &pipeline_layout, swapchain_format);

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    surface.configure(&device, &config);

    let (mut octree, mut octree_buffer, mut octree_bind_group) =
        create_octree(&device, &octree_bind_group_layout);

    let mut angle = std::f32::consts::PI / 4.0;
    let (mut uniforms, mut uniform_buffer, mut uniform_bind_group) =
        create_uniforms(&device, &uniform_bind_group_layout, &config, &octree, angle);

    let mut now = Instant::now();
    let mut count = 0;

    event_loop.run(move |event, _, control_flow| {
        let _ = (&instance, &adapter, &pipeline_layout);

        *control_flow = ControlFlow::Poll;
        match event {
            // Event::WindowEvent {
            //     event: WindowEvent::Resized(size),
            //     ..
            // } => {
            //     if size.width != 0 && size.height != 0 {
            //         config.width = size.width.max(1);
            //         config.height = size.height.max(1);
            //         surface.configure(&device, &config);
            //     }
            // }
            Event::RedrawRequested(_) => {
                let output = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations::default(),
                        }],
                        depth_stencil_attachment: None,
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &uniform_bind_group, &[]);
                    rpass.set_bind_group(1, &octree_bind_group, &[]);
                    rpass.draw(0..6, 0..1);
                }
                queue.submit(Some(encoder.finish()));
                output.present();

                count += 1;
                if count >= 60 {
                    println!("{}", count as f64 / now.elapsed().as_secs_f64());
                    now = Instant::now();
                    count = 0;
                }
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
            } => match keycode {
                VirtualKeyCode::R => {
                    render_pipeline = reload_shaders(
                        &device,
                        &pipeline_layout,
                        swapchain_format,
                    );
                }
                VirtualKeyCode::B => {
                    (octree, octree_buffer, octree_bind_group) =
                        create_octree(&device, &octree_bind_group_layout);
                }
                VirtualKeyCode::A | VirtualKeyCode::Left => {
                    angle += 3.0 * std::f32::consts::PI / 180.0;
                    (uniforms, uniform_buffer, uniform_bind_group) = create_uniforms(
                        &device,
                        &uniform_bind_group_layout,
                        &config,
                        &octree,
                        angle,
                    );
                }
                VirtualKeyCode::D | VirtualKeyCode::Right => {
                    angle -= 3.0 * std::f32::consts::PI / 180.0;
                    (uniforms, uniform_buffer, uniform_bind_group) = create_uniforms(
                        &device,
                        &uniform_bind_group_layout,
                        &config,
                        &octree,
                        angle,
                    );
                }
                VirtualKeyCode::M => {
                    for (index, node) in octree.data.iter().enumerate() {
                        print!("Node({}, int[](", node.material_id);
                        for (index, sub_voxel) in node.sub_voxels.iter().enumerate() {
                            print!("{}", *sub_voxel);
                            if index != 7 {
                                print!(", ");
                            }
                        }
                        print!("))");
                        if index != octree.data.len() - 1 {
                            println!(",");
                        }
                    }
                }
                _ => {}
            },
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
