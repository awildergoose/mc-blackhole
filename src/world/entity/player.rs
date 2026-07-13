use std::{any::Any, collections::HashSet};

use cgmath::{MetricSpace, Vector2, Vector3};
use tokio::task::JoinSet;

use crate::{
    AsyncTraitFn, async_trait_fn,
    net::handles::PacketWriterHandle,
    proto::{
        packet_bytes::PacketBytes,
        packets::play::{
            GameMode, sc_chunk_batch_finished::ScChunkBatchFinished,
            sc_chunk_batch_start::ScChunkBatchStart, sc_forget_level_chunk::ScForgetLevelChunk,
            sc_level_chunk_with_light::ScLevelChunkWithLight,
            sc_set_center_chunk::ScSetCenterChunk,
        },
    },
    world::{
        entity::{Entity, EntityBase},
        level::{ChunkPos, Level},
    },
};

pub struct PlayerEntity {
    base: EntityBase,
    pub prev_position: Vector3<f64>,
    pub position: Vector3<f64>,
    sent_chunks: HashSet<ChunkPos>,
    chunk_queue: Vec<ChunkPos>,
    pub packet_writer: PacketWriterHandle,
    pub flying: bool,
    pub game_mode: GameMode,
}

impl PlayerEntity {
    #[must_use]
    pub fn new(packet_writer: PacketWriterHandle) -> Self {
        Self {
            base: EntityBase::default(),
            prev_position: Vector3::new(0.0, 0.0, 0.0),
            position: Vector3::new(0.0, 0.0, 0.0),
            sent_chunks: HashSet::new(),
            chunk_queue: Vec::new(),
            packet_writer,
            flying: false,
            game_mode: GameMode::Survival,
        }
    }

    #[must_use]
    const fn get_chunk_position(&self) -> ChunkPos {
        let (cx, cz) = Level::world_to_chunk(
            self.position.x.floor() as i32,
            self.position.z.floor() as i32,
        );

        Vector2::new(cx, cz)
    }

    #[must_use]
    pub fn has_seen_chunk(&self, pos: ChunkPos) -> bool {
        self.sent_chunks.contains(&pos)
    }

    pub async fn kick(&self, reason: String) -> anyhow::Result<()> {
        let mut body = PacketBytes::new();
        body.put_u8(0x08)?; // TAG_String
        body.put_u8(0x00)?; // TAG_END?
        body.put_string(reason)?;

        self.packet_writer.write_packet(0x20, body.to_vec()).await?;

        Ok(())
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

    #[expect(clippy::too_many_lines)]
    fn tick<'a>(&'a mut self, level: &'a mut Level) -> AsyncTraitFn<'a, anyhow::Result<()>> {
        async_trait_fn!({
            // can you please not cheat, that'd be great :)
            if self.prev_position.xz().distance2(self.position.xz()) >= 4.0
                && !self.game_mode.can_fly()
            {
                // roblox work at a pizza place angry sound effect
                self.kick(">:(".to_owned()).await?;
                return Ok(());
            }
            self.prev_position = self.position;

            let mut has_sent_chunks = false;
            let center_x = (self.position.x / 16.0).floor() as i32;
            let center_z = (self.position.z / 16.0).floor() as i32;

            let mut join = JoinSet::new();

            let radius = level.view_distance;
            let mut chunk_count = 0;

            let player_chunk = self.get_chunk_position();

            self.chunk_queue.retain(|pos| {
                // prevent overflow crashes earlier by converting to i64
                let dx = i64::from(pos.x - player_chunk.x);
                let dz = i64::from(pos.y - player_chunk.y);
                let radius = i64::from(radius);

                dx * dx + dz * dz <= radius * radius
            });

            let precise_radius = i64::from(level.view_distance);

            for x in -radius..=radius {
                for z in -radius..=radius {
                    let dx = i64::from(x);
                    let dz = i64::from(z);
                    if dx * dx + dz * dz > precise_radius * precise_radius {
                        continue;
                    }

                    let pos = Vector2::new(player_chunk.x + x, player_chunk.y + z);

                    if !self.sent_chunks.contains(&pos) && !self.chunk_queue.contains(&pos) {
                        self.chunk_queue.push(pos);
                    }
                }
            }

            self.chunk_queue.sort_by_key(|p| {
                let dx = p.x - player_chunk.x;
                let dz = p.y - player_chunk.y;

                dx * dx + dz * dz
            });

            // 20 TPS over 2 seconds, decrease this to make more chunks be queued at once
            let ticks_to_fill = 1; //20 * 2;
            let chunks_per_tick = ((std::f64::consts::PI * f64::from(radius) * f64::from(radius))
                / f64::from(ticks_to_fill))
            .ceil() as usize;

            let amount = chunks_per_tick.min(5);
            for pos in self.chunk_queue.drain(..self.chunk_queue.len().min(amount)) {
                if !self.sent_chunks.insert(pos) {
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
                    join.spawn_blocking(move || Ok::<_, anyhow::Error>((pos, None)));
                } else {
                    let seed = level.seed;
                    let patches = level.patches.get(&pos).cloned().unwrap_or_default();

                    join.spawn_blocking(move || {
                        Ok::<_, anyhow::Error>((
                            pos,
                            Some(Level::generate_chunk(seed, pos, patches)),
                        ))
                    });
                }

                chunk_count += 1;
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
                    .write_pkt(ScChunkBatchFinished::new(chunk_count))
                    .await;
            }

            let player_chunk = Vector2::new(
                (self.position.x / 16.0).floor() as i32,
                (self.position.z / 16.0).floor() as i32,
            );

            // unload far away chunks
            // TODO: add a little margin so it doesn't unload chunks at chunk
            // borders, and then it reloads them, which would be inefficient
            let mut to_forget = vec![];

            level.chunks.retain(|pos, _chunk| {
                let dx = pos.x - player_chunk.x;
                let dz = pos.y - player_chunk.y;

                let res = dx * dx + dz * dz <= radius * radius;
                if !res && self.sent_chunks.contains(pos) {
                    to_forget.push(*pos);
                }
                res
            });
            self.sent_chunks.retain(|chunk| {
                let dx = chunk.x - player_chunk.x;
                let dz = chunk.y - player_chunk.y;

                dx * dx + dz * dz <= radius * radius
            });

            for pos in to_forget {
                let _ = self
                    .packet_writer
                    .write_pkt(ScForgetLevelChunk::new(pos.y, pos.x))
                    .await;
            }

            Ok(())
        })
    }
}
