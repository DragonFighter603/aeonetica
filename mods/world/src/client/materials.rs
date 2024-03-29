use std::{rc::Rc, char::MAX};

use aeonetica_client::{renderer::{buffer::*, shader, material::{Material, FlatTexture}, RenderID, texture::{Sampler2D, Sprite}, builtin::Quad}, vertex, data_store::DataStore};
use aeonetica_engine::math::vector::Vector2;
use aeonetica_engine::error::ExpectLog;

struct TerrainMaterial(Rc<FlatTexture>);
struct TerrainShader(Rc<shader::Program>);

thread_local! {
    static GLOW_TEXTURE_LAYOUT: Rc<BufferLayout> = Rc::new(<GlowTexture as Material>::Layout::build());
}

fn create_terrain_shader() -> TerrainShader {
    TerrainShader(Rc::new(shader::Program::from_source(include_str!("../../assets/terrain-shader.glsl")).expect_log()))
}

pub fn terrain_material(store: &mut DataStore) -> Rc<FlatTexture> {
    let shader = store.get_or_create(create_terrain_shader).0.clone();
    store.get_or_create(|| TerrainMaterial(Rc::new(FlatTexture::with_shader(shader)))).0.clone()
}

pub fn terrain_shader(store: &mut DataStore) -> Rc<shader::Program> {
    store.get_or_create(create_terrain_shader).0.clone()
}

pub trait WithTerrain {
    fn with_terrain_texture(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, texture: RenderID, material: Rc<FlatTexture>) -> Self;
    fn with_terrain_sprite(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, sprite: Sprite, material: Rc<FlatTexture>) -> Self;
}

impl WithTerrain for Quad<FlatTexture> {
    fn with_terrain_texture(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, texture: RenderID, material: Rc<FlatTexture>) -> Self {
        Self::new(position, size, z_index, material, ([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], texture))
    }

    fn with_terrain_sprite(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, sprite: Sprite, material: Rc<FlatTexture>) -> Self {
        Self::new(position, size, z_index, material, ([
            [sprite.left(),  sprite.top()   ],
            [sprite.right(), sprite.top()   ],
            [sprite.right(), sprite.bottom()],
            [sprite.left(),  sprite.bottom()]
        ], sprite.texture()))
    }
}

struct GlowTextureShader(Rc<shader::Program>);

pub struct GlowTexture {
    shader: Rc<shader::Program>
}

impl GlowTexture {
    pub fn get(store: &mut DataStore) -> Rc<Self> {
        let shader = store.get_or_create(|| GlowTextureShader(Rc::new(shader::Program::from_source(include_str!("../../assets/glow-shader.glsl")).expect_log()))).0.clone();
        store.get_or_create(|| Rc::new(Self { shader })).clone()
    }
}

impl Material for GlowTexture {
    type Layout = BufferLayoutBuilder<(Vertex, TexCoord, TextureID, Color)>;
    type Data<const N: usize> = ([[f32; 2]; N], RenderID, [f32; 4]);
    type VertexTuple = VertexTuple4<[f32; 2], [f32; 2], Sampler2D, [f32; 4]>;

    fn shader(&self) -> &Rc<shader::Program> {
        &self.shader
    }

    fn texture_id<const N: usize>(data: &Self::Data<N>) -> Option<RenderID> {
        Some(data.1)
    }

    fn layout<'a>() -> &'a Rc<BufferLayout> {
        unsafe {
            let x: *const Rc<BufferLayout> = GLOW_TEXTURE_LAYOUT.with(|l| l as *const _);
            x.as_ref().unwrap_unchecked()
        }
    }

    fn vertices<const N: usize>(&self, vertices: [[f32; 2]; N], data: &Self::Data<N>) -> [Self::VertexTuple; N] {
        Self::Layout::array(std::array::from_fn(|i| vertex!(vertices[i], data.0[i], Sampler2D(0), data.2)))
    }

    fn data_slice<const N: usize, const NN: usize>(&self, data: &Self::Data<N>, offset: usize) -> Self::Data<NN> {
        (std::array::from_fn(|i| data.0[offset + i]), data.1, data.2)
    }

    fn default_data<const N: usize>(&self) -> Self::Data<N> {
        (std::array::from_fn(|_| [0.0; 2]), 0, [0.0; 4])
    }
}

pub trait WithGlow {
    fn with_glow_texture(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, texture: RenderID, glow_color: [f32; 4], material: Rc<GlowTexture>) -> Self;
    fn with_glow_sprite(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, sprite: Sprite, glow_color: [f32; 4], material: Rc<GlowTexture>) -> Self;

    fn light_color(&self) -> [f32; 4];
}

impl WithGlow for Quad<GlowTexture> {
    fn with_glow_texture(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, texture: RenderID, glow_color: [f32; 4], material: Rc<GlowTexture>) -> Self {
        Self::new(position, size, z_index, material, ([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], texture, glow_color))
    }

    fn with_glow_sprite(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, sprite: Sprite, glow_color: [f32; 4], material: Rc<GlowTexture>) -> Self {
        Self::new(position, size, z_index, material, ([
            [sprite.left(),  sprite.top()   ],
            [sprite.right(), sprite.top()   ],
            [sprite.right(), sprite.bottom()],
            [sprite.left(),  sprite.bottom()]
        ], sprite.texture(), glow_color))
    }

    fn light_color(&self) -> [f32; 4] {
        self.params().2
    }
}

pub const MAX_WATER_DEPTH: f32 = 10.0;

thread_local! {
    static WATER_TEXTURE_LAYOUT: Rc<BufferLayout> = Rc::new(<WaterMaterial as Material>::Layout::build());
}

pub struct WaterMaterial {
    shader: Rc<shader::Program>
}

struct WaterShader(Rc<shader::Program>);
impl WaterMaterial {
    pub fn get(store: &mut DataStore) -> Rc<Self> {
        let shader = store.get_or_create(|| WaterShader(Rc::new(shader::Program::from_source(include_str!("../../assets/water-shader.glsl")).expect_log()))).0.clone();
        store.get_or_create(|| Rc::new(Self { shader })).clone()
    }
}

impl Material for WaterMaterial {
    type Layout = BufferLayoutBuilder<(Vertex, TexCoord, TextureID, Float)>;
    type Data<const N: usize> = ([[f32; 2]; N], RenderID, [f32; N]);
    type VertexTuple = VertexTuple4<[f32; 2], [f32; 2], Sampler2D, f32>;


    fn shader(&self) -> &Rc<shader::Program> {
        &self.shader
    }

    fn texture_id<const N: usize>(data: &Self::Data<N>) -> Option<RenderID> {
        Some(data.1)
    }

    fn layout<'a>() -> &'a Rc<BufferLayout> {
        unsafe {
            let x: *const Rc<BufferLayout> = WATER_TEXTURE_LAYOUT.with(|l| l as *const _);
            x.as_ref().unwrap_unchecked()
        }
    }

    fn vertices<const N: usize>(&self, vertices: [[f32; 2]; N], data: &Self::Data<N>) -> [Self::VertexTuple; N] {
        Self::Layout::array(std::array::from_fn(|i| vertex!(vertices[i], data.0[i], Sampler2D(0), data.2[i])))
    }

    fn data_slice<const N: usize, const NN: usize>(&self, data: &Self::Data<N>, offset: usize) -> Self::Data<NN> {
        (std::array::from_fn(|i| data.0[offset + i]), data.1, std::array::from_fn(|i| data.2[offset + i]))
    }

    fn default_data<const N: usize>(&self) -> Self::Data<N> {
        (std::array::from_fn(|_| [0.0; 2]), 0, std::array::from_fn(|_| 0.0))
    }
}

pub trait WithWater {
    fn with_water_texture(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, texture: RenderID, material: Rc<WaterMaterial>, distance_to_surface: f32) -> Self;
    fn with_water_sprite(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, sprite: Sprite, material: Rc<WaterMaterial>, distance_to_surface: f32) -> Self;
}

impl WithWater for Quad<WaterMaterial> {
    fn with_water_texture(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, texture: RenderID, material: Rc<WaterMaterial>, distance_to_surface: f32) -> Self {
        let d0 = position.y - distance_to_surface * 2.0 + size.y;
        let d1 = d0 + size.y;
        Self::new(position, size, z_index, material, ([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]], texture,  [d1, d1, d0, d0]))
    }

    fn with_water_sprite(position: Vector2<f32>, size: Vector2<f32>, z_index: u8, sprite: Sprite, material: Rc<WaterMaterial>, distance_to_surface: f32) -> Self {
        let d0 = position.y - distance_to_surface * 2.0 + size.y;
        let d1 = d0 + size.y;
        Self::new(position, size, z_index, material, ([
            [sprite.left(),  sprite.top()   ],
            [sprite.right(), sprite.top()   ],
            [sprite.right(), sprite.bottom()],
            [sprite.left(),  sprite.bottom()]
        ], sprite.texture(), [d1, d1, d0, d0]))
    }
}