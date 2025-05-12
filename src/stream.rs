use std::{
    io::{self, Cursor, Write},
    path::PathBuf,
    pin::pin,
};

use async_fn_stream::try_fn_stream;
use bytes::{Bytes, BytesMut};
use futures::TryStream;
use log::debug;
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

        let enc_buffer = vec![0; 8 * 1024];
        let enc_cursor = Cursor::new(enc_buffer);
        let mut encoder = FrameEncoder::new(enc_cursor);

        let mut count = 0;

        for entry in iter {
            let path = entry.path();
            count += 1;

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
                encoder.write_all(&bytes)?;
                bytes.clear();

                let cursor = encoder.get_mut();
                let buffer = cursor.get_mut();
                let slice = &buffer[..bytes.len()];

                let emit_bytes = Bytes::copy_from_slice(slice);
                emitter.emit(emit_bytes).await;
                cursor.set_position(0);
            }
        }

        debug!("Iterations: {count}");

        let mut cursor = encoder.finish().unwrap();
        let buffer = cursor.get_mut();
        let slice = &buffer[..bytes.len()];

        let emit_bytes = Bytes::copy_from_slice(slice);
        emitter.emit(emit_bytes).await;

        Ok(())
    })
}
