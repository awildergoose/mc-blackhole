use std::{
    any::Any,
    sync::atomic::{AtomicI32, Ordering},
};

use cgmath::{Vector2, Vector3};
use tokio::task::JoinSet;

use crate::{
    AsyncTraitFn, async_trait_fn,
    net::handles::PacketWriterHandle,
    proto::packets::play::{
        sc_chunk_batch_finished::ScChunkBatchFinished, sc_chunk_batch_start::ScChunkBatchStart,
        sc_level_chunk_with_light::ScLevelChunkWithLight, sc_set_center_chunk::ScSetCenterChunk,
    },
    world::{
        entity::{Entity, EntityBase},
        level::{ChunkPos, Level},
    },
};

pub struct PlayerEntity {
    base: EntityBase,
    position: Vector3<f64>,
    packet_writer: PacketWriterHandle,
    sent_chunks: Vec<ChunkPos>,
}

impl PlayerEntity {
    #[must_use]
    pub fn new(packet_writer: PacketWriterHandle) -> Self {
        Self {
            base: EntityBase::default(),
            position: Vector3::new(0.0, 0.0, 0.0),
            packet_writer,
            sent_chunks: Vec::new(),
        }
    }

    pub const fn update_position(&mut self, position: Vector3<f64>) {
        self.position = position;
    }

    #[must_use]
    pub const fn get_position(&self) -> Vector3<f64> {
        self.position
    }

    pub fn can_send_chunk(&mut self, pos: ChunkPos) -> bool {
        if !self.sent_chunks.contains(&pos) {
            self.sent_chunks.push(pos);
            return true;
        }

        false
    }
}

impl Entity for PlayerEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn tick<'a>(&'a mut self, level: &'a mut Level) -> AsyncTraitFn<'a, anyhow::Result<()>> {
        async_trait_fn!({
            let mut has_sent_chunks = false;
            let center_x = (self.position.x / 16.0).floor() as i32;
            let center_z = (self.position.z / 16.0).floor() as i32;

            let mut join = JoinSet::new();

            // TODO: gradually do this in other ticks
            let radius = level.view_distance;
            let chunk_count = AtomicI32::new(0);

            for cx in -radius..=radius {
                for cz in -radius..=radius {
                    let nx = center_x + cx;
                    let nz = center_z + cz;
                    let pos = Vector2::new(nx, nz);

                    if !self.can_send_chunk(pos) {
                        continue;
                    }

                    if !has_sent_chunks {
                        let _ = self
                            .packet_writer
                            .write_pkt(ScSetCenterChunk::new(center_x, center_z))
                            .await;
                        let _ = self.packet_writer.write_pkt(ScChunkBatchStart::new()).await;
                        has_sent_chunks = true;
                    }

                    if level.chunks.contains_key(&pos) {
                        join.spawn(async move { Ok::<_, anyhow::Error>((pos, None)) });
                    } else {
                        let seed = level.seed;
                        let patches = level.patches.get(&pos).cloned().unwrap_or_default();

                        join.spawn(async move {
                            Ok::<_, anyhow::Error>((
                                pos,
                                Some(Level::generate_chunk(seed, pos, patches)),
                            ))
                        });
                    }

                    chunk_count.fetch_add(1, Ordering::SeqCst);
                }
            }

            while let Some(res) = join.join_next().await {
                let (pos, generated_chunk) = res??;

                if let Some(chunk) = generated_chunk {
                    if self
                        .packet_writer
                        .write_pkt(ScLevelChunkWithLight::new(chunk.encode()?))
                        .await
                        .is_err()
                    {
                        return Ok(());
                    }

                    level.chunks.insert(pos, chunk);
                } else if self
                    .packet_writer
                    .write_pkt(ScLevelChunkWithLight::new(
                        level
                            .chunks
                            .get(&pos)
                            .ok_or_else(|| anyhow::anyhow!("channel sent invalid chunk"))?
                            .encode()?,
                    ))
                    .await
                    .is_err()
                {
                    return Ok(());
                }
            }

            if has_sent_chunks {
                let _ = self
                    .packet_writer
                    .write_pkt(ScChunkBatchFinished::new(
                        chunk_count.load(Ordering::SeqCst),
                    ))
                    .await;
            }

            let player_chunk = Vector2::new(
                (self.position.x / 16.0).floor() as i32,
                (self.position.z / 16.0).floor() as i32,
            );

            // unload far away chunks
            self.sent_chunks.retain(|chunk| {
                let dx = chunk.x - player_chunk.x;
                let dz = chunk.y - player_chunk.y;

                dx.abs() <= radius && dz.abs() <= radius
            });
            level.chunks.retain(|pos, _chunk| {
                let dx = pos.x - player_chunk.x;
                let dz = pos.y - player_chunk.y;

                dx.abs() <= radius && dz.abs() <= radius
            });

            Ok(())
        })
    }
}
