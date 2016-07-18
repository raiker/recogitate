use openssl;
use openssl::crypto::hmac;
use openssl::crypto::hash;
use byteorder::{BigEndian, WriteBytesExt};
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
	client_first_message_bare: String,
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
	
	let bare_message = format!("n={},r={}", sanitise(&user), nonce);
	let auth_str = format!("n,,{}", bare_message);
	
	let mut msg_obj = BTreeMap::new();
	
	msg_obj.insert(String::from("protocol_version"), Json::U64(0));
	msg_obj.insert(String::from("authentication_method"), Json::String(String::from("SCRAM-SHA-256")));
	msg_obj.insert(String::from("authentication"), Json::String(auth_str));
	
	(Json::Object(msg_obj), HandshakeA {user: user.clone(), pass: pass.clone(), nonce: nonce, client_first_message_bare: bare_message})
}

//see https://tools.ietf.org/html/rfc5802#page-7
fn h_i(s: &[u8], salt: &[u8], n: u64) -> Vec<u8> {
	assert!(n > 0);
	
	let data = {
		let mut x = Vec::from(s);
		x.write_u32::<BigEndian>(1).unwrap();
		x
	};
	
	(0..n).fold(
		(data, None),
		|(data, h): (Vec<u8>, Option<Vec<u8>>), _| {
			let u = hmac::hmac(hash::Type::SHA256, &s, &data);
			
			let new_h = match h {
				//Some(old_h) => old_h.into_iter().zip(u.iter()).map(|(a, &b)| a ^ b).collect::<Vec<_>>(),
				Some(old_h) => string_xor(&old_h, &u),
				None => u.clone()
			};
			
			(u, Some(new_h))
		}
	).1.unwrap()
}

fn string_xor<'a, T1, T2>(a: T1, b: T2) -> Vec<u8>
	where T1: IntoIterator<Item=&'a u8>, T2: IntoIterator<Item=&'a u8>
{
	a.into_iter().zip(b.into_iter()).map(|(a, b)| a ^ b).collect::<Vec<_>>()
}

impl HandshakeA {
	pub fn handshake_b(self, msg: &Json) -> Result<Json, AuthError> {
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
		
		let client_final_message_without_proof = format!("c=biws,r={}", new_nonce); //biws is base64("n,,")
		
		let salted_password = h_i(self.pass.as_bytes(), salt.as_bytes(), iterations);
		let client_key = hmac::hmac(hash::Type::SHA256, &salted_password, "Client Key".as_bytes());
		let stored_key = hash::hash(hash::Type::SHA256, &client_key);
		let auth_message = format!("{},{},{}",
			self.client_first_message_bare,
			auth_str,
			client_final_message_without_proof);
		let client_signature = hmac::hmac(hash::Type::SHA256, &stored_key, auth_message.as_bytes());
		let client_proof = string_xor(&client_key, &client_signature);
		let client_proof_b64 = client_proof.to_base64(base64::Config{
			char_set: base64::CharacterSet::Standard,
			newline: base64::Newline::LF,
			pad: true,
			line_length: None
		});
		let client_final_message = format!("{},p={}", client_final_message_without_proof, client_proof_b64);
		
		let mut msg_obj = BTreeMap::new();
		msg_obj.insert(String::from("authentication"), Json::String(client_final_message));
		
		Ok(Json::Object(msg_obj))
	}
}

#[cfg(test)]
mod test {
	use rustc_serialize::json::Json;
	use super::*;
	
	#[test]
	fn test_handshake() {
		let user = String::from("testuser");
		let pass = String::from("hunter2");
		
		let (msg, handshake_a) = begin_handshake(&user, &pass);
		
		println!("{}", msg.pretty());
		panic!();
	}
	
	#[test]
	fn test_handshake_b() {
		let hs_a = HandshakeA {
			user: String::from("user"),
			pass: String::from("pencil"),
			nonce: String::from("fyko+d2lbbFgONRv9qkxdawL"),
			client_first_message_bare: String::from("n=user,r=fyko+d2lbbFgONRv9qkxdawL")
		};
		
		let packet = Json::from_str("{
			\"success\": true,
			\"authentication\": \"r=fyko+d2lbbFgONRv9qkxdawL3rfcNHYJY1ZVvWVs7j,s=QSXCR+Q6sek8bf92,i=4096\"
		}").unwrap();
		
		let msg = hs_a.handshake_b(&packet).unwrap();
		
		println!("{}", msg);
		panic!();
	}
}