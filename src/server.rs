use tokio::net::TcpListener;

use crate::{handlers::handle_connection, net::framing::FramedConn};

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
    handle_connection(framed).await
}
