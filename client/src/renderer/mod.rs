pub mod window;
pub mod layer;
pub mod context;
pub mod util;
pub mod quad;
pub mod shader;
pub mod texture;
pub mod text_area;

mod buffer;
mod batch;

pub use batch::VertexLocation;
pub use quad::*;

use std::rc::Rc;

use aeonetica_engine::{util::{vector::Vector2, matrix::Matrix4}, collections::OrderedMap};
use buffer::*;
use shader::*;
use texture::*;
use batch::*;

pub(self) use aeonetica_engine::util::camera::Camera;

use self::{sprite_sheet::Sprite, font::BitmapFont};

pub(self) type RenderID = gl::types::GLuint;

pub trait Renderable {
    fn vertex_data(&mut self) -> VertexData<'_>;
    fn texture_id(&self) -> Option<RenderID>;

    fn location(&self) -> &Option<VertexLocation>;
    fn set_location(&mut self, location: VertexLocation);
}

pub struct Renderer {
    shader: Option<Rc<Program>>,
    view_projection: Option<Matrix4<f32>>,
    batches: OrderedMap<BatchID, Batch, u8>,

    batch_counter: BatchID
}

impl Renderer {
    const VIEW_PROJECTION_UNIFORM: UniformStr = uniform_str!("u_ViewProjection");

    pub fn new() -> Self {
        Self {
            shader: None,
            view_projection: None,
            batches: OrderedMap::new(),
            batch_counter: 0,
        }
    }

    pub fn begin_scene(&mut self, camera: &Camera) {
        if let Some(shader) = &self.shader {
            shader.upload_uniform(&Self::VIEW_PROJECTION_UNIFORM, camera.view_projection_matrix());
        }
        self.view_projection = Some(camera.view_projection_matrix().clone());
    }

    pub fn end_scene(&mut self) {
        self.view_projection = None;
    }

    pub fn load_shader(&mut self, shader: Rc<Program>) {
        if self.shader.as_ref() == Some(&shader) {
            return;
        }

        shader.bind();
        if let Some(view_projection) = &self.view_projection {
            shader.upload_uniform(&Self::VIEW_PROJECTION_UNIFORM, view_projection);
        }
        self.shader = Some(shader);
    }

    pub fn unload_shader(&mut self) {
        if let Some(shader) = &self.shader {
            shader.unbind();
        }
        self.shader = None;
    }

    pub fn shader(&self) -> &Option<Rc<Program>> {
        &self.shader
    }

    pub fn draw_vertices(&mut self) {
        let mut_ref_ptr = self as *mut _;
        self.batches.iter().rev().for_each(|(_, batch)|
            batch.draw_vertices(unsafe { &mut *mut_ref_ptr })
        );

        self.unload_shader();
    }

    fn next_id(&mut self) -> BatchID {
        self.batch_counter += 1;
        self.batch_counter
    }

    pub fn add_vertices(&mut self, data: &mut VertexData) -> VertexLocation {
        if let Some(idx) = self.batches.iter().position(|(_, batch)| batch.has_space_for(data)) {
            self.batches.nth_mut(idx, |batch| batch.add_vertices(data)).unwrap()
        }
        else {
            let mut batch = Batch::new(self.next_id(), data).expect("Error creating new render batch");
            let location = batch.add_vertices(data);
            self.batches.insert(*batch.id(), batch);

            location
        }
    }

    pub fn modify_vertices(&mut self, location: &VertexLocation, data: &mut [u8], texture: Option<RenderID>) -> Result<(), ()> {
        self.batches.get_mut(
            location.batch(), 
            |batch| batch.modify_vertices(location, data, texture)
        ).ok_or(()).flatten()
    }

    pub fn add(&mut self, item: &mut impl Renderable) {
        let location = self.add_vertices(&mut item.vertex_data());
        item.set_location(location);
    }

    pub fn modify(&mut self, item: &mut impl Renderable) -> Result<(), ()> {
        let texture = item.texture_id();
        self.modify_vertices(&item.location().as_ref().unwrap().clone(), item.vertex_data().vertices(), texture)
    }

    pub fn static_string(&mut self, string: &str, position: &Vector2<f32>, size: f32, spacing: f32, font: &BitmapFont, shader: &Rc<Program>, z_index: u8) {
        type Vertices = BufferLayoutBuilder<(Vertex, TexCoord, TextureID)>;
        let layout = Rc::new(Vertices::build());

        let size = size / font.char_size().y;

        let half_size =  font.char_size() * [size, size].into() / Vector2::new(2.0, 2.0);

        let texture_id = font.sprite_sheet().texture().id();

        const INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];

        let mut x_offset = 0.0;

        for c in string.chars() {
            let position = Vector2::new(x_offset, position.y());

            let char_idx = font.char_index(c);
            if char_idx.is_none() {
                continue;
            }
            let char_idx = *char_idx.unwrap();

            let width = font.index_width(char_idx) as f32;
            x_offset += width * size + spacing;

            let char_sprite = font.sprite_sheet().get(char_idx);
            if char_sprite.is_none() {
                continue;
            }
            let char_sprite = char_sprite.unwrap();

            let vertices = Vertices::array([
                vertex!([position.x() - half_size.x(), position.y() - half_size.y(), 0.0], [char_sprite.left(), char_sprite.top()], Sampler2D(0)),
                vertex!([position.x() + half_size.x(), position.y() - half_size.y(), 0.0], [char_sprite.right(), char_sprite.top()], Sampler2D(0)),
                vertex!([position.x() + half_size.x(), position.y() + half_size.y(), 0.0], [char_sprite.right(), char_sprite.bottom()], Sampler2D(0)),
                vertex!([position.x() - half_size.x(), position.y() + half_size.y(), 0.0], [char_sprite.left(), char_sprite.bottom()], Sampler2D(0))
            ]);

            self.add_vertices(
                &mut VertexData::new_textured(
                    util::to_raw_byte_slice!(&vertices),
                    INDICES.as_slice(),
                    &layout,
                    shader,
                    z_index,
                    texture_id
                )
            );
        }
    }
} 
