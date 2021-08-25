use cgmath::{prelude::*, Vector3};
use wgpu::util::DeviceExt;
use winit::event::{DeviceEvent, ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

//use crevice::std140::{AsStd140, Std140};
use glsl_layout::{vec3, Std140, Uniform};

mod experiments;
struct Octree {
    data: Vec<u32>,
    depth: u32,
    size: f32,
}

impl Octree {
    fn new_random(depth: u32, size: f32) -> Self {
        let mut data = vec![];
        Self::new_random_internal(depth, &mut data);
        Self { data, depth, size }
    }

    fn new_random_internal(depth: u32, data: &mut Vec<u32>) {
        data.push(if rand::random() {
            Self::SOLID
        } else {
            Self::EMPTY
        });
        let first_address = data.len();
        if data[first_address - 1] & Self::SOLID == 0 && depth != 0 {
            for i in 0..8 {
                if rand::random() {
                    data[first_address - 1] |= Self::SUBVOXELS[i];
                    data.push(0);
                }
            }
            for i in 0..(data.len() - first_address) {
                data[first_address + i] = data.len() as u32;
                Self::new_random_internal(depth - 1, data);
            }
        }
    }

    const SOLID: u32 = 256;
    const EMPTY: u32 = 0;
    const SUBVOXELS: [u32; 8] = [1, 2, 4, 8, 16, 32, 64, 128];
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Uniform)]
struct Uniforms {
    view_pos: vec3,
    view_dir: vec3,
    view_up: vec3,
    view_right: vec3,
    fov: f32,
    width: u32,
    height: u32,
    octree_size: f32,
    octree_depth: u32,
}

fn reload_shaders(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, swapchain_format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let vertex_shader = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: None,
            source: wgpu::util::make_spirv_raw(
                std::fs::read(
                    "C:\\Users\\trist\\CLionProjects\\wgpu_rust2\\res\\shaders\\vert.spv",
                )
                .unwrap()
                .as_slice(),
            ),
        })
    };
    let fragment_shader = unsafe {
        device.create_shader_module_spirv(&wgpu::ShaderModuleDescriptorSpirV {
            label: None,
            source: wgpu::util::make_spirv_raw(
                std::fs::read(
                    "C:\\Users\\trist\\CLionProjects\\wgpu_rust2\\res\\shaders\\frag.spv",
                )
                .unwrap()
                .as_slice(),
            ),
        })
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            targets: &[swapchain_format.into(), wgpu::TextureFormat::Rgba32Float.into()],
        }),
    })
}

async fn run(event_loop: EventLoop<()>, window: Window) {
    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::Backends::all());
    let surface = unsafe { instance.create_surface(&window) };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
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

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            // wgpu::BindGroupLayoutEntry {
            //     binding: 2,
            //     visibility: wgpu::ShaderStages::FRAGMENT,
            //     ty: wgpu::BindingType::StorageTexture {
            //         access: wgpu::StorageTextureAccess::WriteOnly,
            //         format: wgpu::TextureFormat::Rgba32Float,
            //         view_dimension: wgpu::TextureViewDimension::D2,
            //     },
            //     count: None,
            // },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let swapchain_format = surface.get_preferred_format(&adapter).unwrap();

    let mut render_pipeline = reload_shaders(&device, &pipeline_layout, swapchain_format);

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: swapchain_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };

    surface.configure(&device, &config);

    let octree = Octree::new_random(1, 5.0);
    let octree_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: &bytemuck::cast_slice(octree.data.as_slice()),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let uniforms = {
        let origin = Vector3::<f32>::new(20.0, 0.0, 0.0);
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
        contents: uniforms.std140().as_raw(),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
    });
    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: octree_buffer.as_entire_binding(),
            },
            // wgpu::BindGroupEntry {
            //     binding: 2,
            //     resource: wgpu::BindingResource::TextureView(&texture_view),
            // },
        ],
    });

    event_loop.run(move |event, _, control_flow| {
        let _ = (
            &instance,
            &adapter,
            &pipeline_layout,
        );

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                config.width = size.width;
                config.height = size.height;
                surface.configure(&device, &config);
            }
            Event::RedrawRequested(_) => {
                let frame = surface
                    .get_current_frame()
                    .expect("Failed to acquire next swap chain texture")
                    .output;
                let view = frame
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
                        },
                        wgpu::RenderPassColorAttachment {
                            view: &texture_view,
                            resolve_target: None,
                            ops: wgpu::Operations::default(),
                        }],
                        depth_stencil_attachment: None,
                    });
                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &bind_group, &[]);
                    rpass.draw(0..6, 0..1);
                }
                queue.submit(Some(encoder.finish()));
            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::DeviceEvent {
                event: DeviceEvent::Key(KeyboardInput {
                    state: ElementState::Pressed,
                    virtual_keycode: Some(VirtualKeyCode::R),
                    ..
                }),
                ..
            } => {
                render_pipeline = reload_shaders(&device, &pipeline_layout, swapchain_format);
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
        .build(&event_loop)
        .unwrap();
    pollster::block_on(run(event_loop, window));
}
