//! A connection to a herdr socket on Windows: a named pipe. A read asks a thread of its own for one chunk and waits for it at
//! most the read timeout. The thread reads only while a read is waiting for a chunk: a synchronous pipe queues every I/O on
//! its handle, so an idle reader would hold up the writes. Dropping the stream cancels a read still in progress, so a
//! silent server leaves no thread and no open pipe behind. A pipe that ends, or fails, reads as end of file.
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::Duration;
use windows_sys::Win32::Foundation::{CloseHandle, DuplicateHandle, DUPLICATE_SAME_ACCESS, HANDLE};
use windows_sys::Win32::System::IO::CancelSynchronousIo;
use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetCurrentThread};

/// A connected named pipe: written directly, read through the reader thread (see above).
pub struct LocalStream {
    file: File,
    /// The reader's own handle to the pipe, given to the thread at its first read.
    reader: Option<File>,
    /// One message per chunk a read waits for; dropping it ends the thread once its read returns.
    asks: Option<Sender<()>>,
    chunks: Option<Receiver<Vec<u8>>>,
    /// The reader thread's handle (a duplicate of its thread handle), to cancel a read that is in progress.
    thread: Option<usize>,
    /// A chunk was asked for and has not come yet (a read timed out and the next one waits for the same chunk).
    waiting: bool,
    ended: bool,
    pending: Vec<u8>,
    timeout: Option<Duration>,
}

impl LocalStream {
    /// Takes a connected pipe. The reader's own handle is a duplicate of it, made now so that a failure shows at once.
    pub fn connect(file: File) -> io::Result<Self> {
        let reader = file.try_clone()?;
        Ok(Self { file, reader: Some(reader), asks: None, chunks: None, thread: None, waiting: false, ended: false, pending: Vec::new(), timeout: None })
    }
    /// How long a read waits for the server; None: as long as it takes.
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.timeout = timeout;
        Ok(())
    }
    /// Starts the reader thread at the first read, and waits until it has its own thread handle.
    fn start(&mut self) {
        if self.asks.is_some() {
            return;
        }
        let reader = self.reader.take().expect("the reader is taken once");
        let (ask_tx, ask_rx) = mpsc::channel::<()>();
        let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<u8>>();
        let (me_tx, me_rx) = mpsc::channel::<usize>();
        std::thread::spawn(move || {
            let mut me: HANDLE = 0 as HANDLE;
            // SAFETY: duplicates this thread's own handle into a handle another thread can cancel with.
            let ok = unsafe { DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &mut me, 0, 0, DUPLICATE_SAME_ACCESS) };
            let _ = me_tx.send(if ok != 0 { me as usize } else { 0 });
            let mut reader = reader;
            while ask_rx.recv().is_ok() {
                let mut buffer = [0u8; 4096];
                // A failed read is the end of the pipe, as is a cancelled one: an empty chunk.
                let chunk = match reader.read(&mut buffer) {
                    Ok(n) => buffer[..n].to_vec(),
                    Err(_) => Vec::new(),
                };
                let last = chunk.is_empty();
                if chunk_tx.send(chunk).is_err() || last {
                    break;
                }
            }
        });
        self.thread = me_rx.recv().ok().filter(|h| *h != 0);
        self.asks = Some(ask_tx);
        self.chunks = Some(chunk_rx);
    }
}

impl Read for LocalStream {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        // An empty buffer asks for nothing: no request is sent, and nothing is waited for (Read's contract).
        if out.is_empty() {
            return Ok(0);
        }
        if self.pending.is_empty() {
            if self.ended {
                return Ok(0);
            }
            self.start();
            if !self.waiting {
                let asked = self.asks.as_ref().is_some_and(|asks| asks.send(()).is_ok());
                if !asked {
                    self.ended = true;
                    return Ok(0);
                }
                self.waiting = true;
            }
            let rx = self.chunks.as_ref().expect("started above");
            let got = match self.timeout {
                Some(t) => rx.recv_timeout(t),
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match got {
                Ok(chunk) => {
                    self.waiting = false;
                    if chunk.is_empty() {
                        self.ended = true;
                        return Ok(0);
                    }
                    self.pending = chunk;
                }
                Err(RecvTimeoutError::Timeout) => {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "named pipe read timed out"));
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.ended = true;
                    return Ok(0);
                }
            }
        }
        let n = out.len().min(self.pending.len());
        out[..n].copy_from_slice(&self.pending[..n]);
        self.pending.drain(..n);
        Ok(n)
    }
}

impl Drop for LocalStream {
    fn drop(&mut self) {
        // A read still in progress is cancelled, so its thread ends now instead of waiting for the server.
        if let Some(h) = self.thread.take() {
            if self.waiting {
                // SAFETY: the handle is the reader thread's own, duplicated for this purpose and closed below.
                unsafe { CancelSynchronousIo(h as HANDLE) };
            }
            // SAFETY: closes the duplicate made in `start`; nothing uses it after this.
            unsafe { CloseHandle(h as HANDLE) };
        }
        self.asks = None;
    }
}

impl Write for LocalStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}
