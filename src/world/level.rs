use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use cgmath::{Vector2, Vector3};
use noise::Perlin;
use rand::{SeedableRng, rngs::StdRng};

use crate::{
    codecs::position::Position,
    proto::packets::play::sc_block_update::ScBlockUpdate,
    world::{
        chunk::{Chunk, determine_chunk_seed},
        chunk_gen::{ChunkGenerationParams, do_chunk_generation},
        entity::{Entity, player::PlayerEntity},
        palette::PaletteBlockKind,
    },
};

pub type ChunkPos = Vector2<i32>;
pub type EntityId = usize;

#[derive(Debug, Clone, Copy)]
pub struct BlockPatch {
    pub idx: u64,
    pub x: u32,
    pub y: i32,
    pub z: u32,
    pub kind: PaletteBlockKind,
    pub original: PaletteBlockKind,
}

impl BlockPatch {
    #[must_use]
    pub const fn new(
        idx: u64,
        pos: Vector3<i32>,
        kind: PaletteBlockKind,
        original: PaletteBlockKind,
    ) -> Self {
        Self {
            idx,
            x: pos.x as u32,
            y: pos.y,
            z: pos.z as u32,
            kind,
            original,
        }
    }

    #[must_use]
    pub const fn new2(
        idx: u64,
        x: u32,
        y: i32,
        z: u32,
        kind: PaletteBlockKind,
        original: PaletteBlockKind,
    ) -> Self {
        Self {
            idx,
            x,
            y,
            z,
            kind,
            original,
        }
    }

    #[inline]
    pub fn apply(&self, chunk: &mut Chunk) {
        chunk.set_block_local(self.x, self.y, self.z, self.kind);
    }

    #[inline]
    pub fn unapply(&self, chunk: &mut Chunk) {
        chunk.set_block_local(self.x, self.y, self.z, self.original);
    }
}

pub struct Level {
    pub entities: Vec<Arc<RwLock<Box<dyn Entity>>>>,
    pub players: Vec<EntityId>,
    pub chunks: HashMap<ChunkPos, Chunk>,
    pub patches: HashMap<ChunkPos, Vec<BlockPatch>>,
    pub patch_counter: u64,
    pub seed: u64,
    pub view_distance: i32,
    pub tick_counter: u64,
}

impl Level {
    #[must_use]
    pub fn new(view_distance: i32) -> Self {
        let seed = 69420;

        Self {
            entities: vec![],
            players: vec![],
            chunks: HashMap::new(),
            patches: HashMap::new(),
            patch_counter: 0,
            seed,
            view_distance,
            tick_counter: 0,
        }
    }

    #[must_use]
    pub fn make_rng(&self, offset: u64) -> StdRng {
        StdRng::seed_from_u64(self.seed + offset)
    }

    pub fn add_player(&mut self, entity: PlayerEntity) -> EntityId {
        let id = self.add_entity(entity);
        self.players.push(id);
        id
    }

    pub fn add_entity<T: Entity + 'static>(&mut self, entity: T) -> EntityId {
        self.entities.push(Arc::new(RwLock::new(Box::new(entity))));
        self.entities.len() - 1
    }

    #[expect(clippy::significant_drop_tightening)]
    pub async fn with_entity<T: Entity + 'static, F, R>(
        &self,
        id: EntityId,
        f: F,
    ) -> anyhow::Result<R>
    where
        F: FnOnce(&Self, &T) -> R,
    {
        let entity = self
            .entities
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("failed to find entity"))?
            .write()
            .await;
        let entity = entity
            .as_any()
            .downcast_ref::<T>()
            .ok_or_else(|| anyhow::anyhow!("failed to downcast entity to T"))?;

        Ok(f(self, entity))
    }

    #[expect(clippy::significant_drop_tightening)]
    pub async fn with_entity_mut<T: Entity + 'static, F, R>(
        &self,
        id: EntityId,
        f: F,
    ) -> anyhow::Result<R>
    where
        F: FnOnce(&Self, &mut T) -> R,
    {
        let mut entity = self
            .entities
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("failed to find entity"))?
            .write()
            .await;
        let entity = entity
            .as_any_mut()
            .downcast_mut::<T>()
            .ok_or_else(|| anyhow::anyhow!("failed to downcast entity to T"))?;

        Ok(f(self, entity))
    }

    #[must_use]
    pub fn generate_chunk(seed: u64, pos: ChunkPos, patches: Vec<BlockPatch>) -> Chunk {
        let mut chunk = Chunk::new(pos.x, pos.y);
        let mut random = StdRng::seed_from_u64(determine_chunk_seed(seed, pos.x, pos.y));
        let mut noise = Perlin::new(seed as u32);
        let mut params = ChunkGenerationParams {
            cx: pos.x,
            cz: pos.y,
            chunk: &mut chunk,
            random: &mut random,
            noise: &mut noise,
        };

        do_chunk_generation(&mut params);

        for patch in patches {
            patch.apply(&mut chunk);
        }

        chunk
    }

    #[must_use]
    pub fn is_chunk_loaded(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    #[must_use]
    #[expect(clippy::missing_panics_doc)]
    // This will *never* panic, I think, I hope.
    pub fn get_chunk(&mut self, pos: ChunkPos) -> &mut Chunk {
        if !self.chunks.contains_key(&pos) {
            let chunk = Self::generate_chunk(
                self.seed,
                pos,
                self.patches.get(&pos).cloned().unwrap_or_default(),
            );
            self.chunks.insert(pos, chunk);
        }

        self.chunks.get_mut(&pos).unwrap()
    }

    #[expect(clippy::cast_precision_loss)]
    pub async fn add_metaball(
        &mut self,
        center_world: Vector3<i32>,
        radius: f32,
        iso: f32,
        kind: PaletteBlockKind,
        perma: bool,
    ) -> anyhow::Result<()> {
        let r2 = radius * radius;
        let eps = 1e-4f32;

        let (bx, by, bz) = (center_world.x, center_world.y, center_world.z);

        let min_x = (bx as f32 - radius - 1.0).floor() as i32;
        let max_x = (bx as f32 + radius + 1.0).ceil() as i32;
        let min_y = (by as f32 - radius - 1.0).floor() as i32;
        let max_y = (by as f32 + radius + 1.0).ceil() as i32;
        let min_z = (bz as f32 - radius - 1.0).floor() as i32;
        let max_z = (bz as f32 + radius + 1.0).ceil() as i32;

        // TODO: make it melt with the surrounding ground
        for wy in min_y..=max_y {
            for wx in min_x..=max_x {
                for wz in min_z..=max_z {
                    let dx = (wx - bx) as f32;
                    let dy = (wy - by) as f32;
                    let dz = (wz - bz) as f32;

                    let d2 = dz.mul_add(dz, dy.mul_add(dy, dx * dx));
                    let f = r2 / (d2 + eps);

                    if f >= iso {
                        if perma {
                            self.set_block_perma(wx, wy, wz, kind).await?;
                        } else {
                            self.set_block(wx, wy, wz, kind).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    #[must_use]
    pub const fn world_to_chunk_and_local(wx: i32, wz: i32) -> (i32, i32, u32, u32) {
        let cx = wx.div_euclid(16);
        let cz = wz.div_euclid(16);

        let lx = wx.rem_euclid(16) as u32;
        let lz = wz.rem_euclid(16) as u32;

        (cx, cz, lx, lz)
    }

    #[must_use]
    pub const fn world_to_chunk_local(wx: i32, wz: i32) -> (u32, u32) {
        let lx = wx.rem_euclid(16) as u32;
        let lz = wz.rem_euclid(16) as u32;

        (lx, lz)
    }

    #[must_use]
    pub const fn world_to_chunk(wx: i32, wz: i32) -> (i32, i32) {
        let cx = wx.div_euclid(16);
        let cz = wz.div_euclid(16);

        (cx, cz)
    }

    pub async fn set_block(
        &mut self,
        wx: i32,
        wy: i32,
        wz: i32,
        kind: PaletteBlockKind,
    ) -> anyhow::Result<()> {
        let (cx, cz, lx, lz) = Self::world_to_chunk_and_local(wx, wz);
        let cpos = Vector2::new(cx, cz);

        self.get_chunk(cpos).set_block_local(lx, wy, lz, kind);

        for player in &self.players {
            let seen = self
                .with_entity::<PlayerEntity, _, _>(*player, |_level, player| {
                    player.has_seen_chunk(cpos)
                })
                .await?;

            if seen {
                self.send_block_update(*player, wx, wy, wz, kind).await?;
            }
        }

        Ok(())
    }

    pub async fn send_block_update(
        &self,
        player: EntityId,
        wx: i32,
        wy: i32,
        wz: i32,
        kind: PaletteBlockKind,
    ) -> anyhow::Result<()> {
        let location = Position::from_pos(i64::from(wx), i64::from(wy), i64::from(wz));
        let writer = self
            .with_entity::<PlayerEntity, _, _>(player, |_level, player| {
                player.packet_writer.clone()
            })
            .await?;
        writer
            .write_pkt(ScBlockUpdate::new(location, kind.as_minecraft_id() as i32))
            .await?;
        Ok(())
    }

    fn make_block_patch(
        &mut self,
        wx: i32,
        wy: i32,
        wz: i32,
        kind: PaletteBlockKind,
    ) -> BlockPatch {
        let (cx, cz, lx, lz) = Self::world_to_chunk_and_local(wx, wz);

        self.patch_counter += 1;
        BlockPatch::new2(
            self.patch_counter,
            lx,
            wy,
            lz,
            kind,
            self.get_chunk(Vector2::new(cx, cz))
                .get_block_local(lx, wy, lz),
        )
    }

    pub async fn set_block_perma(
        &mut self,
        wx: i32,
        wy: i32,
        wz: i32,
        kind: PaletteBlockKind,
    ) -> anyhow::Result<u64> {
        let patch = self.make_block_patch(wx, wy, wz, kind);
        let (cx, cz) = Self::world_to_chunk(wx, wz);
        let cpos = Vector2::new(cx, cz);

        self.patches
            .entry(cpos)
            .and_modify(|v| v.push(patch))
            .or_insert_with(|| vec![patch]);
        if self.is_chunk_loaded(cpos) {
            self.set_block(wx, wy, wz, kind).await?;
        }

        Ok(patch.idx)
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

    pub async fn tick(&mut self) -> anyhow::Result<()> {
        let entities = self.entities.clone();

        for ent in &entities {
            let mut e = ent.write().await;
            e.tick(self).await?;
            drop(e);
        }

        self.tick_counter += 1;
        Ok(())
    }
}
