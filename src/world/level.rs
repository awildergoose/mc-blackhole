use std::collections::HashMap;

use cgmath::Vector2;

use crate::world::{chunk::Chunk, entity::Entity};

pub type ChunkPos = Vector2<i32>;

pub struct Level {
    pub entities: Vec<Box<dyn Entity>>,
    pub chunks: HashMap<ChunkPos, Chunk>,
}

impl Level {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: vec![],
            chunks: HashMap::new(),
        }
    }

    pub fn get_chunk(&mut self, pos: ChunkPos) -> &mut Chunk {
        self.chunks
            .entry(pos)
            .or_insert_with(|| Chunk::new(pos.x, pos.y))
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new()
    }
}
