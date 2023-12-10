//! JSON-RPC 2.0 types.

//---------------------------------------------------------------------------------------------------- Lints
#![allow(
	clippy::len_zero,
	clippy::type_complexity,
	clippy::module_inception,
)]

#![deny(
	nonstandard_style,
	unused_unsafe,
	unused_mut,
)]

#![forbid(
	future_incompatible,
	break_with_label_and_loop,
	coherence_leak_check,
	deprecated,
	duplicate_macro_attributes,
	exported_private_dependencies,
	for_loops_over_fallibles,
	large_assignments,
	overlapping_range_endpoints,
	semicolon_in_expressions_from_macros,
	redundant_semicolons,
	unconditional_recursion,
	unreachable_patterns,
	unused_allocation,
	unused_braces,
	unused_comparisons,
	unused_doc_comments,
	unused_parens,
	unused_labels,
	while_true,
	keyword_idents,
//	missing_docs, // TODO(hinto): add docs
	non_ascii_idents,
	noop_method_call,
	unreachable_pub,
	single_use_lifetimes,
	variant_size_differences,
)]

//---------------------------------------------------------------------------------------------------- Mod/Use
/// Error codes and objects
pub mod error;

mod key;

mod id;
pub use id::*;

mod version;
pub use version::*;

mod request;
pub use request::*;

mod response;
pub use response::*;

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
	use super::*;
	use std::net::*;
	use std::io::prelude::*;
	use serde_json::*;
	use std::borrow::Cow;
	use std::sync::atomic::AtomicBool;
	use std::sync::Arc;

	#[test]
	fn request_response() {
		//------------------------------------------------ Request 1, full
		let expected_request1 = json!({
			"jsonrpc": "2.0",
			"method": "method_1",
			"params": [0, 1, 2],
			"id": 123,
		});
		let request1: Request<&str, [u8; 3]> = Request::new(
			Cow::Borrowed(&"method_1"),
			Some(Cow::Borrowed(&[0, 1, 2])),
			Some(Id::Num(123)),
		);

		let request1 = to_value(request1).unwrap();
		assert_eq!(expected_request1, request1);

		//------------------------------------------------ Request 2, null/no id
		let expected_request2 = json!({
			"jsonrpc": "2.0",
			"method": "method_2",
			"params": [2, 3, 4],
		});
		let request2: Request<&str, [u8; 3]> = Request::new(
			Cow::Borrowed(&"method_2"),
			Some(Cow::Borrowed(&[2, 3, 4])),
			None,
		);

		let request2 = to_value(request2).unwrap();
		assert_eq!(expected_request2, request2);

		//------------------------------------------------ Request 3, string id, no params
		let expected_request3 = json!({
			"jsonrpc": "2.0",
			"method": "method_3",
			"id": "string_id",
		});
		let request3: Request<&str, ()> = Request::new(
			Cow::Borrowed(&"method_3"),
			None,
			Some(Id::Str("string_id".into())),
		);

		let request3 = to_value(request3).unwrap();
		assert_eq!(expected_request3, request3);

		//------------------------------------------------ The (incorrect) server `Response`
		let expected_response = json!({
			"jsonrpc": "2.0",
			"result": "OK",
			"id": null,
		});
		let response = Response::result(
			Cow::Owned("OK".into()),
			None,
		);
		assert_eq!(expected_response, to_value(&response).unwrap());

		//------------------------------------------------ Spawn the server.
		let er1   = expected_request1.clone();
		let er2   = expected_request2.clone();
		let er3   = expected_request3.clone();
		let resp  = response.clone();
		let park  = std::thread::current();
		let ready = Arc::new(AtomicBool::new(false));
		let rdy   = Arc::clone(&ready);
		std::thread::spawn(move || {
			let listen = TcpListener::bind("127.0.0.1:18425").unwrap();

			// Wake up client.
			rdy.store(true, std::sync::atomic::Ordering::SeqCst);
			park.unpark();

			let mut vec = vec![];

			for i in [er1, er2, er3] {
				let (mut stream, _) = listen.accept().unwrap();

				// Assert received bytes are the same as expected.
				stream.read_to_end(&mut vec).unwrap();
				let json: Request<&str, [u8; 3]> = from_slice(&vec).unwrap();
				assert_eq!(i, to_value(&json).unwrap());

				// Return a `Response`.
				to_writer(stream, &resp).unwrap();

				vec.clear();
			}
		});

		//------------------------------------------------ Client
		// Wait until server is ready.
		while !ready.load(std::sync::atomic::Ordering::SeqCst) {
			std::thread::park();
		}

		let mut vec = vec![];

		// Start client.
		for i in [expected_request1, expected_request2, expected_request3] {
			let mut stream = TcpStream::connect("127.0.0.1:18425").unwrap();

			// Send `Request`'s
			let bytes = to_vec(&i).unwrap();
			stream.write_all(&bytes).unwrap();

			// Read the `Response`.
			stream.shutdown(std::net::Shutdown::Write).unwrap();
			stream.read_to_end(&mut vec).unwrap();
			let json: Response<Cow<str>> = from_slice(&vec).unwrap();
			assert_eq!(json, response);

			vec.clear();
		}
	}
}
