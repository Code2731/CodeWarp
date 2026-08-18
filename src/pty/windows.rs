use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};

pub(super) fn capture_direct_children(parent_pid: u32) -> Vec<u32> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Vec::new();
        }

        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut children = Vec::new();
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                if entry.th32ParentProcessID == parent_pid {
                    children.push(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
        children
    }
}

pub(super) fn terminate_children(process_ids: Vec<u32>) {
    for process_id in process_ids {
        unsafe {
            let process = OpenProcess(PROCESS_TERMINATE, 0, process_id);
            if process.is_null() {
                continue;
            }
            let _ = TerminateProcess(process, 1);
            CloseHandle(process);
        }
    }
}
