use std::{collections::HashMap, sync::nonpoison::Mutex};

use cgmath::{Vector2, Vector3};
use noise::Perlin;
use rand::{SeedableRng, rngs::StdRng};

use crate::{
    net::framing::FramedConn,
    proto::packet_bytes::PacketBytes,
    world::{
        chunk::{Chunk, determine_chunk_seed},
        chunk_gen::ChunkGenerationParams,
        entity::{Entity, PlayerEntity},
    },
};

pub type ChunkPos = Vector2<i32>;
pub type EntityId = usize;

pub struct Level {
    pub entities: Vec<Mutex<Box<dyn Entity>>>,
    pub chunks: HashMap<ChunkPos, Chunk>,
    pub sent_chunks: Vec<ChunkPos>,
    pub seed: u64,
    pub view_distance: i32,
}

impl Level {
    #[must_use]
    pub fn new(view_distance: i32) -> Self {
        let seed = 69420;

        Self {
            entities: vec![],
            chunks: HashMap::new(),
            sent_chunks: vec![],
            seed,
            view_distance,
        }
    }

    pub fn add_entity<T: Entity + 'static>(&mut self, entity: T) -> EntityId {
        self.entities.push(Mutex::new(Box::new(entity)));
        self.entities.len() - 1
    }

    #[allow(clippy::significant_drop_tightening)]
    pub fn with_entity<T: Entity + 'static, F, R>(&self, id: EntityId, f: F) -> Option<R>
    where
        F: FnOnce(&Self, &T) -> R,
    {
        let entity = self.entities.get(id)?.lock();
        let entity = entity.as_any().downcast_ref::<T>()?;

        Some(f(self, entity))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub fn with_entity_mut<T: Entity + 'static, F, R>(&self, id: EntityId, f: F) -> Option<R>
    where
        F: FnOnce(&Self, &mut T) -> R,
    {
        let mut entity = self.entities.get(id)?.lock();
        let entity = entity.as_any_mut().downcast_mut::<T>()?;

        Some(f(self, entity))
    }

    pub fn get_chunk(&mut self, pos: ChunkPos) -> &mut Chunk {
        self.chunks.entry(pos).or_insert_with(|| {
            let mut params = ChunkGenerationParams {
                cx: pos.x,
                cz: pos.y,
                random: &mut StdRng::seed_from_u64(determine_chunk_seed(self.seed, pos.x, pos.y)),
                noise: &mut Perlin::new(self.seed as u32),
            };

            Chunk::new(pos.x, pos.y, &mut params)
        })
    }

    pub fn can_send_chunk(&mut self, pos: ChunkPos) -> bool {
        if !self.sent_chunks.contains(&pos) {
            self.sent_chunks.push(pos);
            return true;
        }

        false
    }

    pub async fn update_player_position(
        &mut self,
        player: EntityId,
        conn: &mut FramedConn,
        position: Vector3<f64>,
    ) -> anyhow::Result<()> {
        self.with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
            p.update_position(position);
        });

        let mut has_sent_chunks = false;
        let center_x = (position.x / 16.0).floor() as i32;
        let center_z = (position.z / 16.0).floor() as i32;

        let radius = self.view_distance;
        for cx in -radius..=radius {
            for cz in -radius..=radius {
                let nx = center_x + cx;
                let nz = center_z + cz;

                if self.can_send_chunk(ChunkPos::new(nx, nz)) {
                    if !has_sent_chunks {
                        // set center
                        let mut body = PacketBytes::new();
                        body.put_var_int(center_x)?;
                        body.put_var_int(center_z)?;
                        conn.write_packet(0x5C, &body).await?;

                        // chunks begin
                        conn.write_packet(0x0C, &[]).await?;
                        has_sent_chunks = true;
                    }

                    conn.write_packet(0x2C, &self.get_chunk(Vector2::new(nx, nz)).encode()?)
                        .await?;
                }
            }
        }

        if has_sent_chunks {
            // chunks end
            conn.write_packet(0x0B, &[0x01]).await?;
        }

        let player_chunk = Vector2::new(
            (position.x / 16.0).floor() as i32,
            (position.z / 16.0).floor() as i32,
        );

        // unload far away chunks
        self.sent_chunks.retain(|chunk| {
            let dx = chunk.x - player_chunk.x;
            let dz = chunk.y - player_chunk.y;

            dx.abs() <= radius && dz.abs() <= radius
        });
        self.chunks.retain(|pos, _chunk| {
            let dx = pos.x - player_chunk.x;
            let dz = pos.y - player_chunk.y;

            dx.abs() <= radius && dz.abs() <= radius
        });

        Ok(())
    }
}
