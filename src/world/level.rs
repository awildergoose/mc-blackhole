use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicI32, Ordering},
        nonpoison::Mutex,
    },
};

use cgmath::{Vector2, Vector3};
use noise::Perlin;
use rand::{SeedableRng, rngs::StdRng};
use tokio::task::JoinSet;

use crate::{
    net::handles::PacketWriterHandle,
    proto::packets::play::{
        sc_chunk_batch_finished::ScChunkBatchFinished, sc_chunk_batch_start::ScChunkBatchStart,
        sc_level_chunk_with_light::ScLevelChunkWithLight, sc_set_center_chunk::ScSetCenterChunk,
    },
    world::{
        chunk::{Chunk, determine_chunk_seed},
        chunk_gen::{ChunkGenerationParams, do_chunk_generation},
        entity::{Entity, PlayerEntity},
        palette::PaletteBlockKind,
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

    pub fn generate_chunk(&mut self, pos: ChunkPos) -> Chunk {
        let mut chunk = Chunk::new(pos.x, pos.y);
        let mut random = StdRng::seed_from_u64(determine_chunk_seed(self.seed, pos.x, pos.y));
        let mut noise = Perlin::new(self.seed as u32);

        let mut params = ChunkGenerationParams {
            cx: pos.x,
            cz: pos.y,
            chunk: &mut chunk,
            random: &mut random,
            noise: &mut noise,
        };

        do_chunk_generation(&mut params);

        chunk
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    // This will *never* panic, I think, I hope.
    pub fn get_chunk(&mut self, pos: ChunkPos) -> &mut Chunk {
        if !self.chunks.contains_key(&pos) {
            let chunk = self.generate_chunk(pos);
            self.chunks.insert(pos, chunk);
        }

        self.chunks.get_mut(&pos).unwrap()
    }

    #[allow(clippy::cast_precision_loss)]
    pub fn add_metaball(
        &mut self,
        center_world: Vector3<i32>,
        radius: f32,
        iso: f32,
        kind: PaletteBlockKind,
    ) {
        let r2 = radius * radius;
        let eps = 1e-4f32;

        let (bx, by, bz) = (center_world.x, center_world.y, center_world.z);

        let min_x = (bx as f32 - radius - 1.0).floor() as i32;
        let max_x = (bx as f32 + radius + 1.0).ceil() as i32;
        let min_y = (by as f32 - radius - 1.0).floor() as i32;
        let max_y = (by as f32 + radius + 1.0).ceil() as i32;
        let min_z = (bz as f32 - radius - 1.0).floor() as i32;
        let max_z = (bz as f32 + radius + 1.0).ceil() as i32;

        for wy in min_y..=max_y {
            for wx in min_x..=max_x {
                for wz in min_z..=max_z {
                    let dx = (wx - bx) as f32;
                    let dy = (wy - by) as f32;
                    let dz = (wz - bz) as f32;

                    let d2 = dz.mul_add(dz, dy.mul_add(dy, dx * dx));
                    let f = r2 / (d2 + eps);

                    if f >= iso {
                        self.set_block(wx, wy, wz, kind);
                    }
                }
            }
        }
    }

    pub fn set_block(&mut self, wx: i32, wy: i32, wz: i32, kind: PaletteBlockKind) {
        let cx = wx.div_euclid(16);
        let cz = wz.div_euclid(16);

        let lx = wx.rem_euclid(16) as u32;
        let lz = wz.rem_euclid(16) as u32;

        self.get_chunk(Vector2::new(cx, cz))
            .set_block_local(lx, wy, lz, kind);
    }

    #[must_use]
    pub fn get_block(&mut self, wx: i32, wy: i32, wz: i32) -> PaletteBlockKind {
        let chunk_x = wx.rem_euclid(16);
        let chunk_z = wz.rem_euclid(16);

        let lx = wx.rem_euclid(16) as u32;
        let lz = wz.rem_euclid(16) as u32;

        self.get_chunk(ChunkPos::new(chunk_x, chunk_z))
            .get_block_local(lx, wy, lz)
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
        position: Vector3<f64>,
        writer: PacketWriterHandle,
    ) -> anyhow::Result<()> {
        self.with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
            p.update_position(position);
        });

        let mut has_sent_chunks = false;
        let center_x = (position.x / 16.0).floor() as i32;
        let center_z = (position.z / 16.0).floor() as i32;

        let mut join = JoinSet::new();

        let radius = self.view_distance;
        let chunk_count = AtomicI32::new(0);

        println!("updating player position");

        for cx in -radius..=radius {
            for cz in -radius..=radius {
                let nx = center_x + cx;
                let nz = center_z + cz;
                let pos = Vector2::new(nx, nz);

                if !self.can_send_chunk(pos) {
                    continue;
                }

                if !has_sent_chunks {
                    let _ = writer
                        .write_pkt(ScSetCenterChunk::new(center_x, center_z))
                        .await;
                    let _ = writer.write_pkt(ScChunkBatchStart::new()).await;
                    has_sent_chunks = true;
                }

                if let Some(entry) = self.chunks.get(&pos) {
                    let chunk = entry.clone();
                    join.spawn(async move { Ok::<_, anyhow::Error>((pos, chunk, false)) });
                } else {
                    let seed = self.seed;
                    join.spawn(async move {
                        let mut chunk = Chunk::new(pos.x, pos.y);
                        let mut rng =
                            StdRng::seed_from_u64(determine_chunk_seed(seed, pos.x, pos.y));
                        let mut noise = Perlin::new(seed as u32);
                        let mut params = ChunkGenerationParams {
                            cx: pos.x,
                            cz: pos.y,
                            chunk: &mut chunk,
                            random: &mut rng,
                            noise: &mut noise,
                        };
                        do_chunk_generation(&mut params);
                        Ok::<_, anyhow::Error>((pos, chunk, true))
                    });
                }

                chunk_count.fetch_add(1, Ordering::SeqCst);
            }
        }

        while let Some(res) = join.join_next().await {
            let (pos, chunk, insert) = res??;

            if writer
                .write_pkt(ScLevelChunkWithLight::new(chunk.encode()?))
                .await
                .is_err()
            {
                return Ok(());
            }

            if insert {
                self.chunks.insert(pos, chunk);
            }
        }

        println!("finished receiving chunks");

        if has_sent_chunks {
            println!("sending finish");
            let _ = writer
                .write_pkt(ScChunkBatchFinished::new(
                    chunk_count.load(Ordering::SeqCst),
                ))
                .await;
            println!("sent finish");
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

        println!("removed unused chunks");

        Ok(())
    }
}

impl Drop for Level {
    fn drop(&mut self) {
        eprintln!(
            "Dropping Level: chunks={}, sent_chunks={}",
            self.chunks.len(),
            self.sent_chunks.len()
        );
    }
}
