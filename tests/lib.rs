extern crate recogitate;
extern crate rustc_serialize;

use rustc_serialize::json::ToJson;
use recogitate as r;
use recogitate::prelude::*;

#[test]
fn integration() {
	let mut state = r::ReQLGenState::new();
	
	//let a = 10.eq(&15);
	/*let q = &10u32 as &Selection;
	let () = q;
	let a = Value::eq(&10u32, &15u32);
	let () = a;*/
	
	//let q = &5u32 /*as &ToJson*/ as &Value;
	//println!("{}", q.get_reql_json(&mut state));
	//println!("{}", q.to_json());
	
	let json_output = r::db("blog").table("users").filter_fn(|x| {
		//let () = x;
		//x.eq(&x)
		x
	}).get_reql_json(&mut state);
	
	println!("{}", json_output);
	//panic!();
}