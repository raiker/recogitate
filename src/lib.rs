extern crate rustc_serialize;
use rustc_serialize::json;

pub mod recogitate {
	use rustc_serialize::json::{self, ToJson};
	use std::ops::Fn;
	use std::marker::Sized;
	use std::result::Result;
	use std::collections::BTreeMap;
	
	pub mod prelude {
		pub use super::{
			TreeNode,
			Selection,
			Value,
		};
	}
	
	enum TermTypes {
		DATUM = 1,
		MAKE_ARRAY = 2,
		VAR = 10,
		DB = 14,
		TABLE = 15,
		EQ = 17,
		FILTER = 39,
		FUNC = 69,
	}
	
	pub struct ReQLGenState {
		nvars: u64
	}
	
	impl ReQLGenState {
		pub fn new() -> ReQLGenState {
			ReQLGenState { nvars: 0 }
		}
		
		fn gen_closure_var(&mut self) -> ClosureVar {
			let retval = ClosureVar { n: self.nvars };
			self.nvars += 1;
			retval
		}
	}
	 
	pub trait TreeNode {
		fn get_reql_json(&self, state: &mut ReQLGenState) -> json::Json;
	}
	
	pub trait Value : TreeNode {
		fn eq<'a, T>(&'a self, other: &'a T) -> Eq<'a, Self, T>
			where
				T: 'a+Value,
				Self: Value+Sized,
		{
			Eq {a: self, b: other}
		}
	}
	
	pub struct Eq<'a, T1, T2>
		where T1: 'a+Value, T2: 'a+Value
	{
		a: &'a T1,
		b: &'a T2,
	}
	
	impl<'a, T1, T2> Value for Eq<'a, T1, T2>
		where T1: Value, T2: Value
	{}
	
	impl<'a, T1, T2> TreeNode for Eq<'a, T1, T2>
		where T1: Value, T2: Value
	{
		fn get_reql_json(&self, mut state: &mut ReQLGenState) -> json::Json {
			json::Json::Array(vec![
				(TermTypes::EQ as u32).to_json(),
				json::Json::Array(vec![
					self.a.get_reql_json(&mut state),
					self.b.get_reql_json(&mut state),
				])
			])
		}
	}
	
	//impl TreeNode for json::ToJson {}
	
	pub struct Connection {
	}
	
	pub struct ResultSet {
	}
	
	pub enum QueryError {
	}
	
	#[derive(Copy,Clone)]
	pub struct ClosureVar {
		n: u64,
	}
	
	impl TreeNode for ClosureVar {
		fn get_reql_json(&self, _state: &mut ReQLGenState) -> json::Json {
			json::Json::Array(vec![
				(TermTypes::VAR as u32).to_json(),
				json::Json::Array(vec![self.n.to_json()])
			])
		}
	}
	
	impl Value for ClosureVar {}
	
	pub trait Query : TreeNode {
		fn run(self, conn: Connection) -> Result<ResultSet, QueryError>
			where Self: Sized
		{
			let mut state = ReQLGenState::new();
			println!("{}", self.get_reql_json(&mut state));
			Ok(ResultSet {})
		}
	}
	
	//Primitives
	/*impl Value for json::ToJson {}
	
	impl TreeNode for json::ToJson {
		fn get_reql_json(&self, _state: &mut ReQLGenState) -> json::Json {
			self.to_json()
		}
	}*/
	
	impl TreeNode for u32 {
		fn get_reql_json(&self, _state: &mut ReQLGenState) -> json::Json {
			self.to_json()
		}
	}

	/*impl TreeNode for str {
		fn get_reql_json(&self, state: &mut ReQLGenState) -> json::Json {
			json::Json::Array(vec![
				(TermTypes::DATUM as u32).to_json(),
				self.to_json(),
				json::Json::Object(BTreeMap::new())
			])
		}
	}*/
	
	//Predicates
	
	impl TreeNode for Fn(&ClosureVar) -> bool {
		fn get_reql_json(&self, _state: &mut ReQLGenState) -> json::Json {
			"foobar".to_json()
		}
	}
	
	//Selection
	
	pub trait Selection : TreeNode {
		fn filter_fn<P, T>(self, predicate: P) -> Filter<Self, P, T>
			where 
				P: Fn(ClosureVar) -> T,
				T: TreeNode,
				Self: Sized
		{
			Filter {source: self, predicate: predicate}
		}
	}
	
	impl Query for Selection {}
	
	//Filter
	pub struct Filter<S, P, T>
		where
			S: Selection,
			P: Fn(ClosureVar) -> T,
			T: TreeNode
	{
		source: S,
		predicate: P,
	}
	
	impl<S, P, T> Selection for Filter<S, P, T>
		where
			S: Selection,
			P: Fn(ClosureVar) -> T,
			T: TreeNode
	{}
	
	impl<S, P, T> TreeNode for Filter<S, P, T>
		where
			S: Selection,
			P: Fn(ClosureVar) -> T,
			T: TreeNode
	{
		fn get_reql_json(&self, state: &mut ReQLGenState) -> json::Json {
			let cv = state.gen_closure_var();
			
			let func_call = json::Json::Array(vec![
				(TermTypes::FUNC as u32).to_json(),
				json::Json::Array(vec![
					json::Json::Array(vec![
						(TermTypes::MAKE_ARRAY as u32).to_json(),
						json::Json::Array(vec![
							cv.n.to_json()
						])
					]),
					(self.predicate)(cv).get_reql_json(state)
				])
			]);
			
			json::Json::Array(vec![
				(TermTypes::FILTER as u32).to_json(),
				self.source.get_reql_json(state),
				func_call
			])
		}
	}
	
	//DB
	
	pub struct DB<'a> {
		name: &'a str
	}
	
	impl<'a> DB<'a> {
		pub fn table<'b>(&'b self, name: &'b str) -> Table {
			Table {name: name, db: Some(&self)}
		}
	}
	
	pub fn db(db_name: &str) -> DB {
		DB {name: db_name}
	}
	
	impl<'a> TreeNode for DB<'a> {
		fn get_reql_json(&self, _state: &mut ReQLGenState) -> json::Json {
			json::Json::Array(vec![
				(TermTypes::DB as u32).to_json(),
				json::Json::Array(vec![
					self.name.to_json()
				])
			])
		}
	}
	
	//Table
	
	pub struct Table<'a> {
		name: &'a str,
		db: Option<&'a DB<'a>>,
	}
	
	impl<'a> Table<'a> {
	}
	
	impl<'a> Selection for Table<'a> {
	}
	
	impl<'a> TreeNode for Table<'a> {
		fn get_reql_json(&self, state: &mut ReQLGenState) -> json::Json {
			match self.db {
				Some(db) =>
					json::Json::Array(vec![
						(TermTypes::TABLE as u32).to_json(),
						json::Json::Array(vec![
							db.get_reql_json(state),
							self.name.to_json()
						])
					]),
				None =>
					json::Json::Array(vec![
						(TermTypes::TABLE as u32).to_json(),
						json::Json::Array(vec![
							self.name.to_json()
						])
					])
			}
		}
	}
	
	pub fn table(name: &str) -> Table {
		Table {name: name, db: None}
	}

	#[cfg(test)]
	mod tests {
		#[test]
		fn it_works() {
		}
	}
}

#[cfg(test)]
mod tests {
	use rustc_serialize::json::ToJson;
    use recogitate as r;
	use recogitate::prelude::*;

	#[test]
    fn it_works() {
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
		panic!();
    }
}
