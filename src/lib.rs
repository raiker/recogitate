extern crate rustc_serialize;
use rustc_serialize::json;

mod recogitate {
	use rustc_serialize::json;
	 
	pub trait ReQLExpr : json::ToJson {
	}
	
	impl ReQLExpr for u32 {
	}
	
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

	pub fn expr_slice<'a, T: ReQLExpr>(slice_data: &'a [T]) -> ReQLExprSlice<'a, T> {
		ReQLExprSlice { elems: slice_data }
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
		let json_output = r::expr_slice(&[1, 2, 3, 4, 5]).to_json();
		
		println!("{}", json_output);
		panic!();
    }
}
