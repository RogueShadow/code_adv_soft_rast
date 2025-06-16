use crate::{Color, random_color};
use nalgebra::{Matrix3, Matrix4, OMatrix, Point2, Point3, Vector2, Vector3};
use rand::{rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::fs::read_to_string;
use std::ops::RangeInclusive;

#[derive(Debug, Clone)]
pub struct Model {
    pub vertices: Vec<Vertex>,
}
impl Model {
    pub fn new() -> Self {
        Self::from_vertices(&[])
    }
    pub fn from_vertices(vertices: &[Vertex]) -> Model {
        Self {
            vertices: vertices.to_vec(),
        }
    }
}
pub fn load_model(file: &str, random_colors: bool) -> Model {
    let file = read_to_string(file).unwrap();
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
                    let position =split_line.next().unwrap().parse::<usize>().unwrap();
                    let uv =split_line.next().unwrap().parse::<usize>().ok();
                    let normal = split_line.next().unwrap().parse::<usize>().ok();
                    (position,uv,normal)
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
        let color = if random_colors {
            Some(random_color(&mut rng).as_u32())
        } else {
            None
        };
        match face.len() {
            3 => {
                vertices.push(vertex_from_face(&face[0],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                vertices.push(vertex_from_face(&face[1],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                vertices.push(vertex_from_face(&face[2],&vertice_positions,&vertice_uvs,&vertice_normals,color));
            }
            4 => {
                vertices.push(vertex_from_face(&face[0],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                vertices.push(vertex_from_face(&face[1],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                vertices.push(vertex_from_face(&face[2],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                
                vertices.push(vertex_from_face(&face[0],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                vertices.push(vertex_from_face(&face[2],&vertice_positions,&vertice_uvs,&vertice_normals,color));
                vertices.push(vertex_from_face(&face[3],&vertice_positions,&vertice_uvs,&vertice_normals,color));
            }
            n => eprintln!("Unsupported face {} vertices", n),
        }
    }
    Model::from_vertices(&vertices)
}

#[inline(always)]
pub fn vertex_from_face(face: &(usize,Option<usize>,Option<usize>), pos: &[Point3<f32>], uv: &[Vector2<f32>], norm: &[Vector3<f32>], color: Option<u32>) -> Vertex {
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
    pub fn from_triangle(triangle: &Triangle2d) -> Bounds {
        Bounds {
            min_x: triangle.a.x.min(triangle.b.x.min(triangle.c.x)),
            min_y: triangle.a.y.min(triangle.b.y.min(triangle.c.y)),
            max_x: triangle.a.x.max(triangle.b.x.max(triangle.c.x)),
            max_y: triangle.a.y.max(triangle.b.y.max(triangle.c.y)),
        }
    }
    pub fn from_vertices(vertices: &[Vertex]) -> Bounds {
        Bounds {
            min_x: vertices[0].position.x.min(vertices[1].position.x.min(vertices[2].position.x)),
            min_y: vertices[0].position.y.min(vertices[1].position.y.min(vertices[2].position.y)),
            max_x: vertices[0].position.x.max(vertices[1].position.x.max(vertices[2].position.x)),
            max_y: vertices[0].position.y.max(vertices[1].position.y.max(vertices[2].position.y)),
        }
    }
    pub fn x_range(&self) -> RangeInclusive<u32> {
        self.min_x as u32..=self.max_x as u32
    }
    pub fn y_range(&self) -> RangeInclusive<u32> {
        self.min_y as u32..=self.max_y as u32
    }
}

pub struct Triangle2d {
    pub a: Point2<f32>,
    pub b: Point2<f32>,
    pub c: Point2<f32>,
}
impl Triangle2d {
    pub fn new(a: Point2<f32>, b: Point2<f32>, c: Point2<f32>) -> Self {
        Self { a, b, c }
    }
    pub fn contains(&self, p: &Point2<f32>) -> (bool, Vector3<f32>) {
        let area_abp = signed_area(&self.a, &self.b, p);
        let area_bcp = signed_area(&self.b, &self.c, p);
        let area_cap = signed_area(&self.c, &self.a, p);
        let in_triangle = area_abp >= 0.0 && area_bcp >= 0.0 && area_cap >= 0.0;

        let inv_area_sum = 1.0 / (area_abp + area_bcp + area_cap);
        let weight_a = area_bcp * inv_area_sum;
        let weight_b = area_cap * inv_area_sum;
        let weight_c = area_abp * inv_area_sum;
        let weights = Vector3::new(weight_a, weight_b, weight_c);

        (in_triangle, weights)
    }
    pub fn bounds(&self) -> Bounds {
        Bounds::from_triangle(&self)
    }
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

#[derive(Default,Debug,Copy,Clone)]
pub struct Vertex {
    pub position: Point3<f32>,
    pub normal: Option<Vector3<f32>>,
    pub color: Option<u32>,
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
    pub fn apply_matrix(&self, mat: &Matrix4<f32>) -> Self {
        let mut vert = self.clone();
        vert.position = mat.transform_point(&self.position);
        vert
    }
}