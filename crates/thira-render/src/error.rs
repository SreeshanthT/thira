use thiserror::Error;

/// Why rendering could not start or complete.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("no GPU adapter is available: {0}")]
    NoAdapter(String),

    #[error("could not open the GPU device: {0}")]
    NoDevice(String),

    #[error("could not read the frame back from the GPU: {0}")]
    Readback(String),

    #[error("the window surface failed: {0}")]
    Surface(String),

    #[error("frame size {width}x{height} is invalid")]
    InvalidSize { width: u32, height: u32 },
}
