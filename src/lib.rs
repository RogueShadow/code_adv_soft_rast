mod camera;
mod geometry;
mod my_app;
mod renderer;

use crate::camera::Camera;
use crate::geometry::Model;
use crate::my_app::MyApp;
use crate::renderer::{draw_buffer, DrawMode, RenderTarget};
use nalgebra::{Isometry3, Scale3};
use rand::Rng;
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
        shaded: bool,
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
    pub fn set_render_mode(&mut self, shaded: bool, wireframe: bool, points: bool) {
        self.commands.push(SoftRastCommand::SetRenderingMode {
            shaded,
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
    draw_mode: DrawMode,
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
            draw_mode: DrawMode {shaded: true, wireframe: false, points: false}
        }
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
                    shaded,
                    wireframe,
                    points,
                } => {
                    self.draw_mode = DrawMode {
                        shaded: *shaded,
                        wireframe: *wireframe,
                        points: *points,
                    }
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
                                draw_buffer(target, &entity.position,&entity.scale, camera, &entity.model,&self.draw_mode);
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


struct Entity {
    id: String,
    model: Model,
    position: Isometry3<f32>,
    scale: Scale3<f32>,
}
impl Entity {
    pub fn new(id: &str, model: &Model, position: &Isometry3<f32>, scale: &Scale3<f32>) -> Self {
        Self {
            id: id.to_string(),
            model: model.to_owned(),
            position: position.clone(),
            scale: scale.to_owned(),
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
