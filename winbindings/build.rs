fn main() {
    windows::build! {
        Windows::Win32::System::Threading::{
            CreateSemaphoreA,
            ReleaseSemaphore,
            WaitForMultipleObjects,
            OpenProcess,
            GetExitCodeProcess,
            CreateProcessWithLogonW,
            PROCESS_CREATION_FLAGS,
        },
        Windows::Win32::Foundation::CloseHandle,
        Windows::Win32::System::Diagnostics::Debug::{GetLastError,WIN32_ERROR},

    }
}