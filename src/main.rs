#[tokio::main]
async fn main() -> anyhow::Result<()> {
    av1_operator::init_global().await?;
    av1_operator::http_server()?.await?;
    Ok(())
}
