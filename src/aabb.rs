use crate::ray::Ray;
use crate::vec3::{vec3, Vec3};

// An axis-aligned bounding box: the smallest box (aligned to the x/y/z axes)
// that fully contains an object. Testing a ray against a box is far cheaper
// than against the real geometry, so the BVH uses boxes to quickly rule out
// whole groups of objects a ray can't possibly hit.
#[derive(Clone, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    // The smallest box that contains both `a` and `b`. Used to build a parent
    // box around its two children in the tree.
    pub fn surrounding(a: Aabb, b: Aabb) -> Aabb {
        Aabb {
            min: vec3(
                a.min.x.min(b.min.x),
                a.min.y.min(b.min.y),
                a.min.z.min(b.min.z),
            ),
            max: vec3(
                a.max.x.max(b.max.x),
                a.max.y.max(b.max.y),
                a.max.z.max(b.max.z),
            ),
        }
    }

    // Does the ray pass through the box within [t_min, t_max]? The "slab"
    // method: each axis defines a pair of parallel planes (a slab); the ray is
    // inside the box only where all three slabs' intervals overlap.
    pub fn hit(&self, r: Ray, mut t_min: f32, mut t_max: f32) -> bool {
        for a in 0..3 {
            let (origin, dir, lo, hi) = match a {
                0 => (r.origin.x, r.direction.x, self.min.x, self.max.x),
                1 => (r.origin.y, r.direction.y, self.min.y, self.max.y),
                _ => (r.origin.z, r.direction.z, self.min.z, self.max.z),
            };
            let inv_d = 1.0 / dir;
            // Distances at which the ray crosses this axis' two planes.
            let mut t0 = (lo - origin) * inv_d;
            let mut t1 = (hi - origin) * inv_d;
            // If the ray goes the negative direction, the planes are crossed in
            // the opposite order; swap so t0 is the near one.
            if inv_d < 0.0 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t0.max(t_min);
            t_max = t1.min(t_max);
            // The overlapping interval collapsed -> the ray misses the box.
            if t_max <= t_min {
                return false;
            }
        }
        true
    }
}
