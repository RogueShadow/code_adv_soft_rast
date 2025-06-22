mod camera;
mod geometry;
mod my_app;

use crate::camera::Camera;
use crate::geometry::{Bounds, Model, Vertex, point_in_triangle};
use crate::my_app::MyApp;
use nalgebra::{Isometry3, Point2, Point3, Vector2, Vector3};
use rand::Rng;
use rand_xorshift::XorShiftRng;
use softbuffer::{Context, Surface};
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalSize, Size};
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{CursorGrabMode, Window, WindowAttributes, WindowId};

const WIDTH: f32 = 1600.0;
const HEIGHT: f32 = 900.0;

pub enum SoftRastEvent<'a> {
    Resume {},
    Update {
        input: InputState,
        delta: Duration,
    },
    Render {
        delta: Duration,
        scene: &'a mut Scene,
    },
}
pub enum SoftRastCommand {
    SetTitle(String),
    SetRenderingMode {
        tris: bool,
        wireframe: bool,
        points: bool,
    },
}

trait UserState {
    fn handle_event(&mut self, command: &mut Command, event: SoftRastEvent);
}

struct Command {
    commands: Vec<SoftRastCommand>,
    timer: Instant,
}
#[allow(unused)]
impl Command {
    pub fn set_title(&mut self, title: &str) {
        self.commands
            .push(SoftRastCommand::SetTitle(title.to_owned()));
    }
    pub fn set_render_mode(&mut self, tris: bool, wireframe: bool, points: bool) {
        self.commands.push(SoftRastCommand::SetRenderingMode {
            tris,
            wireframe,
            points,
        })
    }
    pub fn elapsed(&self) -> Duration {
        self.timer.elapsed()
    }
}
impl Default for Command {
    fn default() -> Self {
        Self {
            commands: vec![],
            timer: Instant::now(),
        }
    }
}
#[derive(Default, Clone)]
pub struct InputState {
    pressed_keys: HashSet<String>,
    mouse_dx: f64,
    mouse_dy: f64,
}

impl InputState {
    fn reset_mouse_motion(&mut self) {
        self.mouse_dx = 0.0;
        self.mouse_dy = 0.0;
    }
}
struct AppContext {
    user_state: Box<dyn UserState>,
    window: Option<Rc<Window>>,
    context: Option<Context<Rc<Window>>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    render_target: Option<RenderTarget>,
    command: Command,
    scene: Option<Scene>,
    timer: Instant,
    input: InputState,
    draw_tris: bool,
    draw_wireframe: bool,
    draw_points: bool,
}
impl AppContext {
    pub fn new(user_state: impl UserState + 'static) -> Self {
        Self {
            user_state: Box::new(user_state),
            window: None,
            context: None,
            surface: None,
            render_target: None,
            command: Command::default(),
            scene: None,
            timer: Instant::now(),
            input: InputState::default(),
            draw_tris: true,
            draw_wireframe: true,
            draw_points: false,
        }
    }
}

struct RenderTarget {
    color: Vec<u32>,
    depth: Vec<f32>,
    width: u32,
    height: u32,
    clear_color: u32,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            color: vec![u32::MIN; (width * height) as usize],
            depth: vec![f32::MAX; (width * height) as usize],
            width,
            height,
            clear_color: u32::MIN,
        }
    }
    pub fn clear(&mut self) {
        self.color.fill(self.clear_color);
        self.depth.fill(f32::MAX);
    }
}

impl ApplicationHandler for AppContext {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let mut attributes = WindowAttributes::default();
            attributes.inner_size = Some(Size::new(PhysicalSize::new(WIDTH, HEIGHT)));

            let window = match event_loop.create_window(attributes) {
                Ok(window) => Rc::new(window),
                Err(err) => {
                    panic!("{}", err);
                }
            };
            let context = match Context::new(window.clone()) {
                Ok(context) => context,
                Err(err) => {
                    panic!("{}", err);
                }
            };
            let surface = match Surface::new(&context, window.clone()) {
                Ok(surface) => surface,
                Err(err) => {
                    panic!("{}", err);
                }
            };

            if let Err(err) = window.set_cursor_grab(CursorGrabMode::Confined) {
                eprintln!("{:?}", err);
            }
            window.set_cursor_visible(false);

            self.window = Some(window.clone());
            self.context = Some(context);
            self.surface = Some(surface);
        }
        self.user_state
            .handle_event(&mut self.command, SoftRastEvent::Resume {});
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().expect("Couldn't get the window.");
        let surface = self.surface.as_mut().expect("Couldn't get the surface.");

        for command in self.command.commands.iter() {
            match command {
                SoftRastCommand::SetTitle(title) => {
                    window.set_title(&title);
                }
                SoftRastCommand::SetRenderingMode {
                    tris,
                    wireframe,
                    points,
                } => {
                    self.draw_tris = tris.to_owned();
                    self.draw_wireframe = wireframe.to_owned();
                    self.draw_points = points.to_owned();
                }
            }
        }

        self.command.commands.clear();

        match event {
            WindowEvent::Resized(size) => {
                if size.width == 0 || size.height == 0 {
                    self.render_target = None;
                    thread::sleep(Duration::from_secs_f32(0.015));
                    return;
                }
                let (width, height) = { (size.width, size.height) };
                if let Err(err) = surface.resize(
                    NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN),
                    NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN),
                ) {
                    eprintln!("{}", err);
                }
                self.render_target = Some(RenderTarget::new(width, height));
                if let Some(scene) = self.scene.as_mut() {
                    scene.camera.aspect_ratio = width as f32 / height as f32;
                }

                window.set_title(&format!("Software Renderer Windowed {}x{}", width, height));
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                let delta = self.timer.elapsed();
                self.timer = Instant::now();
                let (width, height) = {
                    let size = window.inner_size();
                    (size.width, size.height)
                };

                self.user_state.handle_event(
                    &mut self.command,
                    SoftRastEvent::Update {
                        delta,
                        input: self.input.clone(),
                    },
                );
                self.input.reset_mouse_motion();

                if let Some(target) = &mut self.render_target {
                    target.clear();

                    if let Some(scene) = &mut self.scene {
                        self.user_state.handle_event(
                            &mut self.command,
                            SoftRastEvent::Render { delta, scene },
                        );
                        let camera = &scene.camera;
                        for entity in &scene.entities {
                            let transform = &entity.position;
                            for vertices in entity.model.vertices.chunks_exact(3) {
                                draw_buffer(target, transform, camera, vertices,self.draw_wireframe,self.draw_points,self.draw_tris);
                            }
                        }
                    } else {
                        self.scene = Some(Scene {
                            entities: vec![],
                            camera: Camera::default(),
                        });
                    }
                    if let Ok(mut buffer) = surface.buffer_mut() {
                        buffer.copy_from_slice(target.color.as_slice());
                        if let Err(err) = buffer.present() {
                            eprintln!("{}", err);
                        }
                    }
                }

                window.set_title(&format!(
                    "Software Renderer Windowed {}x{} @ {:?}",
                    width,
                    height,
                    1.0 / delta.as_secs_f32()
                ));
                window.request_redraw();
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if event.state.is_pressed() {
                    match event.logical_key {
                        Key::Named(name) => match name {
                            NamedKey::Escape => {
                                event_loop.exit();
                            }
                            _ => {}
                        },
                        Key::Character(ch) => {
                            self.input.pressed_keys.insert(ch.to_string());
                        }
                        _ => {}
                    }
                } else {
                    match event.logical_key {
                        Key::Character(ch) => {
                            self.input.pressed_keys.remove(ch.as_str());
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => {
                self.input.mouse_dx = delta.0;
                self.input.mouse_dy = delta.1;
            }
            _ => {}
        }
    }
}

pub fn run() {
    match EventLoop::new() {
        Ok(event_loop) => match event_loop.run_app(&mut AppContext::new(MyApp::default())) {
            Ok(_) => {}
            Err(err) => {
                eprintln!("{}", err);
            }
        },
        Err(e) => {
            panic!("{}", e);
        }
    };
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }
    pub fn interpolate(&self, b: &Color, c: &Color, weights: &Vector3<f32>) -> Color {
        Color {
            r: self.r * weights.x + b.r * weights.y + c.r * weights.z,
            g: self.g * weights.x + b.g * weights.y + c.g * weights.z,
            b: self.b * weights.x + b.b * weights.y + c.b * weights.z,
            a: 1.0,
        }
    }
    pub fn as_u32(&self) -> u32 {
        let red = (self.r * 255.0) as u32;
        let green = (self.g * 255.0) as u32;
        let blue = (self.b * 255.0) as u32;
        blue | (green << 8) | (red << 16)
    }
}

pub fn random_color(rng: &mut XorShiftRng) -> Color {
    Color::new(
        rng.random_range(0.0..=1.0),
        rng.random_range(0.0..=1.0),
        rng.random_range(0.0..=1.0),
        1.0,
    )
}


#[inline(always)]
fn draw_buffer(
    target: &mut RenderTarget,
    transform: &Isometry3<f32>,
    camera: &Camera,
    vertices: &[Vertex],
    wireframe: bool,
    points: bool,
    shaded: bool,
) {
    let mut screen_vertices = Vec::with_capacity(3);

    for vertex in vertices {
        let mvp_mat =
            camera.get_perspective_matrix() * camera.get_view_matrix() * transform.to_homogeneous();
        let clip_v = mvp_mat * vertex.position.to_homogeneous();
        if clip_v.z < camera.near || clip_v.z > camera.far {continue}
        let ndc_v = if clip_v.w != 0.0 {
            clip_v / clip_v.w
        } else {
            clip_v
        };
        let screen_x = (ndc_v.x + 1.0) * 0.5 * target.width as f32;
        let screen_y = (1.0 - ndc_v.y) * 0.5 * target.height as f32;
        let screen_z = ndc_v.z;
        if screen_x > target.width as f32 || screen_y > target.height as f32 || screen_x < 0.0 || screen_y < 0.0
        {
            continue;
        }
        let mut sv = vertex.clone();
        sv.position = Point3::new(screen_x, screen_y, screen_z);
        screen_vertices.push(sv);
    }

    for triangle in screen_vertices.chunks_exact(3) {
        if shaded {
            draw_triangle(target, triangle);
        }
        if wireframe {
            let color = Color::new(1.0,1.0,1.0,1.0).as_u32();
            draw_line(target, &triangle[0].position.xy(), &triangle[1].position.xy(), color );
            draw_line(target, &triangle[1].position.xy(), &triangle[2].position.xy(), color);
            draw_line(target, &triangle[2].position.xy(), &triangle[0].position.xy(), color);
        }
        if points {
            let size = 2.0;
            let color = Color::new(0.5,0.5,0.5,1.0).as_u32();
            draw_point(target,&triangle[0],size,color);
            draw_point(target,&triangle[1],size,color);
            draw_point(target,&triangle[2],size,color);
        }
    }
}
fn draw_triangle(target: &mut RenderTarget, triangle: &[Vertex]) {
    let bounds = Bounds::new(&triangle);
    for x in bounds.x_range() {
        for y in bounds.y_range() {
            let (in_triangle, weights) = point_in_triangle(
                &triangle[0].position.xy(),
                &triangle[1].position.xy(),
                &triangle[2].position.xy(),
                &Point2::new(x as f32, y as f32),
            );
            if in_triangle {
                let depths = Vector3::new(
                    if triangle[0].position.z.abs() > 1e-6 {
                        1.0 / triangle[0].position.z
                    } else {
                        0.0
                    },
                    if triangle[1].position.z.abs() > 1e-6 {
                        1.0 / triangle[1].position.z
                    } else {
                        0.0
                    },
                    if triangle[2].position.z.abs() > 1e-6 {
                        1.0 / triangle[2].position.z
                    } else {
                        0.0
                    },
                );
                let depth = if depths.dot(&weights).abs() > 1e-6 {
                    1.0 / depths.dot(&weights)
                } else {
                    0.0
                };
                let idx = (y * target.width + x) as usize;
                if idx < (target.width * target.height) as usize {
                    if depth > target.depth[idx] {
                        continue;
                    }
                    let color = match (triangle[0].color, triangle[1].color, triangle[2].color) {
                        (Some(c1), Some(c2), Some(c3)) => c1.interpolate(&c2, &c3, &weights),
                        _ => Color::new(1.0, 1.0, 1.0, 1.0),
                    };
                    target.color[idx] = color.as_u32();
                    target.depth[idx] = depth;
                }
            }
        }
    }
}

fn draw_line(target: &mut RenderTarget, p1: &Point2<f32>, p2: &Point2<f32>, color: u32) {
    let color_buffer = target.color.as_mut_slice();

    let screen_size = &Vector2::new(target.width as f32, target.height as f32);

    let x0 = p1.x as i32;
    let y0 = p1.y as i32;
    let x1 = p2.x as i32;
    let y1 = p2.y as i32;
    let width = screen_size.x as usize;
    let height = screen_size.y as usize;

    let x0 = x0.clamp(0, width as i32 - 1);
    let y0 = y0.clamp(0, height as i32 - 1);
    let x1 = x1.clamp(0, width as i32 - 1);
    let y1 = y1.clamp(0, height as i32 - 1);

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        // Set pixel at (x, y) if within bounds
        if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
            let index = y as usize * width + x as usize;
            if index < color_buffer.len() {
                color_buffer[index] = color;
            }
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
fn draw_point(target: &mut RenderTarget, point: &Vertex, size: f32, color: u32) {
    for x in (point.position.x - size.ceil()) as u32..(point.position.x + size.ceil()) as u32 {
        for y in (point.position.y - size.ceil()) as u32..(point.position.y + size.ceil()) as u32 {
            let idx = (y * target.width + x) as usize;
            if idx < (target.width * target.height) as usize {
                let test_pos = Point2::new(x as f32, y as f32);
                let pos = point.position.xy();
                if (test_pos - pos).magnitude() < size {
                    target.color[idx] = color;
                }
            }
        }
    }
}

struct Entity {
    id: String,
    model: Model,
    position: Isometry3<f32>,
}
impl Entity {
    pub fn new(id: &str, model: &Model, position: &Isometry3<f32>) -> Self {
        Self {
            id: id.to_string(),
            model: model.to_owned(),
            position: position.clone(),
        }
    }
}

pub struct Scene {
    entities: Vec<Entity>,
    camera: Camera,
}

#[cfg(test)]
mod tests {
    //  use super::*;
}
