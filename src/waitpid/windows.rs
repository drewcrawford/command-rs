use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};
use once_cell::sync::OnceCell;
use std::sync::{Mutex, Arc, MutexGuard};
use winbindings::Windows::Win32::Foundation::HANDLE;
use std::ops::{Deref, DerefMut};
use winbindings::Windows::Win32::System::Diagnostics::Debug::GetLastError;
use std::collections::{HashMap};
use winbindings::Windows::Win32::Foundation::CloseHandle;
use std::hash::{Hash};
use std::mem::MaybeUninit;
use std::collections::hash_map::Entry;

///Waits for the given pid
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub (crate) struct ProcessFuture(u32);


impl ProcessFuture {
    ///new future with pid
    pub fn new(pid: u32) -> ProcessFuture {
        ProcessFuture(pid)
    }
}

const BUFFER_SIZE: i32 = 512;

//wrap windows semaphore type, need this to merge our events with windows 'wait' fns
#[derive(Debug)]
struct WinSemaphore(HANDLE);
impl WinSemaphore {
    fn new() -> Self {
        use winbindings::Windows::Win32::System::Threading::CreateSemaphoreA;
        use winbindings::Windows::Win32::Foundation::PSTR;
        //need to pick some number that can buffer until the thread is up
        let handle = unsafe{ CreateSemaphoreA(std::ptr::null_mut(), 0, BUFFER_SIZE, PSTR(std::ptr::null_mut()))};
        assert!(handle.0 != 0);
        Self(handle)
    }
}
impl Drop for WinSemaphore {
    fn drop(&mut self) {
        let r = unsafe{ CloseHandle(self.0)};
        assert!(r.0 != 0);
    }
}

unsafe impl Sync for WinSemaphore {}

#[derive(Debug)]
enum PollState {
    //the given worker should be notified
    Notify(Waker),
    //got the return code specified
    Done(u32)
}

#[derive(Debug)]
struct WorkerInfo {
    ///We want to avoid using a channel to communicate about this, for a few reasons
    /// 1.  The channel can get backed up, if the runtime is poor and repeatedly polls.  If it's
    /// a bounded channel it may hang holding a lock, or if unbounded just wastes memory
    /// 2.  We don't want to update the child thread, just because the parent thread is noisy.
    /// Let the parent thread do its dumb thing and only bother child if there's new pids
    /// here, Notify means 'still waiting', Done(code) means 'returned with the code', wheraes 'no value'
    /// (None) means we're not waiting on this pid anymore, maybe you already polled and got its info?
    pids: HashMap<u32,PollState>,
    ///also need to signal the semaphore to wake up the worker, as it's in a windows wait fn
    win_semaphore: Arc<WinSemaphore>,
    thread_launched: bool,
}
///Holds a reference to the WorkerInfo struct, if the child was started
static WORKER: OnceCell<Mutex<Option<WorkerInfo>>> = OnceCell::new();

#[derive(PartialEq,Eq,Clone)]
struct ProcessHandle(HANDLE);
impl ProcessHandle {
    fn new(pid: u32) -> Self {
        use winbindings::Windows::Win32::System::Threading::{OpenProcess,PROCESS_SYNCHRONIZE,PROCESS_QUERY_LIMITED_INFORMATION};
        let options = PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION ;
        ProcessHandle(unsafe{ OpenProcess(options,false,pid )})
    }
}
impl Drop for ProcessHandle {
    fn drop(&mut self) {
        let r = unsafe{ CloseHandle(self.0)};
        assert!(r.0 != 0);
    }
}

#[derive(Debug)]
struct WorkerInfoLock(MutexGuard<'static, Option<WorkerInfo>>);
impl Deref for WorkerInfoLock {
    type Target = WorkerInfo;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().unwrap()
    }
}
impl DerefMut for WorkerInfoLock {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().unwrap()
    }
}
fn get_worker() -> WorkerInfoLock {
    let mut lock = WORKER.get_or_init(|| {
        Mutex::new(None)
    }
    ).lock().unwrap();
    if lock.is_none() {
        let win_semaphore = Arc::new(WinSemaphore::new());
        *lock = Some(WorkerInfo {
            pids: HashMap::new(),
            win_semaphore,
            thread_launched: false,
        });
    }
    WorkerInfoLock(lock)
}
fn launch_worker_if_needed(lock: WorkerInfoLock) {
    if !lock.thread_launched{
        //move into thread
        launch_worker(lock)
    }
}
fn launch_worker(mut lock: WorkerInfoLock) {
    let move_semaphore = lock.win_semaphore.clone();
    lock.thread_launched = true;
    let _handle = std::thread::spawn(move || {
        use winbindings::Windows::Win32::System::Threading::{WaitForMultipleObjects,WAIT_OBJECT_0};
        //the assumption here is that we might get polled a lot, so we want a fast way to look at a pid
        //and evaluate it as seen/not seen
        let mut handles = HashMap::<u32,ProcessHandle>::new();
        //back from raw handle to pid, this gives a fast way to drop the handle
        let mut unhandles = HashMap::new();
        loop {
            //build our object list
            let mut objects = Vec::with_capacity(handles.len() + 1);
            objects.push(move_semaphore.0);
            for handle in handles.values() {
                objects.push(handle.0);
            }
            let r = unsafe{ WaitForMultipleObjects(objects.len() as u32, (&objects).as_ptr(), false, u32::MAX)};
            //wait complete!
            if r == WAIT_OBJECT_0 {
                //wakeup from semaphore, look for new handles
                //safe because WE ARE THE WORKER, of course it was initialized
                let lock = unsafe{WORKER.get_unchecked()}.lock().unwrap();
                for (pid,pollstate) in &lock.as_ref().unwrap().pids {
                    match pollstate {
                        PollState::Notify(_) => {
                            let entry = handles.entry(*pid);
                            entry.or_insert_with(|| {
                                let handle = ProcessHandle::new(*pid);
                                //we use this as an identifier, but we don't need windows open/close semantics.
                                //Dont' want to use the pid, because we won't have that at lookup time
                                let raw_handle = handle.0.0;
                                unhandles.insert(raw_handle, *pid);
                                handle
                            });
                        }
                        PollState::Done(_) => {
                            //this case can be ignored for the purposes of creating new handles, we're just
                            //waiting on somebody to poll it.
                            //todo: This could potentially be separated into a different structure, not sure if worth it
                        }
                    }
                }
            }
            else if r.0 < WAIT_OBJECT_0.0 + objects.len() as u32 {
                //some other object, e.g. process exit
                let index = (r.0 - WAIT_OBJECT_0.0) as usize;
                let raw_handle = objects[index].0;
                let pid = unhandles[&raw_handle];

                let handle = handles.remove(&pid).unwrap();

                let mut return_code = MaybeUninit::uninit();
                use winbindings::Windows::Win32::System::Threading::GetExitCodeProcess;
                let r = unsafe{ GetExitCodeProcess(handle.0, return_code.assume_init_mut())};
                assert!(r.0 != 0);
                //we are the worker, right?  The worker was certainly intialized

                let mut lock = unsafe{ WORKER.get_unchecked()}.lock().unwrap();
                //and return code was def initialized by GetExitCodeProcess
                let return_code = unsafe { return_code.assume_init()};

                let entry = lock.as_mut().unwrap().pids.entry(pid);
                let mut occupied = match entry {
                    Entry::Occupied(o) => {o}
                    Entry::Vacant(_) => {unreachable!()}
                };
                let mut swapped = PollState::Done(return_code);
                std::mem::swap(&mut swapped, occupied.get_mut());
                let waker = match swapped {
                    PollState::Notify(waker) => {waker}
                    PollState::Done(_) => {unreachable!()}
                };

                waker.wake();

                if handles.len() == 0 {
                    //ok to shutdown thread
                    //mark thread as shutdown, so it will start up again if that's needed
                    lock.as_mut().unwrap().thread_launched = false;
                    return; //bye!
                }
            }
            else {
                let last_error = unsafe{ GetLastError()};
                panic!("Unexpected response from WaitForMultipleObjects {:?}, last error: {:?}",r,last_error);
            }

        }

    });
    // handle.join(); //todo: remove this
}

impl Future for ProcessFuture {
    type Output = u32; //on windows, we use u32 for return code

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut lock = get_worker();
        match lock.pids.entry(self.0) {
            Entry::Occupied(mut occupied) => {
                match occupied.get() {
                    PollState::Notify(_) => {
                        *occupied.get_mut() = PollState::Notify(cx.waker().clone());
                        launch_worker_if_needed(lock);
                        Poll::Pending

                    }
                    PollState::Done(code) => {
                        Poll::Ready(*code)
                    }
                }
            }
            Entry::Vacant(vacant) => {

                vacant.insert(PollState::Notify(cx.waker().clone()));
                use winbindings::Windows::Win32::System::Threading::ReleaseSemaphore;
                let r = unsafe{ ReleaseSemaphore(lock.win_semaphore.0, 1, std::ptr::null_mut())};

                assert!(r.0 != 0, "{:?}",unsafe{GetLastError()});
                launch_worker_if_needed(lock);
                Poll::Pending
            }
        }

    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;
    use crate::waitpid::ProcessFuture;
    use std::task::Poll;

    #[test] fn waidpid() {
        let child = Command::new("systeminfo").spawn().unwrap();
        let item = child.id();
        let future = ProcessFuture::new(item);
        kiruna::test::test_await(future, std::time::Duration::from_secs(10));

    }
    #[test] fn wait_multi() {
        let child1 = Command::new("powercfg.exe").spawn().unwrap();
        let item1 = child1.id();
        let future1 = ProcessFuture::new(item1);

        let child2 = Command::new("powercfg.exe").spawn().unwrap();
        let item2 = child2.id();
        let future2 = ProcessFuture::new(item2);

        let child3 = Command::new("powercfg.exe").spawn().unwrap();
        let item3 = child3.id();
        let future3 = ProcessFuture::new(item3);

        //todo: consider moving this multipoll implementation to kiruna::test
        let mut futures = [Some(future1),Some(future2),Some(future3)];
        let mut outputs = [None; 3];
        loop {
            let mut did_something = false;
            for i in 0..futures.len() {
                let future = match &mut futures[i] {
                    Some(v) => v,
                    None => {continue;}
                };
                did_something = true;
                let result = kiruna::test::test_poll(future);
                match result {
                    Poll::Ready(r) => {
                        futures[i] = None;
                        outputs[i] = Some(r);
                    }
                    Poll::Pending => {
                        //go around again
                    }
                }
            }
            if !did_something { break; }
        }
    }
}