use paste::paste;

use crate::proto::packet_bytes::PacketBytes;

pub trait MCEncode: Sized {
    fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()>;
}

pub trait MCDecode: Sized {
    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self>;
}

macro_rules! impl_encdec {
    ($($type:tt),*) => {
        paste! {
            $(
                impl MCEncode for $type {
                    fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
                        dst.[<put_ $type>](*self)
                    }
                }

                impl MCDecode for $type {
                    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
                        src.[<get_ $type>]()
                    }
                }
            )*
        }
    }
}

impl_encdec!(u8, u16, u32, u64);
impl_encdec!(i8, i16, i32, i64);
impl_encdec!(f32, f64);

impl MCEncode for String {
    fn encode(&self, dst: &mut PacketBytes) -> anyhow::Result<()> {
        dst.put_string(self.clone())
    }
}

impl MCDecode for String {
    fn decode(src: &mut PacketBytes) -> anyhow::Result<Self> {
        src.get_string()
    }
}
