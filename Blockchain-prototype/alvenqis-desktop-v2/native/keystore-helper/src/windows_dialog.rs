//! Windows-only recovery dialogs for the keystore helper.
//!
//! # Safety model
//! All `unsafe` is confined to Win32 UI APIs (`MessageBoxW` / window class + message loop).
//! Invariants:
//! - UTF-16 strings are null-terminated and kept alive for the duration of each API call.
//! - HWND values are only used while the window is alive (message loop ownership).
//! - Recovery phrase buffers are zeroized after copy out of Win32.
//! - No raw pointer arithmetic; no lifetime extension across threads.
//!
//! These APIs are inherently FFI; the goal is a minimal, auditable surface rather than
//! eliminating `unsafe` (which would require a third-party GUI crate with its own unsafe).

use crate::{HelperError, Result};
use std::sync::{
    atomic::{AtomicIsize, Ordering},
    Mutex, OnceLock,
};
use zeroize::Zeroize;

/// Convert Rust `&str` to a null-terminated UTF-16 buffer for Win32.
fn wide(value: &str) -> Vec<u16> {
    // Cap to avoid pathological MessageBox allocations from attacker-controlled phrases.
    let capped: String = value.chars().take(4_096).collect();
    capped.encode_utf16().chain(std::iter::once(0)).collect()
}

/// One-time display of the recovery phrase. Returns `true` if the user confirms OK.
pub fn show_recovery_phrase(phrase: &str) -> Result<bool> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDOK, MB_ICONWARNING, MB_OKCANCEL, MB_TOPMOST,
    };

    // Reject unexpected control chars before handing text to the OS dialog.
    if phrase.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
        return Err(HelperError::Input(
            "recovery phrase contains invalid control characters".into(),
        ));
    }

    let text = wide(&format!(
        "Write down these 24 words in order. They will not be shown again.\n\n{phrase}\n\nPress OK only after your offline backup is complete."
    ));
    let title = wide("Alvenqis recovery phrase - one-time display");

    // SAFETY: `text`/`title` are valid null-terminated UTF-16, live for the call duration.
    // HWND is null (desktop owner). MessageBoxW is a standard user32 entry point.
    let result = unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OKCANCEL | MB_ICONWARNING | MB_TOPMOST,
        )
    };
    Ok(result == IDOK)
}

/// Modal import dialog. Phrase never crosses the React/WebView boundary.
pub fn prompt_recovery_phrase() -> Result<String> {
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Gdi::UpdateWindow,
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
            GetWindowTextLengthW, GetWindowTextW, LoadCursorW, PostQuitMessage, RegisterClassW,
            ShowWindow, TranslateMessage, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, ES_AUTOHSCROLL,
            HMENU, IDC_ARROW, MSG, SW_SHOW, WM_COMMAND, WM_CREATE, WM_DESTROY, WNDCLASSW,
            WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPED, WS_SYSMENU,
            WS_VISIBLE,
        },
    };

    // Single-threaded modal dialog; atomics only store HWND between WM_CREATE and WM_COMMAND.
    static EDIT: AtomicIsize = AtomicIsize::new(0);
    static RESULT: OnceLock<Mutex<Option<String>>> = OnceLock::new();

    /// SAFETY: Invoked only by the Windows message pump for our registered class.
    /// - `window` is a valid HWND owned by this process for the dialog lifetime.
    /// - Child HWNDs are created with this `window` as parent.
    /// - GetWindowTextW writes into a Rust Vec sized to the reported length + 1.
    unsafe extern "system" fn window_proc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CREATE => {
                let instance = GetModuleHandleW(std::ptr::null());
                let _ = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Enter the 24-word Alvenqis recovery phrase. It stays in the native helper and is not exposed to the web UI.").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    20,
                    20,
                    660,
                    36,
                    window,
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                );
                let edit = CreateWindowExW(
                    WS_EX_CLIENTEDGE,
                    wide("EDIT").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_BORDER | ES_AUTOHSCROLL as u32,
                    20,
                    66,
                    660,
                    30,
                    window,
                    std::ptr::null_mut(),
                    instance,
                    std::ptr::null(),
                );
                EDIT.store(edit as isize, Ordering::SeqCst);
                let _ = CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide("Import wallet").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    438,
                    116,
                    116,
                    34,
                    window,
                    1usize as HMENU,
                    instance,
                    std::ptr::null(),
                );
                let _ = CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide("Cancel").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    564,
                    116,
                    116,
                    34,
                    window,
                    2usize as HMENU,
                    instance,
                    std::ptr::null(),
                );
                0
            }
            WM_COMMAND => {
                match wparam & 0xffff {
                    1 => {
                        let edit = EDIT.load(Ordering::SeqCst) as HWND;
                        if edit.is_null() {
                            DestroyWindow(window);
                            return 0;
                        }
                        let length = GetWindowTextLengthW(edit);
                        // Guard against API failure / absurd lengths (DoS on allocation).
                        if length < 0 || length > 4_096 {
                            DestroyWindow(window);
                            return 0;
                        }
                        let mut buffer = vec![0u16; length as usize + 1];
                        // SAFETY: buffer is large enough for length+null; edit HWND still valid.
                        let written = GetWindowTextW(edit, buffer.as_mut_ptr(), buffer.len() as i32);
                        let take = written.max(0) as usize;
                        let phrase = String::from_utf16_lossy(&buffer[..take.min(buffer.len())]);
                        buffer.zeroize();
                        if let Ok(mut guard) = RESULT
                            .get_or_init(|| Mutex::new(None))
                            .lock()
                        {
                            *guard = Some(phrase);
                        }
                        DestroyWindow(window);
                    }
                    2 => {
                        DestroyWindow(window);
                    }
                    _ => {}
                }
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            // SAFETY: DefWindowProcW is the standard default handler for unhandled messages.
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }

    *RESULT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| HelperError::Service("recovery dialog lock failed".into()))? = None;

    // SAFETY: Registers a process-local window class and runs a modal message loop on this
    // thread only. Class name is unique. HWND null checks prevent use of failed creates.
    unsafe {
        let instance = GetModuleHandleW(std::ptr::null());
        let class_name = wide("AlvenqisRecoveryImportWindow");
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            lpszClassName: class_name.as_ptr(),
            ..std::mem::zeroed()
        };
        // Ignore already-registered (ERROR_CLASS_ALREADY_EXISTS) — second import is fine.
        let _ = RegisterClassW(&class);
        let window = CreateWindowExW(
            0,
            class_name.as_ptr(),
            wide("Import Alvenqis wallet").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            210,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        );
        if window.is_null() {
            return Err(HelperError::Service(
                "could not create recovery import dialog".into(),
            ));
        }
        ShowWindow(window, SW_SHOW);
        UpdateWindow(window);
        let mut message: MSG = std::mem::zeroed();
        while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    RESULT
        .get()
        .and_then(|result| result.lock().ok()?.take())
        .filter(|phrase| !phrase.trim().is_empty())
        .ok_or_else(|| HelperError::Input("wallet import cancelled".into()))
}
