// ANSI text: the registry's `REG_SZ` values and the edit boxes are bytes in the system's code page, while the core's text is UTF-8. A byte
// maps to the character of the same number (Latin-1, which Windows-1252 matches for the letters the savers use), and a character past
// 255 becomes a question mark. Also the decimal parsing the dialog needs.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::string::String;
use alloc::vec::Vec;

/// The text for a run of ANSI bytes, stopping at the first NUL.
pub fn from_ansi(bytes: &[u8]) -> String {
	let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
	bytes[..end].iter().map(|&b| char::from(b)).collect()
}

/// ANSI bytes for text, with a NUL at the end.
pub fn to_ansi(text: &str) -> Vec<u8> {
	let mut bytes: Vec<u8> = text.chars().map(|c| u8::try_from(u32::from(c)).unwrap_or(b'?')).collect();
	bytes.push(0);
	bytes
}

/// Text for an edit box: every line break becomes carriage return and line feed.
pub fn to_edit(text: &str) -> Vec<u8> {
	let mut bytes = Vec::with_capacity(text.len() + 8);
	for c in text.chars() {
		match c {
			'\r' => {}
			'\n' => bytes.extend_from_slice(b"\r\n"),
			c => bytes.push(u8::try_from(u32::from(c)).unwrap_or(b'?')),
		}
	}
	bytes.push(0);
	bytes
}

/// Text from an edit box: carriage returns dropped.
pub fn from_edit(bytes: &[u8]) -> String {
	let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
	bytes[..end].iter().filter(|&&b| b != b'\r').map(|&b| char::from(b)).collect()
}

/// An unsigned decimal number, ignoring blanks around it; `None` when the text is empty or has anything else, and the largest
/// value when it overflows.
pub fn parse_number(bytes: &[u8]) -> Option<u32> {
	let text = from_ansi(bytes);
	let text = text.trim();
	if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
		return None;
	}
	let mut value = 0u32;
	for b in text.bytes() {
		value = value.saturating_mul(10).saturating_add(u32::from(b - b'0'));
	}
	Some(value)
}

/// A number as decimal ANSI bytes with a NUL.
pub fn number_text(mut value: u32) -> Vec<u8> {
	let mut digits = Vec::new();
	loop {
		digits.push(b'0' + (value % 10) as u8);
		value /= 10;
		if value == 0 {
			break;
		}
	}
	digits.reverse();
	digits.push(0);
	digits
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn text_round_trips_through_ansi() {
		let text = "Greetings, café";
		assert_eq!(from_ansi(&to_ansi(text)), text);
		assert_eq!(from_ansi(&to_ansi("→")), "?");
	}

	#[test]
	fn line_breaks_survive_an_edit_box() {
		assert_eq!(from_edit(&to_edit("one\ntwo\n")), "one\ntwo\n");
		assert_eq!(&to_edit("a\nb")[..], b"a\r\nb\0");
	}

	#[test]
	fn numbers_parse_and_print() {
		assert_eq!(parse_number(b" 42 \0"), Some(42));
		assert_eq!(parse_number(b"\0"), None);
		assert_eq!(parse_number(b"4x\0"), None);
		assert_eq!(parse_number(b"99999999999\0"), Some(u32::MAX));
		assert_eq!(number_text(0), b"0\0");
		assert_eq!(number_text(4_294_967_295), b"4294967295\0");
	}
}
