use anyhow::Result;
use bytes::{BufMut, BytesMut};
use tokio::{io::AsyncReadExt, net::TcpStream};

pub type VarInt = i32;

pub async fn read_varint_from_stream(stream: &mut TcpStream) -> Result<i32> {
    let mut num_read = 0;
    let mut result: i32 = 0;

    loop {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf).await?;
        let byte = buf[0];
        let value = i32::from(byte & 0x7F);
        result |= value << (7 * num_read);
        num_read += 1;

        if num_read > 5 {
            anyhow::bail!("VarInt too big");
        }
        if (byte & 0x80) == 0 {
            break;
        }
    }

    Ok(result)
}

pub fn read_var_int(src: &mut BytesMut) -> Result<i32> {
    let mut num_read = 0usize;
    let mut result: i32 = 0;
    let mut consumed = 0usize;

    loop {
        if consumed >= src.len() {
            anyhow::bail!("Unexpected EOF while reading varint");
        }
        let byte = src[consumed];
        let value = i32::from(byte & 0x7F);
        result |= value << (7 * num_read);
        num_read += 1;
        consumed += 1;

        if num_read > 5 {
            anyhow::bail!("VarInt too big");
        }
        if (byte & 0x80) == 0 {
            break;
        }
    }

    let _ = src.split_to(consumed);
    Ok(result)
}

pub fn write_var_int(dst: &mut BytesMut, mut value: i32) -> anyhow::Result<()> {
    loop {
        if (value & !0x7F) == 0 {
            dst.put_u8(value as u8);
            return Ok(());
        }
        dst.put_u8(((value & 0x7F) | 0x80) as u8);
        value = ((value as u32) >> 7) as i32;
    }
}
