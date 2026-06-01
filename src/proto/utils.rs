use bytes::{Buf, BufMut, BytesMut};
use uuid::Uuid;

use crate::proto::varint::{read_var_int, write_var_int};

pub fn write_uuid(dst: &mut BytesMut, uuid: Uuid) -> anyhow::Result<()> {
    let u = uuid.as_u128();
    let msb = (u >> 64) as u64;
    let lsb = (u & 0xffff_ffff_ffff_ffff) as u64;
    dst.put_u64(msb);
    dst.put_u64(lsb);
    Ok(())
}

pub fn read_uuid(src: &mut BytesMut) -> anyhow::Result<Uuid> {
    let msb = src.get_u64();
    let lsb = src.get_u64();
    Ok(Uuid::from_u64_pair(msb, lsb))
}

pub fn read_string(src: &mut BytesMut) -> anyhow::Result<String> {
    let len = read_var_int(src)?;
    if src.len() < len as usize {
        anyhow::bail!("String truncated");
    }
    let bytes = src.split_to(len as usize);
    Ok(String::from_utf8(bytes.to_vec())?)
}

pub fn write_string(dst: &mut BytesMut, s: &str) -> anyhow::Result<()> {
    write_var_int(dst, s.len() as i32)?;
    dst.extend_from_slice(s.as_bytes());
    Ok(())
}
