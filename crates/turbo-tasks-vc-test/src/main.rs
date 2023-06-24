#![feature(arbitrary_self_types)]

use anyhow::Result;

fn main() {
    use turbo_tasks_malloc::TurboMalloc;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .on_thread_stop(|| {
            TurboMalloc::thread_stop();
        })
        .build()
        .unwrap()
        .block_on(main_inner())
        .unwrap()
}

async fn main_inner() -> Result<()> {
    vc_test::run().await
}
