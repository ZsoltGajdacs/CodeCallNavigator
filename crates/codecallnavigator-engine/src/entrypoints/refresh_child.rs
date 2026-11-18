//! Bounded nonblocking pipes shared by the persistent analysis-session transport.
use anyhow::{Result, ensure};
use std::{
    collections::VecDeque,
    io::Read,
    os::fd::AsRawFd,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub(super) fn nonblocking(input: &impl AsRawFd) -> Result<()> {
    let descriptor = input.as_raw_fd();
    let flags = unsafe { libc::fcntl(descriptor, libc::F_GETFL) };
    ensure!(
        flags >= 0
            && unsafe { libc::fcntl(descriptor, libc::F_SETFL, flags | libc::O_NONBLOCK) } >= 0,
        "Cannot configure bounded analysis pipe"
    );
    Ok(())
}

// Readers must stop independently of EOF: an unrecorded inherited pipe holder
// must not leak a reader thread in a persistent observer after failed analysis.
#[derive(Default)]
pub(super) struct Readers {
    stopped: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
}
impl Readers {
    pub(super) fn pipe<T: Read>(&self, pipe: T) -> StoppablePipe<T> {
        StoppablePipe {
            pipe,
            stopped: self.stopped.clone(),
        }
    }
    pub(super) fn spawn(&mut self, run: impl FnOnce() + Send + 'static) {
        self.threads.push(std::thread::spawn(run));
    }
    pub(super) fn stop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
pub(super) struct StoppablePipe<T> {
    pipe: T,
    stopped: Arc<AtomicBool>,
}
impl<T: Read> Read for StoppablePipe<T> {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        while !self.stopped.load(Ordering::Acquire) {
            match self.pipe.read(bytes) {
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                result => return result,
            }
        }
        Ok(0)
    }
}
pub(super) fn drain_log(mut stderr: impl Read, log: Arc<Mutex<VecDeque<u8>>>) {
    let mut bytes = [0u8; 4096];
    while let Ok(count) = stderr.read(&mut bytes) {
        if count == 0 {
            break;
        }
        append_log(&log, &bytes[..count]);
    }
}

fn append_log(log: &Mutex<VecDeque<u8>>, bytes: &[u8]) {
    if let Ok(mut log) = log.lock() {
        log.extend(bytes);
        while log.len() > 8192 {
            log.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::{Child, Command, Stdio},
        sync::mpsc,
        time::Instant,
    };

    struct HeldPipe(Child);
    impl Drop for HeldPipe {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn reader_shutdown_does_not_require_pipe_holder_exit() {
        // A still-live holder has the same pipe behavior as an unrecorded orphan
        // inheriting an exited compiler's descriptors. No analyzer is simulated.
        let mut holder = HeldPipe(
            Command::new("/bin/sleep")
                .arg("30")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap(),
        );
        let stdout = holder.0.stdout.take().unwrap();
        let stderr = holder.0.stderr.take().unwrap();
        nonblocking(&stdout).unwrap();
        nonblocking(&stderr).unwrap();
        let mut readers = Readers::default();
        let stdout = readers.pipe(stdout);
        let stderr = readers.pipe(stderr);
        let (send, result) = mpsc::sync_channel(2);
        let first = send.clone();
        readers.spawn(move || {
            let mut bytes = Vec::new();
            let mut stdout = stdout;
            stdout.read_to_end(&mut bytes).unwrap();
            first.send(()).unwrap();
        });
        readers.spawn(move || {
            let mut bytes = Vec::new();
            let mut stderr = stderr;
            stderr.read_to_end(&mut bytes).unwrap();
            send.send(()).unwrap();
        });
        assert!(result.recv_timeout(Duration::from_millis(50)).is_err());
        let started = Instant::now();
        readers.stop();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(result.try_iter().count(), 2);
        assert!(holder.0.try_wait().unwrap().is_none());
    }
}
