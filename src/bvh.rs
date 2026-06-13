use std::cmp::Ordering;

use crate::aabb::Aabb;
use crate::hittable::{HitRecord, Hittable};
use crate::ray::Ray;
use crate::vec3::Vec3;

// One node of the Bounding Volume Hierarchy. It owns a bounding box and two
// children (each either another node or a single object). A ray first tests
// the cheap box; only if it hits does it descend into the children. This turns
// "test every object" (linear) into "walk a tree" (logarithmic).
pub struct BvhNode {
    left: Box<dyn Hittable>,
    right: Box<dyn Hittable>,
    bbox: Aabb,
}

// Read one component (0=x, 1=y, 2=z) of a vector.
fn axis_value(v: Vec3, axis: usize) -> f32 {
    match axis {
        0 => v.x,
        1 => v.y,
        _ => v.z,
    }
}

impl BvhNode {
    // Recursively build a tree from a list of objects. Returns a single
    // `Hittable`: a leaf is just the object itself; everything else is a node.
    pub fn build(mut objects: Vec<Box<dyn Hittable>>) -> Box<dyn Hittable> {
        // A single object needs no box around it -- it *is* the leaf.
        if objects.len() == 1 {
            return objects.pop().unwrap();
        }

        // Box that encloses every object, so we can pick a good split axis.
        let mut bounds = objects[0].bounding_box();
        for o in &objects[1..] {
            bounds = Aabb::surrounding(bounds, o.bounding_box());
        }
        // Split along whichever axis the objects spread out the most -- that
        // separates them best and keeps the child boxes small.
        let extent = bounds.max - bounds.min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };

        // Sort by box position along that axis, then split down the middle.
        objects.sort_by(|a, b| {
            let ka = axis_value(a.bounding_box().min, axis);
            let kb = axis_value(b.bounding_box().min, axis);
            ka.partial_cmp(&kb).unwrap_or(Ordering::Equal)
        });
        let right_objects = objects.split_off(objects.len() / 2);

        let left = BvhNode::build(objects);
        let right = BvhNode::build(right_objects);
        let bbox = Aabb::surrounding(left.bounding_box(), right.bounding_box());

        Box::new(BvhNode { left, right, bbox })
    }
}

impl Hittable for BvhNode {
    fn hit(&self, r: Ray, t_min: f32, t_max: f32) -> Option<HitRecord> {
        // Skip this whole subtree if the ray misses its box.
        if !self.bbox.hit(r, t_min, t_max) {
            return None;
        }

        let hit_left = self.left.hit(r, t_min, t_max);
        // If we hit the left child, only accept a right-child hit that's nearer
        // (shrink t_max), so we still return the closest surface overall.
        let new_t_max = hit_left.as_ref().map(|h| h.t).unwrap_or(t_max);
        let hit_right = self.right.hit(r, t_min, new_t_max);

        hit_right.or(hit_left)
    }

    fn bounding_box(&self) -> Aabb {
        self.bbox
    }
}
