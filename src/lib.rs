mod geometry;
mod my_app;

use crate::geometry::{Bounds, Model, Vertex};
use crate::my_app::MyApp;
use geometry::Triangle2d;
use nalgebra::{Isometry3, Matrix4, Point2, Point3, Vector, Vector2, Vector3};
use rand::Rng;
use rand_xorshift::XorShiftRng;
use softbuffer::{Context, Surface};
use std::collections::HashSet;
use std::num::NonZeroU32;
use std::ops::Div;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, KeyCode, NamedKey, SmolStr};
use winit::window::{CursorGrabMode, Window, WindowId};

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
}

trait UserState {
    fn handle_event(&mut self, command: &mut Command, event: SoftRastEvent);
}

struct Command {
    commands: Vec<SoftRastCommand>,
    timer: Instant,
}
impl Command {
    pub fn set_title(&mut self, title: &str) {
        self.commands
            .push(SoftRastCommand::SetTitle(title.to_owned()));
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
struct InputState {
    pressed_keys: HashSet<String>,
    mouse_dx: f64,
    mouse_dy: f64,
}

impl InputState {
    fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            mouse_dx: 0.0,
            mouse_dy: 0.0,
        }
    }

    // Optional: Reset mouse motion deltas at the start of each frame
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
    mouse_last_pos: (f64, f64),
    mouse_pos: (f64, f64),
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
            mouse_last_pos: (0.0, 0.0),
            mouse_pos: (0.0, 0.0),
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
            self.window = Some(Rc::new(
                event_loop
                    .create_window(Window::default_attributes())
                    .unwrap(),
            ));
        }
        self.context = Some(Context::new(self.window.clone().unwrap()).unwrap());
        self.surface = Some(
            Surface::new(self.context.as_ref().unwrap(), self.window.clone().unwrap()).unwrap(),
        );
        let _ = self
            .window
            .as_ref()
            .unwrap()
            .request_inner_size(PhysicalSize::new(WIDTH, HEIGHT));

        self.window.as_ref().unwrap().set_cursor_grab(CursorGrabMode::Confined).expect("TODO: panic message");
        
        self.user_state
            .handle_event(&mut self.command, SoftRastEvent::Resume {});
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let window = self.window.as_ref().unwrap();
        let surface = self.surface.as_mut().unwrap();

        for command in self.command.commands.iter() {
            match command {
                SoftRastCommand::SetTitle(title) => {
                    window.set_title(&title);
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
                surface
                    .resize(
                        NonZeroU32::new(width).unwrap(),
                        NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();
                self.render_target = Some(RenderTarget::new(width, height));
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

                self.input.mouse_dy = 0.0;
                self.input.mouse_dx = 0.0;

                let mut buffer = surface.buffer_mut().unwrap();
                buffer.fill(0u32);
                if let Some(target) = &mut self.render_target {
                    target.clear();

                    if let Some(scene) = &mut self.scene {
                        self.user_state.handle_event(
                            &mut self.command,
                            SoftRastEvent::Render { delta, scene },
                        );

                        for entity in &scene.entities {
                            let transform = &entity.position;
                            for (vertices) in entity.model.vertices.chunks_exact(3) {
                                let color = vertices[0]
                                    .color
                                    .unwrap_or(Color::new(1.0, 1.0, 1.0, 1.0).as_u32());
                                let mut verts = vertices.to_owned();
                                draw_triangle_buffer(
                                    target,
                                    transform,
                                    verts.as_mut(),
                                    &scene.camera,
                                    color,
                                )
                            }
                        }
                    } else {
                        self.scene = Some(Scene {
                            entities: vec![],
                            camera: Camera::default(),
                        });
                    }
                    buffer.copy_from_slice(target.color.as_slice());
                }

                buffer.present().unwrap();

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
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_last_pos = self.mouse_pos;
                let (dx, dy) = (
                    position.x - self.mouse_last_pos.0,
                    position.y - self.mouse_last_pos.1,
                );
                self.mouse_pos = (position.x, position.y);
                self.input.mouse_dx = dx;
                self.input.mouse_dy = dy;
            }
            _ => {

            }
        }
    }
}

pub fn run() {
    EventLoop::new()
        .unwrap()
        .run_app(&mut AppContext::new(MyApp::default()))
        .unwrap();
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
fn draw_triangle_buffer(
    target: &mut RenderTarget,
    transform: &Isometry3<f32>,
    vertices: &mut [Vertex],
    camera: &Camera,
    color: u32,
) {
    let color_buffer = target.color.as_mut_slice();
    let depth_buffer = target.depth.as_mut_slice();

    let screen_size = &Vector2::new(target.width as f32, target.height as f32);

    let model_mat = transform.to_homogeneous();
    let view_mat = camera.get_view_matrix();
    let proj_mat = camera.get_perspective_matrix();
    let mvp_mat = proj_mat * view_mat * model_mat;

    let mut screen_vertices = Vec::with_capacity(vertices.len());
    for v in vertices.iter_mut() {
        let clip_v = mvp_mat * v.position.to_homogeneous();
        if clip_v.z < camera.near {return}
        let ndc_v = if clip_v.w != 0.0 {
            clip_v / clip_v.w
        } else {
            clip_v
        };
        let screen_x = (ndc_v.x + 1.0) * 0.5 * screen_size.x;
        let screen_y = (1.0 - ndc_v.y) * 0.5 * screen_size.y; // Flip Y for screen coords
        let screen_z = ndc_v.z; // Keep Z for potential depth testing
        screen_vertices.push(Point3::new(screen_x, screen_y, screen_z));
    }

    for v in screen_vertices.iter() {
        if !(0.0..WIDTH).contains(&v.x) || !(0.0..HEIGHT).contains(&v.y) {
            return;
        }
    }

    let triangle2d = Triangle2d::new(
        screen_vertices[0].xy(),
        screen_vertices[1].xy(),
        screen_vertices[2].xy(),
    );

    let bounds = triangle2d.bounds();

    for x in bounds.x_range() {
        for y in bounds.y_range() {
            let (in_triangle, weights) = triangle2d.contains(&Point2::new(x as f32, y as f32));
            if in_triangle {
                let depths = Vector3::new(
                    if screen_vertices[0].z.abs() > 1e-6 { 1.0 / screen_vertices[0].z } else { 0.0 },
                    if screen_vertices[1].z.abs() > 1e-6 { 1.0 / screen_vertices[1].z } else { 0.0 },
                    if screen_vertices[2].z.abs() > 1e-6 { 1.0 / screen_vertices[2].z } else { 0.0 },
                );
                let depth = if depths.dot(&weights).abs() > 1e-6 { 1.0 / depths.dot(&weights) } else { 0.0 };
                let idx = (y * screen_size.x as u32 + x) as usize;
                if idx < (screen_size.x * screen_size.y) as usize {
                    if depth > depth_buffer[idx] {
                        continue;
                    }
                    color_buffer[idx] = color;
                    depth_buffer[idx] = depth;
                }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Camera {
    pub fov: f32,
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}
impl Camera {
    pub fn new(fov: f32, aspect_ratio: f32) -> Self {
        Self {
            fov: fov * (std::f32::consts::PI / 180.),
            aspect_ratio,
            ..Default::default()
        }
    }
    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(&self.eye, &self.target, &self.up)
    }
    pub fn get_perspective_matrix(&self) -> Matrix4<f32> {
        Matrix4::new_perspective(self.aspect_ratio, self.fov, self.near, self.far)
    }
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            fov: 60.0 * (std::f32::consts::PI / 180.),
            eye: Point3::origin(),
            target: Point3::origin() + Vector3::new(0., 0., 0.),
            up: Vector3::y(),
            aspect_ratio: 16.0 / 9.0,
            near: 0.1,
            far: 100.0,
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
