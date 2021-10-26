use std::sync::Arc;

use anyhow::Result;
use tokio::spawn;

fn apply<F, Fut>(f: F)
where
    F: Fn() -> Fut + Send + 'static + Sync,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let func = Arc::new(f);

    func();

    let f1 = func.clone();
    let f2 = func.clone();

    spawn(async move { f1().await });
    spawn(async move { f2().await });
}

fn main() -> Result<()> {
    apply(|| async {});

    Ok(())
}
