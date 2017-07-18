use cgmath::*;

use gfx;
use gfx::traits::FactoryExt;

use shape;
use shader;
use shader::{SHADER_V, SHADER_F, Vertex, Locals, pipe};
use support;
use std::marker::PhantomData;

fn calc_surface_normal<V: Into<Vector3<f32>>>(v1: V, v2: V, v3: V) -> Vector3<f32> {
    let (v1, v2, v3) = (v1.into(), v2.into(), v3.into());
    let poly_v1 = Vector3::new(v2.x - v1.x, v2.y - v1.y, v2.z - v1.z);
    let poly_v2 = Vector3::new(v3.x - v1.x, v3.y - v1.y, v3.z - v1.z);
    poly_v1.cross(poly_v2).normalize()
}

#[inline(always)]
fn construct_cube<'a>(dimensions: &'a (f32, f32, f32),
                      colors: &[[f32; 4]; 6])
                      -> ([Vertex; 36], &'a [u16]) {
    let vertices = shape::make_cube_vertices(dimensions);
    let normals = shape::make_cube_normals();
    macro_rules! make_vertex {
        ($idx:expr, $color:expr, $normal:expr) => (
            Vertex {
                pos: [vertices[$idx][0], vertices[$idx][1], vertices[$idx][2], vertices[$idx][3]],
                color: colors[$color],
                normal: normals[$normal],
            }
        )
    }
    macro_rules! make_vertex_for_face {
        ($idx:expr, $color:expr, $normal:expr) => {{
            let v0 = make_vertex!($idx + 0, $color, $normal);
            let v1 = make_vertex!($idx + 1, $color, $normal);
            let v2 = make_vertex!($idx + 2, $color, $normal);
            let v3 = make_vertex!($idx + 3, $color, $normal);
            let v4 = make_vertex!($idx + 4, $color, $normal);
            let v5 = make_vertex!($idx + 5, $color, $normal);
            [v0, v1, v2, v3, v4, v5]
            }
    }};
    let v0 = make_vertex_for_face!(0, 0, 0);
    let v1 = make_vertex_for_face!(6, 1, 1);
    let v2 = make_vertex_for_face!(12, 2, 2);
    let v3 = make_vertex_for_face!(18, 3, 3);
    let v4 = make_vertex_for_face!(24, 4, 4);
    let v5 = make_vertex_for_face!(30, 5, 5);

    let v = [v0[0], v0[1], v0[2], v0[3], v0[4], v0[5], v1[0], v1[1], v1[2], v1[3], v1[4], v1[5],
             v2[0], v2[1], v2[2], v2[3], v2[4], v2[5], v3[0], v3[1], v3[2], v3[3], v3[4], v3[5],
             v4[0], v4[1], v4[2], v4[3], v4[4], v4[5], v5[0], v5[1], v5[2], v5[3], v5[4], v5[5]];

    const INDICES: &[u16] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
                              19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35];
    (v, &INDICES)
}

fn make_triangle2d(length: f32, colors: &[[f32; 4]; 3]) -> [Vertex; 3] {
    let vertices = shape::make_triangle_vertices(length);

    let normal = [0.0, 0.0, 0.0];
    let a = Vertex {
        pos: [vertices[0][0], vertices[0][1], vertices[0][2], vertices[0][3]],
        color: colors[0],
        normal: normal,
    };
    let b = Vertex {
        pos: [vertices[1][0], vertices[1][1], vertices[1][2], vertices[1][3]],
        color: colors[1],
        normal: normal,
    };
    let c = Vertex {
        pos: [vertices[2][0], vertices[2][1], vertices[2][2], vertices[2][3]],
        color: colors[2],
        normal: normal,
    };
    [a, b, c]
}

pub struct PsoFactory<'a, R, F>
    where R: gfx::Resources,
          F: gfx::Factory<R> + 'a
{
    factory: &'a mut F,
    phantom: PhantomData<R>,
}

impl<'a, R, F> PsoFactory<'a, R, F>
    where R: gfx::Resources,
          F: gfx::Factory<R> + 'a
{
    pub fn new(factory: &'a mut F) -> PsoFactory<'a, R, F> {
        PsoFactory {
            factory: factory,
            phantom: PhantomData,
        }
    }

    pub fn triangle_strip(&mut self) -> gfx::PipelineState<R, pipe::Meta> {
        let set = self.factory.create_shader_set(SHADER_V, SHADER_F).unwrap();
        let primitive = gfx::Primitive::TriangleStrip;
        let rasterizer = gfx::state::Rasterizer::new_fill().with_cull_back();
        let pipe = pipe::new();
        self.factory
            .create_pipeline_state(&set, primitive, rasterizer, pipe)
            .unwrap()
    }

    pub fn triangle_list(&mut self) -> gfx::PipelineState<R, pipe::Meta> {
        let set = self.factory.create_shader_set(SHADER_V, SHADER_F).unwrap();
        let primitive = gfx::Primitive::TriangleList;
        let rasterizer = gfx::state::Rasterizer::new_fill().with_cull_back();
        let pipe = pipe::new();
        self.factory
            .create_pipeline_state(&set, primitive, rasterizer, pipe)
            .unwrap()
    }
}

macro_rules! copy_vertices {
    ($factory:ident, $encoder:ident, $ambient:ident, $light_color:ident, $light_pos:ident, $out_color:ident, $depth:ident, $pso:ident, $model_m:ident, $vertices:ident, $indices:ident) => {{
        let (vertex_buffer, slice) = $factory.create_vertex_buffer_with_slice(&$vertices, $indices);
        let data = pipe::Data {
            vbuf: vertex_buffer,
            locals: $factory.create_constant_buffer(1),
            model: $model_m.into(),
            ambient: $ambient,
            lightcolor: $light_color,
            lightpos: $light_pos,
            out: $out_color.clone(),
            depth: $depth.clone(),
        };
        let locals = Locals {
            model: data.model,
            ambient: data.ambient,
            lightcolor: data.lightcolor,
            lightpos: data.lightpos,
        };
        $encoder.update_buffer(&data.locals, &[locals], 0).unwrap();
        $encoder.draw(&slice, &$pso, &data);
    }};
}

pub struct Gpu<'a, R, F, C>
    where R: gfx::Resources,
          F: gfx::Factory<R> + 'a,
          C: gfx::CommandBuffer<R> + 'a
{
    factory: &'a mut F,
    encoder: &'a mut gfx::Encoder<R, C>,
    out_color: &'a shader::OutColor<R>,
    main_depth: &'a shader::OutDepth<R>,
}

impl<'z, R, F, C> Gpu<'z, R, F, C>
    where R: gfx::Resources,
          F: gfx::Factory<R> + 'z,
          C: gfx::CommandBuffer<R> + 'z
{
    pub fn new(f: &'z mut F,
               e: &'z mut gfx::Encoder<R, C>,
               out_color: &'z shader::OutColor<R>,
               main_depth: &'z shader::OutDepth<R>)
               -> Gpu<'z, R, F, C> {
        Gpu {
            factory: f,
            encoder: e,
            out_color: out_color,
            main_depth: main_depth,
        }
    }

    pub fn draw_cube(&mut self,
                     pso: &gfx::PipelineState<R, pipe::Meta>,
                     dimensions: &(f32, f32, f32),
                     colors: &[[f32; 4]; 6],
                     ambient: [f32; 4],
                     lightcolor: [f32; 4],
                     lightpos: [f32; 3],
                     model_m: Matrix4<f32>) {
        let (vertices, indices) = construct_cube(dimensions, &colors);

        let factory = &mut self.factory;
        let encoder = &mut self.encoder;
        let out_color = &mut self.out_color;
        let main_depth = &mut self.main_depth;

        copy_vertices!(factory,
                       encoder,
                       ambient,
                       lightcolor,
                       lightpos,
                       out_color,
                       main_depth,
                       pso,
                       model_m,
                       vertices,
                       indices)
    }

    pub fn draw_triangle(&mut self,
                         pso: &gfx::PipelineState<R, pipe::Meta>,
                         radius: f32,
                         colors: &[[f32; 4]; 3],
                         ambient: [f32; 4],
                         lightcolor: [f32; 4],
                         lightpos: [f32; 3],
                         model_m: Matrix4<f32>) {
        let vertices = make_triangle2d(radius, &colors);
        let indices = ();

        let factory = &mut self.factory;
        let encoder = &mut self.encoder;
        let out_color = &mut self.out_color;
        let main_depth = &mut self.main_depth;

        copy_vertices!(factory,
                       encoder,
                       ambient,
                       lightcolor,
                       lightpos,
                       out_color,
                       main_depth,
                       pso,
                       model_m,
                       vertices,
                       indices)
    }

    pub fn draw_triangle_from_vertices(&mut self,
                                       pso: &gfx::PipelineState<R, pipe::Meta>,
                                       vertices: &[shader::Vertex],
                                       indices: &[u32],
                                       ambient: [f32; 4],
                                       lightcolor: [f32; 4],
                                       lightpos: [f32; 3],
                                       model_m: Matrix4<f32>) {
        let factory = &mut self.factory;
        let encoder = &mut self.encoder;
        let out_color = &mut self.out_color;
        let main_depth = &mut self.main_depth;

        copy_vertices!(factory,
                       encoder,
                       ambient,
                       lightcolor,
                       lightpos,
                       out_color,
                       main_depth,
                       pso,
                       model_m,
                       vertices,
                       indices)
    }
}
