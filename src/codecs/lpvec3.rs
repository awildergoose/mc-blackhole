use crate::{
    codecs::base::{MCDecode, MCEncode},
    proto::packet_bytes::PacketBytes,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LpVec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

static MAX_QUANTIZED_VALUE: f64 = 32766.0;

#[expect(clippy::cast_precision_loss)]
fn from_long(quantized: i64, scale: f64) -> f64 {
    // Reverse: ((v * 0.5 + 0.5) * 32766) -> v
    let normalized = (quantized as f64 / MAX_QUANTIZED_VALUE) - 0.5;
    (normalized / 0.5) * scale
}

impl LpVec3 {
    #[must_use]
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl MCEncode for LpVec3 {
    fn encode(&self, _dst: &mut PacketBytes) -> anyhow::Result<()> {
        todo!()
    }
}

impl MCDecode for LpVec3 {
    #[expect(clippy::cast_precision_loss)]
    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
        // TODO: this is currently broken!
        let low = i64::from(src.get_u16()?);
        if low == 0 {
            return Ok(Self::new(0.0, 0.0, 0.0));
        }

        let mid = i64::from(src.get_i32()?);
        let packed_data = low | (mid << 16);

        let header = packed_data & 0x07;
        let is_extended = (header & 4) != 0;

        let scale_factor = if is_extended {
            let scale_tail = src.get_var_int()?;
            (i64::from(scale_tail) << 2) | (header & 3)
        } else {
            header & 3
        };

        if scale_factor == 0 && !is_extended {
            return Ok(Self::new(0.0, 0.0, 0.0));
        }

        let q_x = (packed_data >> 3) & 0x7FFF;
        let q_y = (packed_data >> 18) & 0x7FFF;
        let q_z = (packed_data >> 33) & 0x7FFF;

        let scale = scale_factor as f64;

        Ok(Self::new(
            from_long(q_x, scale),
            from_long(q_y, scale),
            from_long(q_z, scale),
        ))
    }
}

#[cfg(test)]
mod test {
    use crate::{
        codecs::{base::MCDecode, lpvec3::LpVec3},
        proto::packet_bytes::PacketBytes,
    };

    #[test]
    fn test_encdec() {
        // Sample LpVec3:
        // Value 	Hex bytes 	Decimal bytes
        // (0.0, 0.0, 0.0) 	0x00 	0
        // (1.0, 0.0, -1.0) 	0xF1 0xFF 0x00 0x00 0xFF 0xFF 	241 255 0 0 255 255
        // (10.0, 0.2, -5.0) 	0xF6 0xFF 0x40 0x01 0x05 0x1F 0x02 	246 255 64 1 5 31 2
        // (123457.0, 15.071, 0.0) 	0xF5 0xFF 0x7F 0xFF 0x00 0x07 0x90 0xF1 0x01 	245 255 127 255 0 7 144 241 1

        let expected = [
            (LpVec3::new(0.0, 0.0, 0.0), vec![0x00]),
            (
                LpVec3::new(1.0, 0.0, -1.0),
                vec![0xF1, 0xFF, 0x00, 0x00, 0xFF, 0xFF],
            ),
            (
                LpVec3::new(10.0, 0.2, -5.0),
                vec![0xF6, 0xFF, 0x40, 0x01, 0x05, 0x1F, 0x02],
            ),
            (
                LpVec3::new(123_457.0, 15.071, 0.0),
                vec![0xF5, 0xFF, 0x7F, 0xFF, 0x00, 0x07, 0x90, 0xF1, 0x01],
            ),
        ];

        for (expected, hex) in expected {
            let mut pb = PacketBytes::new();

            for v in &hex {
                pb.put_i32(i32::from_be(*v)).unwrap();
            }

            let decoded = LpVec3::decode(&mut pb).unwrap();
            assert_eq!(decoded, expected, "pb: {pb:?}");

            // let mut pb = PacketBytes::new();
            // expected.encode(&mut pb).unwrap();
            // let remaining: Vec<i32> = pb.get_remaining_array().unwrap();
            // assert_eq!(remaining, hex);
        }
    }
}
