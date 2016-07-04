extern crate rustc_serialize;
use rustc_serialize::json;

mod recogitate {
	use rustc_serialize::json;
	 
	pub trait ReQLExpr : json::ToJson {}
	
	impl ReQLExpr for u32 {}
	
	//ReQLExprSlice
	
	pub struct ReQLExprSlice<'a, T>
		where T: ReQLExpr+'a
	{
		elems: &'a [T]
	}
	
	impl<'a, T> json::ToJson for ReQLExprSlice<'a, T>
		where T: ReQLExpr
	{
		fn to_json(&self) -> json::Json {
			json::Json::Array(self.elems.into_iter().map(|x| x.to_json()).collect())
		}
	}
	
	impl<'a, T> ReQLExpr for ReQLExprSlice<'a, T>
		where T: ReQLExpr
	{}

	pub fn expr_slice<'a, T: ReQLExpr>(slice_data: &'a [T]) -> ReQLExprSlice<'a, T> {
		ReQLExprSlice { elems: slice_data }
	}
	
	//ReQLExprStr
	
	pub struct ReQLExprStr<'a> {
		string: &'a str
	}
	
	impl<'a> json::ToJson for ReQLExprStr<'a> {
		fn to_json(&self) -> json::Json {
			json::Json::String(String::from(self.string))
		}
	}
	
	impl<'a> ReQLExpr for ReQLExprStr<'a> {}
	
	pub fn expr_str<'a>(string: &'a str) -> ReQLExprStr<'a> {
		ReQLExprStr { string: string }
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
		let json_output = r::expr_slice(&[r::expr_str("foo"), r::expr_str("bar")]).to_json();
		
		println!("{}", json_output);
		panic!();
    }
}
