use crate::{Color, random_color};
use nalgebra::{Point2, Point3, Vector2, Vector3};
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use std::fs::read_to_string;
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
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
pub fn load_model(file: &str, _random_colors: bool) -> Model {
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

    let mut rng = XorShiftRng::seed_from_u64(0);
    for face in faces {
        match face.len() {
            3 => {
                vertices.push(vertex_from_face(
                    &face[0],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
                vertices.push(vertex_from_face(
                    &face[1],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
                vertices.push(vertex_from_face(
                    &face[2],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
            }
            4 => {
                vertices.push(vertex_from_face(
                    &face[0],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
                vertices.push(vertex_from_face(
                    &face[1],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
                vertices.push(vertex_from_face(
                    &face[2],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));

                vertices.push(vertex_from_face(
                    &face[0],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
                vertices.push(vertex_from_face(
                    &face[2],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
                ));
                vertices.push(vertex_from_face(
                    &face[3],
                    &vertice_positions,
                    &vertice_uvs,
                    &vertice_normals,
                    Some(random_color(&mut rng)),
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
    let mut vertex = Vertex::new(pos[face.0 - 1]);
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
    pub fn new<T: AsRef<[Vertex]>>(points: T) -> Self {
        let points = points.as_ref();
        if points.is_empty() {
            return Self {
                min_x: 0.0,
                min_y: 0.0,
                max_x: 0.0,
                max_y: 0.0,
            };
        }

        let first = points[0];
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
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
    pub fn x_range(&self) -> RangeInclusive<u32> {
        self.min_x as u32..=self.max_x as u32
    }
    pub fn y_range(&self) -> RangeInclusive<u32> {
        self.min_y as u32..=self.max_y as u32
    }
}

pub fn point_in_triangle(
    a: &Point2<f32>,
    b: &Point2<f32>,
    c: &Point2<f32>,
    p: &Point2<f32>,
) -> (bool, Vector3<f32>) {
    let area_abp = signed_area(&a, &b, p);
    let area_bcp = signed_area(&b, &c, p);
    let area_cap = signed_area(&c, &a, p);
    let in_triangle = area_abp >= 0.0 && area_bcp >= 0.0 && area_cap >= 0.0;

    let inv_area_sum = 1.0 / (area_abp + area_bcp + area_cap);
    let weight_a = area_bcp * inv_area_sum;
    let weight_b = area_cap * inv_area_sum;
    let weight_c = area_abp * inv_area_sum;
    let weights = Vector3::new(weight_a, weight_b, weight_c);

    (in_triangle, weights)
}
pub fn signed_area(a: &Point2<f32>, b: &Point2<f32>, c: &Point2<f32>) -> f32 {
    let ac = c - a;
    let ab_perp = perpendicular_vector(&(b - a));
    ac.dot(&ab_perp) / 2.0
}

#[inline(always)]
pub fn perpendicular_vector(v: &Vector2<f32>) -> Vector2<f32> {
    Vector2::new(v.y, -v.x)
}

#[derive(Default, Debug, Copy, Clone)]
pub struct Vertex {
    pub position: Point3<f32>,
    pub normal: Option<Vector3<f32>>,
    pub color: Option<Color>,
    pub uv: Option<Vector2<f32>>,
}
impl Vertex {
    pub fn new(position: Point3<f32>) -> Self {
        Self {
            position,
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
}
