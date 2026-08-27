//! Windows named-pipe control channel (`\\.\pipe\rgctl_<hash>`).

use super::config::DaemonHome;
use anyhow::{Context, Result, bail};
use std::fs::File;
use std::os::windows::io::{FromRawHandle, RawHandle};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, GetLastError,
    INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS,
    PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, WaitNamedPipeW,
};

const PIPE_BUFFER: u32 = 65_536;
const CONNECT_WAIT_MS: u32 = 5_000;

pub fn pipe_name(home: &DaemonHome) -> String {
    let digest = blake3::hash(home.root().to_string_lossy().as_bytes());
    let hex = digest.to_hex();
    format!(r"\\.\pipe\rgctl_{}", &hex[..16])
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn accept(home: &DaemonHome) -> Result<File> {
    let name = pipe_name(home);
    let wide = wide(&name);
    // SAFETY: `wide` is a valid NUL-terminated UTF-16 pipe path. The returned handle is
    // exclusive to this caller until moved into `File` or closed on error.
    let handle = unsafe {
        CreateNamedPipeW(
            wide.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            PIPE_BUFFER,
            PIPE_BUFFER,
            0,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        let err = std::io::Error::from_raw_os_error(unsafe { GetLastError() } as i32);
        bail!("CreateNamedPipeW {name}: {err}");
    }
    // SAFETY: `handle` is a live pipe instance from CreateNamedPipeW.
    let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) };
    if connected == 0 {
        let code = unsafe { GetLastError() };
        if code != ERROR_PIPE_CONNECTED {
            unsafe { CloseHandle(handle) };
            let err = std::io::Error::from_raw_os_error(code as i32);
            bail!("ConnectNamedPipe {name}: {err}");
        }
    }
    // SAFETY: `handle` is a connected pipe; `File` takes ownership and will CloseHandle.
    Ok(unsafe { File::from_raw_handle(handle as RawHandle) })
}

pub fn connect(home: &DaemonHome) -> Result<File> {
    let name = pipe_name(home);
    let wide = wide(&name);
    for _ in 0..40 {
        // SAFETY: `wide` is a valid NUL-terminated pipe path; share mode is none.
        let handle = unsafe {
            CreateFileW(
                wide.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
                std::ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            // SAFETY: connected client handle; `File` owns it.
            return Ok(unsafe { File::from_raw_handle(handle as RawHandle) });
        }
        let code = unsafe { GetLastError() };
        if code == ERROR_PIPE_BUSY {
            unsafe { WaitNamedPipeW(wide.as_ptr(), CONNECT_WAIT_MS) };
            continue;
        }
        let err = std::io::Error::from_raw_os_error(code as i32);
        return Err(err).with_context(|| format!("CreateFileW {name}"));
    }
    bail!("named pipe {name} busy")
}
