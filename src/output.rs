use std::process::{ExitStatus, Child};
use crate::waitpid::ProcessFuture;
use kiruna::io::stream::{Read, OSReadOptions};
use dispatchr::data::Unmanaged;

pub struct OutputBuffer(kiruna::io::stream::Buffer);
impl OutputBuffer {
    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }
    pub fn as_dispatch_data(&self) -> &Unmanaged {
        self.0.as_dispatch_data()
    }
}
///compare with [std::process::Output]
pub struct Output {
    pub status: ExitStatus,
    pub stdout: OutputBuffer,
    pub stderr: OutputBuffer
}

impl Output {
    pub(crate) async fn from_child<'a,O: Into<OSReadOptions<'a>> + Clone>(child: Child,options:O) -> Output {
        use std::os::unix::process::ExitStatusExt;
        use std::os::unix::io::IntoRawFd;
        //pipe input and output
        let status = ProcessFuture::new(child.id() as i32);
        let output_future = Read::new(child.stdout.unwrap().into_raw_fd()).all(options.clone().into());
        let error_future = Read::new(child.stderr.unwrap().into_raw_fd()).all(options.into());
        let all = futures::join!(status,output_future,error_future);
        Output {
            status: ExitStatus::from_raw(all.0),
            stdout: OutputBuffer(all.1),
            stderr: OutputBuffer(all.2)
        }
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