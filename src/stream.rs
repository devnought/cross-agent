use std::{io, path::PathBuf, pin::pin};

use async_fn_stream::try_fn_stream;
use bytes::{Bytes, BytesMut};
use futures::TryStream;
use lz4_flex::frame::FrameEncoder;
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

use crate::{
    file_offline::FileOffline,
    files_iter::{root_iterator, root_iterator_package},
};

pub fn build_stream(
    roots: impl Iterator<Item = PathBuf>,
) -> impl TryStream<Ok = Bytes, Error = io::Error> {
    try_fn_stream(|emitter| async move {
        let mut bytes = BytesMut::with_capacity(1024 * 8);
        let package = root_iterator_package(roots, ["/*"]).unwrap();
        let iter = pin!(root_iterator(package));

        // let mut enc_buffer = Vec::new();
        // let encoder = FrameEncoder::new(&mut enc_buffer);

        for entry in iter {
            let path = entry.path();

            println!("--- {}", path.display());

            if let Ok(meta) = path.metadata() {
                if meta.is_file() && meta.is_offline() {
                    // Skip opening/reading cloud-hosted content
                    continue;
                }
            }

            let file = File::open(path).await?;
            let mut reader = BufReader::new(file);

            while reader.read_buf(&mut bytes).await? > 0 {
                // println!("Read {} bytes", bytes.len());
                emitter.emit(bytes.clone().into()).await;
                bytes.clear();
            }
        }

        Ok(())
    })
}
