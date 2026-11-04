//! A connection to a herdr socket on Windows: a named pipe, read with a timeout. Reads come from a thread of their own,
//! started by the first read (after the request is written: a synchronous pipe queues a write behind a read that is
//! blocked), and each chunk reaches the reader on a channel, so a silent server times out instead of hanging. A pipe
//! that ends reads as end of file.
use std::fs::File;
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

/// A connected named pipe: written directly, read through a thread (see the comment above).
pub struct LocalStream {
    file: File,
    reader: Option<File>,
    received: Option<Receiver<Vec<u8>>>,
    pending: Vec<u8>,
    timeout: Option<Duration>,
}

impl LocalStream {
    /// Takes a connected pipe. The reader's own handle is a duplicate of it, made now so that a failure shows at once.
    pub fn connect(file: File) -> io::Result<Self> {
        let reader = file.try_clone()?;
        Ok(Self { file, reader: Some(reader), received: None, pending: Vec::new(), timeout: None })
    }
    /// How long a read waits for the server; None: as long as it takes.
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.timeout = timeout;
        Ok(())
    }
    /// Starts the reader thread at the first read.
    fn start(&mut self) {
        if self.received.is_some() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        let mut reader = self.reader.take().expect("the reader is taken once");
        std::thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        if tx.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    // The end of the pipe, or it failed: the channel ends, and a read sees end of file.
                    _ => break,
                }
            }
        });
        self.received = Some(rx);
    }
}

impl Read for LocalStream {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if self.pending.is_empty() {
            self.start();
            let rx = self.received.as_ref().expect("started above");
            let next = match self.timeout {
                Some(t) => rx.recv_timeout(t),
                None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match next {
                Ok(chunk) => self.pending = chunk,
                Err(RecvTimeoutError::Timeout) => {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "named pipe read timed out"));
                }
                Err(RecvTimeoutError::Disconnected) => return Ok(0),
            }
        }
        let n = out.len().min(self.pending.len());
        out[..n].copy_from_slice(&self.pending[..n]);
        self.pending.drain(..n);
        Ok(n)
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
