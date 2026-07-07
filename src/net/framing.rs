use std::io::{Read, Write};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::proto::{packet_bytes::PacketBytes, packets::Packet, varint::read_varint_from_stream};

pub struct FramedConn {
    stream: TcpStream,
    compression_threshold: Option<i32>,
}

impl FramedConn {
    pub const fn new(stream: TcpStream) -> Self {
        Self {
            stream,
            compression_threshold: None,
        }
    }

    pub const fn enable_compression(&mut self, threshold: i32) {
        self.compression_threshold = Some(threshold);
    }

    pub const fn disable_compression(&mut self) {
        self.compression_threshold = None;
    }

    pub const fn is_compression_enabled(&self) -> bool {
        self.compression_threshold.is_some()
    }

    pub async fn read_packet(&mut self) -> anyhow::Result<(i32, PacketBytes)> {
        let len = read_varint_from_stream(&mut self.stream).await?;
        let mut buf = vec![0u8; len as usize];
        self.stream.read_exact(&mut buf).await?;

        let mut bytes = PacketBytes::from(&buf[..]);

        if self.compression_threshold.is_some() {
            let data_len = bytes.get_var_int()?;

            if data_len != 0 {
                let compressed = bytes.split().freeze();
                let mut decoder = ZlibDecoder::new(&compressed[..]);
                let mut decompressed = Vec::with_capacity(data_len as usize);
                decoder.read_to_end(&mut decompressed)?;
                bytes = PacketBytes::from(&decompressed[..]);
            }
        }

        let packet_id = bytes.get_var_int()?;
        Ok((packet_id, bytes))
    }

    pub async fn write_pkt<T>(&mut self, pkt: T) -> anyhow::Result<()>
    where
        T: Packet,
    {
        self.write_packet(T::ID, &pkt.encoded()?).await
    }

    pub async fn write_packet(&mut self, packet_id: i32, body: &[u8]) -> anyhow::Result<()> {
        let mut packet_uncompressed = PacketBytes::new();
        packet_uncompressed.put_var_int(packet_id)?;
        packet_uncompressed.extend_from_slice(body);

        // if compression is enabled
        if let Some(threshold) = self.compression_threshold {
            // if we cross the threshold to compress
            if (packet_uncompressed.len() as i32) >= threshold {
                // we compress the following
                let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
                encoder.write_all(&packet_uncompressed)?;
                let compressed = encoder.finish()?;

                let mut payload = PacketBytes::new();
                payload.put_var_int(packet_uncompressed.len() as i32)?;
                payload.extend_from_slice(&compressed);

                let mut framed = PacketBytes::new();
                framed.put_var_int(payload.len() as i32)?;
                framed.extend_from_slice(&payload);
                self.stream.write_all(&framed).await?;
                Ok(())
            } else {
                // we don't compress anything
                let mut payload = PacketBytes::new();
                payload.put_var_int(0)?;
                payload.extend_from_slice(&packet_uncompressed);

                let mut framed = PacketBytes::new();
                framed.put_var_int(payload.len() as i32)?;
                framed.extend_from_slice(&payload);
                self.stream.write_all(&framed).await?;
                Ok(())
            }
        } else {
            // compression is disabled, write the packet raw
            let mut framed = PacketBytes::new();
            framed.put_var_int(packet_uncompressed.len() as i32)?;
            framed.extend_from_slice(&packet_uncompressed);
            self.stream.write_all(&framed).await?;
            Ok(())
        }
    }
}
