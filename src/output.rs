use std::process::{ExitStatus, Child};
use crate::waitpid::ProcessFuture;
use kiruna::io::stream::{Read, OSReadOptions};
use crate::Error;

pub struct OutputBuffer(kiruna::io::stream::Contiguous);
impl OutputBuffer {
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
    pub fn as_dispatch_data(&self) -> &dispatchr::data::Unmanaged { self.0.as_dispatch_data() }
}
///compare with [std::process::Output]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: OutputBuffer,
    pub stderr: OutputBuffer
}

impl Output {
    pub(crate) async fn from_child<'a,O: Into<OSReadOptions<'a>> + Clone>(child: Child,options:O) -> Result<Output,Error> {
        use std::os::unix::process::ExitStatusExt;
        use std::os::unix::io::IntoRawFd;
        //pipe input and output
        let status = ProcessFuture::new(child.id() as i32);
        let output_task = Read::new(child.stdout.unwrap().into_raw_fd());
        let error_task = Read::new(child.stderr.unwrap().into_raw_fd());
        let output_future = output_task.all(options.clone().into());
        let error_future = error_task.all(options.into());
        let joined_io = kiruna::join::try_join2(output_future,error_future);

        let all = kiruna::join::join2(status,joined_io);

        let result = all.await;
        let nonerr = result.1.map_err(|e| e.merge())?;

        Ok(Output {
            status: ExitStatus::from_raw(result.0),
            stdout: OutputBuffer(nonerr.0.as_contiguous()),
            stderr: OutputBuffer(nonerr.1.as_contiguous())
        })
    }
}

#[test] fn test_output() {
    use crate::Command;
    use kiruna::Priority;


    let mut c = Command::new("echo");
    let c2 = c.arg("foo").arg("bar").output(Priority::Testing);
    let r = kiruna::test::test_await(c2,std::time::Duration::from_secs(1));
    assert!(r.is_ok());
    assert_eq!(r.unwrap().stdout.as_slice(), "foo bar\n".as_bytes());
}