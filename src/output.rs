use std::process::{ExitStatus, Child};
use crate::waitpid::ProcessFuture;
use kiruna::io::stream::read::{Read, OSOptions};
use crate::Error;

pub struct OutputBuffer(kiruna::io::stream::read::ContiguousBuffer);
impl OutputBuffer {
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
    #[cfg(target_os = "macos")]
    pub fn as_dispatch_data(&self) -> &dispatchr::data::Unmanaged { self.0.as_dispatch_data() }
}
///compare with [std::process::Output]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: OutputBuffer,
    pub stderr: OutputBuffer
}

impl Output {
    pub(crate) async fn from_child<'a,O: Into<OSOptions<'a>> + Clone>(child: Child,options:O) -> Result<Output,Error> {
        #[cfg(not(any(target_os = "windows",target_os="mac")))]
        complie_error!("Unsupported");

        #[cfg(target_os = "macos")]
        let (status_arg, output_arg, error_arg) = {
            use std::os::unix::process::ExitStatusExt;
            use std::os::unix::io::IntoRawFd;
            let status_arg = child.id() as i32;
            let output_arg = child.stdout.unwrap().into_raw_fd();
            let error_arg = child.stderr.unwrap().into_raw_fd();
            (status_arg,output_arg,error_arg)
        };
        #[cfg(target_os = "windows")]
            let (status_arg, output_arg, error_arg) = {
            let status_arg = child.id() as u32;
            let output_arg = child.stdout.unwrap();
            let error_arg = child.stderr.unwrap();
            (status_arg,output_arg,error_arg)
        };

        //pipe input and output
        let status = ProcessFuture::new(status_arg);
        let output_task = Read::new(output_arg);
        let error_task = Read::new(error_arg);
        let output_future = output_task.all(options.clone().into());
        let error_future = error_task.all(options.into());
        let joined_io = kiruna::join::try_join2(output_future,error_future);

        let all = kiruna::join::join2(status,joined_io);

        let result = all.await;
        let nonerr = result.1.map_err(|e| e.merge())?;
        #[cfg(target_os = "windows")]
        use std::os::windows::process::ExitStatusExt;
        #[cfg(target_os = "macos")]
        use std::os::unix::process::ExitStatusExt;
        Ok(Output {
            status: ExitStatus::from_raw(result.0),
            stdout: OutputBuffer(nonerr.0.into_contiguous()),
            stderr: OutputBuffer(nonerr.1.into_contiguous())
        })
    }
}
#[cfg(test)]
mod test {
    use crate::Command;
    use kiruna::Priority;

    //note that 'echo' is builtin on windows...
    #[cfg(target_os = "macos")]
    #[test] fn test_output() {
        use crate::Command;
        use kiruna::Priority;


        let mut c = Command::new("echo");
        let c2 = c.arg("foo").arg("bar").output(Priority::Testing);
        let r = kiruna::test::test_await(c2,std::time::Duration::from_secs(1));
        assert_eq!(r.unwrap().stdout.as_slice(), "foo bar\n".as_bytes());
    }

    #[cfg(target_os = "windows")]
    #[test] fn test_output() {
        let mut c = Command::new("ipconfig");
        let c2 = c.output(Priority::Testing);
        let r = kiruna::test::test_await(c2,std::time::Duration::from_secs(3));
        let slice = r.as_ref().unwrap().stdout.as_slice();
        let str = std::str::from_utf8(slice).unwrap();
        assert!(str.contains("Windows IP Configuration"));
    }
}
