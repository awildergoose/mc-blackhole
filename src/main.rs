use mc_blackhole::server::run_server;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    run_server("0.0.0.0:25565").await
}
