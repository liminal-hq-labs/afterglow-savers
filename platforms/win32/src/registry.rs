// The settings in the registry (SAV-D7): `HKEY_CURRENT_USER\Software\Liminal HQ\Afterglow Screensavers\<Name>`, `REG_DWORD` and
// `REG_SZ` values through the ANSI calls. Reading never fails: a missing key or value is simply absent. Only the dialog writes.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use savers_core::settings::{SaverKind, SettingsStore};

use crate::ansi::{from_ansi, to_ansi};
use crate::sys;

const ROOT: &str = "Software\\Liminal HQ\\Afterglow Screensavers\\";

/// One saver's key, open for reading or for writing.
pub struct RegStore {
	key: usize,
}

impl RegStore {
	/// Opens the key to read. When there is none (a saver never configured), the store is empty.
	pub fn open(kind: SaverKind) -> RegStore {
		let path = to_ansi(&alloc::format!("{ROOT}{}", kind.registry_name()));
		let mut key = 0usize;
		// SAFETY: `path` ends in a NUL and `key` is a valid out pointer.
		let status = unsafe { sys::RegOpenKeyExA(sys::HKEY_CURRENT_USER, path.as_ptr(), 0, sys::KEY_QUERY_VALUE, &mut key) };
		RegStore {
			key: if status == sys::ERROR_SUCCESS { key } else { 0 },
		}
	}

	/// Opens the key to write, making it when needed. When that fails, writes are dropped.
	pub fn create(kind: SaverKind) -> RegStore {
		let path = to_ansi(&alloc::format!("{ROOT}{}", kind.registry_name()));
		let mut key = 0usize;
		let mut disposition = 0u32;
		// SAFETY: `path` ends in a NUL; the out pointers are valid; the class and security arguments may be null.
		let status = unsafe {
			sys::RegCreateKeyExA(
				sys::HKEY_CURRENT_USER,
				path.as_ptr(),
				0,
				core::ptr::null(),
				sys::REG_OPTION_NON_VOLATILE,
				sys::KEY_ALL_ACCESS,
				core::ptr::null(),
				&mut key,
				&mut disposition,
			)
		};
		RegStore {
			key: if status == sys::ERROR_SUCCESS { key } else { 0 },
		}
	}

	fn read(&self, name: &str, want: u32) -> Option<Vec<u8>> {
		if self.key == 0 {
			return None;
		}
		let name = to_ansi(name);
		let mut kind = 0u32;
		let mut size = 0u32;
		// SAFETY: the name ends in a NUL; a null data pointer asks only for the size.
		let status = unsafe {
			sys::RegQueryValueExA(
				self.key,
				name.as_ptr(),
				core::ptr::null_mut(),
				&mut kind,
				core::ptr::null_mut(),
				&mut size,
			)
		};
		if status != sys::ERROR_SUCCESS || kind != want || size > 65_536 {
			return None;
		}
		let mut data = vec![0u8; size as usize + 1];
		// SAFETY: `data` holds `size` bytes and one more for a NUL.
		let status = unsafe {
			sys::RegQueryValueExA(
				self.key,
				name.as_ptr(),
				core::ptr::null_mut(),
				&mut kind,
				data.as_mut_ptr(),
				&mut size,
			)
		};
		if status != sys::ERROR_SUCCESS {
			return None;
		}
		data.truncate(size as usize);
		Some(data)
	}

	fn write(&mut self, name: &str, kind: u32, data: &[u8]) {
		if self.key == 0 {
			return;
		}
		let name = to_ansi(name);
		// SAFETY: the name ends in a NUL and `data` is as long as the size says.
		unsafe {
			sys::RegSetValueExA(self.key, name.as_ptr(), 0, kind, data.as_ptr(), data.len() as u32);
		}
	}
}

impl Drop for RegStore {
	fn drop(&mut self) {
		if self.key != 0 {
			// SAFETY: the key was opened by this store.
			unsafe { sys::RegCloseKey(self.key) };
		}
	}
}

impl SettingsStore for RegStore {
	fn get_dword(&self, name: &str) -> Option<u32> {
		let data = self.read(name, sys::REG_DWORD)?;
		let bytes: [u8; 4] = data.get(..4)?.try_into().ok()?;
		Some(u32::from_le_bytes(bytes))
	}

	fn get_text(&self, name: &str) -> Option<String> {
		self.read(name, sys::REG_SZ).map(|data| from_ansi(&data))
	}

	fn set_dword(&mut self, name: &str, value: u32) {
		self.write(name, sys::REG_DWORD, &value.to_le_bytes());
	}

	fn set_text(&mut self, name: &str, value: &str) {
		self.write(name, sys::REG_SZ, &to_ansi(value));
	}
}
