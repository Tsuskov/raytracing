use std::ops::{Add, Div, Mul, Neg, Sub};

// A 3D vector. We use it for three different things:
//   - points in space   (where something is)
//   - directions        (which way a ray goes)
//   - colors            (r, g, b stored in x, y, z)
// They're all just three f32s, so one type covers all of them.
#[derive(Clone, Copy, Debug)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// A color is just a vector with components in 0.0..1.0. The alias makes
// intent clearer at call sites.
pub type Color = Vec3;

pub fn vec3(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3 { x, y, z }
}

impl Vec3 {
    // Length squared. Cheaper than length() because it skips the sqrt; handy
    // when you only need to compare magnitudes.
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    // Dot product: a measure of how aligned two vectors are.
    // Positive when they point the same way, zero when perpendicular.
    pub fn dot(self, other: Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    // Cross product: a vector perpendicular to both inputs.
    pub fn cross(self, other: Vec3) -> Vec3 {
        vec3(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    // Return a vector pointing the same way but with length 1.
    pub fn unit(self) -> Vec3 {
        self / self.length()
    }
}

// --- Operator overloading -------------------------------------------------
// These let us write a + b, a - b, a * 2.0, -a, etc. instead of verbose
// method calls. They make the rendering math read like the math on paper.

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        vec3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        vec3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        vec3(-self.x, -self.y, -self.z)
    }
}

// Vec3 * Vec3 multiplies component by component. We use this for colors:
// tinting a light color by a surface color.
impl Mul for Vec3 {
    type Output = Vec3;
    fn mul(self, o: Vec3) -> Vec3 {
        vec3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}

// Vec3 * f32 scales the vector.
impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        vec3(self.x * s, self.y * s, self.z * s)
    }
}

// f32 * Vec3, so we can write 0.5 * v as well as v * 0.5.
impl Mul<Vec3> for f32 {
    type Output = Vec3;
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}

impl Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        vec3(self.x / s, self.y / s, self.z / s)
    }
}
