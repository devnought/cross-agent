use std::{io, path::Path};

use async_fn_stream::{TryFnStream, try_fn_stream};
use bytes::{Bytes, BytesMut};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};
use walkdir::{DirEntry, WalkDir};

pub fn file_iter<P: AsRef<Path>>(root: P) -> impl Iterator<Item = DirEntry> {
    #[cfg(any(target_os = "android", target_os = "linux"))]
    let iter = WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            path != Path::new("/dev") && path != Path::new("/proc") && path != Path::new("/sys")
        });

    #[cfg(target_os = "windows")]
    let iter = WalkDir::new(root)
        // .follow_links(true)
        .into_iter();

    let iter_common = iter.filter_map(Result::ok).filter(|e| {
        if let Ok(meta) = e.metadata() {
            meta.is_file()
        } else {
            false
        }
    });

    iter_common
}

pub fn build_stream<P>(
    root: P,
) -> TryFnStream<Bytes, io::Error, impl Future<Output = Result<(), io::Error>>>
where
    P: AsRef<Path>,
{
    try_fn_stream(|emitter| async move {
        let iter = file_iter(root);
        let mut bytes = BytesMut::with_capacity(1024 * 8);

        for entry in iter {
            let file = File::open(entry.path()).await?;
            let mut reader = BufReader::new(file);

            println!("--- {}", entry.path().display());

            while reader.read_buf(&mut bytes).await? > 0 {
                // println!("Read {} bytes", bytes.len());
                emitter.emit(bytes.clone().into()).await;
                bytes.clear();
            }
        }

        Ok(())
    })
}
