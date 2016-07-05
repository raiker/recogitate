extern crate rustc_serialize;
use rustc_serialize::json;

pub mod recogitate {
	use rustc_serialize::json;
	use std::ops::Fn;
	use std::marker::Sized;
	 
	pub trait Expr : json::ToJson {}
	
	impl Expr for json::ToJson {}
	
	//ExprSlice
	
	/*pub struct ExprSlice<'a, T>
		where T: Expr+'a
	{
		elems: &'a [T]
	}
	
	impl<'a, T> json::ToJson for ExprSlice<'a, T>
		where T: Expr
	{
		fn to_json(&self) -> json::Json {
			json::Json::Array(self.elems.into_iter().map(|x| x.to_json()).collect())
		}
	}
	
	impl<'a, T> Expr for ExprSlice<'a, T>
		where T: Expr
	{}

	pub fn expr_slice<'a, T: Expr>(slice_data: &'a [T]) -> ExprSlice<'a, T> {
		ExprSlice { elems: slice_data }
	}
	
	//ExprStr
	
	pub struct ExprStr<'a> {
		string: &'a str
	}
	
	impl<'a> json::ToJson for ExprStr<'a> {
		fn to_json(&self) -> json::Json {
			json::Json::String(String::from(self.string))
		}
	}
	
	impl<'a> Expr for ExprStr<'a> {}
	
	pub fn expr_str<'a>(string: &'a str) -> ExprStr<'a> {
		ExprStr { string: string }
	}*/
	
	pub trait ClosureVar {
	}
	
	pub trait Query {
	}
	
	//Selection
	
	pub trait Selection {
		fn filter_fn<P>(self, predicate: P) -> Filter<Self, P>
			where P: Fn(&ClosureVar) -> bool, Self: Sized
		{
			Filter {source: self, predicate: predicate}
		}
	}
	
	impl Query for Selection {}
	
	//Filter
	pub struct Filter<S, P>
		where S: Selection, P: Fn(&ClosureVar) -> bool
	{
		source: S,
		predicate: P,
	}
	
	impl<S, P> Selection for Filter<S, P>
		where S: Selection, P: Fn(&ClosureVar) -> bool
	{}
	
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
	
	//Table
	
	pub struct Table<'a> {
		name: &'a str,
		db: Option<&'a DB<'a>>,
	}
	
	impl<'a> Table<'a> {
		
	}
	
	impl<'a> Selection for Table<'a> {
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
    use recogitate as r;
	use rustc_serialize::json::ToJson;

	#[test]
    fn it_works() {
		//let json_output = r::expr_slice(&[r::expr_str("foo"), r::expr_str("bar")]).to_json();
		
		//println!("{}", json_output);
		panic!();
    }
}
