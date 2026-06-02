#![allow(clippy::needless_pass_by_value)]
use std::ops::{Deref, DerefMut};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use fastnbt::ByteArray;
use uuid::Uuid;

use crate::{
    codecs::{
        array::Array,
        base::{MCDecode, MCEncode},
        enums::Enum,
        game_profile::GameProfile,
    },
    proto::varint::{read_var_int, write_var_int},
};

#[derive(Debug, Clone)]
pub struct PacketBytes {
    data: BytesMut,
}

pub type GetRes<T> = anyhow::Result<T>;
pub type PutRes = anyhow::Result<()>;

impl PacketBytes {
    #[must_use]
    pub fn new() -> Self {
        Self {
            data: BytesMut::new(),
        }
    }

    pub fn put_var_int(&mut self, value: i32) -> PutRes {
        write_var_int(&mut self.data, value)
    }

    pub fn get_var_int(&mut self) -> GetRes<i32> {
        read_var_int(&mut self.data)
    }

    pub fn put_uuid(&mut self, uuid: Uuid) -> PutRes {
        let u = uuid.as_u128();
        let msb = (u >> 64) as u64;
        let lsb = (u & 0xffff_ffff_ffff_ffff) as u64;
        self.put_u64(msb)?;
        self.put_u64(lsb)?;
        Ok(())
    }

    pub fn get_uuid(&mut self) -> GetRes<Uuid> {
        let msb = self.get_u64()?;
        let lsb = self.get_u64()?;
        Ok(Uuid::from_u64_pair(msb, lsb))
    }

    pub fn put_string(&mut self, s: String) -> PutRes {
        self.put_var_int(s.len() as i32)?;
        self.extend_from_slice(s.as_bytes());
        Ok(())
    }

    pub fn get_string(&mut self) -> GetRes<String> {
        let len = self.get_var_int()?;
        if self.len() < len as usize {
            anyhow::bail!("String truncated");
        }
        let bytes = self.split_to(len as usize);
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    pub fn put_str(&mut self, s: &str) -> PutRes {
        self.put_string(s.to_owned())
    }

    pub fn get_str(&mut self) -> GetRes<String> {
        self.get_string()
    }

    pub fn put_game_profile(&mut self, gp: GameProfile) -> PutRes {
        gp.encode(self)
    }

    pub fn get_game_profile(&mut self) -> GetRes<GameProfile> {
        GameProfile::decode(self)
    }

    pub fn put_byte_array(&mut self, ba: ByteArray) -> PutRes {
        self.data.extend(ba.iter().copied().map(|u| u as u8));
        Ok(())
    }

    pub fn get_byte_array(&mut self) -> GetRes<ByteArray> {
        Ok(ByteArray::new(
            self.data
                .iter()
                .copied()
                .map(|u| u as i8)
                .collect::<Vec<i8>>(),
        ))
    }

    pub fn put_array<T: MCEncode + MCDecode>(&mut self, arr: Array<T>) -> PutRes {
        self.put_var_int(arr.len() as i32)?;
        for ele in arr {
            ele.encode(self)?;
        }
        Ok(())
    }

    pub fn get_array<T: MCEncode + MCDecode>(&mut self) -> GetRes<Array<T>> {
        let len = self.get_var_int()?;
        let mut arr = Vec::with_capacity(len as usize);

        for _ in 0..len {
            arr.push(T::decode(self)?);
        }

        Ok(arr)
    }

    pub fn put_enum<T: MCEncode + MCDecode>(&mut self, enm: Enum<T>) -> PutRes {
        enm.encode(self)
    }

    pub fn get_enum<T: MCEncode + MCDecode>(&mut self) -> GetRes<Enum<T>> {
        T::decode(self)
    }
}

impl Default for PacketBytes {
    fn default() -> Self {
        Self::new()
    }
}

impl AsRef<[u8]> for PacketBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl Deref for PacketBytes {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &[u8] {
        self.data.deref()
    }
}

impl AsMut<[u8]> for PacketBytes {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.data.as_mut()
    }
}

impl DerefMut for PacketBytes {
    #[inline]
    fn deref_mut(&mut self) -> &mut [u8] {
        self.data.deref_mut()
    }
}

impl<'a> From<&'a [u8]> for PacketBytes {
    fn from(src: &'a [u8]) -> Self {
        Self {
            data: BytesMut::from(src),
        }
    }
}

impl Extend<Bytes> for PacketBytes {
    fn extend<T>(&mut self, iter: T)
    where
        T: IntoIterator<Item = Bytes>,
    {
        self.data.extend(iter);
    }
}

impl PacketBytes {
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn extend_from_slice(&mut self, extend: &[u8]) {
        self.data.extend_from_slice(extend);
    }

    #[must_use]
    pub fn split(&mut self) -> Self {
        Self {
            data: self.data.split(),
        }
    }

    #[must_use]
    pub fn split_to(&mut self, at: usize) -> Self {
        Self {
            data: self.data.split_to(at),
        }
    }

    pub fn advance(&mut self, cnt: usize) {
        self.data.advance(cnt);
    }

    #[must_use]
    pub fn freeze(self) -> Bytes {
        self.data.freeze()
    }
}

// interop
use paste::paste;

macro_rules! put_interops {
    ($($type:tt),*) => {
        paste! {
            $(
                pub fn [<put_ $type>](&mut self, value: $type) -> anyhow::Result<()> {
                    Ok(self.data.[<put_ $type>](value))
                }
            )*
        }
    };
}

macro_rules! get_interops {
    ($($type:tt),*) => {
        paste! {
            $(
                pub fn [<get_ $type>](&mut self) -> anyhow::Result<$type> {
                    Ok(self.data.[<try_get_ $type>]()?)
                }
            )*
        }
    };
}

impl PacketBytes {
    put_interops!(u8, u16, u32, u64);

    put_interops!(i8, i16, i32, i64);

    put_interops!(f32, f64);

    get_interops!(u8, u16, u32, u64);

    get_interops!(i8, i16, i32, i64);

    get_interops!(f32, f64);

    pub fn put_bool(&mut self, value: bool) -> anyhow::Result<()> {
        self.put_u8(u8::from(value))
    }

    pub fn get_bool(&mut self) -> anyhow::Result<bool> {
        Ok(self.get_u8()? == 1)
    }
}
