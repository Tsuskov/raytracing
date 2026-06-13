use crate::ray::Ray;
use crate::vec3::Vec3;

// What we learn when a ray strikes a surface.
pub struct HitRecord {
    pub normal: Vec3, // outward surface direction at the hit (unit length)
    pub t: f32,       // distance along the ray to the hit
}

// Anything a ray can intersect. A sphere, a plane, a triangle... each just
// answers one question: "does this ray hit me between t_min and t_max, and if
// so, where?" `ray_color` only ever talks to this trait, never to concrete
// shapes, so adding new shapes never touches the renderer.
pub trait Hittable {
    fn hit(&self, r: Ray, t_min: f32, t_max: f32) -> Option<HitRecord>;
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
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

        // Dividing by the radius normalizes without a sqrt: for a sphere the
        // vector from center to surface already has length == radius.
        let normal = (r.at(t) - self.center) / self.radius;
        Some(HitRecord { normal, t })
    }
}

// A list of objects is itself hittable: to hit the list, hit every object and
// keep the closest. We shrink the search range to the closest hit so far
// (`closest`), so each object is automatically occluded by nearer ones.
impl Hittable for Vec<Box<dyn Hittable>> {
    fn hit(&self, r: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        let mut closest = t_max;
        let mut result = None;
        for object in self {
            if let Some(rec) = object.hit(r, t_min, closest) {
                closest = rec.t;
                result = Some(rec);
            }
        }
        result
    }
}
