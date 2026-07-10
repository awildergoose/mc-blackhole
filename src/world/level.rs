use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use cgmath::{Vector2, Vector3};
use noise::Perlin;
use rand::{SeedableRng, rngs::StdRng};

use crate::world::{
    chunk::{Chunk, determine_chunk_seed},
    chunk_gen::{ChunkGenerationParams, do_chunk_generation},
    entity::{Entity, player::PlayerEntity},
    palette::PaletteBlockKind,
};

pub type ChunkPos = Vector2<i32>;
pub type EntityId = usize;

#[derive(Debug, Clone, Copy)]
pub struct BlockPatch {
    pub x: u32,
    pub y: i32,
    pub z: u32,
    pub kind: PaletteBlockKind,
}

impl BlockPatch {
    #[must_use]
    pub const fn new(pos: Vector3<i32>, kind: PaletteBlockKind) -> Self {
        Self {
            x: pos.x as u32,
            y: pos.y,
            z: pos.z as u32,
            kind,
        }
    }

    #[must_use]
    pub const fn new2(x: u32, y: i32, z: u32, kind: PaletteBlockKind) -> Self {
        Self { x, y, z, kind }
    }

    #[inline]
    pub fn apply(&self, chunk: &mut Chunk) {
        chunk.set_block_local(self.x, self.y, self.z, self.kind);
    }
}

pub struct Level {
    pub entities: Vec<Arc<RwLock<Box<dyn Entity>>>>,
    pub chunks: HashMap<ChunkPos, Chunk>,
    pub patches: HashMap<ChunkPos, Vec<BlockPatch>>,
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
            chunks: HashMap::new(),
            patches: HashMap::new(),
            seed,
            view_distance,
            tick_counter: 0,
        }
    }

    pub fn add_entity<T: Entity + 'static>(&mut self, entity: T) -> EntityId {
        self.entities.push(Arc::new(RwLock::new(Box::new(entity))));
        self.entities.len() - 1
    }

    #[allow(clippy::significant_drop_tightening)]
    pub async fn with_entity<T: Entity + 'static, F, R>(&self, id: EntityId, f: F) -> Option<R>
    where
        F: FnOnce(&Self, &T) -> R,
    {
        let entity = self.entities.get(id)?.write().await;
        let entity = entity.as_any().downcast_ref::<T>()?;

        Some(f(self, entity))
    }

    #[allow(clippy::significant_drop_tightening)]
    pub async fn with_entity_mut<T: Entity + 'static, F, R>(&self, id: EntityId, f: F) -> Option<R>
    where
        F: FnOnce(&Self, &mut T) -> R,
    {
        let mut entity = self.entities.get(id)?.write().await;
        let entity = entity.as_any_mut().downcast_mut::<T>()?;

        Some(f(self, entity))
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
    #[allow(clippy::missing_panics_doc)]
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
                        self.set_block(wx, wy, wz, kind);
                    }
                }
            }
        }
    }

    #[must_use]
    pub const fn world_to_chunk_and_local(wx: i32, wz: i32) -> (i32, i32, u32, u32) {
        let cx = wx.div_euclid(16);
        let cz = wz.div_euclid(16);

        let lx = wx.rem_euclid(16) as u32;
        let lz = wz.rem_euclid(16) as u32;

        (cx, cz, lx, lz)
    }

    pub fn set_block(&mut self, wx: i32, wy: i32, wz: i32, kind: PaletteBlockKind) {
        let (cx, cz, lx, lz) = Self::world_to_chunk_and_local(wx, wz);

        self.get_chunk(Vector2::new(cx, cz))
            .set_block_local(lx, wy, lz, kind);
    }

    pub fn set_block_perma(&mut self, wx: i32, wy: i32, wz: i32, kind: PaletteBlockKind) {
        let (cx, cz, lx, lz) = Self::world_to_chunk_and_local(wx, wz);
        let patch = BlockPatch::new2(lx, wy, lz, kind);
        let cpos = Vector2::new(cx, cz);
        self.patches
            .entry(cpos)
            .and_modify(|v| v.push(patch))
            .or_insert_with(|| vec![patch]);

        if self.is_chunk_loaded(cpos) {
            self.set_block(wx, wy, wz, kind);
        }
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

    pub async fn update_player_position(
        &mut self,
        player: EntityId,
        position: Vector3<f64>,
    ) -> anyhow::Result<()> {
        self.with_entity_mut::<PlayerEntity, _, _>(player, |_, p| {
            p.update_position(position);
        })
        .await;

        Ok(())
    }
}
