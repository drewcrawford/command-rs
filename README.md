command-rs is an experimental async API for [std::process::Command](https://doc.rust-lang.org/std/process/struct.Command.html).  This crate uses the [Kiruna](https://github.com/drewcrawford/kiruna) async executor,
and can be considered the implementation of the [Kiruna manifesto](https://github.com/drewcrawford/kiruna) in the context of processes.

A quick comparison of alternative crates:

* [async-process](https://docs.rs/async-process/1.2.0/async_process/)
    * `async-process` uses a blocking IO on windows, whereas this crate uses `kiruna::stream` for IO, which uses OS-native async APIs, including on Windows.
    * `async-process` uses SIGCHLD on Unix, but [signals are very hard](https://man7.org/linux/man-pages/man7/signal-safety.7.html).  In theory, there
      are very complicated crates that solve signaly problems, however, I prefer to avoid them to begin with where possible, and it's possible here.
* [tokio::process](https://docs.rs/tokio/1.5.0/tokio/process/index.html) uses tokio.  A more complete discussion of 'why not tokio' is in the [Kiruna manifesto](https://github.com/drewcrawford/kiruna).
* [async-std](https://github.com/async-rs/async-std/issues/22) is unimplemented on this topic, but may get an implementation someday.

# Practicalities

* macOS works, Windows is 'passing tests', Linux is planned but not implemented
* The crate has two optional features: `output` (which supports redirecting stdout) and `sudo` (which can run commands with elevated privileges).  This makes
  the crate useful for system administration and scripting situations.
* Free for noncommercial and 'small commercial' use.

# Process context

This crate generally implements the [Kiruna manifesto](https://github.com/drewcrawford/kiruna) for processes, (processes are out of scope
for mainline Kiruna).  This includes:
* processes have an associated priority, as to avoid interrupting an interactive task with some background one
    * In some cases priorities are not yet used, so there may not be any effect, but these should get filled in over time.
* command-rs generally consists of separate high-level functions that may be specific to a set of functionalities and one platform.  That is, launching a process
  and redirecting output is a different function than if not redirecting its output, and may be different on macos than windows, etc.

## Waiting on processes

A design note about waiting on processes.  Arguably, the 'kiruna way' to do it is an OS-specific API like kevent.  And I reserve
the right to adopt that implementation in the future if it turns out there is some benefit to it.

However, I tried it, and launching a process is in general a heavy operation, the overhead of everything else is negligible so you might as well
do something portable.  On supported platforms, `waitpid` is currently used.