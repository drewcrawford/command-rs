use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::process::Command;
use libc::waitpid;

///A dedicated loop to watch for child process events.
#[derive(Debug)]
struct Waitpid {
    num_pids: u8
}
impl Waitpid {
    fn wait_one(&mut self)  {
        let mut status: i32 = 0;
        let waited_pid = unsafe{ waitpid(-1, &mut status, 0)};
        assert!(waited_pid > 0);
        self.num_pids -= 1;
    }
    fn done_waiting(&self) -> bool {
        self.num_pids == 0
    }
    fn shared() -> MutexGuard<'static, Option<Waitpid>> {
        static SHARED_WAIT: Lazy<Mutex<Option<Waitpid>>> = Lazy::new(||Mutex::new(None));
        SHARED_WAIT.lock().unwrap()
    }
    fn add(_future: ProcessFuture) {
        //todo: use the argument?
        let mut lock = Self::shared();
        if lock.is_none() {
            *lock = Some(Waitpid { num_pids: 1});
            println!("Assigning lock to {:?}",lock);
            std::thread::spawn(|| {
                loop {
                    //get a new lock in each loop iteration.
                    //This is so that we can interleave with calls to `add`.
                    let mut thread_lock = Waitpid::shared();
                    println!("Bringing up thread_lock {:?}",thread_lock);
                    if !thread_lock.as_ref().unwrap().done_waiting() {
                        thread_lock.as_mut().unwrap().wait_one();
                    }
                    else {
                        *thread_lock = None; //shutting down the waitpid
                        break; //exit loop
                    }
                }

            });
        }
        else {
            lock.as_mut().unwrap().num_pids += 1;
        }

    }
}

struct ProcessFuture(i32);

impl ProcessFuture {
    fn new(pid: i32) -> ProcessFuture {
        ProcessFuture(pid)
    }
    fn toy_await(self) {
        println!("await {:?}",self.0);
        Waitpid::add(self);
    }

}

#[test] fn toy_await() {
    use std::time::Duration;
    let mut command = Command::new("sleep");
    command.arg("0.1");
    let child = command.spawn().unwrap();
    let future = ProcessFuture::new(child.id() as i32);
    future.toy_await();
    std::thread::sleep(Duration::from_secs(1));
}

#[test] fn toy_await_2() {
    use std::time::Duration;
    fn spawn(duration: Duration) -> std::process::Child {
        let mut command = Command::new("sleep");
        command.arg(duration.as_secs().to_string());
        command.spawn().unwrap()
    }
    let f1 =  ProcessFuture::new(spawn(Duration::from_millis(100)).id() as i32);
    let f2 =  ProcessFuture::new(spawn(Duration::from_millis(200)).id() as i32);
    let f3 =  ProcessFuture::new(spawn(Duration::from_millis(300)).id() as i32);
    let f4 =  ProcessFuture::new(spawn(Duration::from_millis(400)).id() as i32);
    let f5 =  ProcessFuture::new(spawn(Duration::from_millis(500)).id() as i32);

    f1.toy_await();
    f2.toy_await();
    f3.toy_await();
    f4.toy_await();
    f5.toy_await();

    std::thread::sleep(Duration::from_secs(1));
}