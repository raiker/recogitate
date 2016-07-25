use rustc_serialize::json;
use std::fmt;
use std::error::{self, Error};

//***Declarations***

#[derive(Debug)]
pub enum QueryError {
	ConnectionError(ConnectionError),
	ClientError(json::Json),
	CompileError(json::Json),
	RuntimeError(json::Json),
}

#[derive(Debug,Clone)]
pub enum DataError {
	InvalidUtf8,
	InvalidJson(String),
	NoDataReceived,
	MalformedPacket(json::Json),
}

#[derive(Debug,Clone)]
pub enum AuthError {
	ReqlAuthError(u64, String),
	ChangedNonce,
	IncorrectServerValidation
}

#[derive(Debug)]
pub enum ConnectionError {
	Io(::std::io::Error),
	Data(DataError),
	Auth(AuthError),
}

//***Display implementations***

impl fmt::Display for QueryError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl fmt::Display for DataError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl fmt::Display for AuthError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl fmt::Display for ConnectionError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

//***Error implementations

impl error::Error for QueryError {
	fn description(&self) -> &str {
		match *self {
			QueryError::ConnectionError(ref ce) => ce.description(),
			QueryError::ClientError(ref _j) => "The server failed to run the query due to a bad client request",
			QueryError::CompileError(ref _j) => "The server failed to run the query due to an ReQL compilation error",
			QueryError::RuntimeError(ref _j) => "The query compiled correctly, but failed at runtime",
		}
	}
}

impl error::Error for DataError {
	fn description(&self) -> &str {
		match *self {
			DataError::InvalidUtf8 => "The received packet could not be parsed as UTF-8",
			DataError::InvalidJson(ref _s) => "The received packet could not be parsed as JSON",
			DataError::NoDataReceived => "No data was received",
			DataError::MalformedPacket(ref _json) => "A malformed packet was received",
		}
	}
}

impl error::Error for AuthError {
	fn description(&self) -> &str {
		match *self {
			AuthError::ReqlAuthError(_errcode, ref msg) => &msg,
			AuthError::ChangedNonce => "The nonce returned by the server does not extend the client nonce",
			AuthError::IncorrectServerValidation => "The server failed to validate successfully"
		}
	}
}

impl error::Error for ConnectionError {
	fn description(&self) -> &str {
		match *self {
			ConnectionError::Io(ref e) => e.description(),
			ConnectionError::Data(ref e) => e.description(),
			ConnectionError::Auth(ref e) => e.description(),
		}
	}
}

//***From implementations

impl From<::std::io::Error> for ConnectionError {
	fn from(err: ::std::io::Error) -> ConnectionError {
		ConnectionError::Io(err)
	}
}

impl From<DataError> for ConnectionError {
	fn from(err: DataError) -> ConnectionError {
		ConnectionError::Data(err)
	}
}

impl From<AuthError> for ConnectionError {
	fn from(err: AuthError) -> ConnectionError {
		ConnectionError::Auth(err)
	}
}


