// A minimal PNG writer: 8-bit RGB, zlib stored blocks, no compression, no dependencies.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn crc_table() -> [u32; 256] {
	let mut t = [0u32; 256];
	for (n, slot) in t.iter_mut().enumerate() {
		let mut c = n as u32;
		for _ in 0..8 {
			c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
		}
		*slot = c;
	}
	t
}

fn chunk(out: &mut impl Write, kind: &[u8; 4], data: &[u8], table: &[u32; 256]) -> std::io::Result<()> {
	out.write_all(&(data.len() as u32).to_be_bytes())?;
	out.write_all(kind)?;
	out.write_all(data)?;
	let mut c = 0xFFFF_FFFFu32;
	for &b in kind.iter().chain(data) {
		c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
	}
	out.write_all(&(c ^ 0xFFFF_FFFF).to_be_bytes())
}

/// Writes 0x00RRGGBB pixels, `w` per row, top row first, as an RGB PNG.
pub fn write(path: &Path, w: usize, h: usize, px: &[u32]) -> std::io::Result<()> {
	let table = crc_table();
	let mut raw = Vec::with_capacity((w * 3 + 1) * h);
	for y in 0..h {
		raw.push(0);
		for &c in &px[y * w..(y + 1) * w] {
			raw.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, c as u8]);
		}
	}
	let mut z = vec![0x78, 0x01];
	let blocks = raw.chunks(65_535).count();
	for (i, block) in raw.chunks(65_535).enumerate() {
		z.push(if i + 1 == blocks { 1 } else { 0 });
		let len = block.len() as u16;
		z.extend_from_slice(&len.to_le_bytes());
		z.extend_from_slice(&(!len).to_le_bytes());
		z.extend_from_slice(block);
	}
	let (mut a, mut b) = (1u32, 0u32);
	for &byte in &raw {
		a = (a + byte as u32) % 65_521;
		b = (b + a) % 65_521;
	}
	z.extend_from_slice(&((b << 16) | a).to_be_bytes());
	let mut out = BufWriter::new(File::create(path)?);
	out.write_all(b"\x89PNG\r\n\x1a\n")?;
	let mut ihdr = Vec::new();
	ihdr.extend_from_slice(&(w as u32).to_be_bytes());
	ihdr.extend_from_slice(&(h as u32).to_be_bytes());
	ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
	chunk(&mut out, b"IHDR", &ihdr, &table)?;
	chunk(&mut out, b"IDAT", &z, &table)?;
	chunk(&mut out, b"IEND", &[], &table)?;
	out.flush()
}
