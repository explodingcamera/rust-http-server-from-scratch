use anyhow::{anyhow, Result};
use bytes::{Buf, BytesMut};

#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub fin: bool,
    pub rsv1: bool,
    pub rsv2: bool,
    pub rsv3: bool,
    pub opcode: u8,
    pub mask: Option<u32>,
    pub data_length: DataLength,
    pub header_length: u64,
}

#[derive(Debug, Clone)]
pub enum DataLength {
    Small(u8),
    Medium(u16),
    Large(u64),
}

impl FrameHeader {
    pub fn to_bytes(&self) -> BytesMut {
        unimplemented!()

        // let mut buf = BytesMut::new();
    }

    pub fn from_bytes(buf: &mut BytesMut) -> Result<Self> {
        if buf.len() < 2 {
            return Err(anyhow!("header too short"));
        }

        let first = buf.get_u8();
        let second = buf.get_u8();

        let fin = first & 0x80 != 0;
        let rsv1 = first & 0x40 != 0;
        let rsv2 = first & 0x20 != 0;
        let rsv3 = first & 0x10 != 0;
        let opcode = first & 0x0f;
        let masked = second & 0x80 != 0;

        let mut header_length = 2;
        let length_byte = second & 0x7F;

        let data_length: DataLength = match length_byte {
            // Extended payload length continued, if payload len == 127
            127 => {
                if buf.len() < 10 {
                    return Err(anyhow!("payload: length too large: {}", buf.len()));
                }

                header_length += 8;
                buf.advance(2);
                DataLength::Large(buf.get_u64())
            }
            // Extended payload length, (if payload len==126/127)
            126 => {
                if buf.len() < 4 {
                    return Err(anyhow!("payload: length too large: {}", buf.len()));
                }

                header_length += 2;
                buf.advance(2);
                DataLength::Medium(buf.get_u16())
            }
            // Payload len (7)
            len => {
                if len > 126 {
                    return Err(anyhow!("payload: length too large: {}", len));
                }

                buf.advance(2);
                DataLength::Small(len)
            }
        };

        let mask = if masked {
            None
        } else {
            if buf.len() < 4 {
                return Err(anyhow!("mask: length too small"));
            }

            header_length += 4;
            Some(buf.get_u32())
        };

        Ok(Self {
            fin,
            data_length,
            header_length,
            mask,
            opcode,
            rsv1,
            rsv2,
            rsv3,
        })
    }
}
