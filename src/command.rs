use std::ffi::OsStr;
use std::process::{ExitStatus};
use super::waitpid::ProcessFuture;

#[cfg(feature="output")]
use kiruna::io::stream::{OSReadOptions};
#[cfg(feature="output")]
use crate::output::{Output};
#[cfg(feature="output")]
use std::io::Result;


///A process builder; compare with [std::process::Command]
pub struct Command(std::process::Command);

impl Command {
    pub fn new<S: AsRef<OsStr>>(program: S) -> Self {
        Command(std::process::Command::new(program))
    }
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Command {
        self.0.arg(arg);
        self
    }
    #[cfg(feature="output")]
    pub async fn output<'a>(&mut self, options: OSReadOptions<'a>) -> Result<Output> {
        use std::process::Stdio;
        self.0.stdout(Stdio::piped());
        self.0.stderr(Stdio::piped());
        let spawned = self.0.spawn()?;
        Ok(Output::from_child(spawned,options).await)
    }
    pub async fn status(&mut self) -> std::io::Result<ExitStatus> {
        let spawned = self.0.spawn()?;
        let future = ProcessFuture::new(spawned.id() as i32);
        use std::os::unix::process::ExitStatusExt;
        Ok(ExitStatus::from_raw(future.await))
    }
}

