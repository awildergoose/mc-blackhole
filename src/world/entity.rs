use std::any::Any;

use cgmath::Vector3;
use uuid::Uuid;

pub trait Entity: Send + Sync {
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

pub struct PlayerEntity {
    base: EntityBase,
    position: Vector3<f64>,
}

impl PlayerEntity {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: EntityBase::default(),
            position: Vector3::new(0.0, 0.0, 0.0),
        }
    }

    pub const fn update_position(&mut self, position: Vector3<f64>) {
        self.position = position;
    }

    #[must_use]
    pub const fn get_position(&self) -> Vector3<f64> {
        self.position
    }
}

impl Default for PlayerEntity {
    fn default() -> Self {
        Self::new()
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
}
