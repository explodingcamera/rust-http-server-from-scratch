use anyhow::Result;
use futures::future::BoxFuture;
use parking_lot::Mutex;
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};
use tokio::net::TcpStream;

// https://stackoverflow.com/questions/27883509/can-you-clone-a-closure

use crate::{
    http_request::{Method, Request},
    http_response::ResponseBuilder,
    HTTPServer,
};

pub trait HandlerFn = Fn(MiddlewareCtx) -> HandlerFut + Send + 'static + Sync + ?Sized;

// pub trait HandlerFn =
//     (Fn(&MiddlewareContext) -> dyn Future<Output = Result<()>>) + Send + 'static + Sync + ?Sized;

// A Box<Future> is `impl<F: ?Sized + Future> Future for Box<F>`
// so we can just use it to get an unsized future (so we can call it later on)
// however we also need to pin it, so they actually implement the future trait.
// BoxFuture is a helper type which does this for us:
pub type HandlerFut = BoxFuture<'static, Result<(), anyhow::Error>>;
// for reference, the equivalent would be
// pub type HandlerFut = Box<dyn Future<Output = Result<()>> + Unpin + Send + 'static>;

pub type MiddlewareCtx = Arc<Mutex<MiddlewareContext>>;
pub struct MiddlewareContext {
    /// Current request
    pub request: Request,

    /// Response builder
    pub response: ResponseBuilder,

    /// Params
    pub params: BTreeMap<String, RequestPathParams>,

    /// Socket
    pub socket: TcpStream,

    /// End the request prematurely
    ended: bool,

    // enable raw socket mode
    raw: bool,
}

impl MiddlewareContext {
    pub fn new(request: Request, response: ResponseBuilder, socket: TcpStream) -> Self {
        Self {
            socket,
            request,
            response,
            ended: false,
            params: BTreeMap::new(),
            raw: false,
        }
    }

    pub fn set_raw(&mut self, val: bool) {
        self.raw = val;
    }

    pub fn is_raw(&mut self) -> bool {
        self.raw
    }

    pub fn end(&mut self) {
        self.ended = true
    }

    pub fn has_ended(&self) -> bool {
        self.ended
    }
}

pub struct Route {
    pub path: String,
    pub method: Option<Method>,
    pub handler: Arc<Box<dyn HandlerFn>>,
}

impl Debug for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Point")
            .field("path", &self.path)
            .field("method", &self.method)
            .field("handler", &"[handlerFn]".to_string())
            .finish()
    }
}

impl Clone for Route {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            method: self.method.clone(),
            handler: self.handler.clone(),
        }
    }
}

pub trait Router<F>
where
    F: HandlerFn,
{
    fn handle(&mut self, method: Method, path: &str, handler: F) -> &mut Self;
    fn any(&mut self, path: &str, handler: F) -> &mut Self;
    fn get(&mut self, path: &str, handler: F) -> &mut Self;
    fn head(&mut self, path: &str, handler: F) -> &mut Self;
    fn post(&mut self, path: &str, handler: F) -> &mut Self;
    fn put(&mut self, path: &str, handler: F) -> &mut Self;
    fn delete(&mut self, path: &str, handler: F) -> &mut Self;
    fn connect(&mut self, path: &str, handler: F) -> &mut Self;
    fn options(&mut self, path: &str, handler: F) -> &mut Self;
    fn trace(&mut self, path: &str, handler: F) -> &mut Self;
    fn patch(&mut self, path: &str, handler: F) -> &mut Self;
}

#[derive(Debug, Clone)]
pub struct RequestPath {
    pub path: String,
    pub params: BTreeMap<String, RequestPathParams>,
}

#[derive(Debug, Clone)]
pub struct RequestPathParams {
    pub param: String,
    pub value: String,
}

pub fn middleware_matches_request(request: &Request, route: &Route) -> Result<Option<RequestPath>> {
    let request_path = request.path.clone().unwrap_or_else(|| "".to_string());
    let request_segments = request_path.split('/').peekable();

    let route_path = &route.path;
    let mut route_segments = route_path.split('/').peekable();

    let mut params: BTreeMap<String, RequestPathParams> = BTreeMap::new();

    for request_segment in request_segments {
        let route_segment = match route_segments.next() {
            Some(val) => val,
            None => return Ok(None),
        };

        match route_segment {
            // wildcard parameter
            "*" => {
                // path = `/*/123/sadfsadf`   will behave like a param
                // path = `/123/*/`           will also behave like a param
                // path = `/123/*`            will accept any path, even when nesting /'s (e.g `/123/456`, `/123/456/789`)

                if route_segments.peek().is_none() && !route_path.ends_with('/') {
                    break;
                }

                params.insert(
                    "*".to_string(),
                    RequestPathParams {
                        param: "*".to_string(),
                        value: request_segment.to_string(),
                    },
                );
            }

            // named param
            s if s.starts_with(':') => {
                params.insert(
                    s.to_string(),
                    RequestPathParams {
                        param: s.to_string(),
                        value: request_segment.to_string(),
                    },
                );
            }

            // not matching
            s if s != request_segment => return Ok(None),

            // matching
            _ => {}
        }
    }

    let path = RequestPath {
        path: request_path,
        params,
    };

    Ok(Some(path))
}

impl<F> Router<F> for HTTPServer
where
    F: HandlerFn,
{
    fn handle(&mut self, method: Method, path: &str, handler: F) -> &mut Self {
        let handler: Arc<Box<dyn HandlerFn>> = Arc::new(Box::new(handler));
        let route: Route = Route {
            path: path.to_string(),
            method: Some(method),
            handler,
        };

        self.add_route(route);
        self
    }

    fn any(&mut self, path: &str, handler: F) -> &mut Self {
        let handler: Arc<Box<dyn HandlerFn>> = Arc::new(Box::new(handler));
        let route: Route = Route {
            path: path.to_string(),
            method: None,
            handler,
        };
        self.add_route(route);
        self
    }

    fn get(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::GET, path, handler)
    }
    fn head(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::HEAD, path, handler)
    }
    fn post(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::POST, path, handler)
    }
    fn put(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::PUT, path, handler)
    }
    fn delete(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::DELETE, path, handler)
    }
    fn connect(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::CONNECT, path, handler)
    }
    fn options(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::OPTIONS, path, handler)
    }
    fn trace(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::TRACE, path, handler)
    }
    fn patch(&mut self, path: &str, handler: F) -> &mut Self {
        self.handle(Method::PATCH, path, handler)
    }
}
