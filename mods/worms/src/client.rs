use std::f32::consts::PI;

use aeonetica_client::{ClientMod, networking::messaging::{ClientHandle, ClientMessenger}, renderer::{Renderer, texture::{SpriteSheet, Texture}, builtin::{Quad, Line}, material::FlatTexture}, data_store::DataStore};
use aeonetica_engine::{time::Time, networking::messaging::ClientEntity, util::{type_to_id, nullable::Nullable}, math::vector::Vector2};
use debug_mod::Debug;
use world_mod::client::{WorldLayer, materials::terrain_material};
use world_mod::client::materials::WithTerrain;

use crate::server::{WORM_SPEED};


pub struct WormsModClient {

}

impl WormsModClient {
    pub(crate) fn new() -> Self {
        println!("created wormmodclient");
        Self {}
    }
}

impl ClientMod for WormsModClient {
    fn register_handlers(&self, handlers: &mut aeonetica_engine::util::id_map::IdMap<fn() -> Box<dyn ClientHandle>>, _store: &mut DataStore) {
        handlers.insert(type_to_id::<WormHandle>(),  WormHandle::new_boxed);
    }
}

struct WormSheet(SpriteSheet);

impl WormSheet {
    fn load() -> Self {
        println!("loaded textures");
        Self(SpriteSheet::from_texture(Texture::from_bytes(include_bytes!("../assets/include/wormsheet.png")).expect("err loading texture"),
            Vector2::new(16, 16)).expect("err loading worm sheet"))
    }
}

pub(crate) struct WormHandle {
    quads: Vec<Quad<FlatTexture>>,
    segments: Vec<Vector2<f32>>,
    p_segments: Vec<Vector2<f32>>,
    looking_dir: Vector2<f32>,
    interpolation_delta: f32,
}

impl WormHandle {
    fn new_boxed() -> Box<dyn ClientHandle> {
        Box::new(Self{
            quads: vec![],
            segments: vec![],
            p_segments: vec![],
            looking_dir: Default::default(),
            interpolation_delta: 1.0,
        })
    }

    pub(crate) fn receive_position(&mut self, _messenger: &mut ClientMessenger, mut renderer: Nullable<&mut Renderer>, store: &mut DataStore, (segments, looking_dir, teleporting): (Vec<Vector2<f32>>, Vector2<f32>, bool)) {
        if self.segments.is_empty() {
            let material = terrain_material(store);
            let sheet = store.get_or_create(WormSheet::load);
            self.p_segments = segments.clone();
            self.segments = segments.clone();
            self.interpolation_delta = 1.0;
            for (i, segment) in self.segments.iter().enumerate() {
                let quad = Quad::with_terrain_sprite(
                    *segment,
                    Vector2::new(1.0, 1.0),
                    11,
                    sheet.0.get(match i { 0 => 0, _ if i == self.segments.len() - 1 => 2, _ => 1 }).unwrap(),
                    material.clone()
                );
                self.quads.push(quad);
            }
            self.quads.iter_mut().rev().for_each(|quad| renderer.draw(quad).expect("unable to draw quad"));
        }

        self.looking_dir = looking_dir;

        if teleporting {
            self.p_segments = segments.clone();
            self.interpolation_delta = 1.0; 
            self.segments = segments;
            self.looking_dir = looking_dir;
        } else {
            self.p_segments = self.p_segments.iter().zip(&self.segments).map(|(&ps, &s)| ps + (s - ps) * self.interpolation_delta).collect();
            self.interpolation_delta = 0.0;
            self.segments = segments;
            for (i, segment) in self.segments.iter().enumerate() {
                self.quads[i].set_position(*segment);
                self.quads[i].set_rotation(if i == 0 { self.looking_dir } else { self.segments[i]-self.segments[i-1] }.euler() - PI / 2.0);
                renderer.draw(&mut self.quads[i]).expect("unable to draw quad");
            }
        }
    }
}

impl ClientEntity for WormHandle {}

impl ClientHandle for WormHandle {
    fn owning_layer(&self) -> aeonetica_engine::TypeId {
        type_to_id::<WorldLayer>()
    }

    fn start(&mut self, messenger: &mut ClientMessenger, mut renderer: Nullable<&mut Renderer>, _store: &mut DataStore) {
        messenger.register_receiver(WormHandle::receive_position);
        let pos = Vector2::new(2.0, 2.0);
        let size = Vector2::new(2.0, 2.0);
        //renderer.add(&mut Line::new(pos, pos + (size.x, 0.0).into(), 0.2,  255, [1.0, 0.0, 0.0, 1.0]));
        //renderer.add(&mut Line::new(pos + (size.x, 0.0).into(), pos + size, 0.2,  255, [0.0, 1.0, 0.0, 1.0]));
        //renderer.add(&mut Line::new(pos + size, pos + (0.0, size.y).into(), 0.2,  255, [0.0, 0.0, 1.0, 1.0]));
        //renderer.add(&mut Line::new(pos + (0.0, size.y).into(), pos, 0.2,  255, [1.0, 0.0, 1.0, 1.0]));
    }

    fn update(&mut self, _messenger: &mut ClientMessenger, renderer: &mut Renderer, store: &mut DataStore, time: Time) {
        if self.interpolation_delta < 1.0 {
            for (i, (segment, p_segment)) in self.segments.iter().zip(&self.p_segments).enumerate() {
                let delta = *segment - *p_segment;
                self.quads[i].set_position(*segment + delta * self.interpolation_delta);
                self.quads[i].set_rotation(if i == 0 { -self.looking_dir } else { self.segments[i]-self.segments[i-1] }.euler() - PI / 2.0);
                renderer.draw(&mut self.quads[i]).expect("unable to draw quad");
            }
        }
        self.interpolation_delta = (time.delta as f32 * WORM_SPEED * 20.0 + self.interpolation_delta).min(1.0);
    }

    fn remove(&mut self, _messenger: &mut ClientMessenger, mut renderer: Nullable<&mut Renderer>, _store: &mut DataStore) {
        for quad in &mut self.quads {
            renderer.remove(quad)
        }
    }
}