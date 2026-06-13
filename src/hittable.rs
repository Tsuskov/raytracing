use crate::aabb::Aabb;
use crate::material::Material;
use crate::ray::Ray;
use crate::vec3::{vec3, Vec3};

// What we learn when a ray strikes a surface.
pub struct HitRecord {
    pub point: Vec3,       // where the hit happened, in world space
    pub normal: Vec3,      // surface direction, always facing against the ray
    pub front_face: bool,  // did we hit the outside (true) or inside (false)?
    pub t: f32,            // distance along the ray to the hit
    pub material: Material, // what the surface is made of
}

// Anything a ray can intersect. A sphere, a plane, a triangle... each just
// answers one question: "does this ray hit me between t_min and t_max, and if
// so, where?" `ray_color` only ever talks to this trait, never to concrete
// shapes, so adding new shapes never touches the renderer.
//
// The `: Sync` bound means objects can be shared across threads -- required so
// rayon can render many pixels in parallel against the same world.
pub trait Hittable: Sync {
    fn hit(&self, r: Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
    // The axis-aligned box enclosing this object. The BVH needs it to build
    // and traverse its tree of bounding boxes.
    fn bounding_box(&self) -> Aabb;
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: Material,
}

impl Hittable for Sphere {
    fn hit(&self, r: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        // Same quadratic as before: a*t^2 + b*t + c = 0.
        let oc = r.origin - self.center;
        let a = r.direction.dot(r.direction);
        let b = 2.0 * oc.dot(r.direction);
        let c = oc.dot(oc) - self.radius * self.radius;

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return None;
        }
        let sqrtd = discriminant.sqrt();

        // Try the nearer root first; if it's outside the allowed range, try
        // the farther one. This is what lets us ignore hits behind the camera
        // (t_min) or beyond a closer object (t_max).
        let mut t = (-b - sqrtd) / (2.0 * a);
        if t < t_min || t > t_max {
            t = (-b + sqrtd) / (2.0 * a);
            if t < t_min || t > t_max {
                return None;
            }
        }

        let point = r.at(t);
        // Dividing by the radius normalizes without a sqrt: for a sphere the
        // vector from center to surface already has length == radius.
        let outward_normal = (point - self.center) / self.radius;

        // Decide which side we hit. If the ray and the outward normal point
        // the same way (dot > 0), we struck the inside (e.g. a ray exiting
        // glass). We always store the normal facing *against* the ray, and
        // remember the side in `front_face` -- glass needs to know.
        let front_face = r.direction.dot(outward_normal) < 0.0;
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };

        Some(HitRecord {
            point,
            normal,
            front_face,
            t,
            material: self.material,
        })
    }

    fn bounding_box(&self) -> Aabb {
        let r = vec3(self.radius, self.radius, self.radius);
        Aabb {
            min: self.center - r,
            max: self.center + r,
        }
    }
}
