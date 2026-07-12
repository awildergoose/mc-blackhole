use std::any::Any;

use cgmath::{MetricSpace, Vector3};
use rand::{RngExt, SeedableRng, rngs::StdRng};

use crate::{
    AsyncTraitFn, async_trait_fn,
    world::{
        entity::{Entity, EntityBase, player::PlayerEntity},
        level::Level,
        palette::PaletteBlockKind,
    },
};

pub struct StructureEntity {
    base: EntityBase,
    pub position: Vector3<i32>,
    rng: StdRng,
    seeded: bool,
}

impl StructureEntity {
    #[must_use]
    pub fn new(position: Vector3<i32>) -> Self {
        Self {
            base: EntityBase::default(),
            position,
            rng: StdRng::seed_from_u64(0),
            seeded: false,
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
            if !self.seeded {
                self.rng = level.make_rng(112);
                self.seeded = true;
            }

            level
                .set_block_perma(
                    self.position.x,
                    self.position.y,
                    self.position.z,
                    PaletteBlockKind::OakPlanks,
                )
                .await?;

            let possible: [cgmath::Vector3<f64>; 6] = [
                Vector3::unit_x(),
                Vector3::unit_y(),
                Vector3::unit_z(),
                -Vector3::unit_x(),
                -Vector3::unit_y(),
                -Vector3::unit_z(),
            ];

            if self.rng.random_bool(0.5) {
                let player = *level
                    .players
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("failed to find a player"))?;
                let ppos = level
                    .with_entity_mut::<PlayerEntity, _, _>(player, |_level, player| player.position)
                    .await?;

                let us = Vector3::new(
                    f64::from(self.position.x),
                    f64::from(self.position.y),
                    f64::from(self.position.z),
                );

                let mut best_position = us;
                let mut best_distance = f64::MAX;

                for pos in possible {
                    let distance = (us + pos).distance(ppos);

                    if distance <= best_distance {
                        best_distance = distance;
                        best_position = us + pos;
                    }
                }

                self.position = Vector3::new(
                    best_position.x as i32,
                    best_position.y as i32,
                    best_position.z as i32,
                );
            } else {
                let pick = possible[self.rng.random_range(0..possible.len())];
                self.position += Vector3::new(pick.x as i32, pick.y as i32, pick.z as i32);
            }

            self.position.y = self.position.y.clamp(-64, 318);

            level
                .set_block_perma(
                    self.position.x,
                    self.position.y,
                    self.position.z,
                    PaletteBlockKind::OakLog,
                )
                .await?;

            Ok(())
        })
    }
}
