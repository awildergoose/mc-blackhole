use std::time::Duration;

use tokio::{net::TcpListener, sync::mpsc};

use crate::{
    handlers::handle_connection,
    net::{
        framing::FramedConn,
        handles::{PacketWriterHandle, PacketWriterMessage},
    },
    proto::packet_bytes::PacketBytes,
};

pub async fn run_server(addr: &str) -> anyhow::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {addr}");

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("Accepted {peer}");

        tokio::spawn(async move {
            if let Err(e) = serve_conn(stream).await {
                eprintln!("{peer} error: {e:?}");
            }
        });
    }
}

async fn serve_conn(stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let framed = FramedConn::new(stream);
    let (rd, mut wr) = framed.split();
    let (write_tx, mut write_rx) = mpsc::channel::<PacketWriterMessage>(32);
    let writer = PacketWriterHandle::from(write_tx.clone());
    let writer_handle = tokio::spawn(async move {
        while let Some(pkt) = write_rx.recv().await {
            match pkt {
                PacketWriterMessage::SetCompression(threshold) => {
                    wr.set_compression(threshold);
                }
                PacketWriterMessage::Write(id, body) => {
                    wr.write_packet(id, &body).await?;
                }
            }
        }

        Ok::<(), anyhow::Error>(())
    });

    let res = handle_connection(rd, writer.clone()).await;

    if let Err(e) = res {
        eprintln!("Disconnected: {e}");

        // disconnect
        let s = e.to_string();
        let mut body = PacketBytes::new();
        body.put_u8(0x08)?; // TAG_String
        body.put_u8(0x00)?; // TAG_END?
        body.put_string(s)?;

        writer.write_packet(0x20, body.to_vec()).await?;

        // until it sends, so we don't abort immediately
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    writer_handle.abort();
    Ok(())
}
