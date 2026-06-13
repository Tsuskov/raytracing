use crate::vec3::Vec3;

// What a surface is made of. The renderer reads this to decide how a ray
// behaves when it hits: diffuse light, mirror reflection, or glassy refraction.
#[derive(Clone, Copy)]
pub enum Material {
    // Matte surface lit directly by the sun. `albedo` is its color.
    Lambertian { albedo: Vec3 },
    // Mirror-like. `albedo` tints the reflection; `fuzz` roughens it (0 = a
    // perfect mirror, higher = blurrier, brushed-metal look).
    Metal { albedo: Vec3, fuzz: f32 },
    // Transparent like glass/water. `ior` is the index of refraction
    // (~1.5 for glass, 1.33 for water) -- how strongly it bends light.
    Dielectric { ior: f32 },
}
