use openssl;
use openssl::crypto::hmac;
use openssl::crypto::hash;
use openssl::crypto::pkcs5::pbkdf2_hmac_sha256;
use rustc_serialize::base64::{self,ToBase64,FromBase64};
use rustc_serialize::json::Json;
use std::collections::BTreeMap;
use std::borrow::Cow;
use err::{ConnectionError, AuthError, DataError};

const NONCE_LEN: usize = 16;

#[derive(Debug)]
pub struct HandshakeA {
	user: String,
	pass: String,
	nonce: String,
	client_first_message_bare: String,
}

#[derive(Debug)]
pub struct HandshakeB {
	expected_server_signature: Vec<u8>
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
	
	let msg_obj = btreemap!{
		"protocol_version".to_owned() => Json::U64(0),
		"authentication_method".to_owned() => Json::String("SCRAM-SHA-256".to_owned()),
		"authentication".to_owned() => Json::String(auth_str)
	};
	
	(Json::Object(msg_obj), HandshakeA {user: user.clone(), pass: pass.clone(), nonce: nonce, client_first_message_bare: bare_message})
}

fn string_xor<'a, T1, T2>(a: T1, b: T2) -> Vec<u8>
	where T1: IntoIterator<Item=&'a u8>, T2: IntoIterator<Item=&'a u8>
{
	a.into_iter().zip(b.into_iter()).map(|(a, b)| a ^ b).collect::<Vec<_>>()
}

impl HandshakeA {
	pub fn handshake_b(self, msg: &Json) -> Result<(Json, HandshakeB), ConnectionError> {
		let auth_str;
		match msg.find("success") {
			Some(&Json::Boolean(true)) => {
				match msg.find("authentication") {
					Some(&Json::String(ref s)) => {
						auth_str = s.clone()
					},
					_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone())))
				}
			},
			Some(&Json::Boolean(false)) => {
				match msg.find("error") {
					Some(&Json::String(ref s)) => {
						match msg.find("error_code") {
							Some(&Json::U64(code)) => return Err(ConnectionError::Auth(AuthError::ReqlAuthError(code, s.clone()))),
							_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))),
						}
					},
					_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone())))
				}
			},
			_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))),
		};
		
		//json structure seems valid, check the authentication packet format
		//"r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096"
		let mapped_fields = auth_str.split(",").filter_map(|s| {
			s.find("=").map(|x| {
				let (s1, s2) = s.split_at(x);
				(Cow::Borrowed(s1), Cow::Borrowed(&s2[1..]))
			})
		}).collect::<BTreeMap<_,_>>();
		
		//check that new nonce is an extension of old nonce
		let new_nonce = try!(mapped_fields.get("r")
			.ok_or(ConnectionError::Data(DataError::MalformedPacket(msg.clone())))
			.and_then(|s| 
				if s.starts_with(&self.nonce) {
					Ok(s.clone())
				} else {
					Err(ConnectionError::Auth(AuthError::ChangedNonce))
				}));
		
		let salt = try!(mapped_fields.get("s")
			.and_then(|s| s.from_base64().ok())
			.ok_or(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))));
		
		let iterations = try!(mapped_fields.get("i")
			.and_then(|s| s.parse::<u64>().ok())
			.ok_or(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))));
		
		//println!("nonce={}", new_nonce);
		//println!("salt={}", salt.to_hex());
		//println!("iterations={}", iterations);
		
		let client_final_message_without_proof = format!("c=biws,r={}", new_nonce); //biws is base64("n,,")
		
		let salted_password = pbkdf2_hmac_sha256(&self.pass, &salt, iterations as usize, 32);
		//println!("{}", salted_password.len());
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
		
		let msg_obj = btreemap!{
			"authentication".to_owned() => Json::String(client_final_message)
		};
		
		let server_key = hmac::hmac(hash::Type::SHA256, &salted_password, "Server Key".as_bytes());
		let server_signature = hmac::hmac(hash::Type::SHA256, &server_key, auth_message.as_bytes());
		
		Ok((Json::Object(msg_obj), HandshakeB {expected_server_signature: server_signature}))
	}
}

impl HandshakeB {
	pub fn handshake_c(self, msg: &Json) -> Result<(), ConnectionError> {
		let auth_str;
		match msg.find("success") {
			Some(&Json::Boolean(true)) => {
				match msg.find("authentication") {
					Some(&Json::String(ref s)) => {
						auth_str = s.clone()
					},
					_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone())))
				}
			},
			Some(&Json::Boolean(false)) => {
				match msg.find("error") {
					Some(&Json::String(ref s)) => {
						match msg.find("error_code") {
							Some(&Json::U64(code)) => return Err(ConnectionError::Auth(AuthError::ReqlAuthError(code, s.clone()))),
							_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))),
						}
					},
					_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone())))
				}
			},
			_ => return Err(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))),
		};
		
		//json structure seems valid, check the authentication packet format
		//"v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4="
		let mapped_fields = auth_str.split(",").filter_map(|s| {
			s.find("=").map(|x| {
				let (s1, s2) = s.split_at(x);
				(Cow::Borrowed(s1), Cow::Borrowed(&s2[1..]))
			})
		}).collect::<BTreeMap<_,_>>();
		
		let server_signature = try!(mapped_fields.get("v")
			.and_then(|s| s.from_base64().ok())
			.ok_or(ConnectionError::Data(DataError::MalformedPacket(msg.clone()))));
		
		if openssl::crypto::memcmp::eq(&server_signature, &self.expected_server_signature) {
			Ok(())
		} else {
			Err(ConnectionError::Auth(AuthError::IncorrectServerValidation))
		}
	}
}

#[cfg(test)]
mod test {
	use rustc_serialize::json::Json;
	use rustc_serialize::base64::FromBase64;
	use openssl::crypto::memcmp;
	use super::*;
	
	#[test]
	fn test_handshake() {
		let user = String::from("testuser");
		let pass = String::from("hunter2");
		
		let (msg, handshake_a) = begin_handshake(&user, &pass);
		
		println!("{}", msg.pretty());
		//panic!();
	}
	
	#[test]
	fn test_handshake_b() {
		let hs_a = HandshakeA {
			user: String::from("user"),
			pass: String::from("pencil"),
			nonce: String::from("rOprNGfwEbeRWgbNEkqO"),
			client_first_message_bare: String::from("n=user,r=rOprNGfwEbeRWgbNEkqO")
		};
		
		let packet = Json::from_str("{
			\"success\": true,
			\"authentication\": \"r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,s=W22ZaJ0SNY7soEsUEjb6gQ==,i=4096\"
		}").unwrap();
		
		let (msg, hs_b) = hs_a.handshake_b(&packet).unwrap();
		
		println!("{}", msg);
		let msg_str = format!("{}", msg);
		
		assert_eq!(msg_str, "{\"authentication\":\"c=biws,r=rOprNGfwEbeRWgbNEkqO%hvYDpWUa2RaTCAfuxFIlj)hNlF$k0,p=dHzbZapWIk4jUhN+Ute9ytag9zjfMHgsqmmiz7AndVQ=\"}");
		
		assert!(memcmp::eq(&hs_b.expected_server_signature, &("6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=".from_base64().unwrap())));
	}
	
	#[test]
	fn test_handshake_c() {
		let hs_b = HandshakeB {
			expected_server_signature: "6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=".from_base64().unwrap()
		};
		
		let packet = Json::from_str("{
			\"success\": true,
			\"authentication\": \"v=6rriTRBi23WpRR/wtup+mMhUZUn/dB5nLTJRsjl95G4=\"
		}").unwrap();
		
		hs_b.handshake_c(&packet).unwrap();
	}
}