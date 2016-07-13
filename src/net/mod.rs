use std::io::prelude::*;
use std::net::TcpStream;
use std::io;
use std::str;
use std::io::BufReader;
use std::collections::BTreeMap;
use rustc_serialize::json;

pub mod scram;

const PROTOCOL_VERSION: u64 = 0;

//connection
pub struct Connection {
	br: BufReader<TcpStream>
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
	
	fn validate_server_reply(obj: json::Json) -> bool {
		match obj.find("success") {
			Some(&json::Json::Boolean(true)) => (),
			_ => return false
		};
		match obj.find("min_protocol_version") {
			Some(&json::Json::U64(x)) => return x <= PROTOCOL_VERSION,
			_ => return false
		};
	}
	
	pub fn connect(self) -> io::Result<Connection> {
		let mut stream = try!(TcpStream::connect((self.hostname.as_str(), self.port)));
		//try!(stream.set_nonblocking(true));
		try!(stream.set_nodelay(true));
		try!(stream.write_all(&[0xc3, 0xbd, 0xc2, 0x34]));
		
		let mut br = BufReader::new(stream);
		let mut buffer = Vec::new();
		
		let bytes_read = try!(br.read_until(0x00, &mut buffer));
		
		if bytes_read == 0 {
			return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "No data received"));
		}
		
		let ret_msg = try!(str::from_utf8(&buffer[0..bytes_read-1]).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e)));
		let obj_reply = try!(json::Json::from_str(ret_msg).or(Err(io::Error::new(io::ErrorKind::InvalidData, ret_msg))));
		
		//reply validation
		if Self::validate_server_reply(obj_reply) {
			Ok(Connection {br: br})
		} else {
			Err(io::Error::new(io::ErrorKind::InvalidData, "Malformed reply"))
		}
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