use strum_macros::EnumIter;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy, EnumIter)]
pub enum PaletteBlockKind {
    AIR,
    STONE,
    BEDROCK,
}

impl PaletteBlockKind {
    #[must_use]
    pub const fn as_minecraft_id(&self) -> u64 {
        match self {
            Self::AIR => 0,
            Self::STONE => 1,
            Self::BEDROCK => 85,
        }
    }

    #[must_use]
    pub const fn as_palette_index(&self) -> u64 {
        // TODO: improve this
        match self {
            Self::AIR => 0,
            Self::STONE => 1,
            Self::BEDROCK => 2,
        }
    }
}
