use crate::geometry::{Model, load_model};
use crate::{Camera, Command, Entity, SoftRastEvent, UserState};
use nalgebra::{Isometry3, Point3, Vector3};

pub struct MyApp {
    pub models: Vec<Model>,
    pub cam: Camera,
}
impl Default for MyApp {
    fn default() -> Self {
        Self {
            models: vec![],
            cam: Camera::new(Point3::origin(),Point3::origin(),Vector3::new(0.0,1.0,0.0),75.0,16./9.,0.01,100.0),
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
                

                if scene.entities.is_empty() {
                    scene.camera.position = Point3::new(0.0, 0.0, -10.0);
                    self.cam = scene.camera;
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
                    scene.camera = self.cam;
                    if let Some(entity) = scene
                        .entities
                        .iter_mut()
                        .filter(|e| e.id == "monkey".to_string())
                        .next()
                    {
                        entity.position = transform;
                    }
                    if let Some(entity) = scene
                        .entities
                        .iter_mut()
                        .filter(|e| e.id == "test".to_string())
                        .next()
                    {
                        entity.position = transform2;
                    }
                }
            }
            SoftRastEvent::Update { delta, input } => {
                let speed = delta.as_secs_f32() * 5.0;
                if input.pressed_keys.contains("a") {
                    self.cam.move_local(0.0, -speed, 0.0);
                }
                if input.pressed_keys.contains("d") {
                    self.cam.move_local(0.0, speed, 0.0);
                }
                if input.pressed_keys.contains("w") {
                    self.cam.move_local(speed, 0.0, 0.0);
                }
                if input.pressed_keys.contains("s") {
                    self.cam.move_local(-speed, 0.0, 0.0);
                }
                if input.pressed_keys.contains("z") {
                    self.cam.move_local(0.0, 0.0, speed);
                }
                if input.pressed_keys.contains("c") {
                    self.cam.move_local(0.0, 0.0, -speed);
                }
                self.cam.look(
                    input.mouse_dx as f32,
                    input.mouse_dy as f32,
                    delta.as_secs_f32(),
                );
                if input.pressed_keys.contains("q") {
                    self.cam.roll(speed);
                }
                if input.pressed_keys.contains("e") {
                    self.cam.roll(-speed);
                }
            }
            SoftRastEvent::Resume {} => {
                self.models.push(load_model("assets/monkey.obj", true));
            }
        }
    }
}
