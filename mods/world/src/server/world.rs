

use std::rc::Rc;
use aeonetica_engine::{ClientId, EntityId, log};
use aeonetica_engine::networking::SendMode;
use aeonetica_engine::math::vector::Vector2;
use aeonetica_engine::util::id_map::{IdSet};
use aeonetica_engine::util::nullable::Nullable;
use aeonetica_server::ecs::Engine;
use aeonetica_server::ecs::entity::Entity;
use aeonetica_server::ecs::events::ConnectionListener;
use aeonetica_server::ecs::messaging::Messenger;
use aeonetica_server::ecs::module::Module;
use crate::client::WorldHandle;
use crate::common::{Chunk, Population, WorldView};
use crate::server::gen::GenProvider;
use crate::tiles::{Tile, FgTile};

pub const WORLD: &str = "WORLD";

pub(crate) struct ChunkHolder {
    further_x: Option<Box<ChunkHolder>>,
    further_y: Option<Box<ChunkHolder>>,
    chunk: Chunk,
    subscribed_players: IdSet
}

impl ChunkHolder {
    pub(crate) fn new(chunk_pos: Vector2<i32>) -> ChunkHolder {
        ChunkHolder {
            further_x: None,
            further_y: None,
            chunk: Chunk::new(chunk_pos),
            subscribed_players: Default::default()
        }
    }
}

pub struct World {
    pub(crate) generator: Rc<GenProvider>,
    origin_ne: ChunkHolder,
    origin_se: ChunkHolder,
    origin_nw: ChunkHolder,
    origin_sw: ChunkHolder,
    cached_chunk_pos: Vector2<i32>,
    cached_chunk_raw_ptr: usize
}

impl World {
    pub(crate) fn new_wold_entity(engine: &mut Engine, seed: u64) -> EntityId {
        let eid = engine.new_entity();
        engine.tag_entity(eid, WORLD);
        let entity: &mut Entity = &mut engine.mut_entity(&eid);
        entity.add_module(Messenger::new::<WorldHandle>());
        entity.mut_module::<Messenger>().register_receiver(World::request_world_chunk);

        entity.add_module(ConnectionListener::new(
            |id, engine, client| {
                log!("sent chunk whether they wanted or not: {client}");
                let messenger: &mut Messenger = &mut engine.mut_module_of(id);
                messenger.add_client(*client);
            },
            |_id, _engine, client| {
                log!("user said bye bye to world: {client}");

            }));
        let chunk_zero = ChunkHolder::new((0, 0).into());
        entity.add_module(World {
            generator: Rc::new(GenProvider::new(seed)),
            cached_chunk_pos: (0, 0).into(),
            cached_chunk_raw_ptr: &chunk_zero.chunk as *const Chunk as usize,
            origin_ne: chunk_zero,
            origin_se: ChunkHolder::new((0, -1).into()),
            origin_nw: ChunkHolder::new((-1, 0).into()),
            origin_sw: ChunkHolder::new((-1, -1).into()),
        });
        eid
    }

    pub fn get_tile_at(&mut self, pos: Vector2<i32>) -> Tile {
        self.get_chunk_at(World::chunk(pos)).get_tile(World::pos_in_chunk(pos))
    }

    pub fn set_tile_at(&mut self, pos: Vector2<i32>, t: Tile) {
        self.mut_chunk_at(World::chunk(pos)).set_tile(World::pos_in_chunk(pos), t)
    }

    pub fn get_fg_tile_at(&mut self, pos: Vector2<i32>) -> FgTile {
        self.get_chunk_at(World::chunk(pos)).get_fg_tile(World::pos_in_chunk(pos))
    }

    pub fn set_fg_tile_at(&mut self, pos: Vector2<i32>, t: FgTile) {
        self.mut_chunk_at(World::chunk(pos)).set_fg_tile(World::pos_in_chunk(pos), t)
    }

    pub fn mut_chunk_at(&mut self, chunk_pos: Vector2<i32>) -> &mut Chunk {
        self.mut_init_chunk_at(chunk_pos, Population::Finished)
    }


    pub fn mut_chunk_at_raw(&mut self, chunk_pos: Vector2<i32>) -> &mut Chunk {
        if self.cached_chunk_pos == chunk_pos {
            return unsafe {  &mut *(self.cached_chunk_raw_ptr as *mut Chunk) }
        }

        let mut cp = chunk_pos;
        let mut chunk_ref = match (chunk_pos.x >= 0, chunk_pos.y >= 0) {
            (true, true) => {
                &mut self.origin_ne
            },
            (true, false) => {
                cp.y = -cp.y - 1;
                &mut self.origin_se
            },
            (false, true) => {
                cp.x = -cp.x - 1;
                &mut self.origin_nw
            },
            (false, false) => {
                cp.x = -cp.x - 1;
                cp.y = -cp.y - 1;
                &mut self.origin_sw
            },
        };
        while cp.x > 0 {
            cp.x -= 1;
            if chunk_ref.further_x.is_none() {
                let mut pos = chunk_ref.chunk.chunk_pos;
                if chunk_pos.x < 0 { pos.x -= 1 }
                else { pos.x += 1 }
                chunk_ref.further_x = Some(Box::new(ChunkHolder::new(pos)))
            }
            chunk_ref = chunk_ref.further_x.as_mut().unwrap();
        }
        while cp.y > 0 {
            cp.y -= 1;
            if chunk_ref.further_y.is_none() {
                let mut pos = chunk_ref.chunk.chunk_pos;
                if chunk_pos.y < 0 { pos.y -= 1 }
                else { pos.y += 1 }
                chunk_ref.further_y = Some(Box::new(ChunkHolder::new(pos)))
            }
            chunk_ref = chunk_ref.further_y.as_mut().unwrap();
        }
        self.cached_chunk_pos = chunk_pos;
        self.cached_chunk_raw_ptr = &chunk_ref.chunk as *const Chunk as usize;
        &mut chunk_ref.chunk
    }

    pub fn get_chunk_at(&mut self, chunk_pos: Vector2<i32>) -> &Chunk {
        self.mut_chunk_at(chunk_pos)
    }

    pub(crate) fn request_world_chunk(id: &EntityId, engine: &mut Engine, client: &ClientId, chunk_pos: Vector2<i32>) {
        let chunk = engine.mut_module_of::<Self>(id).get_chunk_at(chunk_pos).clone();
        engine.mut_module_of::<Messenger>(id).call_client_fn_for(WorldHandle::receive_chunk_data, client, chunk, SendMode::Safe);
    }

    pub fn try_get_tile_no_gen(&self, pos: Vector2<i32>) -> Nullable<Tile> {
        Nullable::Value(self.try_get_chunk_no_gen(World::chunk(pos))?.get_tile(World::pos_in_chunk(pos)))
    }

    pub fn try_get_fg_tile_no_gen(&self, pos: Vector2<i32>) -> Nullable<FgTile> {
        Nullable::Value(self.try_get_chunk_no_gen(World::chunk(pos))?.get_fg_tile(World::pos_in_chunk(pos)))
    }

    pub fn try_get_water_tile_no_gen(&self, pos: Vector2<i32>) -> Nullable<u8> {
        Nullable::Value(self.try_get_chunk_no_gen(World::chunk(pos))?.get_water_tile(World::pos_in_chunk(pos)))
    }

    pub fn try_get_chunk_no_gen(&self, chunk_pos: Vector2<i32>) -> Nullable<&Chunk> {
        if self.cached_chunk_pos == chunk_pos {
            return Nullable::Value(unsafe { &*(self.cached_chunk_raw_ptr as *const Chunk) })
        }

        let mut cp = chunk_pos;
        let mut chunk_ref = match (chunk_pos.x >= 0, chunk_pos.y >= 0) {
            (true, true) => {
                &self.origin_ne
            },
            (true, false) => {
                cp.y = -cp.y - 1;
                &self.origin_se
            },
            (false, true) => {
                cp.x = -cp.x - 1;
                &self.origin_nw
            },
            (false, false) => {
                cp.x = -cp.x - 1;
                cp.y = -cp.y - 1;
                &self.origin_sw
            },
        };
        while cp.x > 0 {
            cp.x -= 1;
            if chunk_ref.further_x.is_none() {
                return Nullable::Null
            }
            chunk_ref = chunk_ref.further_x.as_ref().unwrap();
        }
        while cp.y > 0 {
            cp.y -= 1;
            if chunk_ref.further_y.is_none() {
                return Nullable::Null
            }
            chunk_ref = chunk_ref.further_y.as_ref().unwrap();
        }
        #[allow(invalid_reference_casting)] // FIXME: Use UnsafeCell
        let mut_self = unsafe { &mut *(self as *const Self as usize as *mut Self)};
        mut_self.cached_chunk_pos = chunk_pos;
        mut_self.cached_chunk_raw_ptr = &chunk_ref.chunk as *const Chunk as usize;
        Nullable::Value(&chunk_ref.chunk)
    }
}

impl WorldView for World {
    fn is_loaded(&self, pos: Vector2<i32>) -> bool {
        self.try_get_chunk_no_gen(World::chunk(pos)).is_value()
    }

    fn get_tile_or_null(&self, pos: Vector2<i32>) -> Nullable<Tile> {
        self.try_get_tile_no_gen(pos)
    }

    fn get_fg_tile_or_null(&self, pos: Vector2<i32>) -> Nullable<FgTile> {
        self.try_get_fg_tile_no_gen(pos)
    }

    fn get_water_tile_or_null(&self, pos: Vector2<i32>) -> Nullable<u8> {
        self.try_get_water_tile_no_gen(pos)
    }
}

impl Module for World {

}