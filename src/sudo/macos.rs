use std::ffi::OsStr;
use std::process::{ExitStatus, Stdio};
use crate::waitpid::ProcessFuture;
use std::os::unix::io::AsRawFd;
use crate::Error;
use kiruna::Priority;

///Type that elevates the permission to sudo.
///
/// This is a distinct type because internally sudo password is passed to stdin, meaning that
/// stdin binding is not generally available.  However, I think it's possible to append to stdin behavior if desired?
pub struct Sudo(std::process::Command, String);
impl Sudo {
    pub fn new<S: AsRef<OsStr>>(program: S, password: String) -> Self {
        let mut p = std::process::Command::new("sudo");
        p.arg("-S"); //read password from stdin
        p.arg("-k"); //force read password regardless of recent timing settings
        p.arg(program);
        Sudo(p, password)
    }
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Sudo {
        self.0.arg(arg);
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr> {
        self.0.args(args);
        self
    }

    pub async fn status<'a>(&mut self, priority: Priority) -> Result<ExitStatus, Error> {
        self.0.stdin(Stdio::piped());
        let mut spawned = self.0.spawn()?;
        let mut password_vec = self.1.clone().into_bytes();
        password_vec.push('\n' as u8);
        let password_boxed = password_vec.into_boxed_slice();
        let pipe_fd = spawned.stdin.as_ref().unwrap().as_raw_fd();
        let write_task = kiruna::io::stream::write::Write::new(pipe_fd);
        let write_result = write_task.write_boxed(password_boxed,priority);
        //we want to drop stdin before leaving this function, so that sudo will give up if we fail
        write_result.await?;
        drop(spawned.stdin.take());
        let future = ProcessFuture::new(spawned.id() as i32);
        use std::os::unix::process::ExitStatusExt;
        Ok(ExitStatus::from_raw(future.await))
    }

}

#[test] fn sudo() {
    let _single_file = crate::waitpid::test::TEST_SEMAPHORE.lock();
    let mut s = Sudo::new("whoami","notmypassword".to_string());
    let future = s.status(Priority::UserWaiting);
    let _result = kiruna::test::test_await(future, std::time::Duration::from_secs(5));
    //evidently sudo requires no password on github's CI runners.
    //todo: Maybe we should use a programmatic API for sudo on macos?
    // assert_eq!(result.unwrap().code(),Some(1));
}