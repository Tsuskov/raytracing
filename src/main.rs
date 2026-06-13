mod camera;
mod hittable;
mod ray;
mod vec3;

use camera::Camera;
use hittable::{Hittable, Sphere};
use minifb::{Key, Window, WindowOptions};
use ray::Ray;
use std::time::Instant;
use vec3::{vec3, Color};

// The size of our image / window, in pixels.
const WIDTH: usize = 800;
const HEIGHT: usize = 450;

// Decide the color seen along a ray. Ask the world for the nearest hit; if
// there is one, shade it with our light, otherwise draw the sky.
fn ray_color(r: Ray, world: &dyn Hittable) -> Color {
    // t_min = 0.001 (not 0) avoids "shadow acne": a hit at the very surface we
    // started from due to floating-point error. t_max = infinity = see forever.
    if let Some(rec) = world.hit(r, 0.001, f32::INFINITY) {
        // The sphere's own color (how much of each channel it reflects).
        let albedo = vec3(0.8, 0.3, 0.3);

        // Direction from the surface toward the light (a far-off light up and
        // to the right-front, like a sun). It's the same everywhere.
        let to_light = vec3(1.0, 1.0, 1.0).unit();

        // Lambert's cosine law: brightest when the surface faces the light
        // head-on; that "facing-ness" is exactly normal . to_light.
        let diffuse = rec.normal.dot(to_light).max(0.0);

        // A little ambient term so the shadowed side isn't pure black.
        let ambient = 0.1;

        return albedo * (ambient + diffuse);
    }

    // Miss: the sky. Vertical gradient from white (bottom) to light blue (top).
    let unit = r.direction.unit();
    let a = 0.5 * (unit.y + 1.0);
    (1.0 - a) * vec3(1.0, 1.0, 1.0) + a * vec3(0.5, 0.7, 1.0)
}

// Convert a color with components in 0.0..1.0 into a packed 0x00RRGGBB u32.
fn to_u32(c: Color) -> u32 {
    let r = (c.x.clamp(0.0, 1.0) * 255.0) as u32;
    let g = (c.y.clamp(0.0, 1.0) * 255.0) as u32;
    let b = (c.z.clamp(0.0, 1.0) * 255.0) as u32;
    (r << 16) | (g << 8) | b
}

fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    // --- The world -------------------------------------------------------
    // A list of objects. The small sphere is our subject; the huge one acts
    // as a ground plane (a sphere of radius 100 looks flat up close).
    let world: Vec<Box<dyn Hittable>> = vec![
        Box::new(Sphere {
            center: vec3(0.0, 0.0, -1.0),
            radius: 0.5,
        }),
        Box::new(Sphere {
            center: vec3(0.0, -100.5, -1.0),
            radius: 100.0,
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
                let s = x as f32 / (WIDTH - 1) as f32;
                let t = y as f32 / (HEIGHT - 1) as f32;

                let pixel = viewport_top_left + s * viewport_u + t * viewport_v;
                let r = Ray::new(cam.position, pixel - cam.position);

                buffer[y * WIDTH + x] = to_u32(ray_color(r, &world));
            }
        }

        window
            .update_with_buffer(&buffer, WIDTH, HEIGHT)
            .expect("failed to update window");
    }
}
