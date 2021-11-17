use crate::{router::MiddlewareContext, websocket::mask::apply_mask};
use anyhow::Result;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use parking_lot::MutexGuard;
use sha1::{Digest, Sha1};
use std::ops::DerefMut;
use std::{io::ErrorKind, str::from_utf8};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::frame::FrameHeader;

pub async fn accept_websocket<'a>(ctx: &mut MutexGuard<'a, MiddlewareContext>) -> Result<()> {
    println!("got incoming websocket connection");

    let connection = match ctx.request.headers.get_str("Connection") {
        Err(_) => return Ok(()),
        Ok(v) => v,
    };

    let upgrade = match ctx.request.headers.get_str("Upgrade") {
        Err(_) => return Ok(()),
        Ok(v) => v,
    };

    if connection != "Upgrade" || upgrade != "websocket" {
        return Ok(());
    }

    let version = ctx.request.headers.get_str("Sec-WebSocket-Version")?;
    let key = ctx.request.headers.get_str("Sec-WebSocket-Key")?;

    println!("websocket client connected with version {}", version);

    let mut hasher = Sha1::new();
    hasher.update(key);
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let result = hasher.finalize();
    let accept = base64::encode(result);

    // ctx.set_raw(true);
    let mut resp = BytesMut::new();
    resp.put_slice(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ");

    ctx.socket.write(b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ").await?;
    ctx.socket.write(&accept.as_bytes()).await?;
    ctx.socket.write(b"\r\n\r\n").await?;

    loop {
        ctx.socket.readable().await?;
        let mut buf = BytesMut::with_capacity(65536);

        match ctx.socket.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                println!("read {} bytes", n);
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            } // https://github.com/1tgr/rust-websocket-lite/blob/master/websocket-codec/src/frame.rs
        };

        let mut buf = buf.freeze();
        let header = FrameHeader::from_bytes(&mut buf);
        match header {
            Ok(header) => {
                println!("got websocket data:\n  header: {:?}", header);
                if let Some(mask) = header.mask {
                    let mut data = buf.to_vec();
                    apply_mask(&mut data, mask);

                    if let Some(data) = from_utf8(&data).ok() {
                        println!("  data: {:?}", data);
                    }
                }
            }
            Err(e) => {
                println!("got invalid data: {:?}", e);
            }
        }
    }

    Ok(())
}
