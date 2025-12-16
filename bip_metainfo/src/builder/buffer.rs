use std::collections::VecDeque;
use std::io;
use std::sync::{Condvar, Mutex};

// Ensures that we have enough buffers to keep workers busy.
const TOTAL_BUFFERS_MULTIPLICATIVE: usize = 2;
const TOTAL_BUFFERS_ADDITIVE: usize = 0;

/// Stores a set number of piece buffers to be used and re-used.
pub struct PieceBuffers {
    piece_queue: Mutex<VecDeque<PieceBuffer>>,
    queue_not_empty: Condvar,
}

impl PieceBuffers {
    /// Create a new queue filled with a number of piece buffers based on the number of workers.
    pub fn new(piece_length: usize, num_workers: usize) -> PieceBuffers {
        let mut piece_queue = VecDeque::new();

        let total_buffers = calculate_total_buffers(num_workers);
        for _ in 0..total_buffers {
            piece_queue.push_back(PieceBuffer::new(piece_length));
        }

        PieceBuffers {
            piece_queue: Mutex::new(piece_queue),
            queue_not_empty: Condvar::new(),
        }
    }

    /// Checkin the given piece buffer to be re-used.
    pub fn checkin(&self, mut buffer: PieceBuffer) {
        buffer.bytes_read = 0;

        let mut queue = self.piece_queue.lock().unwrap();
        queue.push_back(buffer);
        self.queue_not_empty.notify_one();
    }

    /// Checkout a piece buffer (possibly blocking) to be used.
    pub fn checkout(&self) -> PieceBuffer {
        let mut queue = self.piece_queue.lock().unwrap();
        loop {
            if let Some(buffer) = queue.pop_front() {
                return buffer;
            }

            queue = self.queue_not_empty.wait(queue).unwrap();
        }
    }
}

/// Calculates the optimal number of piece buffers given the number of workers.
fn calculate_total_buffers(num_workers: usize) -> usize {
    num_workers * TOTAL_BUFFERS_MULTIPLICATIVE + TOTAL_BUFFERS_ADDITIVE
}

// ----------------------------------------------------------------------------//

/// Piece buffer that can be filled up until it contains a full piece.
#[derive(PartialEq, Eq)]
pub struct PieceBuffer {
    buffer: Vec<u8>,
    bytes_read: usize,
}

impl PieceBuffer {
    /// Create a new piece buffer.
    fn new(piece_length: usize) -> PieceBuffer {
        PieceBuffer {
            buffer: vec![0u8; piece_length],
            bytes_read: 0,
        }
    }

    pub fn write_bytes<C>(&mut self, mut callback: C) -> io::Result<usize>
    where
        C: FnMut(&mut [u8]) -> io::Result<usize>,
    {
        let new_bytes_read = callback(&mut self.buffer[self.bytes_read..])?;
        self.bytes_read += new_bytes_read;

        Ok(new_bytes_read)
    }

    /// Whether or not the given piece buffer is full.
    pub fn is_whole(&self) -> bool {
        self.bytes_read == self.buffer.len()
    }

    /// Whether or not the given piece buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.bytes_read == 0
    }

    /// Access the piece buffer as a byte slice.
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.bytes_read]
    }
}
