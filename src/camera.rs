use crate::vec3::{vec3, Vec3};

// A movable camera. We store where it is and where it looks (as two angles),
// and derive the direction vectors from those on demand.
pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // radians: turning left/right. 0 looks toward -z.
    pub pitch: f32, // radians: looking up/down. Clamped so you can't flip over.
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
}
