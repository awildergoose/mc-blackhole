use std::hash::{DefaultHasher, Hasher};

use crate::{STRICT, proto::packet_bytes::PacketBytes, world::palette::PaletteBlockKind};

#[derive(Clone)]
struct Section {
    data: [u64; 256],
}

// increase this whenever we need more palette slots
const BITS_PER_ENTRY: u32 = 4;
const NUM_ENTRIES: u32 = 4096;
const ENTRIES_PER_LONG: u32 = Section::entries_per_long();
const MASK: u64 = Section::mask();
const NUM_LONGS: usize = NUM_ENTRIES.div_ceil(ENTRIES_PER_LONG) as usize;
const NUM_SECTIONS: usize = 24;

impl Section {
    const fn new() -> Self {
        Self {
            data: [0u64; NUM_LONGS],
        }
    }

    #[inline]
    const fn entries_per_long() -> u32 {
        64 / BITS_PER_ENTRY
    }

    #[inline]
    const fn mask() -> u64 {
        (1u64 << BITS_PER_ENTRY) - 1
    }

    const fn entry_index(x: u32, y: u32, z: u32) -> u32 {
        x + z * 16 + y * 256
    }

    const fn get_at_index(&self, i: u32) -> u64 {
        let epl = ENTRIES_PER_LONG;
        let long_index = (i / epl) as usize;
        let bit_index = (i % epl) * BITS_PER_ENTRY;

        (self.data[long_index] >> bit_index) & MASK
    }

    const fn set_at_index(&mut self, i: u32, value: u64) {
        let epl = ENTRIES_PER_LONG;
        let long_index = (i / epl) as usize;
        let bit_index = (i % epl) * BITS_PER_ENTRY;

        let m = MASK;
        self.data[long_index] &= !(m << bit_index);
        self.data[long_index] |= (value & m) << bit_index;
    }

    pub const fn get_block(&self, x: u32, y: u32, z: u32) -> u64 {
        self.get_at_index(Self::entry_index(x, y, z))
    }

    pub const fn set_block(&mut self, x: u32, y: u32, z: u32, value: u64) {
        self.set_at_index(Self::entry_index(x, y, z), value);
    }
}

#[must_use]
pub fn determine_chunk_seed(world_seed: u64, cx: i32, cz: i32) -> u64 {
    let mut h = DefaultHasher::new();
    h.write_u64(world_seed);
    h.write_i32(cx);
    h.write_i32(cz);
    h.finish()
}

pub struct Chunk {
    sections: Vec<Option<Section>>,
    x: i32,
    z: i32,
}

impl Chunk {
    #[must_use]
    pub fn new(x: i32, z: i32) -> Self {
        Self {
            sections: vec![None; NUM_SECTIONS],
            x,
            z,
        }
    }

    const fn get_height_difference() -> i32 {
        64
    }

    pub fn set_block_local(&mut self, lx: u32, y: i32, lz: u32, value: PaletteBlockKind) {
        if STRICT {
            debug_assert!((0..16).contains(&lx));
            debug_assert!((0..16).contains(&lz));
            debug_assert!((-64..=319).contains(&y));
        }

        let cy = y + Self::get_height_difference();
        let sy = (cy / 16) as usize;
        let ly = (cy & 15) as u32;

        let sec = self.sections[sy].get_or_insert_with(Section::new);
        sec.set_block(lx, ly, lz, value.as_palette_index());
    }

    #[must_use]
    pub fn get_block_local(&self, lx: u32, y: i32, lz: u32) -> PaletteBlockKind {
        if STRICT {
            debug_assert!((0..16).contains(&lx));
            debug_assert!((0..16).contains(&lz));
            debug_assert!((-64..=319).contains(&y));
        }

        let cy = y + Self::get_height_difference();
        let sy = (cy / 16) as usize;
        let ly = (cy & 15) as u32;

        self.sections[sy]
            .as_ref()
            .map_or(PaletteBlockKind::Air, |sec| {
                PaletteBlockKind::from_palette_index(sec.get_block(lx, ly, lz))
            })
    }

    pub fn encode(&self) -> anyhow::Result<PacketBytes> {
        let mut chkbody = PacketBytes::new();
        chkbody.put_i32(self.x)?;
        chkbody.put_i32(self.z)?;

        chkbody.put_var_int(0)?; // no heightmaps

        let mut data_bytes = PacketBytes::new();
        let zeroes = vec![0; NUM_LONGS * size_of::<u64>()];

        for sy in 0..NUM_SECTIONS {
            data_bytes.put_u16(4096)?; // block count, this doesn't matter as long as it's over 0
            data_bytes.put_u8(BITS_PER_ENTRY as u8)?;

            // block stuff
            let pal = PaletteBlockKind::entries();
            data_bytes.put_array(pal)?;

            match &self.sections[sy] {
                Some(section) => {
                    for &v in &section.data {
                        data_bytes.extend_from_slice(&v.to_be_bytes());
                    }
                }
                None => {
                    data_bytes.extend_from_slice(&zeroes);
                }
            }

            // biome stuff
            data_bytes.put_u8(0)?;
            data_bytes.put_var_int(40)?;
        }

        chkbody.put_var_int(data_bytes.len() as i32)?;
        chkbody.put_packet_bytes(data_bytes)?;
        chkbody.put_var_int(0)?; // no block entities

        // light data
        chkbody.put_var_int(1)?;
        chkbody.put_u64(0b11_1111_1111_1111_1111_1111_1111)?;
        chkbody.put_var_int(0)?;
        chkbody.put_var_int(0)?;
        chkbody.put_var_int(0)?;

        // sky light
        chkbody.put_var_int(26)?;
        let light = [0xFF; 2048];
        for _ in 0..26 {
            chkbody.put_var_int(2048)?;
            chkbody.extend_from_slice(&light);
        }
        chkbody.put_var_int(0)?; // block light

        Ok(chkbody)
    }
}
