#[tokio::main]
async fn main() {
    if let Err(error) = geo_agent::run().await {
        eprintln!("geo-agent failed: {error}");
        std::process::exit(1);
    }
}
