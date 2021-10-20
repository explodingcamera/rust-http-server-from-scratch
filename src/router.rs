use std::{ops::DerefMut, pin::Pin, sync::Arc};

use anyhow::Result;
use futures::Future;

// https://stackoverflow.com/questions/27883509/can-you-clone-a-closure

use crate::{
    http_request::{Method, Request},
    http_response::ResponseBuilder,
    HTTPServer,
};

pub struct MiddlewareContext {
    pub request: Arc<Request>,
    pub response: ResponseBuilder,
    ended: bool,
}

impl MiddlewareContext {
    pub fn new(request: Arc<Request>, response: ResponseBuilder) -> Self {
        Self {
            request,
            response,
            ended: false,
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

    // fn get(&mut self, path: String, handler: &dyn MiddlewareT<D>) -> &mut Self;
    // fn head(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn post(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn put(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn delete(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn connect(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn options(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn trace(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn patch(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
    // fn any(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self;
}

#[derive(Debug)]
pub struct RequestPath {
    pub path: String,
    pub params: Vec<RequestPathParams>,
}

#[derive(Debug)]
pub struct RequestPathParams {
    pub param: String,
    pub value: String,
}

pub fn middleware_matches_request(request: &Request, route: &Route) -> Result<Option<RequestPath>> {
    let request_path = request.path.clone().unwrap_or("".to_string());
    let request_segments = request_path.split("/").peekable();

    let route_path = &route.path;
    let mut route_segments = route_path.split("/").peekable();

    let mut params: Vec<RequestPathParams> = Vec::new();

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

                params.push(RequestPathParams {
                    param: "*".to_string(),
                    value: request_segment.to_string(),
                })
            }

            // named param
            s if s.starts_with(":") => params.push(RequestPathParams {
                param: s.to_string(),
                value: request_segment.to_string(),
            }),

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

    // fn get(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::GET),
    //     });
    //     self
    // }

    // fn head(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::HEAD),
    //     });
    //     self
    // }
    // fn post(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::POST),
    //     });
    //     self
    // }
    // fn put(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::PUT),
    //     });
    //     self
    // }
    // fn delete(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::DELETE),
    //     });
    //     self
    // }
    // fn connect(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::CONNECT),
    //     });
    //     self
    // }
    // fn options(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::OPTIONS),
    //     });
    //     self
    // }
    // fn trace(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::TRACE),
    //     });
    //     self
    // }
    // fn patch(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: Some(Method::PATCH),
    //     });
    //     self
    // }
    // fn any(&mut self, path: String, handler: &'static dyn RequestHandlerT) -> &mut Self {
    //     self.add_route(&Route {
    //         path,
    //         handler,
    //         method: None,
    //     });
    //     self
    // }
}
