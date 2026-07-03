use crate::bbox::BBox;
use crate::point::Point2;

#[derive(Debug, Clone, Copy)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
    pub dpr: f32,
    pub screen_w: f32,
    pub screen_h: f32,
}

impl Viewport {
    pub fn new(screen_w: f32, screen_h: f32) -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
            dpr: 1.0,
            screen_w,
            screen_h,
        }
    }

    pub fn screen_to_world(&self, sp: Point2) -> Point2 {
        Point2 {
            x: (sp.x / self.dpr - self.pan_x) / self.zoom,
            y: (sp.y / self.dpr - self.pan_y) / self.zoom,
        }
    }

    pub fn world_to_screen(&self, wp: Point2) -> Point2 {
        Point2 {
            x: (wp.x * self.zoom + self.pan_x) * self.dpr,
            y: (wp.y * self.zoom + self.pan_y) * self.dpr,
        }
    }

    pub fn visible_world_bbox(&self) -> BBox {
        let tl = self.screen_to_world(Point2::new(0.0, 0.0));
        let br = self.screen_to_world(Point2::new(self.screen_w, self.screen_h));
        BBox {
            min_x: tl.x,
            min_y: tl.y,
            max_x: br.x,
            max_y: br.y,
        }
    }

    pub fn pan_by_screen_delta(&mut self, dx: f32, dy: f32) {
        self.pan_x += dx / self.dpr;
        self.pan_y += dy / self.dpr;
    }

    pub fn zoom_at_screen_pos(&mut self, sp: Point2, factor: f32) {
        let wp_before = self.screen_to_world(sp);
        self.zoom = (self.zoom * factor).clamp(0.05, 64.0);
        let wp_after = self.screen_to_world(sp);
        self.pan_x += (wp_after.x - wp_before.x) * self.zoom;
        self.pan_y += (wp_after.y - wp_before.y) * self.zoom;
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self::new(1024.0, 768.0)
    }
}
