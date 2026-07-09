use dashmap::{DashMap, mapref::one::RefMut};
use std::sync::{
    atomic::{AtomicI32, Ordering},
    nonpoison::Mutex,
};

use cgmath::{Vector2, Vector3};
use noise::Perlin;
use rand::{SeedableRng, rngs::StdRng};
use tokio::sync::mpsc;

use crate::{
    net::framing::FramedConn,
    proto::packets::play::{
        sc_chunk_batch_finished::ScChunkBatchFinished, sc_chunk_batch_start::ScChunkBatchStart,
        sc_level_chunk_with_light::ScLevelChunkWithLight, sc_set_center_chunk::ScSetCenterChunk,
    },
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
    pub chunks: DashMap<ChunkPos, Chunk>,
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
            chunks: DashMap::new(),
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

    fn gen_chunk(pos: ChunkPos, seed: u64) -> Chunk {
        let mut params = ChunkGenerationParams {
            cx: pos.x,
            cz: pos.y,
            random: &mut StdRng::seed_from_u64(determine_chunk_seed(seed, pos.x, pos.y)),
            noise: &mut Perlin::new(seed as u32),
        };

        Chunk::new(pos.x, pos.y, &mut params)
    }

    #[must_use]
    pub fn get_chunk(&self, pos: ChunkPos) -> RefMut<'_, Vector2<i32>, Chunk> {
        self.chunks
            .entry(pos)
            .or_insert_with(|| Self::gen_chunk(pos, self.seed))
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

        let (chunks_tx, mut chunks_rx) = mpsc::unbounded_channel();
        let mut handles = vec![];

        let radius = self.view_distance;
        let chunk_count = AtomicI32::new(0);

        for cx in -radius..=radius {
            for cz in -radius..=radius {
                let nx = center_x + cx;
                let nz = center_z + cz;
                let pos = Vector2::new(nx, nz);

                if self.can_send_chunk(pos) {
                    if !has_sent_chunks {
                        conn.write_pkt(ScSetCenterChunk::new(center_x, center_z))
                            .await?;
                        conn.write_pkt(ScChunkBatchStart::new()).await?;
                        has_sent_chunks = true;
                    }

                    if let Some(entry) = self.chunks.get(&pos) {
                        let payload = { entry.value().encode() }?;
                        drop(entry);
                        conn.write_pkt(ScLevelChunkWithLight::new(payload.to_vec()))
                            .await?;
                    } else {
                        let chunks_tx = chunks_tx.clone();
                        let seed = self.seed;

                        handles.push(tokio::spawn(async move {
                            let chunk = Self::gen_chunk(pos, seed);
                            let _ = chunks_tx.send((pos, chunk));
                        }));
                    }

                    chunk_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        }

        for handle in handles {
            handle.await?;
        }

        chunks_rx.close();

        while let Some((pos, chunk)) = chunks_rx.recv().await {
            conn.write_pkt(ScLevelChunkWithLight::new(chunk.encode()?.to_vec()))
                .await?;
            self.chunks.insert(pos, chunk);
        }

        if has_sent_chunks {
            conn.write_pkt(ScChunkBatchFinished::new(
                chunk_count.load(Ordering::SeqCst),
            ))
            .await?;
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
