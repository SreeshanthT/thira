/// A rectangle of the render target in physical pixels, such as one viewer's area
/// inside the window surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Viewport {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Viewport {
    /// The whole of a `width` × `height` target.
    pub const fn full(width: u32, height: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            width,
            height,
        }
    }

    /// The part of this viewport inside a `width` × `height` target, or `None` if
    /// nothing is visible. wgpu rejects viewports that leave the target.
    pub fn clamped(self, width: u32, height: u32) -> Option<Self> {
        let x = self.x.min(width);
        let y = self.y.min(height);
        let w = self.width.min(width - x);
        let h = self.height.min(height - y);
        (w > 0 && h > 0).then_some(Self {
            x,
            y,
            width: w,
            height: h,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_the_target() {
        let v = Viewport {
            x: 100,
            y: 50,
            width: 2000,
            height: 2000,
        };
        assert_eq!(
            v.clamped(1280, 720),
            Some(Viewport {
                x: 100,
                y: 50,
                width: 1180,
                height: 670
            })
        );
        assert_eq!(
            Viewport::full(10, 10).clamped(10, 10),
            Some(Viewport::full(10, 10))
        );
    }

    #[test]
    fn invisible_viewports_are_none() {
        assert_eq!(
            Viewport {
                x: 1280,
                y: 0,
                width: 10,
                height: 10
            }
            .clamped(1280, 720),
            None
        );
        assert_eq!(
            Viewport {
                x: 0,
                y: 0,
                width: 0,
                height: 10
            }
            .clamped(1280, 720),
            None
        );
    }
}
