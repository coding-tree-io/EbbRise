use windows::Win32::Foundation::CloseHandle;
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};

/// Executable names of all running processes, such as `Code.exe`; empty if Windows refuses.
///
/// Matches upstream's `ps-list` on Windows, which also gives names only, not command lines.
pub fn running_process_names() -> Vec<String> {
    // SAFETY: no pointers are passed; the handle is checked and closed below.
    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return Vec::new();
    };
    let mut entry = PROCESSENTRY32W {
        dwSize: size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    let mut names = Vec::new();
    // SAFETY: `snapshot` is a valid process snapshot and `entry` has `dwSize` set, as the API requires.
    let mut found = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
    while found {
        let length = entry
            .szExeFile
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(entry.szExeFile.len());
        names.push(String::from_utf16_lossy(&entry.szExeFile[..length]));
        // SAFETY: as for `Process32FirstW`.
        found = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    // SAFETY: `snapshot` is open and not used after this.
    let _ = unsafe { CloseHandle(snapshot) };
    names
}
