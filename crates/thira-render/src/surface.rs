use crate::{AdapterSummary, Gpu, RenderError, TestPattern, Viewport};

/// Presents viewer frames straight to a window surface (ADR 0003, native backend).
///
/// The surface must be created on the thread that owns the window (the main
/// thread on macOS); this type can then live on the render thread.
#[derive(Debug)]
pub struct SurfaceViewer {
    gpu: Gpu,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    pattern: TestPattern,
    viewport: Option<Viewport>,
}

/// What happened when a frame was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presented {
    Frame,
    /// Nothing shown this time (window hidden, minimized or being reconfigured).
    Skipped,
}

impl SurfaceViewer {
    /// Opens a device that can present to `surface` and configures it at
    /// `width` × `height` physical pixels, with vsync.
    pub fn new(
        instance: wgpu::Instance,
        surface: wgpu::Surface<'static>,
        width: u32,
        height: u32,
    ) -> Result<Self, RenderError> {
        let gpu = Gpu::new(instance, Some(&surface))?;
        let caps = surface.get_capabilities(&gpu.adapter);
        // A non-sRGB format keeps shader values equal to display values, matching
        // the RGBA8 offscreen frames and the sRGB colors of the HTML around it.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| RenderError::NoAdapter("surface reports no formats".into()))?;
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: width.max(1),
            height: height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps
                .alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: Vec::new(),
            color_space: wgpu::SurfaceColorSpace::Auto,
        };
        surface.configure(&gpu.device, &config);
        let pattern = TestPattern::new(&gpu);
        Ok(Self {
            gpu,
            surface,
            config,
            pattern,
            viewport: None,
        })
    }

    pub fn summary(&self) -> AdapterSummary {
        self.gpu.summary()
    }

    /// Surface size in physical pixels.
    pub fn size(&self) -> (u32, u32) {
        (self.config.width, self.config.height)
    }

    /// Follows a window resize. Zero sizes (minimized windows) are ignored.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width, height) == self.size() {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.gpu.device, &self.config);
    }

    /// Where the viewer sits in the window, or `None` to draw only the background.
    pub fn set_viewport(&mut self, viewport: Option<Viewport>) {
        self.viewport = viewport;
    }

    /// Draws one frame at time `t` seconds and presents it. With `Fifo` this waits
    /// for vsync, which paces the render loop.
    pub fn render(&mut self, t: f32, background: wgpu::Color) -> Result<Presented, RenderError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                // Show this frame, then reconfigure so the next one matches.
                self.draw(&frame, t, background);
                self.gpu.queue.present(frame);
                self.surface.configure(&self.gpu.device, &self.config);
                return Ok(Presented::Frame);
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.gpu.device, &self.config);
                return Ok(Presented::Skipped);
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return Ok(Presented::Skipped);
            }
            other => return Err(RenderError::Surface(format!("{other:?}"))),
        };
        self.draw(&frame, t, background);
        self.gpu.queue.present(frame);
        Ok(Presented::Frame)
    }

    fn draw(&mut self, frame: &wgpu::SurfaceTexture, t: f32, background: wgpu::Color) {
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let size = self.size();
        // No viewport: an empty one draws just the background.
        let viewport = self.viewport.unwrap_or_default();
        self.pattern.draw(
            &self.gpu,
            &mut encoder,
            &view,
            self.config.format,
            size,
            viewport,
            background,
            t,
        );
        self.gpu.queue.submit([encoder.finish()]);
    }
}
