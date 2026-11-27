// The settings dialog: shows the template built from the core's field table, fills its controls from the saver's settings, and on OK
// reads them back and writes the registry. Cancel writes nothing; Defaults puts the defaults in the controls.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

use alloc::vec;

use savers_core::settings::{Control, Key, SaverKind, Settings, fields, text_field};

use crate::ansi::{from_edit, number_text, parse_number, to_ansi, to_edit};
use crate::dialog_v2::{chosen_swatch, select_swatch};
use crate::registry::RegStore;
use crate::saver::MakeScene;
use crate::sys::{self, Handle};
use crate::template::{self, DEFAULTS_ID, TEXT_ID, field_id, mask_id};

struct DialogState {
	kind: SaverKind,
	settings: Settings,
}

pub(crate) fn item(dialog: Handle, id: u16) -> Handle {
	// SAFETY: any window handle and identifier are accepted; a missing control gives null.
	unsafe { sys::GetDlgItem(dialog, i32::from(id)) }
}

pub(crate) fn send(window: Handle, message: u32, wparam: usize, lparam: isize) -> isize {
	if window.is_null() {
		return 0;
	}
	// SAFETY: a live control and a message whose arguments the callers build correctly.
	unsafe { sys::SendMessageA(window, message, wparam, lparam) }
}

/// Adds a combo box's choices and a trackbar's range: what only has to happen once.
pub(crate) fn prepare(dialog: Handle, kind: SaverKind) {
	for (index, field) in fields(kind).enumerate() {
		let control = item(dialog, field_id(index));
		match field.control {
			Control::Choice(options) => {
				for (label, _) in options {
					send(control, sys::CB_ADDSTRING, 0, to_ansi(label).as_ptr() as isize);
				}
			}
			Control::Slider => {
				send(control, sys::TBM_SETRANGE, 0, ((field.max << 16) | field.min) as isize);
			}
			_ => {}
		}
	}
	if let Some((_, _, longest)) = text_field(kind) {
		send(item(dialog, TEXT_ID), sys::EM_LIMITTEXT, longest, 0);
	}
}

/// Puts a set of values in the controls.
pub(crate) fn show(dialog: Handle, settings: &Settings) {
	let kind = settings.kind;
	for (index, field) in fields(kind).enumerate() {
		let value = settings.get(field.key);
		let id = field_id(index);
		match field.control {
			Control::Choice(options) => {
				let at = options.iter().position(|&(_, v)| v == value).unwrap_or(0);
				// The Afterglow dialog shows the palette as swatches; the plain one, and every other choice, as a combo box.
				if field.key != Key::Palette || !select_swatch(dialog, index, options.len(), at) {
					send(item(dialog, id), sys::CB_SETCURSEL, at, 0);
				}
			}
			Control::Slider => {
				send(item(dialog, id), sys::TBM_SETPOS, 1, value as isize);
			}
			Control::Check => {
				send(item(dialog, id), sys::BM_SETCHECK, usize::from(value != 0), 0);
			}
			Control::Number => {
				// SAFETY: the text ends in a NUL.
				unsafe { sys::SetDlgItemTextA(dialog, i32::from(id), number_text(value).as_ptr()) };
			}
			Control::Mask(bits) => {
				for bit in 0..bits.len() {
					let on = value & (1 << bit) != 0;
					send(item(dialog, mask_id(index, bit)), sys::BM_SETCHECK, usize::from(on), 0);
				}
			}
		}
	}
	if text_field(kind).is_some() {
		// SAFETY: the text ends in a NUL.
		unsafe { sys::SetDlgItemTextA(dialog, i32::from(TEXT_ID), to_edit(&settings.text).as_ptr()) };
	}
}

/// Reads the controls back into the settings; a value a field does not allow becomes its default.
pub(crate) fn collect(dialog: Handle, settings: &mut Settings) {
	let kind = settings.kind;
	for (index, field) in fields(kind).enumerate() {
		let id = field_id(index);
		let value = match field.control {
			Control::Choice(options) => {
				let swatch = if field.key == Key::Palette {
					chosen_swatch(dialog, index, options.len())
				} else {
					None
				};
				let at = swatch.map_or_else(|| send(item(dialog, id), sys::CB_GETCURSEL, 0, 0), |n| n as isize);
				usize::try_from(at)
					.ok()
					.and_then(|at| options.get(at))
					.map_or(field.default, |&(_, v)| v)
			}
			Control::Slider => send(item(dialog, id), sys::TBM_GETPOS, 0, 0) as u32,
			Control::Check => u32::from(send(item(dialog, id), sys::BM_GETCHECK, 0, 0) != 0),
			Control::Number => {
				let mut text = [0u8; 16];
				// SAFETY: the buffer is as long as the length says.
				unsafe { sys::GetDlgItemTextA(dialog, i32::from(id), text.as_mut_ptr(), text.len() as i32) };
				parse_number(&text).map_or(field.default, |v| v.clamp(field.min, field.max))
			}
			Control::Mask(bits) => (0..bits.len())
				.filter(|&bit| send(item(dialog, mask_id(index, bit)), sys::BM_GETCHECK, 0, 0) != 0)
				.fold(0u32, |mask, bit| mask | (1 << bit)),
		};
		settings.set(field.key, value);
	}
	if let Some((_, _, longest)) = text_field(kind) {
		let mut text = vec![0u8; longest * 2 + 16];
		// SAFETY: the buffer is as long as the length says.
		unsafe { sys::GetDlgItemTextA(dialog, i32::from(TEXT_ID), text.as_mut_ptr(), text.len() as i32) };
		settings.text = from_edit(&text);
	}
}

unsafe extern "system" fn dialog_proc(dialog: Handle, message: u32, wparam: usize, lparam: isize) -> isize {
	// SAFETY: the pointer in the user data is the `DialogState` that `show_settings` keeps alive for the whole dialog.
	unsafe {
		match message {
			sys::WM_INITDIALOG => {
				sys::SetWindowLongPtr(dialog, sys::GWL_USERDATA, lparam);
				// The saver's own icon, when the file has one.
				let icon = sys::LoadIconA(sys::GetModuleHandleA(core::ptr::null()), sys::ICON_ID as *const u8);
				if !icon.is_null() {
					sys::SendMessageA(dialog, sys::WM_SETICON, 1, icon as isize);
					sys::SendMessageA(dialog, sys::WM_SETICON, 0, icon as isize);
				}
				let state = &*(lparam as *const DialogState);
				prepare(dialog, state.kind);
				show(dialog, &state.settings);
				1
			}
			sys::WM_COMMAND => {
				let state = sys::GetWindowLongPtr(dialog, sys::GWL_USERDATA) as *mut DialogState;
				if state.is_null() {
					return 0;
				}
				match (wparam & 0xFFFF) as u16 {
					template::OK_ID => {
						let state = &mut *state;
						collect(dialog, &mut state.settings);
						let mut store = RegStore::create(state.kind);
						state.settings.save(&mut store);
						sys::EndDialog(dialog, 1);
					}
					template::CANCEL_ID => {
						sys::EndDialog(dialog, 0);
					}
					DEFAULTS_ID => {
						show(dialog, &Settings::defaults((*state).kind));
					}
					_ => {}
				}
				0
			}
			_ => 0,
		}
	}
}

/// Runs the settings dialog to the end, owned by `owner` (which may be null): the Afterglow one, or the plain one when the Afterglow
/// one is not wanted (high contrast, a display of 256 colours or fewer, `PlainDialog` set in the saver's key) or cannot be made.
pub fn show_settings(kind: SaverKind, owner: Handle, make: MakeScene) {
	if crate::dialog_v2::wanted(kind) && crate::dialog_v2::show_afterglow(kind, owner, make) {
		return;
	}
	show_plain(kind, owner);
}

/// The plain dialog: the table-generated template with standard controls.
fn show_plain(kind: SaverKind, owner: Handle) {
	let mut state = DialogState {
		kind,
		settings: Settings::load(kind, &RegStore::open(kind)),
	};
	let template = template::build(kind);
	// SAFETY: the template is a complete `DLGTEMPLATE` that outlives the call, and the state outlives the dialog.
	unsafe {
		sys::InitCommonControls();
		let instance = sys::GetModuleHandleA(core::ptr::null());
		sys::DialogBoxIndirectParamA(
			instance,
			template.bytes.as_ptr(),
			owner,
			Some(dialog_proc),
			&mut state as *mut DialogState as isize,
		);
	}
}
