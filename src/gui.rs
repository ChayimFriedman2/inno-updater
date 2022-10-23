/*-----------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See LICENSE in the project root for license information.
 *----------------------------------------------------------------------------------------*/

use std::sync::mpsc::Sender;
use std::{mem, ptr};
use strings::to_utf16;
use windows_sys::core::PCWSTR;
use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, WPARAM};

extern "system" {
	pub fn ShutdownBlockReasonCreate(hWnd: HWND, pwszReason: PCWSTR) -> BOOL;
	pub fn ShutdownBlockReasonDestroy(hWnd: HWND) -> BOOL;
}

struct DialogData {
	silent: bool,
	tx: Sender<ProgressWindow>,
}

unsafe extern "system" fn dlgproc(hwnd: HWND, msg: u32, _: WPARAM, l: LPARAM) -> isize {
	use resources;
	use windows_sys::Win32::Foundation::RECT;
	use windows_sys::Win32::System::Threading::GetCurrentThreadId;
	use windows_sys::Win32::UI::WindowsAndMessaging::{
		GetDesktopWindow, GetWindowRect, SendDlgItemMessageW, SetWindowPos, ShowWindow,
		HWND_TOPMOST, SW_HIDE, WM_DESTROY, WM_INITDIALOG, WM_USER,
	};

	match msg {
		v if v == WM_INITDIALOG => {
			let data = &*(l as *const DialogData);
			if !data.silent {
				SendDlgItemMessageW(hwnd, resources::PROGRESS_SLIDER, WM_USER + 10, 1, 0);

				#[allow(clippy::uninit_assumed_init)]
				let mut rect = mem::MaybeUninit::<RECT>::uninit().assume_init();
				GetWindowRect(hwnd, &mut rect);

				let width = rect.right - rect.left;
				let height = rect.bottom - rect.top;

				GetWindowRect(GetDesktopWindow(), &mut rect);

				SetWindowPos(
					hwnd,
					HWND_TOPMOST,
					rect.right / 2 - width / 2,
					rect.bottom / 2 - height / 2,
					width,
					height,
					0,
				);
			} else {
				ShowWindow(hwnd, SW_HIDE);
			}

			data.tx
				.send(ProgressWindow {
					ui_thread_id: GetCurrentThreadId(),
				})
				.unwrap();

			ShutdownBlockReasonCreate(hwnd, to_utf16("Visual Studio Code is updating...").as_ptr());
			0
		}
		v if v == WM_DESTROY => {
			ShutdownBlockReasonDestroy(hwnd);
			0
		}
		_ => 0,
	}
}

pub struct ProgressWindow {
	ui_thread_id: u32,
}

unsafe impl Send for ProgressWindow {}

impl ProgressWindow {
	pub fn exit(&self) {
		use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};

		unsafe {
			PostThreadMessageW(self.ui_thread_id, WM_QUIT, 0, 0);
		}
	}
}

pub fn run_progress_window(silent: bool, tx: Sender<ProgressWindow>) {
	use resources;
	use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
	use windows_sys::Win32::UI::WindowsAndMessaging::DialogBoxParamW;

	let data = DialogData { silent, tx };

	unsafe {
		DialogBoxParamW(
			GetModuleHandleW(ptr::null_mut()),
			resources::PROGRESS_DIALOG as PCWSTR,
			mem::zeroed(),
			Some(dlgproc),
			(&data as *const DialogData) as LPARAM,
		);
	}
}

pub enum MessageBoxType {
	Error,
	RetryCancel,
}

#[derive(Debug)]
pub enum MessageBoxResult {
	Unknown,
	Abort,
	Cancel,
	Continue,
	Ignore,
	No,
	OK,
	Retry,
	TryAgain,
	Yes,
}

pub fn message_box(text: &str, caption: &str, mbtype: MessageBoxType) -> MessageBoxResult {
	use windows_sys::Win32::UI::WindowsAndMessaging::{
		MessageBoxW, IDABORT, IDCANCEL, IDCONTINUE, IDIGNORE, IDNO, IDOK, IDRETRY, IDTRYAGAIN,
		IDYES, MB_ICONERROR, MB_RETRYCANCEL, MB_SYSTEMMODAL,
	};

	let result: i32;

	unsafe {
		result = MessageBoxW(
			mem::zeroed(),
			to_utf16(text).as_ptr(),
			to_utf16(caption).as_ptr(),
			match mbtype {
				MessageBoxType::Error => MB_ICONERROR | MB_SYSTEMMODAL,
				MessageBoxType::RetryCancel => MB_RETRYCANCEL | MB_ICONERROR | MB_SYSTEMMODAL,
			},
		)
	}

	match result {
		IDABORT => MessageBoxResult::Abort,
		IDCANCEL => MessageBoxResult::Cancel,
		IDCONTINUE => MessageBoxResult::Continue,
		IDIGNORE => MessageBoxResult::Ignore,
		IDNO => MessageBoxResult::No,
		IDOK => MessageBoxResult::OK,
		IDRETRY => MessageBoxResult::Retry,
		IDTRYAGAIN => MessageBoxResult::TryAgain,
		IDYES => MessageBoxResult::Yes,
		_ => MessageBoxResult::Unknown,
	}
}
