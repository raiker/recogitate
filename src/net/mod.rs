use std::io::prelude::*;
use std::net::TcpStream;
use std::io;
use std::str;
use std::io::BufReader;
use rustc_serialize::json;
use byteorder::{LittleEndian, BigEndian, WriteBytesExt, ReadBytesExt};
use err::{ConnectionError, DataError};

mod scram;

const PROTOCOL_VERSION: u64 = 0;

//connection
pub struct Connection {
	br: BufReader<TcpStream>,
	next_token: u64,
}

impl Connection {
	fn get_next_token(&mut self) -> u64 {
		let t = self.next_token;
		self.next_token = self.next_token + 1;
		t
	}
	
	pub fn send_query(&mut self, query: &json::Json) -> io::Result<u64> {
		let serialised_query = format!("{}", query);
		let token = self.get_next_token();
		let length = serialised_query.len() as u32;
		
		try!(self.br.get_mut().write_u64::<BigEndian>(token));
		try!(self.br.get_mut().write_u32::<LittleEndian>(length));
		try!(self.br.get_mut().write_all(serialised_query.as_bytes()));
		Ok(token)
	}
	
	pub fn recv_response(&mut self) -> io::Result<json::Json> {
		let token = try!(self.br.read_u64::<BigEndian>());
		let length = try!(self.br.read_u32::<LittleEndian>()) as usize;
		let mut buf = Vec::new();
		buf.resize(length, 0);
		try!(self.br.read_exact(buf.as_mut_slice()));
		let ret_msg = str::from_utf8(&buf).unwrap();
		Ok(json::Json::from_str(ret_msg).unwrap())
	}
}

struct AuthConnection {
	br: BufReader<TcpStream>
}

impl AuthConnection {
	fn send_packet(&mut self, packet: &json::Json) -> io::Result<()> {
		try!(write!(self.br.get_mut(),"{}", packet));
		self.br.get_mut().write_all(&[0x00])
	}
	
	fn recv_packet(&mut self) -> Result<json::Json, ConnectionError> {
		let mut buffer = Vec::new();
		
		let bytes_read = try!(self.br.read_until(0x00, &mut buffer));
		
		if bytes_read == 0 {
			return Err(ConnectionError::Io(io::Error::new(io::ErrorKind::UnexpectedEof, "No data received")));
		}
		
		let ret_msg = try!(str::from_utf8(&buffer[0..bytes_read-1]).map_err(|_| DataError::InvalidUtf8));
		json::Json::from_str(ret_msg).map_err(|_| ConnectionError::Data(DataError::InvalidJson(ret_msg.to_owned())))
	}
	
	fn into_connection(self) -> Connection {
		Connection {br: self.br, next_token: 0}
	}
}

pub struct ConnectionBuilder {
	hostname: String,
	port: u16,
	dbname: String,
	user: String,
	pass: String,
	timeout: u32,
}

impl ConnectionBuilder {
	pub fn hostname(mut self, val: String) -> ConnectionBuilder {
		self.hostname = val;
		self
	}
	
	pub fn port(mut self, val: u16) -> ConnectionBuilder {
		self.port = val;
		self
	}
	
	pub fn dbname(mut self, val: String) -> ConnectionBuilder {
		self.dbname = val;
		self
	}
	
	pub fn user(mut self, user: String, pass: String) -> ConnectionBuilder {
		self.user = user;
		self.pass = pass;
		self
	}
	
	pub fn timeout(mut self, val: u32) -> ConnectionBuilder {
		self.timeout = val;
		self
	}
	
	fn validate_server_reply(obj: &json::Json) -> bool {
		match obj.find("success") {
			Some(&json::Json::Boolean(true)) => (),
			_ => return false
		};
		match obj.find("min_protocol_version") {
			Some(&json::Json::U64(x)) => return x <= PROTOCOL_VERSION,
			_ => return false
		};
	}
	
	pub fn connect(self) -> Result<Connection, ConnectionError> {
		let mut stream = try!(TcpStream::connect((self.hostname.as_str(), self.port)));
		//try!(stream.set_nonblocking(true));
		try!(stream.set_nodelay(true));
		try!(stream.write_all(&[0xc3, 0xbd, 0xc2, 0x34]));
		
		let mut conn = AuthConnection {br: BufReader::new(stream)};
		
		let obj_reply = try!(conn.recv_packet());
		
		//reply validation
		if !Self::validate_server_reply(&obj_reply) {
			return Err(ConnectionError::Data(DataError::MalformedPacket(obj_reply)));
		}
		
		//begin authentication handshake
		let (packet, hs_a) = scram::begin_handshake(&self.user, &self.pass);
		//println!("Client sends:");
		//println!("{}", packet);
		try!(conn.send_packet(&packet));
		
		let packet = try!(conn.recv_packet());
		//println!("Server sends:");
		//println!("{}", packet);
		
		let (packet, hs_b) = try!(hs_a.handshake_b(&packet));
		//println!("Client sends:");
		//println!("{}", packet);
		try!(conn.send_packet(&packet));
		
		let packet = try!(conn.recv_packet());
		//println!("Server sends:");
		//println!("{}", packet);
		
		try!(hs_b.handshake_c(&packet));
		Ok(conn.into_connection())
	}
}

pub fn connection() -> ConnectionBuilder {
	ConnectionBuilder {
		hostname: String::from("localhost"),
		port: 28015,
		dbname: String::from("test"),
		user: String::from("admin"),
		pass: String::new(),
		timeout: 20
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	
	#[test]
	fn test_connection() {
		connection().connect().unwrap();
	}
}