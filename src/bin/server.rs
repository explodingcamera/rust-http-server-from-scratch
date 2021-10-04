use anyhow::Result;
use tokio::runtime;
use webserver_from_scratch::HTTPServer;

fn main() -> Result<()> {
    let server = HTTPServer::new();
    let rt = runtime::Runtime::new()?;
    rt.block_on(server.listen("[::1]:8080".parse().unwrap()))
}
