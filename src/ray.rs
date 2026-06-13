use crate::vec3::Vec3;

// A ray is a half-line: it starts at `origin` and travels forever in
// `direction`. Any point on the ray is  origin + t * direction  for some
// t >= 0. Bigger t means farther along the ray.
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Ray {
        Ray { origin, direction }
    }

    // The point you reach after travelling `t` units along the ray.
    pub fn at(self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}
