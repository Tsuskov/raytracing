mod aabb;
mod bvh;
mod camera;
mod hittable;
mod material;
mod ray;
mod vec3;

use bvh::BvhNode;
use camera::{Camera, Viewport};
use hittable::{Hittable, Sphere};
use material::Material;
use minifb::{Key, Window, WindowOptions};
use ray::Ray;
use rayon::prelude::*;
use std::time::Instant;
use vec3::{Color, Vec3, reflect, refract, vec3};

// The interactive window size.
const WIDTH: usize = 800;
const HEIGHT: usize = 450;

// How many rays we average per pixel for anti-aliasing. More = smoother edges
// but slower. 4 keeps the (heavy) many-sphere scene real-time.
const SAMPLES_PER_PIXEL: usize = 4;

// Defaults for the offline 4K snapshot (cargo run --release -- render). Many
// samples = smooth, photographic result; we're not on a frame budget here.
const SNAPSHOT_WIDTH: usize = 3840;
const SNAPSHOT_SAMPLES: usize = 64;

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

            // Glass: a real surface both reflects and refracts at once. We
            // trace BOTH rays and blend them by the Fresnel factor (Schlick).
            // Doing it deterministically -- instead of randomly picking one --
            // is what keeps the glass noise-free.
            Material::Dielectric { ior } => {
                // Entering the glass divides by ior; exiting multiplies by it.
                let ratio = if rec.front_face { 1.0 / ior } else { ior };
                let unit_dir = r.direction.unit();

                let cos_theta = (-unit_dir).dot(rec.normal).min(1.0);
                let sin_theta = (1.0 - cos_theta * cos_theta).sqrt();

                let reflected = reflect(unit_dir, rec.normal);
                let reflected_color =
                    ray_color(Ray::new(rec.point, reflected), world, depth - 1, rng);

                // If the bent ray would need to exceed 90deg it physically
                // can't refract -- all the light reflects (total internal
                // reflection), so there's nothing to blend.
                if ratio * sin_theta > 1.0 {
                    return reflected_color;
                }

                // Otherwise mix reflection and refraction. Schlick gives how
                // much reflects; the rest refracts through.
                let refracted = refract(unit_dir, rec.normal, ratio);
                let refracted_color =
                    ray_color(Ray::new(rec.point, refracted), world, depth - 1, rng);

                let r = reflectance(cos_theta, ratio);
                reflected_color * r + refracted_color * (1.0 - r)
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

// Build the "Ray Tracing in One Weekend" showcase: a big ground sphere, three
// large feature spheres (glass, matte, metal), and a field of small spheres
// with randomized materials. Deterministic: same seed -> same scene.
fn build_world() -> Vec<Box<dyn Hittable>> {
    let mut rng: u32 = 1;
    let mut world: Vec<Box<dyn Hittable>> = Vec::new();

    // Ground (a giant sphere, looks flat up close).
    world.push(Box::new(Sphere {
        center: vec3(0.0, -1000.0, 0.0),
        radius: 1000.0,
        material: Material::Lambertian {
            albedo: vec3(0.5, 0.5, 0.5),
        },
    }));

    // A grid of small spheres, each nudged randomly and given a random material.
    for a in -11..11 {
        for b in -11..11 {
            let choose = random_f32(&mut rng);
            let center = vec3(
                a as f32 + 0.9 * random_f32(&mut rng),
                0.2,
                b as f32 + 0.9 * random_f32(&mut rng),
            );
            // Leave a gap where the big metal sphere stands.
            if (center - vec3(4.0, 0.2, 0.0)).length() <= 0.9 {
                continue;
            }

            let material = if choose < 0.8 {
                // 80% diffuse, with a random (slightly dark) color.
                let albedo = vec3(
                    random_f32(&mut rng) * random_f32(&mut rng),
                    random_f32(&mut rng) * random_f32(&mut rng),
                    random_f32(&mut rng) * random_f32(&mut rng),
                );
                Material::Lambertian { albedo }
            } else if choose < 0.95 {
                // 15% metal, light color and a little fuzz.
                let albedo = vec3(
                    0.5 + 0.5 * random_f32(&mut rng),
                    0.5 + 0.5 * random_f32(&mut rng),
                    0.5 + 0.5 * random_f32(&mut rng),
                );
                let fuzz = 0.5 * random_f32(&mut rng);
                Material::Metal { albedo, fuzz }
            } else {
                // 5% glass.
                Material::Dielectric { ior: 1.5 }
            };

            world.push(Box::new(Sphere {
                center,
                radius: 0.2,
                material,
            }));
        }
    }

    // Three big feature spheres.
    world.push(Box::new(Sphere {
        center: vec3(0.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Dielectric { ior: 1.5 },
    }));
    world.push(Box::new(Sphere {
        center: vec3(-4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Lambertian {
            albedo: vec3(0.4, 0.2, 0.1),
        },
    }));
    world.push(Box::new(Sphere {
        center: vec3(4.0, 1.0, 0.0),
        radius: 1.0,
        material: Material::Metal {
            albedo: vec3(0.7, 0.6, 0.5),
            fuzz: 0.0,
        },
    }));

    world
}

// Render the whole image into `buffer` using all CPU cores. rayon splits the
// rows across threads; each row gets its own RNG so threads never share state.
fn render_into(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    samples: usize,
    vp: &Viewport,
    world: &dyn Hittable,
    frame_seed: u32,
) {
    buffer
        .par_chunks_mut(width)
        .enumerate()
        .for_each(|(y, row)| {
            // Per-row seed, kept nonzero (xorshift requires it).
            let mut rng =
                ((y as u32).wrapping_mul(0x9E3779B9) ^ frame_seed.wrapping_mul(2654435761)) | 1;

            for (x, pixel) in row.iter_mut().enumerate() {
                // Average several jittered rays per pixel (anti-aliasing).
                let mut color = vec3(0.0, 0.0, 0.0);
                for _ in 0..samples {
                    let s = (x as f32 + random_f32(&mut rng)) / (width - 1) as f32;
                    let t = (y as f32 + random_f32(&mut rng)) / (height - 1) as f32;
                    let r = vp.ray(s, t);
                    color = color + ray_color(r, world, MAX_DEPTH, &mut rng);
                }
                *pixel = to_u32(color / samples as f32);
            }
        });
}

// A camera positioned to frame the whole scene, looking at the origin.
fn showcase_camera() -> Camera {
    let mut cam = Camera {
        position: vec3(13.0, 2.0, 3.0),
        yaw: 0.0,
        pitch: 0.0,
        vfov: 28.0,
    };
    cam.look_at(vec3(0.0, 0.0, 0.0));
    cam
}

// Interactive mode: open a window and let the user fly around in real time.
fn run_interactive(world: &dyn Hittable) {
    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    let mut cam = showcase_camera();

    let mut window = Window::new("Ray Tracer", WIDTH, HEIGHT, WindowOptions::default())
        .expect("failed to open window");
    window.set_target_fps(60);

    let mut last_frame = Instant::now();
    let mut frame: u32 = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let dt = last_frame.elapsed().as_secs_f32();
        last_frame = Instant::now();
        frame = frame.wrapping_add(1);

        // --- Input: update the camera ------------------------------------
        let move_speed = 4.0 * dt; // world units per second
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

        let vp = cam.viewport(WIDTH, HEIGHT);
        render_into(
            &mut buffer,
            WIDTH,
            HEIGHT,
            SAMPLES_PER_PIXEL,
            &vp,
            world,
            frame,
        );

        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .expect("failed to update window");
    }
}

// Offline mode: render one high-resolution, high-sample frame and save a PNG.
fn render_snapshot(world: &dyn Hittable, width: usize, height: usize, samples: usize) {
    let cam = showcase_camera();
    let vp = cam.viewport(width, height);
    let mut buffer = vec![0u32; width * height];

    println!("Rendering {width}x{height} at {samples} samples/pixel...");
    let start = Instant::now();
    render_into(&mut buffer, width, height, samples, &vp, world, 1);
    println!("Rendered in {:.1?}", start.elapsed());

    // Pack the u32 buffer into an RGB PNG.
    let mut img = image::RgbImage::new(width as u32, height as u32);
    for (i, px) in buffer.iter().enumerate() {
        let x = (i % width) as u32;
        let y = (i / width) as u32;
        let rgb = image::Rgb([
            ((px >> 16) & 0xff) as u8,
            ((px >> 8) & 0xff) as u8,
            (px & 0xff) as u8,
        ]);
        img.put_pixel(x, y, rgb);
    }
    img.save("render.png").expect("failed to save PNG");
    println!("Saved render.png");
}

fn main() {
    // Organize the scene into a BVH so each ray tests ~log(N) objects instead
    // of all of them -- the difference between a few FPS and smooth real-time.
    let world = BvhNode::build(build_world());

    // `cargo run --release -- render [width] [samples]` renders a PNG and
    // exits; with no args we open the interactive window.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|s| s == "render").unwrap_or(false) {
        let width = args
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(SNAPSHOT_WIDTH);
        let samples = args
            .get(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(SNAPSHOT_SAMPLES);
        let height = width * 9 / 16; // keep 16:9
        render_snapshot(world.as_ref(), width, height, samples);
    } else {
        run_interactive(world.as_ref());
    }
}
