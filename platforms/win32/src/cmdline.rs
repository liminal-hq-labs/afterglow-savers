// The command line of a screensaver, read the way `scrnsave.lib` reads it (docs/reactos-scr.md): the first letter that follows a
// `/` or `-` picks the mode, in either case, and a window handle follows after a colon or a space.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// What the user asked the program to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
	/// `/s`: full screen until input.
	Full,
	/// `/p <hwnd>`: run inside that window.
	Preview(usize),
	/// `/c:<hwnd>`, `/c <hwnd>` or `/c`: the settings dialog, owned by the window when one is given, else by the foreground window.
	Settings(Option<usize>),
	/// Nothing, or anything unknown: the settings dialog with no owner.
	Plain,
	/// `/a`: the Windows 9x password change, which this program does not offer.
	Password,
}

/// Skips the program's own name: a quoted path, or everything up to the first blank.
fn after_program_name(line: &[u8]) -> &[u8] {
	let mut i = 0;
	if line.first() == Some(&b'"') {
		i = 1;
		while i < line.len() && line[i] != b'"' {
			i += 1;
		}
		i += 1;
	} else {
		while i < line.len() && line[i] != b' ' && line[i] != b'\t' {
			i += 1;
		}
	}
	&line[i.min(line.len())..]
}

/// The decimal number at the start of `text`, after any colons and blanks.
fn handle_after(text: &[u8]) -> Option<usize> {
	let mut i = 0;
	while i < text.len() && matches!(text[i], b':' | b' ' | b'\t' | b'"') {
		i += 1;
	}
	let start = i;
	let mut value = 0usize;
	while i < text.len() && text[i].is_ascii_digit() {
		value = value.wrapping_mul(10).wrapping_add(usize::from(text[i] - b'0'));
		i += 1;
	}
	if i > start { Some(value) } else { None }
}

/// Reads the whole command line, program name included, without its NUL.
pub fn parse(line: &[u8]) -> Mode {
	let args = after_program_name(line);
	let mut i = 0;
	while i < args.len() && matches!(args[i], b' ' | b'\t' | b'/' | b'-') {
		i += 1;
	}
	let Some(&letter) = args.get(i) else {
		return Mode::Plain;
	};
	let rest = &args[i + 1..];
	match letter.to_ascii_lowercase() {
		b's' => Mode::Full,
		b'p' => Mode::Preview(handle_after(rest).unwrap_or(0)),
		b'c' => Mode::Settings(handle_after(rest)),
		b'a' => Mode::Password,
		_ => Mode::Plain,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn a_letter_after_a_slash_or_a_dash_picks_the_mode() {
		assert_eq!(parse(b"AGINK.SCR /s"), Mode::Full);
		assert_eq!(parse(b"AGINK.SCR -S"), Mode::Full);
		assert_eq!(parse(b"\"C:\\Program Files\\a b.scr\" /S"), Mode::Full);
		assert_eq!(parse(b"x s"), Mode::Full);
		assert_eq!(parse(b"x /a 123"), Mode::Password);
	}

	#[test]
	fn the_preview_takes_a_handle_after_a_space_or_a_colon() {
		assert_eq!(parse(b"x /p 65538"), Mode::Preview(65538));
		assert_eq!(parse(b"x /P:65538"), Mode::Preview(65538));
		assert_eq!(parse(b"x /p"), Mode::Preview(0));
	}

	#[test]
	fn settings_take_an_optional_owner() {
		assert_eq!(parse(b"x /c:1234"), Mode::Settings(Some(1234)));
		assert_eq!(parse(b"x /c 1234"), Mode::Settings(Some(1234)));
		assert_eq!(parse(b"x /c"), Mode::Settings(None));
		assert_eq!(parse(b"x"), Mode::Plain);
		assert_eq!(parse(b""), Mode::Plain);
		assert_eq!(parse(b"x /zzz"), Mode::Plain);
	}
}
