use std::{
    io::{self, Write},
    path::PathBuf,
    pin::pin,
};

use async_fn_stream::try_fn_stream;
use bytes::{Bytes, BytesMut};
use filesystem_iter::{file_offline::FileOffline, root_iterator, root_iterator_package};
use futures::TryStream;
use log::debug;
use lz4_flex::frame::{BlockMode, FrameEncoder, FrameInfo};
use md5::Md5;
use rkyv::{
    Archive, Deserialize, Serialize, api::high::to_bytes_with_alloc, rancor, ser::allocator::Arena,
};
use sha3::{Digest, Sha3_256};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};

#[derive(Debug, Archive, Serialize, Deserialize)]
pub enum Message {
    FileHeader { path: String, len: u64 },
    FileBody { data: Bytes },
    FileFooter { sha256: [u8; 32], md5: [u8; 16] },
    Directory { path: String },
}

pub fn build_stream(
    roots: impl Iterator<Item = PathBuf>,
) -> impl TryStream<Ok = Bytes, Error = io::Error> {
    try_fn_stream(|emitter| async move {
        let mut bytes = BytesMut::with_capacity(1024 * 64);
        let mut enc_buffer = Vec::new();
        let mut arena = Arena::new();

        let package = root_iterator_package(roots, &["*"]).unwrap();
        let iter = pin!(root_iterator(package));

        debug!("About to start iterating");

        for entry in iter {
            let path = entry.path();

            debug!("--- {}", path.display());

            let Ok(meta) = path.metadata() else {
                // We probably cannot even access the file at this point, abort!
                continue;
            };

            if meta.is_offline() {
                // Skip opening/reading cloud-hosted content
                continue;
            }

            let header = if path.is_file() {
                Message::FileHeader {
                    path: path.to_string_lossy().into_owned(),
                    len: meta.len(),
                }
            } else {
                Message::Directory {
                    path: path.to_string_lossy().into_owned(),
                }
            };

            emitter
                .emit(Bytes::copy_from_slice(
                    to_bytes_with_alloc::<_, rancor::Error>(&header, arena.acquire())
                        .unwrap()
                        .as_slice(),
                ))
                .await;

            // Nothing left to do if it's a directory.
            if path.is_dir() {
                continue;
            }

            // Build up LZ4 compression for file contents
            let frame_info = FrameInfo::new().block_mode(BlockMode::Linked);

            // Ensure buffer is clear
            enc_buffer.clear();

            let mut encoder = FrameEncoder::with_frame_info(frame_info, &mut enc_buffer);
            let mut md5 = Md5::new();
            let mut sha256 = Sha3_256::new();

            let file = File::open(path).await?;
            let mut reader = BufReader::new(file);

            while reader.read_buf(&mut bytes).await? > 0 {
                encoder.write_all(&bytes).unwrap();
                md5.update(&bytes);
                sha256.update(&bytes);

                bytes.clear();

                // Get data from the encoder, and yield those bytes.
                let buffer = encoder.get_mut();
                let emit_bytes = Bytes::copy_from_slice(buffer.as_slice());
                let body = Message::FileBody { data: emit_bytes };
                emitter
                    .emit(Bytes::copy_from_slice(
                        to_bytes_with_alloc::<_, rancor::Error>(&body, arena.acquire())
                            .unwrap()
                            .as_slice(),
                    ))
                    .await;
                buffer.clear();
            }

            // Finalize the encoder, get its remaining buffer data, and yield those bytes.
            let buffer = encoder.finish().unwrap();
            let emit_bytes = Bytes::copy_from_slice(buffer.as_slice());
            let body = Message::FileBody { data: emit_bytes };
            emitter
                .emit(Bytes::copy_from_slice(
                    to_bytes_with_alloc::<_, rancor::Error>(&body, arena.acquire())
                        .unwrap()
                        .as_slice(),
                ))
                .await;

            // Temp file contents separator for debugging
            let md5_final = md5.finalize();
            let sha256_final = sha256.finalize();

            let footer = Message::FileFooter {
                md5: md5_final.into(),
                sha256: sha256_final.into(),
            };

            emitter
                .emit(Bytes::copy_from_slice(
                    to_bytes_with_alloc::<_, rancor::Error>(&footer, arena.acquire())
                        .unwrap()
                        .as_slice(),
                ))
                .await;
        }

        Ok(())
    })
}
