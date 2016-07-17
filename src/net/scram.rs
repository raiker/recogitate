use openssl;
use rustc_serialize::base64::{self,ToBase64};
use rustc_serialize::json::Json;
use std::collections::BTreeMap;
use std::borrow::Cow;

const NONCE_LEN: usize = 16;

#[derive(Debug)]
pub enum AuthError {
	ReqlAuthError(u64, String),
	ChangedNonce,
	MalformedData
}

pub struct HandshakeA {
	user: String,
	pass: String,
	nonce: String,
}

fn sanitise(user: &String) -> String {
	user.replace("=", "=3D").replace(",", "=2C")
}

pub fn begin_handshake(user: &String, pass: &String) -> (Json, HandshakeA) {
	let nonce_bytes = openssl::crypto::rand::rand_bytes(NONCE_LEN);
	let nonce = nonce_bytes.to_base64(base64::Config{
		char_set: base64::CharacterSet::Standard,
		newline: base64::Newline::LF,
		pad: true,
		line_length: None
	});
	
	let auth_str = format!("n,,n={},r={}", sanitise(&user), nonce);
	
	let mut msg_obj = BTreeMap::new();
	
	msg_obj.insert(String::from("protocol_version"), Json::U64(0));
	msg_obj.insert(String::from("authentication_method"), Json::String(String::from("SCRAM-SHA-256")));
	msg_obj.insert(String::from("authentication"), Json::String(auth_str));
	
	(Json::Object(msg_obj), HandshakeA {user: user.clone(), pass: pass.clone(), nonce: nonce})
}

impl HandshakeA {
	pub fn handshake_b(self, msg: &Json) -> Result<(), AuthError> {
		let auth_str;
		match msg.find("success") {
			Some(&Json::Boolean(true)) => {
				match msg.find("authentication") {
					Some(&Json::String(ref s)) => {
						auth_str = s.clone()
					},
					_ => return Err(AuthError::MalformedData)
				}
			},
			Some(&Json::Boolean(false)) => {
				match msg.find("error") {
					Some(&Json::String(ref s)) => {
						match msg.find("error_code") {
							Some(&Json::U64(code)) => return Err(AuthError::ReqlAuthError(code, s.clone())),
							_ => return Err(AuthError::MalformedData),
						}
					},
					_ => return Err(AuthError::MalformedData)
				}
			},
			_ => return Err(AuthError::MalformedData),
		};
		
		//json structure seems valid, check the authentication packet format
		//"r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096"
		let mapped_fields = auth_str.split(",").filter_map(|s| {
			match s.find("=") {
				None => None,
				Some(x) => {
					let (s1, s2) = s.split_at(x);
					Some((Cow::Borrowed(s1), Cow::Borrowed(&s2[1..])))
				}
			}
		}).collect::<BTreeMap<_,_>>();
		
		//check that new nonce is an extension of old nonce
		let new_nonce;
		match mapped_fields.get("r") {
			Some(ref s) => if !s.starts_with(&self.nonce) {
				//println!("original nonce: {}", self.nonce);
				//println!("new nonce: {}", s);
				return Err(AuthError::ChangedNonce)
			} else {
				new_nonce = s.clone()
			},
			None => return Err(AuthError::MalformedData)
		};
		
		let salt = match mapped_fields.get("s") {
			Some(ref s) => s.clone(),
			None => return Err(AuthError::MalformedData)
		};
		
		let iterations = match mapped_fields.get("i") {
			Some(ref s) => match u64::from_str_radix(&s, 10) {
				Ok(x) => x,
				Err(_) => return Err(AuthError::MalformedData)
			},
			None => return Err(AuthError::MalformedData)
		};
		
		
		println!("nonce={}", new_nonce);
		println!("salt={}", salt);
		println!("iterations={}", iterations);
		
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	
	#[test]
	fn test_handshake() {
		let user = String::from("testuser");
		let pass = String::from("hunter2");
		
		let (msg, handshake_a) = begin_handshake(&user, &pass);
		
		println!("{}", msg.pretty());
		panic!();
	}
}