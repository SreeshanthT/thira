use std::borrow::Cow;

use crate::{Gpu, Viewport};

/// Eight color bars (white, yellow, cyan, green, magenta, red, blue, black) with a
/// white box that moves left to right once every four seconds.
const SHADER: &str = r"
struct Uniforms { origin: vec2<f32>, size: vec2<f32>, t: f32 };
@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let pos = frag.xy - u.origin;
    let bar = u32(clamp(floor(pos.x / u.size.x * 8.0), 0.0, 7.0));
    // Bars in SMPTE order: bit 2 = green off, bit 1 = red off, bit 0 = blue off,
    // giving W Y C G M R B K.
    let level = 0.75;
    var c = vec3<f32>(
        select(0.0, level, (bar & 2u) == 0u),
        select(0.0, level, (bar & 4u) == 0u),
        select(0.0, level, (bar & 1u) == 0u),
    );
    let side = u.size.y * 0.2;
    let left = fract(u.t * 0.25) * (u.size.x - side);
    let top = (u.size.y - side) * 0.5;
    if (pos.x >= left && pos.x < left + side && pos.y >= top && pos.y < top + side) {
        c = vec3<f32>(1.0, 1.0, 1.0);
    }
    return vec4<f32>(c, 1.0);
}
";

/// Draws the viewer test pattern. Keeps one pipeline per target format.
#[derive(Debug)]
pub struct TestPattern {
    shader: wgpu::ShaderModule,
    layout: wgpu::PipelineLayout,
    bind_group: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
    pipelines: Vec<(wgpu::TextureFormat, wgpu::RenderPipeline)>,
}

impl TestPattern {
    pub fn new(gpu: &Gpu) -> Self {
        let device = &gpu.device;
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("pattern uniforms"),
            size: 32,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pattern"),
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
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pattern"),
            layout: &bind_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pattern"),
            bind_group_layouts: &[Some(&bind_layout)],
            ..Default::default()
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pattern"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });
        Self {
            shader,
            layout,
            bind_group,
            uniforms,
            pipelines: Vec::new(),
        }
    }

    fn pipeline(&mut self, gpu: &Gpu, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        let index = match self.pipelines.iter().position(|(f, _)| *f == format) {
            Some(i) => i,
            None => {
                let pipeline = gpu
                    .device
                    .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                        label: Some("pattern"),
                        layout: Some(&self.layout),
                        vertex: wgpu::VertexState {
                            module: &self.shader,
                            entry_point: Some("vs"),
                            buffers: &[],
                            compilation_options: Default::default(),
                        },
                        fragment: Some(wgpu::FragmentState {
                            module: &self.shader,
                            entry_point: Some("fs"),
                            targets: &[Some(format.into())],
                            compilation_options: Default::default(),
                        }),
                        primitive: wgpu::PrimitiveState::default(),
                        depth_stencil: None,
                        multisample: wgpu::MultisampleState::default(),
                        multiview_mask: None,
                        cache: None,
                    });
                self.pipelines.push((format, pipeline));
                self.pipelines.len() - 1
            }
        };
        &self.pipelines[index].1
    }

    /// Records a pass that clears `target` to `background` and draws the pattern,
    /// at time `t` seconds, inside `viewport`. Draws only the background if the
    /// viewport is entirely outside the target.
    ///
    /// Uses one uniform buffer, so draw at most one pattern per queue submission.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        (target_width, target_height): (u32, u32),
        viewport: Viewport,
        background: wgpu::Color,
        t: f32,
    ) {
        let visible = viewport.clamped(target_width, target_height);
        if visible.is_some() {
            // The shader works in the full viewport's coordinates, so a clamped
            // viewer shows the same part of the pattern rather than a squeezed one.
            #[allow(clippy::cast_precision_loss)]
            let values = [
                viewport.x as f32,
                viewport.y as f32,
                viewport.width as f32,
                viewport.height as f32,
                t,
                0.0,
                0.0,
                0.0,
            ];
            let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
            gpu.queue.write_buffer(&self.uniforms, 0, &bytes);
        }
        let bind_group = self.bind_group.clone();
        let pipeline = self.pipeline(gpu, format).clone();
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pattern"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(background),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        if let Some(v) = visible {
            #[allow(clippy::cast_precision_loss)]
            pass.set_viewport(
                v.x as f32,
                v.y as f32,
                v.width as f32,
                v.height as f32,
                0.0,
                1.0,
            );
            pass.set_scissor_rect(v.x, v.y, v.width, v.height);
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
