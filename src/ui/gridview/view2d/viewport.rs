use super::*;

// TODO rewrite zoom

#[derive(Debug, Default)]
pub struct Viewport2D {
    /// Cell position that is at the center of the viewport.
    pub pos: Vec2D,
    /// Offset along the X axis (0..1). This is always rounded to the nearest
    /// number of whole pixels when rendering.
    pub x_offset: f32,
    /// Offset along the Y axis (0..1). This is always rounded to the nearest
    /// number of whole pixels when rendering.
    pub y_offset: f32,
    /// The zoom level.
    pub zoom: Zoom2D,
}

impl Viewport2D {
    /// Scroll the viewport by the given number of pixels along each axis.
    pub fn scroll(&mut self, mut dx: f32, mut dy: f32) {
        // Convert dx and dy from screen space into world space.
        dx *= self.zoom.cells_per_pixel();
        dy *= self.zoom.cells_per_pixel();
        // Add dx and dy.
        self.x_offset += dx;
        self.y_offset += dy;
        // Remove the integral part from self.x_offset and self.y_offset,
        // leaving only the fraction part between 0 and 1.
        let int_dx = self.x_offset.floor();
        let int_dy = self.y_offset.floor();
        self.x_offset -= int_dx;
        self.y_offset -= int_dy;
        // Add the integral part that we removed onto self.pos.
        *self.pos.x_mut() += int_dx as isize;
        *self.pos.y_mut() += int_dy as isize;
    }
    pub fn set_zoom(&mut self, new_zoom: Zoom2D) {
        self.zoom = new_zoom.clamp();
    }
    pub fn zoom_by(&mut self, factor: f32) {
        self.set_zoom(self.zoom * factor);
        self.zoom *= factor;
    }
}
