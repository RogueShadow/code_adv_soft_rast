use crate::camera::Camera;
use crate::geometry::{point_in_triangle, Bounds, Model, Texture, Vertex};
use nalgebra::{Isometry3, Point2, Point3, Scale3, Vector3};
use rand::Rng;
use rand_xorshift::XorShiftRng;
use rayon::prelude::*;
use std::ops::MulAssign;
use std::sync::Arc;

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
}

impl RenderTarget {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            color: vec![u32::MIN; (width * height) as usize],
            depth: vec![f32::MAX; (width * height) as usize],
            width,
            height,
            clear_color: u32::MIN,
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
            });
        }
        slices
    }
}
// Modified draw_triangle to handle a slice of RenderTarget
fn draw_triangle(
    slice: &mut RenderSlice, // End of slice's y-range
    triangle: &[Vertex],
    texture: Option<&Texture>,
) {
    let bounds = Bounds::new(triangle);
    for y in bounds.y_range() {
        // Skip pixels outside the slice's y-range
        if y < slice.start || y >= slice.end {
            continue;
        }
        for x in bounds.x_range() {
            let (in_triangle, weights) = point_in_triangle(
                &triangle[0].position.xy(),
                &triangle[1].position.xy(),
                &triangle[2].position.xy(),
                &Point2::new(x as f32, y as f32),
            );
            if in_triangle {
                let depths = Vector3::new(
                    if triangle[0].position.z.abs() > 1e-6 {
                        1.0 / triangle[0].position.z
                    } else {
                        0.0
                    },
                    if triangle[1].position.z.abs() > 1e-6 {
                        1.0 / triangle[1].position.z
                    } else {
                        0.0
                    },
                    if triangle[2].position.z.abs() > 1e-6 {
                        1.0 / triangle[2].position.z
                    } else {
                        0.0
                    },
                );
                let depth = if depths.dot(&weights).abs() > 1e-6 {
                    1.0 / depths.dot(&weights)
                } else {
                    0.0
                };
                // Adjust index to be relative to the slice
                let idx = ((y - slice.start) * slice.width + x) as usize;
                if idx < slice.color_slice.len() && x >= 0 && x < slice.width {
                    if depth > slice.depth_slice[idx] {
                        continue;
                    }
                    let mut texture_color = if let Some(texture) = texture {
                        let uvs = triangle.iter().filter_map(|v| v.uv).collect::<Vec<_>>();
                        if uvs.len() == 3 {
                            let mut tex_coord = Point2::<f32>::origin();
                            tex_coord += uvs[0] * weights.x;
                            tex_coord += uvs[1] * weights.y;
                            tex_coord += uvs[2] * weights.z;
                            texture
                                .sample(&tex_coord)
                                .unwrap_or_else(|| Color::new(0.0, 1.0, 1.0, 1.0))
                        } else {
                            Color::new(0.0, 0.0, 0.0, 1.0)
                        }
                    } else {
                        match (triangle[0].color, triangle[1].color, triangle[2].color) {
                            (Some(c1), Some(c2), Some(c3)) => c1.interpolate(&c2, &c3, &weights),
                            _ => Color::new(1.0, 1.0, 1.0, 1.0),
                        }
                    };
                    let normals = triangle.iter().filter_map(|v| v.normal).collect::<Vec<_>>();
                    let mut normal = Vector3::<f32>::zeros();
                    if normals.len() == 3 {
                        let light_dir = Vector3::<f32>::new(1.0, 1.0, 0.0).normalize();
                        normal += normals[0] * weights.x;
                        normal += normals[1] * weights.y;
                        normal += normals[2] * weights.z;
                        let intensity = Vector3::dot(&normal.normalize(), &light_dir).max(0.05);
                        normal.add_scalar_mut(1.0);
                        normal.mul_assign(0.5);
                        texture_color.r *= intensity;
                        texture_color.g *= intensity;
                        texture_color.b *= intensity;
                    }
                    slice.color_slice[idx] = texture_color.as_u32();
                    slice.depth_slice[idx] = depth;
                }
            }
        }
    }
}

struct RenderSlice<'a> {
    color_slice: &'a mut [u32],
    depth_slice: &'a mut [f32],
    start: u32,
    end: u32,
    width: u32,
}

pub fn draw_buffer(
    target: &mut RenderTarget,
    transform: &Isometry3<f32>,
    scale: &Scale3<f32>,
    camera: &Camera,
    model: &Model,
    mode: &DrawMode,
) {
    let mut screen_vertices = Vec::with_capacity(model.vertices.len());
    for vertices in model.vertices.chunks(3) {
        for vertex in vertices {
            let mvp_mat = camera.get_perspective_matrix()
                * camera.get_view_matrix()
                * transform.to_homogeneous()
                * scale.to_homogeneous();
            let clip_v = mvp_mat * vertex.position.to_homogeneous();
            if clip_v.z < camera.near || clip_v.z > camera.far {
                continue;
            }
            let ndc_v = if clip_v.w != 0.0 {
                clip_v / clip_v.w
            } else {
                clip_v
            };
            let screen_x = (ndc_v.x + 1.0) * 0.5 * target.width as f32;
            let screen_y = (1.0 - ndc_v.y) * 0.5 * target.height as f32;
            let screen_z = ndc_v.z;
            if screen_x > target.width as f32
                || screen_y > target.height as f32
                || screen_x < 0.0
                || screen_y < 0.0
            {
                continue;
            }
            let mut sv = vertex.clone();
            sv.position = Point3::new(screen_x, screen_y, screen_z);
            if sv.normal.is_some() {
                sv.normal = Some(transform * sv.normal.unwrap());
            }
            screen_vertices.push(sv);
        }
    }

    let model = Arc::new(model);
    let color = Color::new(1.0, 1.0, 1.0, 1.0).as_u32();
    let size = 2.0;
    let mut slices = target.create_slices();
    
    slices.par_iter_mut().for_each(|slice| {
        // Process all triangles, filtering pixels by y-range
        for triangle in screen_vertices.as_slice().chunks_exact(3) {
            if mode.shaded {
                draw_triangle(slice, triangle, model.texture.as_ref());
            }
            if mode.wireframe {
                draw_line(slice, &triangle[0],&triangle[1],color);
                draw_line(slice, &triangle[1], &triangle[2],color);
                draw_line(slice, &triangle[2], &triangle[0],color);
            }
            if mode.points {
                draw_point(slice, &triangle[0],size,color);
                draw_point(slice, &triangle[1],size,color);
                draw_point(slice, &triangle[2],size,color);
            }
        }
    });
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
            if y >= slice.start && y < slice.end && x >= 0 && x < slice.width {
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
