use std::ffi::{OsStr, OsString};
use kiruna::Priority;
use std::process::ExitStatus;
use crate::Error;
use std::os::windows::process::ExitStatusExt;


use winbindings::Windows::Win32::System::Threading::PROCESS_CREATION_FLAGS;
trait PriorityProcess {
    fn as_priority_class(&self) -> PROCESS_CREATION_FLAGS;
}
impl PriorityProcess for Priority {
    fn as_priority_class(&self) -> PROCESS_CREATION_FLAGS {
        use winbindings::Windows::Win32::System::Threading::NORMAL_PRIORITY_CLASS;
        match self {
            Priority::UserWaiting => {NORMAL_PRIORITY_CLASS}
            Priority::Testing => {NORMAL_PRIORITY_CLASS}
            _unknown => {NORMAL_PRIORITY_CLASS}
        }
    }
}

pub struct Sudo {
    program: OsString,
    password: String,
    command_line: OsString,
}

impl Sudo {
    pub fn new<S: AsRef<OsStr>>(program: S, password: String) -> Self {
        Sudo {
            program: program.as_ref().to_os_string(),
            password,
            command_line: program.as_ref().to_os_string(),
        }
    }
    pub fn arg<S: AsRef<OsStr>>(&mut self, arg: S) -> &mut Sudo {
        let barg = arg.as_ref();
        //' ' + '"' + arg + '"' is 3 overhead
        self.command_line.reserve(barg.len() + 3);
        self.command_line.push(" \"");
        self.command_line.push(barg);
        self.command_line.push("\"");
        self
    }
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
        where
            I: IntoIterator<Item = S>,
            S: AsRef<OsStr> {
        for arg in args.into_iter() {
            //I don't think there is a faster algorithm than this as we can't
            //loop over the args twice to reserve a total allocation
            self.arg(arg);
        }
        self
    }

    pub async fn status<'a>(&mut self, priority: Priority) -> Result<ExitStatus, Error> {
        use winbindings::Windows::Win32::System::Threading::{CreateProcessWithLogonW,
                                                             STARTUPINFOW,PROCESS_INFORMATION,
                                                             STARTUPINFOW_FLAGS,
                                                             CREATE_PROCESS_LOGON_FLAGS,CREATE_UNICODE_ENVIRONMENT};
        use winbindings::Windows::Win32::Foundation::{PWSTR,HANDLE};
        use winbindings::Windows::Win32::System::Diagnostics::Debug::GetLastError;
        //https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/ns-processthreadsapi-startupinfoa
        //in that doc, the example mostly uses 0 for fields, with the exception of cb
        let startup_information = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            lpReserved: PWSTR(std::ptr::null_mut()),
            lpDesktop: PWSTR(std::ptr::null_mut()),
            lpTitle: PWSTR(std::ptr::null_mut()),
            dwX: 0,
            dwY: 0,
            dwXSize: 0,
            dwYSize: 0,
            dwXCountChars: 0,
            dwYCountChars: 0,
            dwFillAttribute: 0,
            dwFlags: STARTUPINFOW_FLAGS(0),
            wShowWindow: 0,
            cbReserved2: 0,
            lpReserved2: std::ptr::null_mut(),
            hStdInput: HANDLE(0),
            hStdOutput: HANDLE(0),
            hStdError: HANDLE(0),
        };
        let mut process_information = PROCESS_INFORMATION {
            dwProcessId: 0,
            dwThreadId:0,
            hProcess: HANDLE(0),
            hThread: HANDLE(0)
        };
        /*     If this parameter is NULL and the environment block of the parent process contains Unicode characters,
           you must also ensure that dwCreationFlags includes CREATE_UNICODE_ENVIRONMENT.*/
        let creation_flags = priority.as_priority_class() | CREATE_UNICODE_ENVIRONMENT;
        let r = unsafe {
            CreateProcessWithLogonW("Administrator",
                                                  PWSTR(std::ptr::null_mut()), //domain?
                                                  self.password.clone(),
                                                  CREATE_PROCESS_LOGON_FLAGS(0), //profile not required
                                                  self.program.clone(),
                                                  self.command_line.clone(),
                                                  creation_flags.0,
                                                  std::ptr::null(),
                                                  //inherit directory from calling process
                                                  PWSTR(std::ptr::null_mut()),
                                                  &startup_information,
                                                  &mut process_information
            )
        };

        if r.0 == 0 {
            return Err(Error::WinError(unsafe{ GetLastError()}))
        }
        let s = crate::waitpid::ProcessFuture::new(process_information.dwProcessId);
        let r = s.await;
        Ok(ExitStatus::from_raw(r))
    }
}

#[test] fn test_sudo() {
    let mut c = Sudo::new("whoami","invalid".to_string());
    let r = c.status(kiruna::Priority::Testing);
    let result = kiruna::test::test_await(r, std::time::Duration::from_secs(10));
    match result {
        Err(Error::WinError(winbindings::Windows::Win32::System::Diagnostics::Debug::ERROR_LOGON_FAILURE)) => {},
        other => panic!("Unexpected result {:?}",other),
    }
}