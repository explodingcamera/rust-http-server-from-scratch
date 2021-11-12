// Usage:
// ---- middleware!(|ctx| {
// ----    // User Code (can use .await)
// ---- })
// Resulting Code:
// ---- {
// ----     let closure = |ctx: MiddlewareCtx| -> HandlerFut {
// ----         Box::pin(async move {
// ----             let mut ctx = ctx.lock(); // will be locked after `ctx` is dropped at the end of this block
// ----             // User code
// ----             Ok(()) // enable error catching by returning a result (enables using `?` for catching errors)
// ----         })
// ----     };
// ----     closure // return closure from this block
// ---- }
#[macro_export]
macro_rules! middleware {
    (|$ctx:ident| {$($b:tt)+}) => {{
        let closure = |$ctx: $crate::router::MiddlewareCtx| -> $crate::router::HandlerFut {
            Box::pin(async move {
                let mut $ctx = $ctx.lock();
                let mut func = async move || -> Result<(), anyhow::Error> {
                    $($b)+;
                    Ok(())
                };
                func().await
            })
        };
        closure
    }};
}
