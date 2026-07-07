use uuid::Uuid;

pub trait Entity: Send + Sync {
    fn base(&self) -> &EntityBase;
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
    username: String,
}

impl PlayerEntity {
    #[must_use]
    pub fn new(username: String) -> Self {
        Self {
            base: EntityBase::default(),
            username,
        }
    }
}

impl Entity for PlayerEntity {
    fn base(&self) -> &EntityBase {
        &self.base
    }
}
