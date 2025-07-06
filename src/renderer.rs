use crate::Entity;
use crate::camera::Camera;
use crate::geometry::{Bounds, Texture, Vertex, point_in_triangle, triangle_barycentric};
use nalgebra::{Point2, Vector3};
use rand::Rng;
use rand_xorshift::XorShiftRng;
use rayon::prelude::*;
use std::ops::Mul;

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub fn mul(&self, other: &Self) -> Self {
        Self {
            r: self.r * other.r,
            g: self.g * other.g,
            b: self.b * other.b,
            a: self.a * other.a,
        }
    }
    pub fn from_rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }
    pub fn interpolate(&self, b: &Color, c: &Color, weights: &Vector3<f32>) -> Self {
        Color {
            r: self.r * weights.x + b.r * weights.y + c.r * weights.z,
            g: self.g * weights.x + b.g * weights.y + c.g * weights.z,
            b: self.b * weights.x + b.b * weights.y + c.b * weights.z,
            a: 1.0,
        }
    }
    pub fn as_u32(&self) -> u32 {
        let red = (self.r * 255.0) as u32;
        let green = (self.g * 255.0) as u32;
        let blue = (self.b * 255.0) as u32;
        blue | (green << 8) | (red << 16)
    }
}

impl Mul<f32> for Color {
    type Output = Color;

    fn mul(self, rhs: f32) -> Self::Output {
        Self {
            r: self.r * rhs,
            g: self.g * rhs,
            b: self.b * rhs,
            a: self.a,
        }
    }
}

#[allow(unused)]
pub fn random_color(rng: &mut XorShiftRng) -> Color {
    Color::new(
        rng.random_range(0.0..=1.0),
        rng.random_range(0.0..=1.0),
        rng.random_range(0.0..=1.0),
        1.0,
    )
}
#[derive(Copy, Clone)]
pub struct DrawMode {
    pub(crate) wireframe: bool,
    pub(crate) shaded: bool,
    pub(crate) points: bool,
}
impl Default for DrawMode {
    fn default() -> Self {
        Self {
            wireframe: false,
            shaded: true,
            points: false,
        }
    }
}

pub struct RenderTarget {
    pub(crate) color: Vec<u32>,
    depth: Vec<f32>,
    width: u32,
    height: u32,
    clear_color: u32,
    vertex_buffer: Vec<Vertex>,
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            color: vec![u32::MIN; (width * height) as usize],
            depth: vec![f32::MAX; (width * height) as usize],
            width,
            height,
            clear_color: u32::MIN,
            vertex_buffer: vec![],
        }
    }
    pub fn clear(&mut self) {
        self.color.fill(self.clear_color);
        self.depth.fill(f32::MAX);
    }

    pub fn create_slices(&mut self) -> Vec<RenderSlice> {
        let num_threads = rayon::current_num_threads();
        let rows_per_thread = (self.height as usize + num_threads - 1) / num_threads; // Ceiling division
        let mut slices = Vec::with_capacity(num_threads);
        let mut remaining_color = &mut self.color[..];
        let mut remaining_depth = &mut self.depth[..];

        for i in 0..num_threads {
            let y_start = i * rows_per_thread;
            let y_end = (y_start + rows_per_thread).min(self.height as usize);
            if y_start >= self.height as usize {
                break; // Avoid empty slices for the last thread if height is small
            }
            let start_idx = y_start * self.width as usize;
            let end_idx = y_end * self.width as usize;
            let (color_slice, next_color) = remaining_color.split_at_mut(end_idx - start_idx);
            let (depth_slice, next_depth) = remaining_depth.split_at_mut(end_idx - start_idx);
            remaining_color = next_color;
            remaining_depth = next_depth;
            slices.push(RenderSlice {
                color_slice,
                depth_slice,
                start: y_start as u32,
                end: y_end as u32,
                width: self.width,
                height: self.height,
            });
        }
        slices
    }
}
fn calculate_uvs(triangle: &[Vertex], weights: &Vector3<f32>) -> Option<Point2<f32>> {
    let uv0 = triangle[0].uv?;
    let uv1 = triangle[1].uv?;
    let uv2 = triangle[2].uv?;
    let mut uv = Point2::<f32>::origin();
    uv += uv0 * weights.x;
    uv += uv1 * weights.y;
    uv += uv2 * weights.z;
    Some(uv)
}
fn calculate_normals(triangle: &[Vertex], weights: &Vector3<f32>) -> Option<Vector3<f32>> {
    let n0 = triangle[0].normal?;
    let n1 = triangle[1].normal?;
    let n2 = triangle[2].normal?;
    let mut normal = Vector3::<f32>::zeros();
    normal += n0 * weights.x;
    normal += n1 * weights.y;
    normal += n2 * weights.z;
    Some(normal.normalize())
}

const EPSILON: f32 = 1e-6;

fn calculate_depths(triangle: &[Vertex], weights: &Vector3<f32>) -> f32 {
    let z0 = triangle[0].position.z;
    let z1 = triangle[1].position.z;
    let z2 = triangle[2].position.z;

    let denominator = weights.x / z0 + weights.y / z1 + weights.z / z2;

    if denominator.abs() > EPSILON {
        1.0 / denominator
    } else {
        0.0
    }
}

pub struct RenderSlice<'a> {
    color_slice: &'a mut [u32],
    depth_slice: &'a mut [f32],
    start: u32,
    end: u32,
    width: u32,
    height: u32,
}

pub enum Material {
    SolidColor(Color),
    VertexColors,
    Textured {
        texture: Texture,
    },
    LitTexture {
        texture: Texture,
        light_dir: Vector3<f32>,
    },
    LitSolid {
        color: Color,
        light_dir: Vector3<f32>,
    },
}

pub trait Shader: Sync {
    fn shade(&self, triangle: &[Vertex], weights: &Vector3<f32>) -> Color;
}
impl Shader for Material {
    fn shade(&self, triangle: &[Vertex], weights: &Vector3<f32>) -> Color {
        match self {
            Self::SolidColor(color) => color.clone(),
            Self::VertexColors => match (triangle[0].color, triangle[1].color, triangle[2].color) {
                (Some(c1), Some(c2), Some(c3)) => c1.interpolate(&c2, &c3, &weights),
                _ => Color::new(1.0, 1.0, 1.0, 1.0),
            },
            Self::Textured { texture } => {
                if let Some(uv) = calculate_uvs(&triangle, &weights) {
                    if let Some(color) = texture.sample(&uv) {
                        color
                    } else {
                        Color::new(1.0, 1.0, 1.0, 1.0)
                    }
                } else {
                    Color::new(1.0, 1.0, 1.0, 1.0)
                }
            }
            Self::LitTexture { texture, light_dir } => {
                let uv = calculate_uvs(&triangle, &weights);
                let mut color = if let Some(color) = texture.sample(&uv.unwrap_or(Point2::origin()))
                {
                    color
                } else {
                    Color::new(1.0, 1.0, 1.0, 1.0)
                };
                if let Some(normal) = calculate_normals(&triangle, &weights) {
                    color = color * Vector3::dot(&normal, &light_dir).max(0.01);
                }
                color
            }
            Self::LitSolid { color, light_dir } => {
                let mut color = color.clone();
                if let Some(normal) = calculate_normals(&triangle, &weights) {
                    color = color * Vector3::dot(&normal, &light_dir).max(0.01);
                }
                color
            }
        }
    }
}

pub fn clip_triangle(triangle: &[Vertex], camera: &Camera) -> Vec<Vertex> {
    let clip0 = triangle[0].position.z < camera.near;
    let clip1 = triangle[1].position.z < camera.near;
    let clip2 = triangle[2].position.z < camera.near;
    let clipped_triangle = match [clip0, clip1, clip2] {
        [true, true, true] => triangle.to_vec(),
        _ => Vec::new(),
    };
    clipped_triangle
}

pub fn draw_buffer(target: &mut RenderTarget, entity: &Entity, camera: &Camera, mode: &DrawMode) {
    target.vertex_buffer.clear();
    let mv_mat =
        camera.get_view_matrix() * entity.position.to_homogeneous() * entity.scale.to_homogeneous();
    let p_mat = camera.get_perspective_matrix();

    let mut vertices = &mut target.vertex_buffer;
    vertices.extend_from_slice(entity.model.vertices.as_slice());
    for vertex in vertices.iter_mut() {vertex.model_to_view_mut(&mv_mat);}
    let mut vertices = vertices.chunks_mut(3).flat_map(|v| clip_triangle(v, &camera)).collect::<Vec<_>>();
    for vertex in vertices.iter_mut() {
        vertex.view_to_clip_mut(&p_mat).clip_to_ndc_mut()
        .ndc_to_screen_mut((target.width, target.height))
        .update_normal_mut(&entity.position);
    }

    let color = Color::new(1.0, 1.0, 1.0, 1.0).as_u32();
    let size = 2.0;
    target.create_slices().par_iter_mut().for_each(|slice| {
        for triangle in vertices.as_slice().chunks_exact(3) {
            if mode.shaded {
                draw_triangle(slice, triangle, &entity.shader);
            }
            if mode.wireframe {
                draw_line(slice, &triangle[0], &triangle[1], color);
                draw_line(slice, &triangle[1], &triangle[2], color);
                draw_line(slice, &triangle[2], &triangle[0], color);
            }
            if mode.points {
                draw_point(slice, &triangle[0], size, color);
                draw_point(slice, &triangle[1], size, color);
                draw_point(slice, &triangle[2], size, color);
            }
        }
    });
}

fn draw_triangle(slice: &mut RenderSlice, triangle: &[Vertex], shader: &Box<dyn Shader>) {
    let bounds = Bounds::new(triangle, (slice.width, slice.height));

    for y in bounds.y_range() {
        if y < slice.start || y >= slice.end {
            continue;
        }
        let mut in_triangle = false;
        for x in bounds.x_range() {
            if point_in_triangle(&triangle, &Point2::new(x as f32, y as f32)) {
                let current = triangle_barycentric(&triangle, &Point2::new(x as f32, y as f32));
                in_triangle = true;
                let depth = calculate_depths(triangle, &current);
                // Adjust index to be relative to the slice
                let idx = ((y - slice.start) * slice.width + x) as usize;
                if idx < slice.color_slice.len() {
                    if depth > slice.depth_slice[idx] {
                        continue;
                    }
                    let texture_color = shader.shade(triangle, &current);
                    slice.color_slice[idx] = texture_color.as_u32();
                    slice.depth_slice[idx] = depth;
                }
            } else {
                if in_triangle == true {
                    break; // if we entered then exited the triangle, skip the rest of this scanline.
                }
            }
        }
    }
}
fn draw_line(slice: &mut RenderSlice, p1: &Vertex, p2: &Vertex, color: u32) {
    let p1 = p1.position.xy();
    let p2 = p2.position.xy();

    let x0 = p1.x as i32;
    let y0 = p1.y as i32;
    let x1 = p2.x as i32;
    let y1 = p2.y as i32;

    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    let mut x = x0;
    let mut y = y0;

    loop {
        if y >= slice.start as i32 && y < slice.end as i32 && x >= 0 && x < slice.width as i32 {
            let relative_y = (y - slice.start as i32) as usize;
            let index = relative_y * slice.width as usize + x as usize;
            if index < slice.color_slice.len() {
                slice.color_slice[index] = color;
            }
        }

        if x == x1 && y == y1 {
            break;
        }

        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}
fn draw_point(slice: &mut RenderSlice, point: &Vertex, size: f32, color: u32) {
    for x in (point.position.x - size.ceil()) as u32..(point.position.x + size.ceil()) as u32 {
        for y in (point.position.y - size.ceil()) as u32..(point.position.y + size.ceil()) as u32 {
            if y >= slice.start && y < slice.end && x < slice.width {
                let relative_y = (y - slice.start) as usize;
                let index = relative_y * slice.width as usize + x as usize;
                if index < slice.color_slice.len() {
                    let test_pos = Point2::new(x as f32, y as f32);
                    let pos = point.position.xy();
                    if (test_pos - pos).magnitude() < size {
                        slice.color_slice[index] = color;
                    }
                }
            }
        }
    }
}
