use nalgebra::{Matrix4, Point3, Unit, UnitQuaternion, Vector3};

#[derive(Debug, Copy, Clone)]
pub(crate) struct Camera {
    pub position: Point3<f32>,
    pub orientation: UnitQuaternion<f32>,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}
#[allow(unused)]
impl Camera {
    pub fn new(
        position: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32,
    ) -> Self {
        let direction = (target - position).normalize();
        let orientation = UnitQuaternion::face_towards(&direction, &up);
        Self {
            position,
            orientation,
            fov,
            aspect_ratio,
            near,
            far,
        }
    }
    pub fn forward(&self) -> Vector3<f32> {
        self.orientation * Vector3::new(0.0, 0.0, -1.0)
    }
    pub fn right(&self) -> Vector3<f32> {
        self.orientation * Vector3::new(1.0, 0.0, 0.0)
    }
    pub fn up(&self) -> Vector3<f32> {
        self.orientation * Vector3::new(0.0, 1.0, 0.0)
    }
    pub fn move_world(&mut self, displacement: Vector3<f32>) {
        self.position += displacement;
    }
    pub fn move_local(&mut self, forward: f32, right: f32, up: f32) {
        let forward_vector = self.forward() * forward;
        let right_vector = self.right() * right;
        let up_vector = self.up() * up;
        self.position += forward_vector + right_vector + up_vector;
    }
    pub fn roll(&mut self, roll: f32) {
        let forward = self.forward();
        let roll_rot = UnitQuaternion::from_axis_angle(&Unit::new_normalize(forward), roll);
        self.orientation = roll_rot * self.orientation;
    }
    pub fn look(&mut self, delta_x: f32, delta_y: f32, sensitivity: f32) {
        let yaw = -delta_x * sensitivity;
        let pitch = -delta_y * sensitivity;

        let yaw_rot =
            UnitQuaternion::from_axis_angle(&Unit::new_normalize(Vector3::new(0.0, 1.0, 0.0)), yaw);
        let pitch_rot = UnitQuaternion::from_axis_angle(
            &Unit::new_normalize(Vector3::new(1.0, 0.0, 0.0)),
            pitch,
        );

        self.orientation = self.orientation * pitch_rot * yaw_rot;
    }
    pub fn get_view_matrix(&self) -> Matrix4<f32> {
        let rotation_matrix = self.orientation.to_rotation_matrix();
        let translation = Matrix4::new_translation(&(-self.position.coords));
        rotation_matrix.to_homogeneous().try_inverse().unwrap() * translation
    }
    pub fn get_perspective_matrix(&self) -> Matrix4<f32> {
        Matrix4::new_perspective(self.aspect_ratio, self.fov, self.near, self.far)
    }
}
impl Default for Camera {
    fn default() -> Self {
        Self::new(
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 1.0, 0.0),
            70.0,
            16.0 / 9.0,
            0.01,
            100.0,
        )
    }
}
