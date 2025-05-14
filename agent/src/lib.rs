mod stream;
mod web_service;

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            web_service::start(9001, &["/"]).await.unwrap();
        })
}
