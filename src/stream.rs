use std::{
    io::{self, Write},
    path::PathBuf,
    pin::pin,
};

use async_fn_stream::try_fn_stream;
use bytes::{Bytes, BytesMut};
use futures::TryStream;
use log::debug;
use lz4_flex::frame::{BlockMode, FrameEncoder, FrameInfo};
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
        bytes.clear();

        let package = root_iterator_package(roots, ["/temp/one-file/*"]).unwrap();
        let iter = pin!(root_iterator(package));

        let enc_buffer = Vec::new();
        let frame_info = FrameInfo::new().block_mode(BlockMode::Linked);
        let mut encoder = FrameEncoder::with_frame_info(frame_info, enc_buffer);

        for entry in iter {
            let path = entry.path();

            debug!("--- {}", path.display());

            if let Ok(meta) = path.metadata() {
                if meta.is_file() && meta.is_offline() {
                    // Skip opening/reading cloud-hosted content
                    continue;
                }
            }

            let file = File::open(path).await?;
            let mut reader = BufReader::new(file);

            while reader.read_buf(&mut bytes).await? > 0 {
                encoder.write_all(&bytes).unwrap();
                bytes.clear();

                let buffer = encoder.get_mut();
                let emit_bytes = Bytes::copy_from_slice(buffer.as_slice());
                emitter.emit(emit_bytes).await;

                buffer.clear();
            }
        }

        let buffer = encoder.finish().unwrap();
        let emit_bytes = Bytes::copy_from_slice(buffer.as_slice());
        emitter.emit(emit_bytes).await;

        Ok(())
    })
}
