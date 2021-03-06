#[cfg(target_os="macos")]
mod macos;
#[cfg(target_os="macos")]
pub (crate) use macos::ProcessFuture;
#[cfg(target_os="windows")]
mod windows;
#[cfg(target_os="windows")]
pub (crate) use windows::ProcessFuture;

#[cfg(all(target_os="macos",test))]
pub (crate) use macos::test;
