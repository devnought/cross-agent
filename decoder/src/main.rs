use std::path::PathBuf;

use bytes::BytesMut;
use clap::Parser;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

#[derive(Debug, Parser)]
struct Args {
    /// Path to previously captured stream
    #[clap(short, long)]
    saved_stream: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let file = File::open(args.saved_stream).await?;
    let mut reader = BufReader::new(file);
    let mut bytes = BytesMut::with_capacity(1024 * 8);

    while reader.read_buf(&mut bytes).await? > 0 {}

    Ok(())
}
