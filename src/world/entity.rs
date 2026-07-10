use std::any::Any;

use uuid::Uuid;

use crate::{AsyncTraitFn, async_trait_fn, world::level::Level};

pub trait Entity: Send + Sync {
    fn tick<'a>(&'a mut self, level: &'a mut Level) -> AsyncTraitFn<'a, anyhow::Result<()>> {
        let _ = level;
        async_trait_fn!({ Ok(()) })
    }
    fn base(&self) -> &EntityBase;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct EntityBase {
    pub uuid: Uuid,
}

impl EntityBase {
    #[must_use]
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }
}

impl Default for EntityBase {
    fn default() -> Self {
        Self::new()
    }
}

pub mod player;
