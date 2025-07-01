use crate::geometry::{Model, Texture, load_model};
use crate::{Camera, Command, Entity, Material, SoftRastEvent, UserState};
use nalgebra::{Isometry3, Point3, Scale3, Vector3};

pub struct MyApp {
    pub models: Vec<Model>,
    pub cam: Camera,
}
impl Default for MyApp {
    fn default() -> Self {
        Self {
            models: vec![],
            cam: Camera::new(
                Point3::origin(),
                Point3::origin(),
                Vector3::new(0.0, 1.0, 0.0),
                75.0,
                16. / 9.,
                0.01,
                100.0,
            ),
        }
    }
}
impl UserState for MyApp {
    fn handle_event(&mut self, command: &mut Command, event: SoftRastEvent) {
        let time = command.elapsed().as_secs_f32();
        match event {
            SoftRastEvent::Render { scene, .. } => {
                let transform = Isometry3::new(
                    Vector3::new(-2.0, -1.0, 0.0),
                    Vector3::new(0.0, time / 2.0, 0.0),
                );
                let transform2 = Isometry3::new(
                    Vector3::new(2.0, 1.0, 0.0),
                    Vector3::new(0.0, -time / 1.5, 0.0),
                );

                if scene.entities.is_empty() {
                    scene.camera.position = Point3::new(0.0, 0.0, -10.0);
                    self.cam = scene.camera;
                    let mut models = self.models.iter();
                    if let Some(model) = models.next() {
                        scene.entities.push(Entity::new(
                            "spyro",
                            model,
                            &transform,
                            &Scale3::new(0.05, 0.05, 0.05),
                            Material::LitTexture {
                                texture: Texture::new("assets/SpyroTex.png").unwrap(),
                                light_dir: Vector3::<f32>::new(1.0, 1.0, 0.0).normalize(),
                            },
                        ));
                    }
                    if let Some(model) = models.next() {
                        scene.entities.push(Entity::new(
                            "floor",
                            model,
                            &transform,
                            &Scale3::new(1.0, 1.0, 1.0),
                            Material::LitTexture {
                                texture: Texture::new("assets/Grass.png").unwrap(),
                                light_dir: Vector3::<f32>::new(1.0, 1.0, 0.0).normalize(),
                            },
                        ))
                    }
                    // if let Some(model) = models.next() {
                    //     scene.entities.push(Entity::new(
                    //         "eevee",
                    //         model,
                    //         &transform,
                    //         &Scale3::identity(),
                    //     ));
                    // }
                } else {
                    scene.camera = self.cam;
                    if let Some(entity) = scene
                        .entities
                        .iter_mut()
                        .filter(|e| e.id == "eevee".to_string())
                        .next()
                    {
                        entity.position = transform;
                    }
                    if let Some(entity) = scene
                        .entities
                        .iter_mut()
                        .filter(|e| e.id == "spyro".to_string())
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
                if input.pressed_keys.contains("1") {
                    command.set_render_mode(false, false, true);
                }
                if input.pressed_keys.contains("2") {
                    command.set_render_mode(false, true, false);
                }
                if input.pressed_keys.contains("3") {
                    command.set_render_mode(false, true, true);
                }
                if input.pressed_keys.contains("4") {
                    command.set_render_mode(true, false, false);
                }
                if input.pressed_keys.contains("5") {
                    command.set_render_mode(true, false, true);
                }
                if input.pressed_keys.contains("6") {
                    command.set_render_mode(true, true, false);
                }
                if input.pressed_keys.contains("7") {
                    command.set_render_mode(true, true, true);
                }
            }
            SoftRastEvent::Resume {} => {
                self.models.push(load_model("assets/spyro.obj"));

                self.models.push(load_model("assets/floor.obj"));
            }
        }
    }
}
