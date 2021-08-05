use std::os::unix::io::{IntoRawFd, RawFd};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use std::collections::{HashMap, HashSet};
use libc::{pipe,select,FD_ZERO,FD_SET,fd_set,FD_SETSIZE,FD_ISSET,read, write};
use std::sync::{MutexGuard, Mutex};
use once_cell::sync::Lazy;
use std::mem::MaybeUninit;

///A future that will return Ready when the underlying fd can be read
#[derive(Eq,Hash,PartialEq,Clone)]
struct ReadableFuture(RawFd);
impl ReadableFuture {
    fn from_raw_fd<T: IntoRawFd>(fd: T) -> ReadableFuture {
        ReadableFuture(fd.into_raw_fd())
    }
}

struct Selector {
    ///Holds futures we have awaited
    inbox: HashMap<ReadableFuture, Waker>,
    ///Holds futures we have woken
    outbox: HashSet<ReadableFuture>,
    pipe_write: i32,
    pipe_read: i32,
    thread_active: bool,
}
impl Selector {
    fn new() -> Selector {
        let mut pipes = [0,0];
        let pipe_result = unsafe{ pipe((&mut pipes) as *mut _)};
        assert_eq!(pipe_result, 0);
        Selector {
            inbox: HashMap::new(),
            outbox: HashSet::new(),
            pipe_read: pipes[0],
            pipe_write: pipes[1],
            thread_active: false
        }
    }
    fn shared() -> MutexGuard<'static, Selector> {
        static POLLER: Lazy<Mutex<Selector>> = Lazy::new(|| Mutex::new(Selector::new()));
        POLLER.lock().unwrap()
    }

    ///Writes to the internal pipe, causing select_one_time to re-select.
    fn reselect(&self) {
        //println!("reselect");
        let buf: u8 = 1;
        let result = unsafe{write(self.pipe_write, std::mem::transmute(&buf), 1)};
        assert_eq!(result,1)
    }

    fn poll(future: ReadableFuture, waker: Waker) -> Poll<()> {
        println!("poll");
        let mut lock = Selector::shared();
        if lock.outbox.remove(&future) {
            Poll::Ready(())
        }
        else {
            lock.inbox.insert(future,waker);
            //two cases here.
            if lock.thread_active {
                lock.reselect();
            }
            else {
                //we need to spin up a new thread
                lock.thread_active = true;
                std::thread::spawn(|| {
                    //println!("Thread begin");
                    while Selector::select_one_time().is_ok() {
                        //loop
                    }
                    //println!("Thread complete");

                });
            }
            Poll::Pending
        }
    }
    ///This function will do one round of `select()` on all file descriptors.
    ///
    /// This function is threadsafe and acquires its own locks.  This will return:
    /// 1.  When any of the waited fds are readable
    /// 2.  After reading a byte on the internal pipe.  This can force us to call this function again.
    ///
    /// The return values are
    /// * Ok(()): Call again in a loop
    /// * Err(()): We are done processing and can shut down the thread.  For correctness, thread_active is set to false before this
    /// function returns on an internal lock.  So you must use the return value to decide what to do here.
    fn select_one_time() -> Result<(),()> {
        //set up with info from lock
        let all_fds = {
            let lock =   Selector::shared();

            let mut all_fds = Vec::with_capacity(lock.inbox.len() + 1);
            all_fds.push(lock.pipe_read);
            for fd in lock.inbox.keys() {
                all_fds.push(fd.0);
            }
            all_fds
        };
        //drop lock
        //build fd_set
        //The
        //behavior of these macros is undefined if a descriptor value is less than zero or greater than or equal to
        //FD_SETSIZE
        assert!(all_fds.len() < FD_SETSIZE);

        let mut fd_set = MaybeUninit::<fd_set>::uninit();

        unsafe{ FD_ZERO(fd_set.as_mut_ptr()) }
        for item in all_fds.iter() {
            unsafe { FD_SET(*item, fd_set.as_mut_ptr())}
        }

        //println!("Will select on {:?}",all_fds);
        //for some reason select wants this ridiculous syntax
        let nfds = all_fds.iter().max().unwrap() + 1;
        let select_result = unsafe {
            select(nfds, fd_set.as_mut_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut())
        };
        assert!(select_result > 0);
        //println!("Did select on {:?}",all_fds.len());

        //get another lock
        let mut lock =   Selector::shared();

        //figure out which one(s) were returned
        let mut read_pipe = false;
        for (f,fd) in all_fds.iter().enumerate() {
            if unsafe{ FD_ISSET(*fd,fd_set.as_mut_ptr()) }{
                //item was selected
                if f == 0 {
                    //our pipe.  I guess in this case we want to read exactly 1 byte
                    let mut buf: u8 = 0;
                    let read_result = unsafe{ read(*fd,std::mem::transmute(&buf), 1)};
                    assert_eq!(read_result,1);
                    read_pipe = true;
                }
                else {
                    let future = ReadableFuture(*fd);
                    lock.outbox.insert(future.clone());
                    //wake this future
                    lock.inbox.remove(&future).unwrap().wake();
                }
            }
        }
        //if we removed everything
        if lock.inbox.len() == 0 {
            lock.thread_active = false;
            Err(())
        }
        else {
            Ok(())
        }
        //drop lock
    }
}

impl Future for ReadableFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        Selector::poll(self.clone(), cx.waker().clone())
    }
}

#[test] fn readable() {
    use crate::fake_waker::toy_await;

    use std::path::Path;
    use std::time::Duration;
    let test_path = Path::new("src/readable_future.rs");
    let file = std::fs::File::open(test_path).unwrap();
    let readable_future = ReadableFuture::from_raw_fd(file);
    let instant = std::time::Instant::now();

    while instant.elapsed() < Duration::from_secs(2) {
        println!("poll {:?}",instant.elapsed());
        if toy_await(readable_future.clone()).is_ready() { return }
    }
    panic!("Future never arrived");
}