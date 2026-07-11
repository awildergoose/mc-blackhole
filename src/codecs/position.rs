use crate::{
    codecs::base::{MCDecode, MCEncode},
    proto::packet_bytes::PacketBytes,
};

#[derive(Clone, Copy, Debug)]
pub struct Position(i64);

pub type PositionX = i64;
pub type PositionY = i64;
pub type PositionZ = i64;

impl Position {
    #[must_use]
    pub const fn encode_position(x: PositionX, y: PositionY, z: PositionZ) -> i64 {
        // TODO: assert position min and max values
        ((x & 0x03FF_FFFF) << 38) | ((z & 0x03FF_FFFF) << 12) | (y & 0xFFF)
    }

    #[must_use]
    pub const fn decode_position(value: i64) -> (PositionX, PositionY, PositionZ) {
        let x = value >> 38;
        let y = value << 52 >> 52;
        let z = value << 26 >> 38;

        (x, y, z)
    }

    #[must_use]
    pub const fn from_pos(x: PositionX, y: PositionY, z: PositionZ) -> Self {
        Self(Self::encode_position(x, y, z))
    }

    #[must_use]
    pub const fn decoded(&self) -> (PositionX, PositionY, PositionZ) {
        Self::decode_position(self.0)
    }
}

impl MCEncode for Position {
    fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
        dst.put_i64(self.0)
    }
}

impl MCDecode for Position {
    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
        src.get_i64().map(Position)
    }
}

#[cfg(test)]
mod test {
    use crate::codecs::position::Position;

    #[test]
    fn test_encdec() {
        let p = Position::from_pos(18_357_644, 831, -20_882_616);
        assert_eq!(p.0, 0x4607_632C_15B4_833F);

        let p = Position(0x4607_632C_15B4_833F);
        let position = p.decoded();
        assert_eq!(position, (18_357_644, 831, -20_882_616));
    }
}
