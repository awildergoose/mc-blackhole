use std::{
    io::{Read, Write},
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
};

use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};

use crate::proto::{packet_bytes::PacketBytes, packets::Packet, varint::read_varint_from_stream};

const DISABLED: i32 = -1;

pub struct FramedConnRead {
    stream: OwnedReadHalf,
    compression_threshold: Arc<AtomicI32>,
}

pub struct FramedConnWrite {
    stream: OwnedWriteHalf,
    compression_threshold: Arc<AtomicI32>,
}

impl FramedConnRead {
    pub async fn read_packet(&mut self) -> anyhow::Result<(i32, PacketBytes)> {
        let len = read_varint_from_stream(&mut self.stream).await?;
        let mut buf = vec![0u8; len as usize];
        self.stream.read_exact(&mut buf).await?;

        let mut bytes = PacketBytes::from(&buf[..]);

        let thr = self.compression_threshold.load(Ordering::Relaxed);
        if thr != DISABLED {
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
}

impl FramedConnWrite {
    pub fn enable_compression(&self, threshold: i32) {
        self.compression_threshold
            .store(threshold, Ordering::Relaxed);
    }

    pub fn disable_compression(&self) {
        self.compression_threshold
            .store(DISABLED, Ordering::Relaxed);
    }

    #[must_use]
    pub fn is_compression_enabled(&self) -> bool {
        self.compression_threshold.load(Ordering::Relaxed) != DISABLED
    }

    pub async fn write_pkt<T>(&mut self, pkt: T) -> anyhow::Result<()>
    where
        T: Packet,
    {
        self.write_packet(T::ID, &pkt.encoded()?).await
    }

    #[optimize(speed)]
    pub async fn write_packet(&mut self, packet_id: i32, body: &[u8]) -> anyhow::Result<()> {
        let mut packet_uncompressed = PacketBytes::new();
        packet_uncompressed.put_var_int(packet_id)?;
        packet_uncompressed.extend_from_slice(body);

        let thr = self.compression_threshold.load(Ordering::Relaxed);

        if thr == DISABLED {
            let mut framed = PacketBytes::new();
            framed.put_var_int(packet_uncompressed.len() as i32)?;
            framed.extend_from_slice(&packet_uncompressed);
            self.stream.write_all(&framed).await?;
            Ok(())
        } else if (packet_uncompressed.len() as i32) >= thr {
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
            let mut payload = PacketBytes::new();
            payload.put_var_int(0)?;
            payload.extend_from_slice(&packet_uncompressed);

            let mut framed = PacketBytes::new();
            framed.put_var_int(payload.len() as i32)?;
            framed.extend_from_slice(&payload);
            self.stream.write_all(&framed).await?;
            Ok(())
        }
    }
}

pub struct FramedConn {
    stream: tokio::net::TcpStream,
    compression_threshold: Arc<AtomicI32>,
}

impl FramedConn {
    pub fn new(stream: tokio::net::TcpStream) -> Self {
        Self {
            stream,
            compression_threshold: Arc::new(AtomicI32::new(DISABLED)),
        }
    }

    pub fn split(self) -> (FramedConnRead, FramedConnWrite) {
        let (r, w) = self.stream.into_split();
        let shared = self.compression_threshold;

        (
            FramedConnRead {
                stream: r,
                compression_threshold: shared.clone(),
            },
            FramedConnWrite {
                stream: w,
                compression_threshold: shared,
            },
        )
    }
}
