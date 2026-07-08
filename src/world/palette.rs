use strum_macros::EnumIter;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, EnumIter)]
pub enum PaletteBlockKind {
    Air,
    Stone,
    Bedrock,
    OakLog,
    OakPlanks,
}

impl PaletteBlockKind {
    #[must_use]
    pub const fn as_minecraft_id(&self) -> u64 {
        match self {
            Self::Air => 0,
            Self::Stone => 1,
            Self::Bedrock => 85,
            Self::OakLog => 137,
            Self::OakPlanks => 15,
        }
    }

    #[must_use]
    pub const fn as_palette_index(&self) -> u64 {
        // TODO: improve this
        match self {
            Self::Air => 0,
            Self::Stone => 1,
            Self::Bedrock => 2,
            Self::OakLog => 3,
            Self::OakPlanks => 4,
        }
    }
}
