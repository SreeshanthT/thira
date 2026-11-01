use crate::RenderError;

/// The wgpu adapter, device and queue used for all rendering.
#[derive(Debug)]
pub struct Gpu {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

/// Which adapter was chosen, for logs and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterSummary {
    pub name: String,
    pub backend: String,
    /// True for software adapters such as WARP or llvmpipe.
    pub software: bool,
}

impl Gpu {
    /// Opens a GPU device without a window, for offscreen rendering.
    pub fn headless() -> Result<Self, RenderError> {
        Self::new(wgpu::Instance::default(), None)
    }

    /// Opens a GPU device. With `surface`, the adapter must be able to present to it.
    ///
    /// Prefers a hardware adapter and falls back to a software one, so rendering
    /// also works on machines without a GPU.
    pub fn new(
        instance: wgpu::Instance,
        surface: Option<&wgpu::Surface<'_>>,
    ) -> Result<Self, RenderError> {
        let request = |force_fallback_adapter| {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: surface,
                force_fallback_adapter,
                apply_limit_buckets: false,
            }))
        };
        let adapter = request(false)
            .or_else(|_| request(true))
            .map_err(|e| RenderError::NoAdapter(e.to_string()))?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("thira"),
            ..Default::default()
        }))
        .map_err(|e| RenderError::NoDevice(e.to_string()))?;
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
        })
    }

    pub fn summary(&self) -> AdapterSummary {
        let info = self.adapter.get_info();
        AdapterSummary {
            name: info.name,
            backend: format!("{:?}", info.backend),
            software: info.device_type == wgpu::DeviceType::Cpu,
        }
    }
}
