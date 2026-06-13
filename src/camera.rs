use crate::ray::Ray;
use crate::vec3::{Vec3, vec3};

// A movable camera. We store where it is and where it looks (as two angles),
// and derive the direction vectors from those on demand.
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // radians: turning left/right. 0 looks toward -z.
    pub pitch: f32, // radians: looking up/down. Clamped so you can't flip over.
    pub vfov: f32,  // vertical field of view in degrees: smaller = more zoom.
}

impl Camera {
    // The unit vector the camera is looking along, built from the two angles.
    // At yaw=0, pitch=0 this is (0,0,-1) -- the same view we had before.
    pub fn forward(&self) -> Vec3 {
        vec3(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    // The camera's "right" direction: perpendicular to where it looks and to
    // world-up. The cross product gives us exactly that perpendicular vector.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(vec3(0.0, 1.0, 0.0)).unit()
    }

    // Aim the camera at a point in the world by solving for the yaw/pitch whose
    // `forward()` points from the camera toward `target`.
    pub fn look_at(&mut self, target: Vec3) {
        let dir = (target - self.position).unit();
        self.pitch = dir.y.asin();
        self.yaw = dir.x.atan2(-dir.z);
    }

    // Build everything a renderer needs to shoot rays for a given image size:
    // the ray origin and the viewport's top-left corner plus its across/down
    // edge vectors. A pixel at fractions (s, t) maps to
    //   top_left + s*u + t*v ,  and the ray goes from origin through it.
    pub fn viewport(&self, width: usize, height: usize) -> Viewport {
        // Field of view sets how tall the viewport is. tan(vfov/2) is the
        // half-height at one unit of distance; smaller vfov => smaller => zoom.
        let focal_length = 1.0;
        let theta = self.vfov.to_radians();
        let viewport_height = 2.0 * (theta / 2.0).tan() * focal_length;
        let viewport_width = viewport_height * (width as f32 / height as f32);

        let forward = self.forward();
        let right = self.right();
        let up = right.cross(forward); // camera's own up (already unit length)

        let u = right * viewport_width;
        let v = -up * viewport_height; // down the screen
        let center = self.position + forward * focal_length;
        let top_left = center - u / 2.0 - v / 2.0;

        Viewport {
            origin: self.position,
            top_left,
            u,
            v,
        }
    }
}

// Precomputed per-frame ray geometry (see Camera::viewport).
pub struct Viewport {
    pub origin: Vec3,
    pub top_left: Vec3,
    pub u: Vec3,
    pub v: Vec3,
}

impl Viewport {
    // The ray through viewport fractions (s, t), each in 0..1.
    pub fn ray(&self, s: f32, t: f32) -> Ray {
        let pixel = self.top_left + self.u * s + self.v * t;
        Ray::new(self.origin, pixel - self.origin)
    }
}
