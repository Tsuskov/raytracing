mod camera;
mod hittable;
mod material;
mod ray;
mod vec3;

use camera::Camera;
use hittable::{Hittable, Sphere};
use material::Material;
use minifb::{Key, Window, WindowOptions};
use ray::Ray;
use std::time::Instant;
use vec3::{reflect, refract, vec3, Color, Vec3};

// The size of our image / window, in pixels.
const WIDTH: usize = 800;
const HEIGHT: usize = 450;

// How many rays we average per pixel for anti-aliasing. More = smoother edges
// but slower. 4 keeps us comfortably real-time.
const SAMPLES_PER_PIXEL: usize = 4;

// How many times a ray may bounce (metal/glass) before we give up. Without
// this cap, a ray trapped between mirrors would recurse forever.
const MAX_DEPTH: u32 = 10;

// A tiny, fast pseudo-random generator (xorshift32). We only need cheap jitter
// for anti-aliasing, not cryptographic quality. Returns a float in 0.0..1.0.
fn random_f32(state: &mut u32) -> f32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x as f32 / u32::MAX as f32
}

// Our lighting math works in "linear" light, but screens expect "gamma"
// encoded color. Without this correction the image looks too dark. Gamma 2.0
// is just a square root.
fn linear_to_gamma(x: f32) -> f32 {
    x.sqrt()
}

// A random unit-length vector. We use it to roughen metal reflections (fuzz):
// nudging the reflected ray by a random vector scatters it slightly.
fn random_unit_vector(rng: &mut u32) -> Vec3 {
    // Pick random points in a cube until one lands inside the unit sphere,
    // then normalize it. Rejecting points outside avoids a directional bias.
    loop {
        let p = vec3(
            random_f32(rng) * 2.0 - 1.0,
            random_f32(rng) * 2.0 - 1.0,
            random_f32(rng) * 2.0 - 1.0,
        );
        let len_sq = p.length_squared();
        if len_sq > 1e-6 && len_sq <= 1.0 {
            return p / len_sq.sqrt();
        }
    }
}

// Schlick's approximation: real glass reflects more at grazing angles (think of
// how a lake mirrors the sky near the horizon but is clear when you look
// straight down). This returns the probability of reflecting vs. refracting.
fn reflectance(cosine: f32, ior: f32) -> f32 {
    let r0 = ((1.0 - ior) / (1.0 + ior)).powi(2);
    r0 + (1.0 - r0) * (1.0 - cosine).powi(5)
}

// Decide the color seen along a ray. On a hit, behave according to the
// material: diffuse surfaces are lit directly, while metal and glass spawn a
// new ray and recurse. `depth` counts down to bound the recursion.
fn ray_color(r: Ray, world: &dyn Hittable, depth: u32, rng: &mut u32) -> Color {
    // Ran out of bounces: contribute no more light.
    if depth == 0 {
        return vec3(0.0, 0.0, 0.0);
    }

    // t_min = 0.001 (not 0) avoids "shadow acne": a hit at the very surface we
    // started from due to floating-point error. t_max = infinity = see forever.
    if let Some(rec) = world.hit(r, 0.001, f32::INFINITY) {
        match rec.material {
            // Matte: lit directly by the sun (terminal, no bounce).
            Material::Lambertian { albedo } => {
                let to_light = vec3(1.0, 1.0, 1.0).unit();
                let diffuse = rec.normal.dot(to_light).max(0.0);
                let ambient = 0.1;
                albedo * (ambient + diffuse)
            }

            // Mirror: reflect the ray, optionally roughened by `fuzz`, and see
            // what that reflected ray hits. `albedo` tints the reflection.
            Material::Metal { albedo, fuzz } => {
                let reflected = reflect(r.direction.unit(), rec.normal);
                let scattered_dir = reflected + random_unit_vector(rng) * fuzz;
                // If fuzz knocked the ray below the surface, treat it absorbed.
                if scattered_dir.dot(rec.normal) > 0.0 {
                    let scattered = Ray::new(rec.point, scattered_dir);
                    albedo * ray_color(scattered, world, depth - 1, rng)
                } else {
                    vec3(0.0, 0.0, 0.0)
                }
            }

            // Glass: either refract through or reflect off the surface. Schlick
            // picks which, and total internal reflection forces reflection.
            Material::Dielectric { ior } => {
                // Entering the glass divides by ior; exiting multiplies by it.
                let ratio = if rec.front_face { 1.0 / ior } else { ior };
                let unit_dir = r.direction.unit();

                let cos_theta = (-unit_dir).dot(rec.normal).min(1.0);
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

                // If the bent ray would need to exceed 90deg, it physically
                // can't refract -- it must reflect (total internal reflection).
                let cannot_refract = ratio * sin_theta > 1.0;
                let direction = if cannot_refract
                    || reflectance(cos_theta, ratio) > random_f32(rng)
                {
                    reflect(unit_dir, rec.normal)
                } else {
                    refract(unit_dir, rec.normal, ratio)
                };

                // Clear glass doesn't tint, so we don't multiply by a color.
                let scattered = Ray::new(rec.point, direction);
                ray_color(scattered, world, depth - 1, rng)
            }
        }
    } else {
        // Miss: the sky. Vertical gradient from white (bottom) to blue (top).
        let unit = r.direction.unit();
        let a = 0.5 * (unit.y + 1.0);
        (1.0 - a) * vec3(1.0, 1.0, 1.0) + a * vec3(0.5, 0.7, 1.0)
    }
}

// Convert a color with components in 0.0..1.0 into a packed 0x00RRGGBB u32,
// applying gamma correction on the way out.
fn to_u32(c: Color) -> u32 {
    let r = (linear_to_gamma(c.x.clamp(0.0, 1.0)) * 255.0) as u32;
    let g = (linear_to_gamma(c.y.clamp(0.0, 1.0)) * 255.0) as u32;
    let b = (linear_to_gamma(c.z.clamp(0.0, 1.0)) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // --- The world -------------------------------------------------------
    // The classic showcase: a matte sphere flanked by glass (left) and metal
    // (right), all resting on a big matte ground sphere.
    let world: Vec<Box<dyn Hittable>> = vec![
        // Ground
        Box::new(Sphere {
            center: vec3(0.0, -100.5, -1.0),
            radius: 100.0,
            material: Material::Lambertian {
                albedo: vec3(0.8, 0.8, 0.0),
            },
        }),
        // Center: matte
        Box::new(Sphere {
            center: vec3(0.0, 0.0, -1.0),
            radius: 0.5,
            material: Material::Lambertian {
                albedo: vec3(0.7, 0.3, 0.3),
            },
        }),
        // Left: glass
        Box::new(Sphere {
            center: vec3(-1.0, 0.0, -1.0),
            radius: 0.5,
            material: Material::Dielectric { ior: 1.5 },
        }),
        // Right: metal
        Box::new(Sphere {
            center: vec3(1.0, 0.0, -1.0),
            radius: 0.5,
            material: Material::Metal {
                albedo: vec3(0.8, 0.6, 0.2),
                fuzz: 0.1,
            },
        }),
    ];

    // The camera starts at the origin looking down -z (same view as before),
    // but now we can move it around.
    let mut cam = Camera {
        position: vec3(0.0, 0.0, 0.0),
        yaw: 0.0,
        pitch: 0.0,
    };

    let focal_length = 1.0; // distance from camera to the viewport
    let viewport_height = 2.0;
    let viewport_width = viewport_height * (WIDTH as f32 / HEIGHT as f32);

    let mut window = Window::new("Ray Tracer", WIDTH, HEIGHT, WindowOptions::default())
        .expect("failed to open window");
    window.set_target_fps(60);

    // Track real elapsed time per frame so movement speed is independent of
    // frame rate (move N units *per second*, not per frame).
    let mut last_frame = Instant::now();

    // Random state for anti-aliasing jitter. Any nonzero seed works (xorshift
    // must never be zero); it keeps evolving across frames.
    let mut rng: u32 = 0x9E3779B9;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = last_frame.elapsed().as_secs_f32();
        last_frame = Instant::now();

        // --- Input: update the camera ------------------------------------
        let move_speed = 2.0 * dt; // world units per second
        let look_speed = 1.5 * dt; // radians per second
        let forward = cam.forward();
        let right = cam.right();
        let world_up = vec3(0.0, 1.0, 0.0);

        if window.is_key_down(Key::W) {
            cam.position = cam.position + forward * move_speed;
        }
        if window.is_key_down(Key::S) {
            cam.position = cam.position - forward * move_speed;
        }
        if window.is_key_down(Key::D) {
            cam.position = cam.position + right * move_speed;
        }
        if window.is_key_down(Key::A) {
            cam.position = cam.position - right * move_speed;
        }
        if window.is_key_down(Key::Space) {
            cam.position = cam.position + world_up * move_speed;
        }
        if window.is_key_down(Key::LeftShift) {
            cam.position = cam.position - world_up * move_speed;
        }
        if window.is_key_down(Key::Left) {
            cam.yaw -= look_speed;
        }
        if window.is_key_down(Key::Right) {
            cam.yaw += look_speed;
        }
        if window.is_key_down(Key::Up) {
            cam.pitch += look_speed;
        }
        if window.is_key_down(Key::Down) {
            cam.pitch -= look_speed;
        }
        // Stop just short of straight up/down so the view never flips over.
        cam.pitch = cam.pitch.clamp(-1.5, 1.5);

        // --- Per-frame viewport, rebuilt from the camera's orientation ----
        // Same idea as the fixed viewport before, but the across/down vectors
        // now follow wherever the camera looks.
        let forward = cam.forward();
        let right = cam.right();
        let up = right.cross(forward); // camera's own up (already unit length)

        let viewport_u = right * viewport_width;
        let viewport_v = -up * viewport_height; // down the screen
        let viewport_center = cam.position + forward * focal_length;
        let viewport_top_left = viewport_center - viewport_u / 2.0 - viewport_v / 2.0;

        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                // Average several rays per pixel, each aimed at a slightly
                // random spot *within* the pixel. Edges that would be a hard
                // jagged step become a smooth blend of foreground and
                // background -- that's anti-aliasing.
                let mut color = vec3(0.0, 0.0, 0.0);
                for _ in 0..SAMPLES_PER_PIXEL {
                    let s = (x as f32 + random_f32(&mut rng)) / (WIDTH - 1) as f32;
                    let t = (y as f32 + random_f32(&mut rng)) / (HEIGHT - 1) as f32;

                    let pixel = viewport_top_left + s * viewport_u + t * viewport_v;
                    let r = Ray::new(cam.position, pixel - cam.position);
                    color = color + ray_color(r, &world, MAX_DEPTH, &mut rng);
                }
                color = color / SAMPLES_PER_PIXEL as f32;

                buffer[y * WIDTH + x] = to_u32(color);
            }
        }

        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .expect("failed to update window");
    }
}
