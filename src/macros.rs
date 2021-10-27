#[macro_export]
macro_rules! middleware {
    (|$ctx:ident| {$($b:tt)+}) => {{
        let closure = |$ctx: $crate::router::MiddlewareCtx| -> $crate::router::HandlerFut {
            Box::pin(async move {
                let mut $ctx = $ctx.lock();
                $($b)+;
                Ok(())
            })
        };
        closure
    }};
}
