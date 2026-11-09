//! One HTTPS GET with WinHTTP, for the update check (no HTTP crate; ADR 0002).

use std::ffi::c_void;

use windows::Win32::Networking::WinHttp::{
    INTERNET_DEFAULT_HTTPS_PORT, WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY, WINHTTP_FLAG_SECURE,
    WINHTTP_QUERY_FLAG_NUMBER, WINHTTP_QUERY_STATUS_CODE, WinHttpCloseHandle, WinHttpConnect,
    WinHttpOpen, WinHttpOpenRequest, WinHttpQueryHeaders, WinHttpReadData, WinHttpReceiveResponse,
    WinHttpSendRequest,
};
use windows::core::{Error, HSTRING, PCWSTR, Result, w};

/// Closes a WinHTTP handle when it goes out of scope.
struct Handle(*mut c_void);

impl Handle {
    fn new(raw: *mut c_void) -> Result<Self> {
        if raw.is_null() {
            Err(Error::from_thread())
        } else {
            Ok(Self(raw))
        }
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: the handle came from WinHTTP, is not null and is closed only here.
        let _ = unsafe { WinHttpCloseHandle(self.0) };
    }
}

/// The HTTP status code and body of `https://{host}{path}`. Blocks until the answer arrives
/// (WinHTTP's default timeouts apply), so call it off the UI thread.
///
/// # Errors
///
/// When the connection, the request or reading the answer fails.
pub fn https_get(host: &str, path: &str, user_agent: &str) -> Result<(u32, Vec<u8>)> {
    // SAFETY: the strings are null-terminated HSTRINGs or static wide strings that outlive
    // each call; every handle is checked for null before use and stays open (in scope) while
    // the handles opened from it are used; the buffers passed are as large as stated.
    unsafe {
        let session = Handle::new(WinHttpOpen(
            &HSTRING::from(user_agent),
            WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
            PCWSTR::null(),
            PCWSTR::null(),
            0,
        ))?;
        let connection = Handle::new(WinHttpConnect(
            session.0,
            &HSTRING::from(host),
            INTERNET_DEFAULT_HTTPS_PORT,
            0,
        ))?;
        let request = Handle::new(WinHttpOpenRequest(
            connection.0,
            w!("GET"),
            &HSTRING::from(path),
            PCWSTR::null(),
            PCWSTR::null(),
            std::ptr::null(),
            WINHTTP_FLAG_SECURE,
        ))?;
        WinHttpSendRequest(request.0, None, None, 0, 0, 0)?;
        WinHttpReceiveResponse(request.0, std::ptr::null_mut())?;
        let mut status = 0u32;
        let mut size = size_of::<u32>() as u32;
        WinHttpQueryHeaders(
            request.0,
            WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
            PCWSTR::null(),
            Some((&raw mut status).cast()),
            &mut size,
            std::ptr::null_mut(),
        )?;
        let mut body = Vec::new();
        let mut chunk = [0u8; 8192];
        loop {
            let mut read = 0u32;
            WinHttpReadData(
                request.0,
                chunk.as_mut_ptr().cast(),
                chunk.len() as u32,
                &mut read,
            )?;
            if read == 0 {
                return Ok((status, body));
            }
            body.extend_from_slice(&chunk[..read as usize]);
        }
    }
}
