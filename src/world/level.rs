use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use cgmath::Vector2;

use crate::world::{chunk::Chunk, entity::Entity};

pub type ChunkPos = Vector2<i32>;

pub struct Level {
    pub entities: Vec<Box<dyn Entity>>,
    pub chunks: HashMap<ChunkPos, Chunk>,
    pub chunk_send_timer: Instant,
}

impl Level {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entities: vec![],
            chunks: HashMap::new(),
            chunk_send_timer: Instant::now(),
        }
    }

    pub fn get_chunk(&mut self, pos: ChunkPos) -> &mut Chunk {
        self.chunks
            .entry(pos)
            .or_insert_with(|| Chunk::new(pos.x, pos.y))
    }

    // TODO:
    // take in chunk position, and check if we've already
    // sent this chunk before or not
    pub fn can_send_chunks(&mut self) -> bool {
        if self.chunk_send_timer.elapsed() >= Duration::from_secs_f32(0.5) {
            self.chunk_send_timer = Instant::now();
            return true;
        }

        false
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::new()
    }
}
