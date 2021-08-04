use std::sync::{Mutex, MutexGuard};
use once_cell::sync::Lazy;
use std::process::Command;
use libc::waitpid;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::collections::{HashMap};

///A dedicated loop to watch for child process events.
#[derive(Debug)]
struct Waitpid {
    ///Processes we have been asked to await.  We know how to wake them.
    inbox: HashMap<ProcessFuture,Waker>,
    ///Processes that we know have terminated, but nobody polled us about them
    outbox: HashMap<ProcessFuture,i32>,
    ///waiting thread, if any.  Note that we may need orphan values without any running thread.
    waiting_thread: bool
}
impl Waitpid {
    ///Err indicates we need to stop the thread.  Note that for correctness, waiting_thread is assigned to None
    ///internally, on the same lock that this function acquires.
    fn wait_one() -> Result<(),()> {
        let mut status: i32 = 0;
        let waited_pid = unsafe{ waitpid(-1, &mut status, 0)};
        //println!("waited_pid {:?} status {:?}",waited_pid,status);
        if waited_pid < 0 {
            //println!("error {:?}",unsafe{*libc::__error()});
            //usually, we have no child processes.
            let mut s = Waitpid::shared();
            //Because we aren't holding a lock, it's possible
            //that Waitpid was removed while this thread was still active.
            //If so, handle this gracefully
            s.waiting_thread = false;
            return Err(());
        }
        let found_future = ProcessFuture(waited_pid);
        //at this point (after our wait), we need to lock and report this event
        let mut s = Waitpid::shared();
        //insert the outbox
        s.outbox.insert(found_future.clone(), status);
        //If we know about this, wake the appropriate party
        //If not, wait for somebody to tell us how to do this in the future
        if let Some(waker) = s.inbox.remove(&found_future) {
           // println!("wake_by_ref {:?}",found_future);
            waker.wake_by_ref();
        }
        if s.inbox.len() == 0 {
            //println!("inbox zero");
            s.waiting_thread = false;
            Err(())
        }
        else {
            Ok(())
        }

    }
    //todo: This could be promoted to option to save some memory.  Currently, this static leaks ~150 bytes.
    fn shared() -> MutexGuard<'static, Waitpid> {
        static SHARED_WAIT: Lazy<Mutex<Waitpid>> = Lazy::new(||Mutex::new(Waitpid {
            inbox: Default::default(),
            outbox: Default::default(),
            waiting_thread: false
        }));
        SHARED_WAIT.lock().unwrap()
    }
    ///Poll, inside the lock
    ///
    fn poll_inside(&mut self, future: ProcessFuture, waker: Waker) -> Poll<i32> {
        if let Some(status) = self.outbox.remove(&future){
            return Poll::Ready(status)
        }
        //update with new waker
        self.inbox.insert(future, waker);
        //If we have no running thread, we need to start one
        if !self.waiting_thread {
            self.waiting_thread = true;
            std::thread::spawn(|| {
                //println!("waitpid thread begin");
                //new lock
                while Waitpid::wait_one().is_ok() {
                    //loop
                }
                //println!("waitpid thread shutdown");
                //wait_one will unset the thread handle already
            });
        }
        //try again later
        Poll::Pending
    }
    fn poll(future: ProcessFuture, waker: Waker) -> Poll::<i32> {
        let mut lock = Self::shared();
        let poll_inside = lock.poll_inside(future, waker);
        //println!("poll_inside {:?}",poll_inside);
        poll_inside

    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct ProcessFuture(i32);



impl ProcessFuture {
    fn new(pid: i32) -> ProcessFuture {
        ProcessFuture(pid)
    }
}

impl Future for ProcessFuture {
    type Output = i32;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Waitpid::poll(self.clone(), cx.waker().clone())
    }
}

#[cfg(test)]
mod test {
    use std::task::{Wake, Poll, Context, Waker};
    use std::sync::{Arc, Mutex};
    use crate::waitpid::ProcessFuture;
    use std::pin::Pin;
    use std::future::Future;
    use once_cell::sync::Lazy;

    //fake waker purely for debug purposes
    struct FakeWaker;
    impl Wake for FakeWaker {
        fn wake(self: Arc<Self>) {
            //nothing
        }
    }
    impl ProcessFuture {
        pub fn toy_await(&mut self) -> Poll::<i32> {
            //println!("await {:?}",self.0);
            let fake_waker = Arc::new(FakeWaker);
            let as_waker: Waker = fake_waker.into();
            let mut as_context = Context::from_waker(&as_waker);
            Pin::new(self).poll(&mut as_context)
        }
    }

    //Generally, we want our tests to run only one at a time
    pub static TEST_SEMAPHORE: Lazy<Mutex<()>> = Lazy::new(|| {
        Mutex::new(())
    });

}

#[test] fn toy_await_1() {
    println!("waiting on guard 1");
    let _guard = test::TEST_SEMAPHORE.lock();
    println!("got guard 1");
    let mut command = Command::new("sleep");
    command.arg("0.1");
    let child = command.spawn().unwrap();
    let mut future = ProcessFuture::new(child.id() as i32);
    let clock = std::time::Instant::now();
    while clock.elapsed().as_secs() < 1 && future.toy_await().is_pending() {

    }
    if Waitpid::shared().waiting_thread {
        panic!("Failed to shut down waitpid??");
    }
}

#[test] fn toy_await_2() {
    println!("waiting on guard 2");
    let _guard = test::TEST_SEMAPHORE.lock();
    println!("got guard 2");
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
    let mut poll_me = vec![f1,f2,f3,f4,f5];
    println!("toy_await_2 poll_me {:?}",poll_me);
    let clock = std::time::Instant::now();
    while clock.elapsed().as_secs() < 2 && poll_me.len() > 0 {
        poll_me = poll_me.into_iter().filter(|f| f.clone().toy_await() == Poll::Pending).collect();
    }


    if Waitpid::shared().waiting_thread {
        panic!("Failed to shut down waitpid??");
    }
    assert_eq!(poll_me.len(),0);
}