use crate::geometry::{Model, load_model};
use crate::{Camera, Command, Entity, InputState, SoftRastEvent, UserState};
use nalgebra::{Isometry3, Point3, Unit, UnitQuaternion, Vector3};

pub struct CameraController {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
}
impl CameraController {
    pub fn new() -> CameraController {
        Self {
            eye: Point3::new(0.0, 0.0, 3.0),
            target: Point3::origin(),
        }
    }
    pub fn strafe_left(&mut self, speed: f32) {
        let v = self.target - self.eye;
        let u = Vector3::new(0.0, 1.0, 0.0);
        let r = v.cross(&u).normalize();
        self.eye -= r * speed;
        self.target -= r * speed;
    }

    /// Strafe the camera to the right by the specified speed, moving both eye and target.
    pub fn strafe_right(&mut self, speed: f32) {
        let v = self.target - self.eye;
        let u = Vector3::new(0.0, 1.0, 0.0);
        let r = v.cross(&u).normalize();
        self.eye += r * speed;
        self.target += r * speed;
    }

    /// Move the camera forward toward the target by the specified speed.
    pub fn move_forward(&mut self, speed: f32) {
        let v = (self.target - self.eye).normalize();
        self.eye += v * speed;
    }

    /// Move the camera backward away from the target by the specified speed.
    pub fn move_backward(&mut self, speed: f32) {
        let v = (self.target - self.eye).normalize();
        self.eye -= v * speed;
    }
    pub fn strafe_up(&mut self, speed: f32) {
        let v = (self.target - self.eye).normalize();
        let u = Vector3::new(0.0, 1.0, 0.0);
        let r = v.cross(&u).normalize();
        let local_up = r.cross(&v).normalize();
        self.eye += local_up * speed;
        self.target += local_up * speed;
    }

    /// Strafe the camera downward by the specified speed, moving both eye and target relative to view direction.
    pub fn strafe_down(&mut self, speed: f32) {
        let v = (self.target - self.eye).normalize();
        let u = Vector3::new(0.0, 1.0, 0.0);
        let r = v.cross(&u).normalize();
        let local_up = r.cross(&v).normalize();
        self.eye -= local_up * speed;
        self.target -= local_up * speed;
    }

    /// Orbit the camera around the target based on mouse movement.
    /// - `dx`: Horizontal mouse delta (positive for right movement).
    /// - `dy`: Vertical mouse delta (positive for downward movement).
    /// - `sensitivity`: Scaling factor for rotation speed (e.g., radians per pixel).
    pub fn orbit(&mut self, dx: f32, dy: f32, sensitivity: f32) {
        let v = self.target - self.eye;
        let u = Vector3::new(0.0, 1.0, 0.0);
        let r = v.cross(&u).normalize();
        let rot_vertical =
            UnitQuaternion::from_axis_angle(&Unit::new_normalize(r), -dy * sensitivity);
        let rot_horizontal =
            UnitQuaternion::from_axis_angle(&Unit::new_normalize(u), -dx * sensitivity);
        let total_rot = rot_horizontal * rot_vertical;
        let offset = self.eye - self.target;
        let rotated_offset = total_rot * offset;
        self.eye = self.target + rotated_offset;
    }
    pub fn pan(&mut self, dx: f32, dy: f32, sensitivity: f32) {
        let v = self.target - self.eye;
        let u = Vector3::new(0.0, 1.0, 0.0);
        let r = v.cross(&u).normalize();
        let rot_vertical =
            UnitQuaternion::from_axis_angle(&nalgebra::Unit::new_normalize(r), -dy * sensitivity);
        let rot_horizontal =
            UnitQuaternion::from_axis_angle(&nalgebra::Unit::new_normalize(u), -dx * sensitivity);
        let total_rot = rot_horizontal * rot_vertical;
        let offset = self.target - self.eye;
        let rotated_offset = total_rot * offset;
        self.target = self.eye + rotated_offset;
    }
}

pub struct MyApp {
    pub models: Vec<Model>,
    pub cam: CameraController,
}
impl Default for MyApp {
    fn default() -> Self {
        Self {
            models: vec![],
            cam: CameraController::new(),
        }
    }
}
impl UserState for MyApp {
    fn handle_event(&mut self, command: &mut Command, event: SoftRastEvent) {
        let time = command.elapsed().as_secs_f32();
        match event {
            SoftRastEvent::Render { delta, scene } => {
                let transform = Isometry3::new(
                    Vector3::new(-1.0, 0.0, 0.0),
                    Vector3::new(3.14 + time.sin() * 0.7, 0.0, 0.0),
                );
                let transform2 = Isometry3::new(
                    Vector3::new(1.0, 0.0, -0.0),
                    Vector3::new(0.0, time.sin(), 0.0),
                );

                scene.camera.eye = self.cam.eye;
                scene.camera.target = self.cam.target;

                if scene.entities.is_empty() {
                    scene.camera.eye = Point3::new(0.0, 0.0, 3.0);
                    scene.camera.target = Point3::new(0.0, 0.0, 0.0);
                    scene.camera.fov = 90.0 * std::f32::consts::PI / 180.0;
                    scene.entities.push(Entity::new(
                        "monkey",
                        self.models.first().unwrap(),
                        &transform,
                    ));
                    scene.entities.push(Entity::new(
                        "test",
                        self.models.first().unwrap(),
                        &transform2,
                    ));
                } else {
                    if let Some(entity) = scene
                        .entities
                        .iter_mut()
                        .filter(|e| e.id == "monkey".to_string())
                        .next()
                    {
                        //entity.position = transform;
                    }
                    if let Some(entity) = scene
                        .entities
                        .iter_mut()
                        .filter(|e| e.id == "test".to_string())
                        .next()
                    {
                        //entity.position = transform2;
                    }
                }
            }
            SoftRastEvent::Update { delta, input } => {
                if input.pressed_keys.contains("a") {
                    self.cam.strafe_left(1.0 * delta.as_secs_f32());
                }
                if input.pressed_keys.contains("d") {
                    self.cam.strafe_right(1.0 * delta.as_secs_f32());
                }
                if input.pressed_keys.contains("w") {
                    self.cam.move_forward(1.0 * delta.as_secs_f32());
                }
                if input.pressed_keys.contains("s") {
                    self.cam.move_backward(1.0 * delta.as_secs_f32());
                }
                if input.pressed_keys.contains("q") {
                    self.cam.strafe_up(1.0 * delta.as_secs_f32());
                }
                if input.pressed_keys.contains("z") {
                    self.cam.strafe_down(1.0 * delta.as_secs_f32());
                }
                if input.pressed_keys.contains("e") {
                    self.cam.pan(
                        input.mouse_dx as f32,
                        input.mouse_dy as f32,
                        delta.as_secs_f32(),
                    );                    
                }
            }
            SoftRastEvent::Resume {} => {
                self.models.push(load_model("assets/monkey.obj", true));
            }
        }
    }
}
