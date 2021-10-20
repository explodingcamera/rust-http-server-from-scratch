use std::{collections::BTreeMap, sync::Arc};

use anyhow::Result;

// https://stackoverflow.com/questions/27883509/can-you-clone-a-closure

use crate::{
    http_request::{Method, Request},
    http_response::ResponseBuilder,
    HTTPServer,
};

pub struct MiddlewareContext {
    /// Current request
    pub request: Arc<Request>,

    /// Response builder
    pub response: ResponseBuilder,

    /// Params
    pub params: BTreeMap<String, RequestPathParams>,

    /// End the request prematurely
    ended: bool,
}

impl MiddlewareContext {
    pub fn new(request: Arc<Request>, response: ResponseBuilder) -> Self {
        Self {
            request,
            response,
            ended: false,
            params: BTreeMap::new(),
        }
    }
    pub fn end(&mut self) {
        self.ended = true
    }

    pub fn has_ended(&self) -> bool {
        self.ended
    }
}

#[derive(Clone, Debug)]
pub struct Route {
    pub path: String,
    pub method: Option<Method>,
    pub handler: u64,
}

pub type Handler = Box<dyn FnMut(&mut MiddlewareContext)>;

pub trait Router {
    fn handle(&mut self, method: Method, path: &str, handler: Handler) -> &mut Self;
    fn any(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn get(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn head(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn post(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn put(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn delete(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn connect(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn options(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn trace(&mut self, path: &str, handler: Handler) -> &mut Self;
    fn patch(&mut self, path: &str, handler: Handler) -> &mut Self;
}

#[derive(Debug)]
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
    let request_path = request.path.clone().unwrap_or("".to_string());
    let request_segments = request_path.split("/").peekable();

    let route_path = &route.path;
    let mut route_segments = route_path.split("/").peekable();

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

                if route_segments.peek().is_none() && !route_path.ends_with("/") {
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
            s if s.starts_with(":") => {
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

impl Router for HTTPServer {
    fn handle(&mut self, method: Method, path: &str, handler: Handler) -> &mut Self {
        self.handler_count += 1;
        let route = Route {
            path: path.to_string(),
            handler: self.handler_count,
            method: Some(method),
        };
        self.add_handler(self.handler_count, handler);
        self.add_route(route);
        self
    }

    fn any(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handler_count += 1;
        let route = Route {
            path: path.to_string(),
            handler: self.handler_count,
            method: None,
        };
        self.add_handler(self.handler_count, handler);
        self.add_route(route);
        self
    }

    fn get(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::GET, path, handler)
    }
    fn head(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::HEAD, path, handler)
    }
    fn post(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::POST, path, handler)
    }
    fn put(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::PUT, path, handler)
    }
    fn delete(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::DELETE, path, handler)
    }
    fn connect(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::CONNECT, path, handler)
    }
    fn options(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::OPTIONS, path, handler)
    }
    fn trace(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::TRACE, path, handler)
    }
    fn patch(&mut self, path: &str, handler: Handler) -> &mut Self {
        self.handle(Method::PATCH, path, handler)
    }
}
