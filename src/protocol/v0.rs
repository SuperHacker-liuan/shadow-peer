//! Frame defination, In BigEndian
//!  0                                          31
//! ---------------------------------------------
//! |ver 8bit|opcode 8bit|len / param 16bit     |
//! ---------------------------------------------
//! Json / other, depend on opcode......(len)
//! opcode: 0 -> parse Json

use super::Read;
use crate::error::Error;
use crate::error::Result;
use byteorder::BigEndian;
use byteorder::ReadBytesExt;
use byteorder::WriteBytesExt;

use super::Protocol;
use std::io::Cursor;

mod opcode {
    pub const PING: u8 = 0;
    pub const JSON: u8 = 1;
}

pub async fn parse<R>(reader: &mut R, header: [u8; 4]) -> Result<Protocol>
where
    R: Read + Unpin,
{
    let mut header = Cursor::new(header);
    header.read_u8()?; // drop version
    let op = header.read_u8()?;
    let len = header.read_u16::<BigEndian>()?;
    let protocol = match op {
        opcode::PING => Protocol::Ping(len),
        opcode::JSON => parse_json(reader, len as usize).await?,
        _ => Err(Error::InvalidOperation(format!("invalid opcode {}", op)))?,
    };
    Ok(protocol)
}

pub fn build_protocol(proto: &Protocol) -> Result<Vec<u8>> {
    let mut buf = vec![];
    buf.write_u8(0)?;
    let (op, param, append) = match proto {
        Protocol::Ping(ts) => (opcode::PING, *ts, None),
        _ => {
            let json = serde_json::to_vec(proto)?;
            let len = json.len();
            if len > u16::MAX as usize {
                Err(Error::InvalidOperation(format!("Json too long: {}", len)))?;
            }
            (opcode::JSON, len as u16, Some(json))
        }
    };
    buf.write_u8(op)?;
    buf.write_u16::<BigEndian>(param as u16)?;
    if let Some(mut append) = append {
        buf.append(&mut append);
    }
    Ok(buf)
}

async fn parse_json<R>(reader: &mut R, len: usize) -> Result<Protocol>
where
    R: Read + Unpin,
{
    let mut buf = vec![0u8; len];
    reader.read_exact(buf.as_mut_slice()).await?;
    let r = serde_json::from_slice(buf.as_slice())?;
    Ok(r)
}
