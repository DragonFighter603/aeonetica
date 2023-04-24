use std::{rc::Rc, cell::Cell};

use crate::{uniform_str, renderer::shader::UniformStr};

use super::{buffer::{Buffer, BufferLayout, BufferType, BufferUsage, vertex_array::VertexArray}, RenderID, shader::{self, ShaderDataType}, Renderer};
use aeonetica_engine::{collections::ordered_map::ExtractComparable, log_err};

pub type BatchID = u32;

pub(super) struct Batch {
    id: BatchID,

    layout: Rc<BufferLayout>,
    vertex_array: VertexArray,

    vertices: Vec<u8>,
    vertices_dirty: Cell<bool>,
    indices: Vec<u32>,
    indices_dirty: Cell<bool>,

    shader: shader::Program,
    textures: Vec<RenderID>,
    z_index: u8
}

impl Batch {
    const MAX_BATCH_VERTEX_COUNT: u32 = 1024;
    const MAX_BATCH_INDEX_COUNT: u32 = 1024;

    const TEXTURE_SLOTS: [i32; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]; // 16 is the minimum amount per stage required by OpenGL
    const NUM_TEXTURE_SLOTS: usize = Self::TEXTURE_SLOTS.len();

    pub fn new(id: BatchID, data: &VertexData) -> Option<Batch> {
        let mut vertex_array = VertexArray::new()?;

        let vertex_buffer = Buffer::new_sized(
            BufferType::Array, 
            (Self::MAX_BATCH_VERTEX_COUNT * data.layout().stride()) as isize,
            Some(data.layout().clone()), 
            BufferUsage::DYNAMIC
        )?;
        vertex_array.set_vertex_buffer(vertex_buffer);

        let index_buffer = Buffer::new_sized(
            BufferType::ElementArray,
            Self::MAX_BATCH_INDEX_COUNT as isize * std::mem::size_of::<u32>() as isize,
            None,
            BufferUsage::DYNAMIC
        )?;
        vertex_array.set_index_buffer(index_buffer);

        let vertices = Vec::with_capacity((Self::MAX_BATCH_VERTEX_COUNT * data.layout().stride()) as usize);
        let indices = Vec::with_capacity(Self::MAX_BATCH_INDEX_COUNT as usize * std::mem::size_of::<u32>());

        Some(Self {
            id,

            layout: data.layout().clone(),
            vertex_array,
            
            vertices,
            vertices_dirty: Cell::new(false),
            indices,
            indices_dirty: Cell::new(false),

            shader: data.shader(),
            textures: vec![],
            z_index: data.z_index
        })
    }

    pub fn has_space_for(&self, data: &VertexData) -> bool {
        if self.z_index != data.z_index { return false }
        self.vertex_array.vertex_buffer().as_ref().unwrap().count() < Self::MAX_BATCH_VERTEX_COUNT &&
        self.vertex_array.index_buffer().as_ref().unwrap().count() + data.num_indices() <= Self::MAX_BATCH_INDEX_COUNT &&
        self.shader == data.shader() &&
        self.layout.eq(data.layout()) &&
        if let Some(t) = data.texture { self.textures.contains(&t) || self.textures.len() < Self::NUM_TEXTURE_SLOTS } else { true } 
    }

    pub fn add_vertices(&mut self, data: &mut VertexData) -> VertexLocation {
        if let Some(tex_id) = data.texture {
            let index = self.textures.iter().position(|id| *id == tex_id)
                .unwrap_or_else(|| {
                    self.textures.push(tex_id);
                    self.textures.len() - 1
                });

            data.patch_texture_id(index as u32);
        }

        let num_vertices = self.vertices.len() as u32 / self.layout.stride();
        self.vertices.extend_from_slice(data.vertices);
        self.vertices_dirty.set(true);
        
        let indices = data.indices().iter().map(|i| i + num_vertices);
        self.indices.extend(indices);
        self.indices_dirty.set(true);

        VertexLocation {
            batch: self.id, 
            vertices_offset: num_vertices,
            vertices_count: data.num_vertices()
        }
    }

    pub fn modify_vertices(&mut self, location: &VertexLocation, data: &mut [u8], texture: Option<RenderID>) -> Result<(), ()> {
        let num_bytes = (location.count() * self.layout.stride()) as usize;
        if num_bytes < data.len() {
            log_err!("unexpected vertices lenght; got {}, expected {}", data.len(), num_bytes);
            return Err(());
        }

        if let Some(texture) = texture {
            let slot = self.textures.iter().position(|t| *t == texture).ok_or(())?;            
            patch_texture_id(data, &self.layout, slot as u32);            
        }

        let offset = (location.offset() * self.layout.stride()) as usize;
        self.vertices[offset..offset + num_bytes].copy_from_slice(data);
        self.vertices_dirty.set(true);

        Ok(())
    }

    pub fn draw_vertices(&self, renderer: &mut Renderer) {
        if self.indices_dirty.get() {
            self.update_indices();
        }

        if self.vertices_dirty.get() {
            self.update_vertices();
        }

        renderer.load_shader(self.shader.clone());

        for (slot, texture) in self.textures.iter().enumerate() {
            unsafe {
                gl::ActiveTexture(gl::TEXTURE0 + slot as u32);
                gl::BindTexture(gl::TEXTURE_2D, *texture);
            }
        }
        if !self.textures.is_empty() {
            const TEXTURES_UNIFORM: UniformStr = uniform_str!("u_Textures");
            self.shader.upload_uniform(&TEXTURES_UNIFORM, &Self::TEXTURE_SLOTS.as_slice())
        }

        self.vertex_array.bind();
        let num_indices = self.vertex_array.index_buffer().as_ref().unwrap().count() as i32;
        unsafe {
            gl::DrawElements(gl::TRIANGLES, num_indices, gl::UNSIGNED_INT, std::ptr::null());
        }

        self.vertex_array.unbind();
        for slot in 0..self.textures.len() {
            unsafe {
                gl::ActiveTexture(gl::TEXTURE0 + slot as u32);
                gl::BindTexture(gl::TEXTURE_2D, 0);
            }
        }
    }

    pub fn id(&self) -> &BatchID {
        &self.id
    }

    pub fn update_indices(&self) {
        let num_indices = self.indices.len();

        let index_buffer = self.vertex_array.index_buffer().as_ref().unwrap();
        index_buffer.bind();

        unsafe {
            gl::BufferData(
                index_buffer.gl_typ(),
                (num_indices * std::mem::size_of::<u32>()) as isize,
                self.indices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW
            )
        }
        index_buffer.set_count(num_indices as u32);

        self.indices_dirty.set(false);
    }

    pub fn update_vertices(&self) {
        let num_bytes = self.vertices.len();

        let vertex_buffer = self.vertex_array.vertex_buffer().as_ref().unwrap();
        vertex_buffer.bind();

        unsafe {
            gl::BufferData(
                vertex_buffer.gl_typ(),
                num_bytes as isize,
                self.vertices.as_ptr() as *const _,
                gl::DYNAMIC_DRAW
            );
        }
        vertex_buffer.set_count(num_bytes as u32 / self.layout.stride());

        self.vertices_dirty.set(false);
    }
}

impl ExtractComparable<u8> for Batch {
    fn extract_comparable(&self) -> u8 {
        self.z_index
    }
}

pub struct VertexData<'a> {
    vertices: &'a mut [u8],
    indices: &'a[u32],
    layout: Rc<BufferLayout>,
    shader: shader::Program,
    z_index: u8,
    texture: Option<RenderID>,
}

impl<'a> VertexData<'a> {
    pub fn new(vertices: &'a mut [u8], indices: &'a[u32], layout: Rc<BufferLayout>, shader: shader::Program, z_index: u8) -> Self {
        Self {
            vertices,
            indices,
            layout,
            shader,
            z_index,
            texture: None,
        }
    }

    pub fn new_textured(vertices: &'a mut [u8], indices: &'a[u32], layout: Rc<BufferLayout>, shader: shader::Program, z_index: u8, texture: RenderID) -> Self {
        Self {
            vertices,
            indices,
            layout,
            shader,
            z_index,
            texture: Some(texture),
        }
    }

    pub fn indices(&self) -> &'a[u32] {
        self.indices
    }

    pub fn num_indices(&self) -> u32 {
        self.indices.len() as u32
    }

    pub fn layout(&self) -> &Rc<BufferLayout> {
        &self.layout
    }

    pub fn vertices(&mut self) -> &mut [u8] {
        self.vertices
    }

    pub fn num_vertices(&self) -> u32 {
        self.vertices.len() as u32 / self.layout.stride()
    }

    pub fn texture(&self) -> Option<RenderID> {
        self.texture
    }

    pub fn shader(&self) -> shader::Program {
        self.shader.clone()
    }

    fn patch_texture_id(&mut self, slot: u32) {
        patch_texture_id(self.vertices, &self.layout, slot)
    }

    pub fn z_index(&self) -> u8 {
        self.z_index
    }
}

fn patch_texture_id(vertices: &mut [u8], layout: &BufferLayout, slot: u32) {
    let slot_bytes = slot.to_le_bytes();
    for element in layout.elements().iter().filter(|e| e.typ() == ShaderDataType::Sampler2D) {
        for i in 0..(vertices.len() as u32 / layout.stride()) {
            let pos = (layout.stride() * i + element.offset()) as usize;
            (0..slot_bytes.len()).for_each(|i| vertices[i + pos] = slot_bytes[i]);
        }
    }
}

#[derive(Clone)]
pub struct VertexLocation {
    batch: BatchID,
    vertices_offset: u32,
    vertices_count: u32,
}

impl VertexLocation {
    pub(super) fn batch(&self) -> &BatchID {
        &self.batch
    }

    pub(super) fn offset(&self) -> u32 {
        self.vertices_offset
    }

    pub(super) fn count(&self) -> u32 {
        self.vertices_count
    }
}
