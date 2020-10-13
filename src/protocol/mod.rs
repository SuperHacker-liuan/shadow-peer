use self::net_proto::Establish;
use crate::error::Error;
use crate::error::Result;
use async_std::future::timeout;
use async_std::io::prelude::WriteExt as Write;
use async_std::io::ReadExt as Read;
use serde::Deserialize;
use serde::Serialize;
use std::time::Duration;

pub mod net_proto;
pub mod v0;

pub type ClientId = String;

#[derive(Debug, Serialize, Deserialize)]
pub enum Protocol {
    ClientId(String),
    Establish(Establish),
    Ping(u16),
}

pub const CURRENT_VERSION: u8 = 0;

pub async fn read_protocol_timeout<R>(reader: &mut R, tmout: u64) -> Result<Protocol>
where
    R: Read + Unpin,
{
    let dur = Duration::from_secs(tmout);
    timeout(dur, async { read_protocol(reader).await }).await?
}

pub async fn read_protocol<R>(reader: &mut R) -> Result<Protocol>
where
    R: Read + Unpin,
{
    let mut header = [0u8; 4];
    reader.read_exact(&mut header).await?;
    let version = header[0];
    let r = match version {
        0 => v0::parse(reader, header).await?,
        ver => Err(Error::UnsupportedVersion(ver))?,
    };
    Ok(r)
}

pub async fn write_protocol<W>(writer: &mut W, version: u8, proto: &Protocol) -> Result<()>
where
    W: Write + Unpin,
{
    let buf = match version {
        0 => v0::build_protocol(proto),
        ver => Err(Error::UnsupportedVersion(ver))?,
    }?;
    writer.write_all(buf.as_slice()).await?;
    Ok(())
}
