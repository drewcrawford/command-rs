use std::ffi::OsStr;
use std::process::{ExitStatus};

#[cfg(feature="output")]
use kiruna::io::stream::read::OSOptions;
#[cfg(feature="output")]
use crate::output::{Output};
use crate::Error;


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
    pub fn args<I, S>(&mut self, args: I) -> &mut Command
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr> {
        self.0.args(args);
        self
    }
    #[cfg(feature="output")]
    pub async fn output<'a,O: Into<OSOptions<'a>>>(&mut self, options: O) -> std::result::Result<Output, crate::Error> {
        use std::process::Stdio;
        self.0.stdout(Stdio::piped());
        self.0.stderr(Stdio::piped());
        let spawned = self.0.spawn()?;
        Ok(Output::from_child(spawned,options.into()).await?)
    }
    pub async fn status(&mut self, _priority: kiruna::Priority) -> Result<ExitStatus, Error> {
        //todo: use priority
        let spawned = self.0.spawn()?;
        use crate::waitpid::ProcessFuture;
        #[cfg(target_os = "macos")]
        type Pid = i32;
        #[cfg(target_os = "windows")]
        type Pid = u32;
        let future = ProcessFuture::new(spawned.id() as Pid);
        #[cfg(target_os="macos")]
        use std::os::unix::process::ExitStatusExt;
        #[cfg(target_os = "windows")]
        use std::os::windows::process::ExitStatusExt;
        Ok(ExitStatus::from_raw(future.await))
    }
}

