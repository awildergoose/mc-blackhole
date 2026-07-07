use crate::proto::{packet_bytes::PacketBytes, varint::EncodedVarInt};

#[derive(Clone)]
struct Section {
    data: Vec<u64>,
    bits_per_entry: u32,
}

impl Section {
    fn new(bits_per_entry: u32, num_entries: u32) -> Self {
        let b = bits_per_entry;
        let entries_per_long = 64 / b;
        let num_longs = num_entries.div_ceil(entries_per_long);

        Self {
            data: vec![0u64; num_longs as usize],
            bits_per_entry: b,
        }
    }

    #[inline]
    const fn entry_index(x: u32, y: u32, z: u32) -> u32 {
        x + z * 16 + y * 256
    }

    #[inline]
    const fn entries_per_long(&self) -> u32 {
        64 / self.bits_per_entry
    }

    #[inline]
    const fn mask(&self) -> u64 {
        if self.bits_per_entry == 0 {
            0
        } else {
            (1u64 << self.bits_per_entry) - 1
        }
    }

    fn get_at_index(&self, i: u32) -> u64 {
        if self.bits_per_entry == 0 {
            return 0;
        }

        let b = self.bits_per_entry;
        let epl = self.entries_per_long();
        let long_index = (i / epl) as usize;
        let bit_index = (i % epl) * b;

        let m = self.mask();
        (self.data[long_index] >> bit_index) & m
    }

    fn set_at_index(&mut self, i: u32, value: u64) {
        if self.bits_per_entry == 0 {
            return;
        }

        let b = self.bits_per_entry;
        let epl = self.entries_per_long();
        let long_index = (i / epl) as usize;
        let bit_index = (i % epl) * b;

        let m = self.mask();
        self.data[long_index] &= !(m << bit_index);
        self.data[long_index] |= (value & m) << bit_index;
    }

    pub fn get_block(&self, x: u32, y: u32, z: u32) -> u64 {
        let idx = Self::entry_index(x, y, z);
        self.get_at_index(idx)
    }

    pub fn set_block(&mut self, x: u32, y: u32, z: u32, value: u64) {
        let idx = Self::entry_index(x, y, z);
        self.set_at_index(idx, value);
    }
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

        let mut this = Self { sections, x, z };
        for i in 0..256 {
            this.set_block(i, 0, 0, 1);
            this.set_block(0, 0, i, 1);
        }
        this.set_block(0, 0, 0, 2);
        this
    }

    #[must_use]
    pub fn get_block(&self, wx: i32, wy: i32, wz: i32) -> u64 {
        let wx = wx - 2; // ?? idk
        let wy = wy + 64; // world height dependant!
        let sy = (wy / 16) as usize;
        let ly = (wy & 15) as u32;
        let lx = (wx & 15) as u32;
        let lz = (wz & 15) as u32;

        self.sections[sy].get_block(lx, ly, lz)
    }

    pub fn set_block(&mut self, wx: i32, wy: i32, wz: i32, value: u64) {
        let wx = wx - 2; // ?? idk
        let wy = wy + 64; // world height dependant!
        let sy = (wy / 16) as usize;
        let ly = (wy & 15) as u32;
        let lx = (wx & 15) as u32;
        let lz = (wz & 15) as u32;
        self.sections[sy].set_block(lx, ly, lz, value);
    }

    pub fn encode(&self) -> anyhow::Result<PacketBytes> {
        let mut chkbody = PacketBytes::new();
        chkbody.put_i32(self.x)?;
        chkbody.put_i32(self.z)?;

        chkbody.put_var_int(0)?; // no heightmaps

        let mut data_bytes = PacketBytes::new();

        for sy in 0..32 {
            data_bytes.put_u16(4096)?; // block count

            // block stuff
            let section = &self.sections[sy];
            data_bytes.put_u8(section.bits_per_entry as u8)?;
            data_bytes.put_array(vec![
                EncodedVarInt(0),  // air
                EncodedVarInt(85), // bedrock
                EncodedVarInt(1),  // stone
            ])?;

            let mut packed = Vec::with_capacity(section.data.len() * 8);

            for &x in &section.data {
                packed.extend_from_slice(&x.to_le_bytes());
            }

            data_bytes.extend_from_slice(&packed);

            // biome stuff
            data_bytes.put_u8(0)?;
            data_bytes.put_var_int(40)?;
        }

        chkbody.put_array(data_bytes.to_vec())?;
        chkbody.put_var_int(0)?; // no block entities

        // light data
        chkbody.put_var_int(1)?;
        chkbody.put_u64(0b11_1111_1111_1111_1111_1111_1111)?;
        chkbody.put_var_int(0)?;
        chkbody.put_var_int(0)?;
        chkbody.put_var_int(0)?;

        // sky light
        chkbody.put_var_int(26)?;
        for _ in 0..26 {
            chkbody.put_var_int(2048)?;

            for _ in 0..2048 {
                chkbody.put_u8(0xFF)?;
            }
        }
        chkbody.put_var_int(0)?; // block light

        Ok(chkbody)
    }
}
