use std::hash::{DefaultHasher, Hasher};

use crate::{
    STRICT,
    proto::{packet_bytes::PacketBytes, varint::EncodedVarInt},
    world::palette::PaletteBlockKind,
};
use strum::IntoEnumIterator;

struct Section {
    data: Vec<u64>,

    // DO NOT MUTATE THIS WITHOUT UPDATING `mask` AND `entries_per_long`
    bits_per_entry: u32,
    mask: u64,
    entries_per_long: u32,
}

impl Section {
    fn new(bits_per_entry: u32, num_entries: u32) -> Self {
        if STRICT {
            debug_assert!(bits_per_entry != 0);
        }

        let entries_per_long = Self::entries_per_long(bits_per_entry);
        let num_longs = num_entries.div_ceil(entries_per_long);

        Self {
            data: vec![0u64; num_longs as usize],
            bits_per_entry,
            mask: Self::mask(bits_per_entry),
            entries_per_long,
        }
    }

    #[inline]
    const fn entry_index(x: u32, y: u32, z: u32) -> u32 {
        x + z * 16 + y * 256
    }

    #[inline]
    const fn entries_per_long(bits_per_entry: u32) -> u32 {
        64 / bits_per_entry
    }

    #[inline]
    const fn mask(bits_per_entry: u32) -> u64 {
        (1u64 << bits_per_entry) - 1
    }

    fn get_at_index(&self, i: u32) -> u64 {
        let epl = self.entries_per_long;
        let long_index = (i / epl) as usize;
        let bit_index = (i % epl) * self.bits_per_entry;

        (self.data[long_index] >> bit_index) & self.mask
    }

    fn set_at_index(&mut self, i: u32, value: u64) {
        let epl = self.entries_per_long;
        let long_index = (i / epl) as usize;
        let bit_index = (i % epl) * self.bits_per_entry;

        let m = self.mask;
        self.data[long_index] &= !(m << bit_index);
        self.data[long_index] |= (value & m) << bit_index;
    }

    pub fn get_block(&self, x: u32, y: u32, z: u32) -> u64 {
        self.get_at_index(Self::entry_index(x, y, z))
    }

    pub fn set_block(&mut self, x: u32, y: u32, z: u32, value: u64) {
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
    sections: Vec<Section>,
    x: i32,
    z: i32,
}

impl Chunk {
    #[must_use]
    pub fn new(x: i32, z: i32) -> Self {
        let mut sections = Vec::with_capacity(32);

        for _ in 0..32 {
            sections.push(Section::new(4, 4096));
        }

        Self { sections, x, z }
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

        self.sections[sy].set_block(lx, ly, lz, value.as_palette_index());
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

        PaletteBlockKind::from_palette_index(self.sections[sy].get_block(lx, ly, lz))
    }

    pub fn encode(&self) -> anyhow::Result<PacketBytes> {
        let mut chkbody = PacketBytes::new();
        chkbody.put_i32(self.x)?;
        chkbody.put_i32(self.z)?;

        chkbody.put_var_int(0)?; // no heightmaps

        let mut data_bytes = PacketBytes::new();

        for sy in 0..32 {
            data_bytes.put_u16(4096)?; // block count, this doesn't matter as long as it's over 0

            // block stuff
            let section = &self.sections[sy];
            data_bytes.put_u8(section.bits_per_entry as u8)?;
            let pal = PaletteBlockKind::iter()
                .map(|k| EncodedVarInt(k.as_minecraft_id() as i32))
                .collect::<Vec<EncodedVarInt>>();
            data_bytes.put_array(pal)?;

            let mut packed = Vec::with_capacity(section.data.len() * 8);

            for &x in &section.data {
                packed.extend_from_slice(&x.to_be_bytes());
            }

            data_bytes.extend_from_slice(&packed);

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
