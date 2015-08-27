use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use consts::{KeeperState, OpCode, WatchedEventType};
use std::io::{Cursor, Read, Write, Result, Error, ErrorKind};
use num::FromPrimitive;

pub type ByteBuf = Cursor<Vec<u8>>;

pub trait ReadFrom {
    fn read_from<R: Read>(read: &mut R) -> Self;
}

pub trait WriteTo {
    fn write_to(&self, writer: &mut Write) -> Result<()>;

    #[allow(unused_must_use)]
    fn to_len_prefixed_buf(&self) -> ByteBuf {
        let mut buf = Cursor::new(Vec::new());
        buf.set_position(4);
        self.write_to(&mut buf);
        let len = buf.position() - 4;
        buf.set_position(0);
        buf.write_i32::<BigEndian>(len as i32);
        buf.set_position(0);
        buf
    }
}

#[allow(unused_must_use)]
pub fn to_len_prefixed_buf<Request: WriteTo>(rh: RequestHeader, req: Request) -> ByteBuf {
    let mut buf = Cursor::new(Vec::new());
    buf.set_position(4);
    rh.write_to(&mut buf);
    req.write_to(&mut buf);
    let len = buf.position() - 4;
    buf.set_position(0);
    buf.write_i32::<BigEndian>(len as i32);
    buf.set_position(0);
    buf
}

trait StringReader: Read {
    fn read_string(&mut self) -> Result<String>;
}

pub trait BufferReader: Read {
    fn read_buffer(&mut self) -> Result<Vec<u8>>;
}

impl <R: Read> StringReader for R {
    fn read_string(&mut self) -> Result<String> {
        let raw = try!(self.read_buffer());
        Ok(String::from_utf8(raw).unwrap())
    }
}

// A buffer is an u8 string prefixed with it's length as i32
impl <R: Read> BufferReader for R {
    fn read_buffer(&mut self) -> Result<Vec<u8>> {
        let len = try!(self.read_i32::<BigEndian>()) as usize;
        let mut buf = vec![0; len];
        let read = try!(self.read(&mut buf));
        if read == len {
            Ok(buf)
        } else {
            Err(Error::new(ErrorKind::Other, "read_buffer failed"))
        }
    }
}

impl WriteTo for u8 {
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(writer.write_u8(*self));
        Ok(())
    }
}

impl WriteTo for String {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(writer.write_i32::<BigEndian>(self.len() as i32));
        writer.write_all(self.as_ref())
    }
}

impl <T: WriteTo> WriteTo for Vec<T> {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(writer.write_i32::<BigEndian>(self.len() as i32));
        let mut res = Ok(());
        for elem in self.iter() {
            res = elem.write_to(writer);
            if res.is_err() {
                return res
            }
        }
        res
    }
}

#[derive(Clone,Debug)]
pub struct Acl {
    pub perms: i32,
    pub scheme: String,
    pub id: String
}

impl ReadFrom for Acl {
    fn read_from<R: Read>(read: &mut R) -> Acl {
        let perms = read.read_i32::<BigEndian>().unwrap();
        let scheme = read.read_string().unwrap();
        let id = read.read_string().unwrap();
        Acl{perms: perms, scheme: scheme, id: id}
    }
}

impl WriteTo for Acl {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(writer.write_i32::<BigEndian>(self.perms));
        try!(self.scheme.write_to(writer));
        self.id.write_to(writer)
    }
}

#[derive(Debug)]
pub struct Stat {
    pub czxid: i64,
    pub mzxid: i64,
    pub ctime: i64,
    pub mtime: i64,
    pub version: i32,
    pub cversion: i32,
    pub aversion: i32,
    pub ephemeral_owner: i64,
    pub data_length: i32,
    pub num_children: i32,
    pub pzxid: i64
}

impl ReadFrom for Stat {
    fn read_from<R: Read>(read: &mut R) -> Stat {
        Stat{
            czxid: read.read_i64::<BigEndian>().unwrap(),
            mzxid: read.read_i64::<BigEndian>().unwrap(),
            ctime: read.read_i64::<BigEndian>().unwrap(),
            mtime: read.read_i64::<BigEndian>().unwrap(),
            version: read.read_i32::<BigEndian>().unwrap(),
            cversion: read.read_i32::<BigEndian>().unwrap(),
            aversion: read.read_i32::<BigEndian>().unwrap(),
            ephemeral_owner: read.read_i64::<BigEndian>().unwrap(),
            data_length: read.read_i32::<BigEndian>().unwrap(),
            num_children: read.read_i32::<BigEndian>().unwrap(),
            pzxid: read.read_i64::<BigEndian>().unwrap()}
    }
}

pub struct ConnectRequest {
    protocol_version: i32,
    last_zxid_seen: i64,
    timeout: i32,
    session_id: i64,
    passwd: Vec<u8>,
    read_only: bool
}

impl ConnectRequest {
    pub fn from(conn_resp: ConnectResponse, last_zxid_seen: i64) -> ConnectRequest {
        ConnectRequest{
            protocol_version: conn_resp.protocol_version,
            last_zxid_seen: last_zxid_seen,
            timeout: conn_resp.timeout as i32,
            session_id: conn_resp.session_id,
            passwd: conn_resp.passwd,
            read_only: conn_resp.read_only}
    }
}

impl WriteTo for ConnectRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(writer.write_i32::<BigEndian>(self.protocol_version));
        try!(writer.write_i64::<BigEndian>(self.last_zxid_seen));
        try!(writer.write_i32::<BigEndian>(self.timeout));
        try!(writer.write_i64::<BigEndian>(self.session_id));
        try!(self.passwd.write_to(writer));
        try!(writer.write_u8(self.read_only as u8));
        Ok(())
    }
}

#[derive(Debug)]
pub struct ConnectResponse {
    protocol_version: i32,
    pub timeout: u64, // is handled as i32
    session_id: i64,
    passwd: Vec<u8>,
    pub read_only: bool
}

impl ConnectResponse {
    pub fn initial(timeout: u64) -> ConnectResponse {
        ConnectResponse{
            protocol_version: 0,
            timeout: timeout,
            session_id: 0,
            passwd: [0;16].to_vec(),
            read_only: false}
    }
}

impl ReadFrom for ConnectResponse {
    fn read_from<R: Read>(reader: &mut R) -> ConnectResponse {
        ConnectResponse{
            protocol_version: reader.read_i32::<BigEndian>().unwrap(),
            timeout: reader.read_i32::<BigEndian>().unwrap() as u64,
            session_id: reader.read_i64::<BigEndian>().unwrap(),
            passwd: reader.read_buffer().unwrap(),
            read_only: reader.read_u8().unwrap() != 0}
    }
}

pub struct RequestHeader {
    pub xid: i32,
    pub opcode: OpCode
}

impl WriteTo for RequestHeader {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(writer.write_i32::<BigEndian>(self.xid));
        try!(writer.write_i32::<BigEndian>(self.opcode as i32));
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReplyHeader {
    pub xid: i32,
    pub zxid: i64,
    pub err: i32
}

impl ReadFrom for ReplyHeader {
    fn read_from<R: Read>(read: &mut R) -> ReplyHeader {
        let xid = read.read_i32::<BigEndian>().unwrap();
        let zxid = read.read_i64::<BigEndian>().unwrap();
        let err = read.read_i32::<BigEndian>().unwrap();
        ReplyHeader{xid: xid, zxid: zxid, err: err}
    }
}

pub struct CreateRequest {
    pub path: String,
    pub data: Vec<u8>,
    pub acl: Vec<Acl>,
    pub flags: i32
}

impl WriteTo for CreateRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(self.path.write_to(writer));
        try!(self.data.write_to(writer));
        try!(self.acl.write_to(writer));
        try!(writer.write_i32::<BigEndian>(self.flags));
        Ok(())
    }
}

pub struct CreateResponse {
    pub path: String
}

impl ReadFrom for CreateResponse {
    fn read_from<R: Read>(reader: &mut R) -> CreateResponse {
        CreateResponse{path: reader.read_string().unwrap()}
    }
}

pub struct DeleteRequest {
    pub path: String,
    pub version: i32
}

impl WriteTo for DeleteRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(self.path.write_to(writer));
        try!(writer.write_i32::<BigEndian>(self.version));
        Ok(())
    }
}

struct StringAndBoolRequest {
    pub path: String,
    pub watch: bool
}

impl WriteTo for StringAndBoolRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(self.path.write_to(writer));
        try!(writer.write_u8(self.watch as u8));
        Ok(())
    }
}

pub type ExistsRequest = StringAndBoolRequest;
pub type ExistsResponse = StatResponse;

pub struct StatResponse {
    pub stat: Stat
}

impl ReadFrom for StatResponse {
    fn read_from<R: Read>(read: &mut R) -> StatResponse {
        StatResponse{stat: Stat::read_from(read)}
    }
}

pub struct GetAclRequest {
    pub path: String
}

impl WriteTo for GetAclRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        self.path.write_to(writer)
    }
}

pub struct GetAclResponse {
    pub acl_stat: (Vec<Acl>, Stat)
}

impl ReadFrom for GetAclResponse {
    fn read_from<R: Read>(reader: &mut R) -> GetAclResponse {
        let len = reader.read_i32::<BigEndian>().unwrap();
        let mut acl = Vec::new();
        for _ in 0..len {
            acl.push(Acl::read_from(reader));
        }
        let stat = Stat::read_from(reader);
        GetAclResponse{acl_stat: (acl, stat)}
    }
}

pub struct SetAclRequest {
    pub path: String,
    pub acl: Vec<Acl>,
    pub version: i32
}

impl WriteTo for SetAclRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(self.path.write_to(writer));
        try!(self.acl.write_to(writer));
        try!(writer.write_i32::<BigEndian>(self.version));
        Ok(())
    }
}

pub type SetAclResponse = StatResponse;

pub struct SetDataRequest {
    pub path: String,
    pub data: Vec<u8>,
    pub version: i32
}

impl WriteTo for SetDataRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        try!(self.path.write_to(writer));
        try!(self.data.write_to(writer));
        try!(writer.write_i32::<BigEndian>(self.version));
        Ok(())
    }
}

pub type SetDataResponse = StatResponse;

pub type GetChildrenRequest = StringAndBoolRequest;

pub struct GetChildrenResponse {
    pub children: Vec<String>
}

impl ReadFrom for GetChildrenResponse {
    fn read_from<R: Read>(reader: &mut R) -> GetChildrenResponse {
        let len = reader.read_i32::<BigEndian>().unwrap();
        let mut children = Vec::new();
        for _ in 0..len {
            children.push(reader.read_string().unwrap());
        }
        GetChildrenResponse{children: children}
    }
}

pub type GetDataRequest = StringAndBoolRequest;

pub struct GetDataResponse {
    pub data_stat: (Vec<u8>, Stat)
}

impl ReadFrom for GetDataResponse {
    fn read_from<R: Read>(reader: &mut R) -> GetDataResponse {
        let data = reader.read_buffer().unwrap();
        let stat = Stat::read_from(reader);
        GetDataResponse{data_stat: (data, stat)}
    }
}

pub struct AuthRequest {
    pub typ: i32,
    pub scheme: String,
    pub auth: Vec<u8>
}

impl WriteTo for AuthRequest {
    #[allow(unused_must_use)]
    fn write_to(&self, writer: &mut Write) -> Result<()> {
        writer.write_i32::<BigEndian>(self.typ);
        self.scheme.write_to(writer);
        self.auth.write_to(writer)
    }
}

pub struct EmptyRequest;
pub struct EmptyResponse;

impl WriteTo for EmptyRequest {
    fn write_to(&self, _: &mut Write) -> Result<()> { Ok(()) }
}

impl ReadFrom for EmptyResponse {
    fn read_from<R: Read>(_: &mut R) -> EmptyResponse {
        EmptyResponse
    }
}

#[derive(Debug)]
pub struct WatchedEvent {
    pub event_type: WatchedEventType,
    pub keeper_state: KeeperState,
    pub path: Option<String>
}

impl ReadFrom for WatchedEvent {
    fn read_from<R: Read>(reader: &mut R) -> WatchedEvent {
        let typ = reader.read_i32::<BigEndian>().unwrap();
        let state = reader.read_i32::<BigEndian>().unwrap();
        let path = reader.read_string().unwrap();
        WatchedEvent{
            event_type: FromPrimitive::from_i32(typ).unwrap(),
            keeper_state: FromPrimitive::from_i32(state).unwrap(),
            path: Some(path)}
    }
}
