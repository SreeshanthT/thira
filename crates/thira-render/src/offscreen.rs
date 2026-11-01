use crate::{Gpu, RenderError, TestPattern, Viewport};

/// Renders the test pattern inside `viewport` of a new `width` × `height` RGBA8
/// texture cleared to `background`, and reads it back as tightly packed rows, top
/// row first.
pub fn render_rgba(
    gpu: &Gpu,
    pattern: &mut TestPattern,
    (width, height): (u32, u32),
    viewport: Viewport,
    background: wgpu::Color,
    t: f32,
) -> Result<Vec<u8>, RenderError> {
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return Err(RenderError::InvalidSize { width, height });
    }
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("offscreen frame"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    // Buffer copies need rows aligned to 256 bytes.
    let packed_row = width * 4;
    let padded_row = packed_row.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("offscreen readback"),
        size: u64::from(padded_row) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    pattern.draw(
        gpu,
        &mut encoder,
        &view,
        format,
        (width, height),
        viewport,
        background,
        t,
    );
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_row),
                rows_per_image: Some(height),
            },
        },
        extent,
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    gpu.device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    rx.recv()
        .map_err(|e| RenderError::Readback(e.to_string()))?
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    let mapped = slice
        .get_mapped_range()
        .map_err(|e| RenderError::Readback(e.to_string()))?;
    let mut pixels = Vec::with_capacity(packed_row as usize * height as usize);
    for row in mapped.chunks_exact(padded_row as usize) {
        pixels.extend_from_slice(&row[..packed_row as usize]);
    }
    Ok(pixels)
}
