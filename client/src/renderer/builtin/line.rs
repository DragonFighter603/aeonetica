use std::rc::Rc;

use aeonetica_engine::math::vector::Vector2;

use crate::renderer::{material::{FlatColor, Material}, VertexLocation, shader, Renderable, batch::VertexData, util};

pub struct Line {
    from: Vector2<f32>,
    to: Vector2<f32>,
    weight: f32,
    z_index: u8,

    material: Rc<FlatColor>,
    vertices: Option<[<FlatColor as Material>::VertexTuple; 4]>,
    params: <FlatColor as Material>::Data<4>,

    location: Option<VertexLocation>,
}

impl Line {
    const INDICES: [u32; 6] = [0, 1, 2, 2, 3, 0];
 
    pub fn new(from: Vector2<f32>, to: Vector2<f32>, weight: f32, z_index: u8, color: [f32; 4]) -> Self {
        Self::with_material(from, to, weight, z_index, color, FlatColor::get())
    }

    pub fn with_material(from: Vector2<f32>, to: Vector2<f32>, weight: f32, z_index: u8, color: [f32; 4], material: Rc<FlatColor>) -> Self {
        Self {
            from,
            to,
            weight,
            z_index,
            params: color,
            material,
            vertices: None,
            location: None
        }
    }

    pub fn set_dirty(&mut self) {
        self.vertices = None;
    }

    pub fn from(&self) -> &Vector2<f32> {
        &self.from
    }

    pub fn to(&self) -> &Vector2<f32> {
        &self.to
    }

    pub fn weight(&self) -> f32 {
        self.weight
    }

    pub fn z_index(&self) -> u8 {
        self.z_index
    }

    pub fn color(&self) -> &[f32; 4] {
        &self.params
    }

    pub fn set_from(&mut self, from: Vector2<f32>) {
        self.from = from;
        self.set_dirty();
    }

    pub fn set_to(&mut self, to: Vector2<f32>) {
        self.to = to;
        self.set_dirty();
    }

    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
        self.set_dirty();
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.params = color;
        self.set_dirty();
    }

    pub fn shader(&self) -> &shader::Program {
        self.material.shader()
    }

    fn recalculate_vertex_data(&mut self) {
        let n = (self.to - self.from).normalized().rotate_90();
        let w = Vector2::new(self.weight, self.weight).half();

        self.vertices = Some(self.material.vertices(
            [
                (self.from + n * w).into_array(),
                (self.from - n * w).into_array(),
                (self.to   - n * w).into_array(),
                (self.to   + n * w).into_array()
            ], 
            &self.params
        ));
    }
}

impl Renderable for Line {
    fn vertex_data(&mut self) -> VertexData<'_> {
        if self.is_dirty() {
            self.recalculate_vertex_data();
        }

        let vertices = self.vertices.as_ref().unwrap();
       
        VertexData::from_material::<FlatColor, 4>(
            util::to_raw_byte_slice!(vertices),
            Self::INDICES.as_slice(),
            &self.material,
            &self.params,
            self.z_index
        )
    }

    fn texture_id(&self) -> Option<crate::renderer::RenderID> {
        None
    }

    fn location(&self) -> &Option<VertexLocation> {
        &self.location
    }

    fn set_location(&mut self, location: Option<VertexLocation>) {
        self.location = location;
    }

    fn is_dirty(&self) -> bool {
        self.vertices.is_none()
    }

    fn has_location(&self) -> bool {
        self.location.is_some()
    }
}