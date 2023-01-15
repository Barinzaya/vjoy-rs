use std::string::{FromUtf16Error};

pub unsafe fn decode_utf16(s: *const u16) -> Result<String, FromUtf16Error> {
	let len = (0..).position(|i| s.offset(i).read() == 0).unwrap();
	let slice = std::slice::from_raw_parts(s, len);
	String::from_utf16(slice)
}
