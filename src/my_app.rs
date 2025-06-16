use nalgebra::{Isometry3, Point3, Rotation3, UnitQuaternion, Vector3};
use crate::geometry::{Model, load_model};
use crate::{Command, SoftRastEvent, UserState, Entity, Camera};

pub struct MyApp {
    pub models: Vec<Model>,
}
impl Default for MyApp {
    fn default() -> Self {
        Self {
            models: vec![],
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
                    Vector3::new(3.14 + time.sin() * 0.7,0.0,0.0)
                );
                let transform2 = Isometry3::new(
                    Vector3::new(1.0,0.0,-0.0),
                    Vector3::new(0.0,time.sin(),0.0)
                );
                
                let eye = Point3::new(0.0 + (time / 4.0).cos() * 4.0, 0.0, 0.0 + (time / 4.0).sin() * 4.0);
                scene.camera.eye = eye;
                if scene.entities.is_empty() {
                    scene.camera.eye = Point3::new(0.0, 0.0, 3.0);
                    scene.camera.target = Point3::new(0.0, 0.0, 0.0);
                    scene.camera.fov = 90.0 * std::f32::consts::PI / 180.0;
                    scene.entities.push(Entity::new("monkey",self.models.first().unwrap(),&transform));
                    scene.entities.push(Entity::new("test",self.models.first().unwrap(),&transform2));
                } else {
                    if let Some(entity) = scene.entities.iter_mut().filter(|e| e.id == "monkey".to_string()).next() {
                        entity.position = transform;
                    }
                    if let Some(entity) = scene.entities.iter_mut().filter(|e| e.id == "test".to_string()).next() {
                        entity.position = transform2;
                    }
                }

            }
            SoftRastEvent::Update { .. } => {}
            SoftRastEvent::Resume {} => {
                self.models.push(load_model("assets/monkey.obj", true));
            }
        }
    }
}
