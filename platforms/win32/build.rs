// Writes each `.scr`'s resource file (string 1, the icon, the version information) and tells the linker to include it. The bytes depend
// only on the source tree, so two builds of the same source give the same file (SAV-D9).
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs;
use std::path::{Path, PathBuf};

use savers_core::settings::SaverKind;

const RT_ICON: u16 = 3;
const RT_STRING: u16 = 6;
const RT_GROUP_ICON: u16 = 14;
const RT_VERSION: u16 = 16;
/// The language of every resource: English (United States), which every Windows lists.
const LANGUAGE: u16 = 0x0409;
const ICON_GROUP: u16 = 101;

struct Res(Vec<u8>);

impl Res {
	fn new() -> Res {
		let mut res = Res(Vec::new());
		// The empty first entry that marks a 32-bit resource file.
		res.0
			.extend_from_slice(&[0, 0, 0, 0, 0x20, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0, 0]);
		res.0.extend_from_slice(&[0; 16]);
		res
	}

	fn entry(&mut self, kind: u16, id: u16, data: &[u8]) {
		let header_size = 32u32;
		self.0.extend_from_slice(&(data.len() as u32).to_le_bytes());
		self.0.extend_from_slice(&header_size.to_le_bytes());
		for v in [0xFFFFu16, kind, 0xFFFF, id] {
			self.0.extend_from_slice(&v.to_le_bytes());
		}
		self.0.extend_from_slice(&0u32.to_le_bytes()); // data version
		self.0.extend_from_slice(&0x0030u16.to_le_bytes()); // memory flags: moveable, pure
		self.0.extend_from_slice(&LANGUAGE.to_le_bytes());
		self.0.extend_from_slice(&0u32.to_le_bytes()); // version
		self.0.extend_from_slice(&0u32.to_le_bytes()); // characteristics
		self.0.extend_from_slice(data);
		while !self.0.len().is_multiple_of(4) {
			self.0.push(0);
		}
	}
}

fn wide(text: &str) -> Vec<u8> {
	let mut bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
	bytes.extend_from_slice(&[0, 0]);
	bytes
}

fn pad(bytes: &mut Vec<u8>) {
	while !bytes.len().is_multiple_of(4) {
		bytes.push(0);
	}
}

/// One node of the version resource: its length, value length, type, key, value and children.
fn node(key: &str, value: &[u8], value_length: u16, text: bool, children: &[Vec<u8>]) -> Vec<u8> {
	let mut node = vec![0u8, 0];
	node.extend_from_slice(&value_length.to_le_bytes());
	node.extend_from_slice(&u16::from(text).to_le_bytes());
	node.extend_from_slice(&wide(key));
	pad(&mut node);
	node.extend_from_slice(value);
	for child in children {
		pad(&mut node);
		node.extend_from_slice(child);
	}
	let length = node.len() as u16;
	node[0..2].copy_from_slice(&length.to_le_bytes());
	node
}

fn version_info(kind: SaverKind, version: [u16; 4]) -> Vec<u8> {
	let mut fixed = Vec::new();
	let ms = (u32::from(version[0]) << 16) | u32::from(version[1]);
	let ls = (u32::from(version[2]) << 16) | u32::from(version[3]);
	for v in [0xFEEF_04BDu32, 0x0001_0000, ms, ls, ms, ls, 0x3F, 0, 0x0004_0004, 1, 0, 0, 0] {
		fixed.extend_from_slice(&v.to_le_bytes());
	}
	let strings = [
		("CompanyName", "Liminal HQ"),
		("FileDescription", kind.title()),
		("FileVersion", "0.1.0.0"),
		("InternalName", kind.file_name()),
		("LegalCopyright", "(c) Copyright 2026 Liminal HQ, Scott Morris"),
		("OriginalFilename", kind.file_name()),
		("ProductName", "Afterglow Screensavers"),
		("ProductVersion", "0.1.0.0"),
	];
	let children: Vec<Vec<u8>> = strings
		.iter()
		.map(|(key, value)| {
			let value_bytes = wide(value);
			node(key, &value_bytes, (value_bytes.len() / 2) as u16, true, &[])
		})
		.collect();
	let table = node("040904B0", &[], 0, true, &children);
	let string_info = node("StringFileInfo", &[], 0, true, &[table]);
	let translation = node("Translation", &[0x09, 0x04, 0xB0, 0x04], 4, false, &[]);
	let var_info = node("VarFileInfo", &[], 0, true, &[translation]);
	node("VS_VERSION_INFO", &fixed, fixed.len() as u16, false, &[string_info, var_info])
}

/// The icon group and its images, from an `.ico` file.
fn add_icon(res: &mut Res, ico: &[u8]) {
	let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
	let mut group = vec![0, 0, 1, 0];
	group.extend_from_slice(&(count as u16).to_le_bytes());
	for index in 0..count {
		let entry = &ico[6 + index * 16..6 + index * 16 + 16];
		let size = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]) as usize;
		let offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;
		let id = index as u16 + 1;
		group.extend_from_slice(&entry[..8]);
		group.extend_from_slice(&(size as u32).to_le_bytes());
		group.extend_from_slice(&id.to_le_bytes());
		res.entry(RT_ICON, id, &ico[offset..offset + size]);
	}
	res.entry(RT_GROUP_ICON, ICON_GROUP, &group);
}

fn main() {
	let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
	let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
	println!("cargo:rerun-if-changed=build.rs");
	println!("cargo:rerun-if-changed=res");
	println!("cargo:rerun-if-changed=src/bin");
	for kind in SaverKind::ALL {
		let name = kind.file_name().trim_end_matches(".SCR").to_ascii_lowercase();
		// A saver without a program yet (Quill Ghostwriter) has no resources to link.
		if !root.join("src/bin").join(format!("{name}.rs")).exists() {
			continue;
		}
		let mut res = Res::new();
		// String 1 is the sixteen-string block 1: an empty string 0, then the description.
		let mut table = vec![0u8, 0];
		let title: Vec<u16> = kind.title().encode_utf16().collect();
		table.extend_from_slice(&(title.len() as u16).to_le_bytes());
		table.extend(title.iter().flat_map(|unit| unit.to_le_bytes()));
		for _ in 2..16 {
			table.extend_from_slice(&[0, 0]);
		}
		res.entry(RT_STRING, 1, &table);
		let icon: &Path = &root.join("res").join(format!("{name}.ico"));
		if let Ok(ico) = fs::read(icon) {
			add_icon(&mut res, &ico);
		}
		res.entry(RT_VERSION, 1, &version_info(kind, [0, 1, 0, 0]));
		let path = out.join(format!("{name}.res"));
		fs::write(&path, &res.0).unwrap();
		println!("cargo:rustc-link-arg-bin={name}={}", path.display());
	}
}
