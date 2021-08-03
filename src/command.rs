use std::ffi::OsStr;
use std::io::Result;
use super::child::Child;

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
    pub fn spawn(&mut self) -> Result<Child> {
        self.0.spawn().map(|c| Child::new(c))
    }
    // pub async fn status(&mut self) -> std::io::Result<ExitStatus> {
    //
    // }
}

