fn main() {
    windows::build! {
        Windows::Win32::System::Threading::{CreateSemaphoreA,ReleaseSemaphore,WaitForMultipleObjects,OpenProcess,GetExitCodeProcess},
        Windows::Win32::Foundation::CloseHandle,
        Windows::Win32::System::Diagnostics::Debug::GetLastError,
    }
}