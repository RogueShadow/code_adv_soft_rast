use crate::renderer::{random_color, Color};
use image::{DynamicImage, GenericImageView, Rgba};
use nalgebra::{Isometry3, Matrix4, Point2, Point3, Point4, Vector2, Vector3};
use std::fs::read_to_string;
use std::ops::RangeInclusive;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

#[derive(Debug, Clone)]
pub struct Texture {
    pub texture: DynamicImage,
}
impl Texture {
    pub fn new(path: &str) -> Option<Texture> {
        match image::open(path) {
            Ok(image) => Some(Texture {
                texture: image.to_owned(),
            }),
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }
}
impl Texture {
    pub fn sample(&self, tex_coord: &Point2<f32>) -> Option<Color> {
        let width = self.texture.width();
        let height = self.texture.height();
        let x = (tex_coord.x.clamp(0.0, 1.0) * (width as f32 - 1.0)).round() as u32;
        let y = ((1.0 - tex_coord.y.clamp(0.0, 1.0)) * (height as f32 - 1.0)).round() as u32;

        if (0..self.texture.width()).contains(&x) && (0..self.texture.height()).contains(&y) {
            let Rgba([r, g, b, a]) = self.texture.get_pixel(x, y);
            Some(Color::from_rgba(r, g, b, a))
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub vertices: Vec<Vertex>,
}
impl Model {
    pub fn from_vertices(vertices: &[Vertex]) -> Model {
        Self {
            vertices: vertices.to_vec(),
        }
    }
}
pub fn load_model(file: &str) -> Model {
    let color = Color::new(1.0, 1.0, 1.0, 1.0);
    let file = match read_to_string(file) {
        Ok(file) => file,
        Err(err) => panic!("{}", err),
    };
    let mut vertice_positions = Vec::new();
    let mut vertice_normals = Vec::new();
    let mut vertice_uvs = Vec::new();

    let mut faces = Vec::new();
    let mut vertices = Vec::new();

    for line in file.lines() {
        if line.starts_with("v ") {
            let numbers = line[1..]
                .trim()
                .split_whitespace()
                .map(|n| n.parse().unwrap())
                .collect::<Vec<f32>>();
            if numbers.len() == 3 {
                vertice_positions.push(Point3::new(numbers[0], numbers[1], numbers[2]));
            }
        }
        if line.starts_with("f ") {
            let numbers = line[1..]
                .trim()
                .split_whitespace()
                .map(|n| {
                    let mut split_line = n.split('/');
                    let position = split_line.next().unwrap().parse::<usize>().unwrap();
                    let uv = split_line.next().unwrap().parse::<usize>().ok();
                    let normal = split_line.next().unwrap().parse::<usize>().ok();
                    (position, uv, normal)
                })
                .collect::<Vec<_>>();
            faces.push(numbers.as_slice().to_owned());
        }
        if line.starts_with("vn ") {
            let numbers = line[2..]
                .trim()
                .split_whitespace()
                .map(|n| n.parse().unwrap())
                .collect::<Vec<f32>>();
            if numbers.len() == 3 {
                vertice_normals.push(Vector3::new(numbers[0], numbers[1], numbers[2]));
            }
        }
        if line.starts_with("vt ") {
            let numbers = line[2..]
                .trim()
                .split_whitespace()
                .map(|n| n.parse().unwrap())
                .collect::<Vec<f32>>();
            if numbers.len() == 2 {
                vertice_uvs.push(Vector2::new(numbers[0], numbers[1]));
            }
        }
    }

    for face in faces {
        match face.len() {
            3 => {
                vertices.push(vertex_from_face(
                    &face[0],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
                vertices.push(vertex_from_face(
                    &face[1],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
                vertices.push(vertex_from_face(
                    &face[2],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
            }
            4 => {
                vertices.push(vertex_from_face(
                    &face[0],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
                vertices.push(vertex_from_face(
                    &face[1],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
                vertices.push(vertex_from_face(
                    &face[2],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));

                vertices.push(vertex_from_face(
                    &face[0],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
                vertices.push(vertex_from_face(
                    &face[2],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
                vertices.push(vertex_from_face(
                    &face[3],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(color),
                ));
            }
            n => eprintln!("Unsupported face {} vertices", n),
        }
    }
    Model::from_vertices(&vertices)
}

#[inline(always)]
pub fn vertex_from_face(
    face: &(usize, Option<usize>, Option<usize>),
    pos: &[Point3<f32>],
    uv: &[Vector2<f32>],
    norm: &[Vector3<f32>],
    color: Option<Color>,
) -> Vertex {
    let mut vertex = Vertex::new(&pos[face.0 - 1]);
    if let Some(index) = face.1 {
        vertex = vertex.with_uv(uv[index - 1])
    }
    if let Some(index) = face.2 {
        vertex = vertex.with_normal(norm[index - 1])
    }
    if let Some(color) = color {
        vertex.color = Some(color);
    }
    vertex
}

#[derive(Debug)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}
impl Bounds {
    pub fn new<T: AsRef<[Vertex]>>(points: T, screen: (u32, u32)) -> Self {
        let points = points.as_ref();
        if points.is_empty() {
            return Self {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            }
        }

        let first = points[0].clone();
        let mut min_x = first.position.x;
        let mut min_y = first.position.y;
        let mut max_x = first.position.x;
        let mut max_y = first.position.y;

        for point in points.iter().skip(1) {
            min_x = min_x.min(point.position.x);
            min_y = min_y.min(point.position.y);
            max_x = max_x.max(point.position.x);
            max_y = max_y.max(point.position.y);
        }
        
        Self {
            min_x: min_x.max(0.0),
            min_y: min_y.max(0.0),
            max_x: max_x.min(screen.0 as f32),
            max_y: max_y.min(screen.1 as f32),
        }
    }
    pub fn x_range(&self) -> RangeInclusive<u32> {
        self.min_x as u32..=self.max_x as u32
    }
    pub fn y_range(&self) -> RangeInclusive<u32> {
        self.min_y as u32..=self.max_y as u32
    }
}
// pub fn point_in_triangle(triangle: &[Vertex], p: &Point2<f32>) -> bool {
//     let edge1 = edge_cross(&triangle[0].position.xy(), &triangle[2].position.xy(), p);
//     let edge2 = edge_cross(&triangle[2].position.xy(), &triangle[1].position.xy(), p);
//     let edge3 = edge_cross(&triangle[1].position.xy(), &triangle[0].position.xy(), p);
//     edge1 >= 0.0 && edge2 >= 0.0 && edge3 >= 0.0
// }
pub fn edge_cross(a: &Point2<f32>, b: &Point2<f32>, p: &Point2<f32>) -> f32 {
    let ab = b - a;
    let ap = p - a;
    ab.x * ap.y - ab.y * ap.x
}
pub fn triangle_barycentric(triangle: &[Vertex], p: &Point2<f32>) -> Vector3<f32> {
    let a = triangle[0].position.xy();
    let b = triangle[1].position.xy();
    let c = triangle[2].position.xy();
    let area_abp = signed_area(&a, &b, p);
    let area_bcp = signed_area(&b, &c, p);
    let area_cap = signed_area(&c, &a, p);

    let inv_area_sum = 1.0 / (area_abp + area_bcp + area_cap);
    let weight_a = area_bcp * inv_area_sum;
    let weight_b = area_cap * inv_area_sum;
    let weight_c = area_abp * inv_area_sum;
    let weights = Vector3::new(weight_a, weight_b, weight_c);

    weights
}

pub fn signed_area(a: &Point2<f32>, b: &Point2<f32>, c: &Point2<f32>) -> f32 {
    let ac = c - a;
    let ab_perp = perpendicular_vector(&(b - a));
    ac.dot(&ab_perp) / 2.0
}

pub fn perpendicular_vector(v: &Vector2<f32>) -> Vector2<f32> {
    Vector2::new(v.y, -v.x)
}

#[derive(Default, Clone, Copy, Debug)]
pub struct Vertex {
    pub position: Point4<f32>,
    pub normal: Option<Vector3<f32>>,
    pub color: Option<Color>,
    pub uv: Option<Vector2<f32>>,
}
#[allow(unused)]
impl Vertex {
    pub fn new(position: &Point3<f32>) -> Self {
        Self {
            position: position.to_homogeneous().into(),
            normal: None,
            color: None,
            uv: None,
        }
    }
    pub fn with_normal(mut self, normal: Vector3<f32>) -> Self {
        self.normal = Some(normal);
        self
    }
    pub fn with_uv(mut self, uv: Vector2<f32>) -> Self {
        self.uv = Some(uv);
        self
    }
    pub fn model_to_view(&self, mv_mat: &Matrix4<f32>) -> Vertex {
        let mut v = self.clone();
        v.position = mv_mat
            .transform_point(&v.position.xyz())
            .to_homogeneous()
            .into();
        v
    }
    pub fn model_to_view_mut(&mut self, mv_mat: &Matrix4<f32>) -> &mut Self {
        self.position = mv_mat.transform_point(&self.position.xyz()).to_homogeneous().into();
        self
    }
    pub fn view_to_clip(&self, v_mat: &Matrix4<f32>) -> Vertex {
        let mut v = self.clone();
        v.position = v_mat
            .transform_point(&v.position.xyz())
            .to_homogeneous()
            .into();
        v
    }
    pub fn view_to_clip_mut(&mut self, v_mat: &Matrix4<f32>) -> &mut Self {
        self.position = v_mat.transform_point(&self.position.xyz()).to_homogeneous().into();
        self
    }
    pub fn world_to_clip(&self, mvp_mat: &Matrix4<f32>) -> Vertex {
        let mut v = self.clone();
        v.position = mvp_mat
            .transform_point(&v.position.xyz())
            .to_homogeneous()
            .into();
        v
    }
    pub fn world_to_clip_mut(&mut self, mvp_mat: &Matrix4<f32>) -> &mut Self {
        self.position = mvp_mat.transform_point(&self.position.xyz()).to_homogeneous().into();
        self
    }
    pub fn clip_to_ndc(&self) -> Vertex {
        let mut v = self.clone();
        let position = if v.position.w != 0.0 {
            v.position / v.position.w
        } else {
            v.position
        };
        v.position = position;
        v
    }
    pub fn clip_to_ndc_mut(&mut self) -> &mut Self {
        self.position = if self.position.w != 0.0 {
            self.position / self.position.w
        } else {
            self.position
        };
        self
    }
    pub fn ndc_to_screen(&self, size: (u32, u32)) -> Vertex {
        let mut v = self.clone();
        v.position.x = (v.position.x + 1.0) * 0.5 * size.0 as f32;
        v.position.y = (1.0 - v.position.y) * 0.5 * size.1 as f32;
        v
    }
    pub fn ndc_to_screen_mut(&mut self, size: (u32, u32)) -> &mut Self {
        self.position.x = (self.position.x + 1.0) * 0.5 * size.0 as f32;
        self.position.y = (1.0 - self.position.y) * 0.5 * size.1 as f32;
        self
    }
    pub fn update_normal(&self, model_mat: &Isometry3<f32>) -> Vertex {
        if let Some(normal) = self.normal {
            let mut v = self.clone();
            v.normal = Some(model_mat.transform_vector(&normal).normalize());
            v
        } else {
            self.clone()
        }
    }
    pub fn update_normal_mut(&mut self, model_mat: &Isometry3<f32>) -> &mut Self {
        if let Some(normal) = self.normal {
            self.normal = Some(model_mat.transform_vector(&normal).normalize());
        }
        self
    }
}
pub fn randomize_model_colors(model: &Model) -> Model {
    let mut model = model.clone();
    let mut rng = XorShiftRng::from_os_rng();
    for vertex in model.vertices.iter_mut() {
        vertex.color = Some(random_color(&mut rng));
    }
    model
}