use aeonetica_engine::nanoserde::{SerBin, DeBin};
use aeonetica_engine::nanoserde;
use aeonetica_engine::math::vector::Vector2;
use aeonetica_engine::util::nullable::Nullable;
use crate::tiles::Tile;

pub const CHUNK_SIZE: usize = 16;
pub const GRAVITY: f32 = -20.0;

#[derive(SerBin, DeBin, Debug, Clone)]
pub enum Population {
    Uninit,
    Finished
}

#[derive(SerBin, DeBin, Debug, Clone)]
pub struct Chunk {
    pub population: Population,
    pub chunk_pos: Vector2<i32>,
    pub tiles: [Tile; CHUNK_SIZE*CHUNK_SIZE]
}

impl Chunk {
    pub(crate) fn new(chunk_pos: Vector2<i32>) -> Self {
        Self {
            population: Population::Uninit,
            chunk_pos,
            tiles: [Tile::Wall; CHUNK_SIZE*CHUNK_SIZE]
        }
    }

    pub(crate) fn tiles(&self) -> &[Tile; CHUNK_SIZE * CHUNK_SIZE] {
        &self.tiles
    }

    pub fn get_tile(&self, pos: Vector2<i32>) -> Tile {
        self.tiles[pos.y as usize * CHUNK_SIZE + pos.x as usize]
    }

    pub fn mut_tile(&mut self, pos: Vector2<i32>) -> &mut Tile {
        &mut self.tiles[pos.y as usize * CHUNK_SIZE + pos.x as usize]
    }

    pub fn set_tile(&mut self, pos: Vector2<i32>, tile: Tile) {
        self.tiles[pos.y as usize * CHUNK_SIZE + pos.x as usize] = tile
    }
}

/// THis trait is used for both client and server.
/// This trait is read only, as the name implies.
///
/// The methods rely on the position being loaded when called client side and being alreaddy generted when being called serverside.
/// For ease of handling, they return a sensible default (see doc comments).
/// This should ntot be a problem most of the time, but you can use [`WorldView::is_loaded`]
/// to find out whether a position is available.
pub trait WorldView {
    fn chunk(pos: Vector2<i32>) -> Vector2<i32> {
        (pos.to_f32() / CHUNK_SIZE as f32).floor().to_i32()
    }

    fn pos_in_chunk(pos: Vector2<i32>) -> Vector2<i32> {
        (pos % CHUNK_SIZE as i32 + (CHUNK_SIZE as i32, CHUNK_SIZE as i32).into()) % CHUNK_SIZE as i32
    }

    fn get_tile_or_null(&self, pos: Vector2<i32>) -> Nullable<Tile>;
    /// Returns [`Tile::Wall`] if not loaded
    fn get_tile(&self, pos: Vector2<i32>) -> Tile {
        self.get_tile_or_null(pos).unwrap_or(Tile::Wall)
    }
    fn is_loaded(&self, pos: Vector2<i32>) -> bool;

    /// Returns [`true`] if the aabb bounding box collides with a tile.
    fn overlap_aabb(&self, pos: Vector2<f32>, size: Vector2<f32>) -> bool {
        let start = pos.floor();
        let max_size = (size - (pos - start)).ceil().to_i32();
        let start = start.to_i32();
        for x in start.x..=(start.x+max_size.x) {
            for y in start.y..=(start.y+max_size.y) {
                if self.get_tile((x, y).into()).is_solid() { return true }
            }
        }
        false
    }

    /// Tries to slide along walls instead of stopping movement alltogether.
    fn calc_move(&self, pos: &mut Vector2<f32>, size: Vector2<f32>, delta: Vector2<f32>) {
        let delta_x = (delta.x, 0.0).into();
        let delta_y = (0.0, delta.y).into();
        if !self.overlap_aabb(*pos + delta, size) {
            *pos += delta;
        } else if !self.overlap_aabb(*pos + delta_x, size) {
            *pos += delta_x;
        } else if !self.overlap_aabb(*pos + delta_y, size) {
            *pos += delta_y;
        }
    }
}