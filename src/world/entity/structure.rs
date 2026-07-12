use std::any::Any;

use cgmath::{Vector2, Vector3};

use crate::{
    AsyncTraitFn, async_trait_fn,
    proto::packets::play::sc_bundle_delimiter::ScBundleDelimiter,
    world::{
        entity::{Entity, EntityBase, player::PlayerEntity},
        level::Level,
        palette::PaletteBlockKind,
    },
};

pub struct StructureEntity {
    base: EntityBase,
    position: Vector3<i32>,
    our_patches: Vec<(Vector2<i32>, u64)>,
    counter: i32,
    up: bool,
}

impl StructureEntity {
    #[must_use]
    pub fn new(position: Vector3<i32>) -> Self {
        Self {
            base: EntityBase::default(),
            position,
            our_patches: vec![],
            counter: 0,
            up: true,
        }
    }
}

impl Entity for StructureEntity {
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
            if level.tick_counter.is_multiple_of(4) {
                return Ok(());
            }

            // TODO: this is inefficient
            let mut to_unapply = vec![];

            for (cpos, idx) in &self.our_patches {
                let patches = level.patches.get(cpos);

                if let Some(patches) = patches
                    && let Some(patch) = patches.iter().find(|p| p.idx == *idx)
                {
                    to_unapply.push((*cpos, *patch));
                }
            }

            let mut bundles = vec![];

            for (cpos, patch) in &to_unapply {
                let chunk = level.get_chunk(*cpos);
                patch.unapply(chunk);

                for player in &level.players {
                    let seen = level
                        .with_entity::<PlayerEntity, _, _>(*player, |_level, player| {
                            player.has_seen_chunk(*cpos)
                        })
                        .await?;

                    if seen {
                        if !bundles.contains(player) {
                            let writer = level
                                .with_entity::<PlayerEntity, _, _>(*player, |_level, player| {
                                    player.packet_writer.clone()
                                })
                                .await?;
                            writer.write_pkt(ScBundleDelimiter::new()).await?;

                            bundles.push(*player);
                        }

                        level
                            .send_block_update(
                                *player,
                                cpos.x * 16 + patch.x as i32,
                                patch.y,
                                cpos.y * 16 + patch.z as i32,
                                patch.original,
                            )
                            .await?;
                    }
                }
            }

            drop(to_unapply);
            self.our_patches.clear();

            for i in -self.counter..self.counter {
                let (x, y, z) = (self.position.x, self.position.y + i, self.position.z);
                let idx = level
                    .set_block_perma(
                        x,
                        y,
                        z,
                        if y == self.position.y {
                            PaletteBlockKind::Bedrock
                        } else {
                            PaletteBlockKind::OakLog
                        },
                    )
                    .await?;
                self.our_patches
                    .push((Vector2::from(Level::world_to_chunk(x, z)), idx));
            }

            for player in &bundles {
                let writer = level
                    .with_entity::<PlayerEntity, _, _>(*player, |_level, player| {
                        player.packet_writer.clone()
                    })
                    .await?;
                writer.write_pkt(ScBundleDelimiter::new()).await?;
            }

            self.counter += if self.up { 1 } else { -1 };

            if self.counter >= 10 || self.counter <= 0 {
                self.up = !self.up;
            }

            Ok(())
        })
    }
}
