mod command;
mod waitpid;

#[cfg(feature="output")]
mod output;
#[cfg(feature="sudo")]
mod sudo;
mod status;


#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    #[cfg(any(feature="sudo",feature="output"))]
    KirunaError(kiruna::io::stream::OSError),
    IOError(std::io::Error),
    StatusError(i32),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}",self))
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(f: std::io::Error) -> Self {
        Error::IOError(f)
    }
}
#[cfg(any(feature="sudo",feature="output"))]
impl From<kiruna::io::stream::OSError> for Error {
    fn from(f: kiruna::io::stream::OSError) -> Self {
        Self::KirunaError(f)
    }
}

pub use command::Command;
use std::fmt::Formatter;

#[cfg(feature="output")]
pub use output::Output;


pub use status::ExitStatus;

#[cfg(test)] pub fn test_is_present() {}
#[cfg(feature="sudo")] pub use sudo::Sudo;



