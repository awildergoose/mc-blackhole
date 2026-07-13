macro_rules! entries {
    ($($name:expr => $id:expr => $idx:expr),*) => {
        paste::paste! {
            use crate::proto::varint::EncodedVarInt;

            #[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
            pub enum PaletteBlockKind {
                $($name),*
            }

            impl PaletteBlockKind {
                #[must_use]
                pub const fn as_minecraft_id(&self) -> u64 {
                    match self {
                        $(
                            Self::$name => $id
                        ),*
                    }
                }

                #[must_use]
                pub const fn as_palette_index(&self) -> u64 {
                    match self {
                        $(
                            Self::$name => $idx
                        ),*
                    }
                }

                #[must_use]
                pub fn from_palette_index(index: u64) -> Self {
                    match index {
                        $(
                            $idx => Self::$name
                        ),*,
                        _ => unreachable!("bad palette index"),
                    }
                }

                #[must_use]
                pub fn entries() -> Vec<EncodedVarInt> {
                    vec![
                        $(
                            EncodedVarInt($id)
                        ),*
                    ]
                }
            }
        }
    };
}

// TODO: automate the index counting
entries!(
    Air => 0 => 0,
    Stone => 1 => 1,
    Bedrock => 85 => 2,
    OakLog => 137 => 3,
    OakPlanks => 15 => 4,
    Deepslate => 27722 => 5,
    Andesite => 6 => 6,
    Diorite => 4 => 7,
    Dirt => 10 => 8,
    Grass => 9 => 9,
    Granite => 2 => 10,
    Cobblestone => 14 => 11,
    GoldBlock => 2137 => 12,

    // I hate block states now
    CobblestoneWall => 9783 => 13,
    CobblestoneWallXF => 9898 => 14,
    CobblestoneWallZF => 9837 => 15
);
