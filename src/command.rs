use std::ffi::OsStr;
use std::io::Result;
use super::child::Child;
use std::process::{Stdio, ExitStatus};
use super::waitpid::ProcessFuture;

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
    // pub async fn output(&mut self) -> Result<std::process::Output> {
    //     //pipe input and output
    //     self.0.stdout(Stdio::piped()).stderr(Stdio::piped());
    //     std::process::Output {
    //         status: ,
    //         stdout: vec![],
    //         stderr: vec![]
    //     }
    // }
    pub async fn status(&mut self) -> std::io::Result<ExitStatus> {
        let spawned = self.0.spawn()?;
        let future = ProcessFuture::new(spawned.id() as i32);
        use std::os::unix::process::ExitStatusExt;
        Ok(ExitStatus::from_raw(future.await))
    }
}

