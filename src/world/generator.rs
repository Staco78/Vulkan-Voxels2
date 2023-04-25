use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
    thread::{self, JoinHandle},
};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};

use super::{blocks::BlockId, chunk::Chunk, chunks::Chunks, BLOCKS_PER_CHUNK};

pub const THREADS_COUNT: usize = 1;

pub type Message = Weak<Chunk>;

static EXIT: AtomicBool = AtomicBool::new(false);
static HANDLES: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

pub fn create_sender() -> (Sender<Message>, Receiver<Message>) {
    crossbeam_channel::unbounded()
}

pub fn start_threads(receiver: Receiver<Message>, chunks: &Arc<RwLock<Chunks>>) {
    let mut handles = HANDLES.lock().expect("Mutex poisoned");
    handles.reserve(THREADS_COUNT);
    for i in 0..THREADS_COUNT {
        let receiver = receiver.clone();
        let chunks = Arc::clone(chunks);
        let handle = thread::Builder::new()
            .name(format!("Meshing {}", i))
            .spawn(|| {
                #[allow(clippy::unwrap_used)]
                thread_main(receiver, chunks).unwrap()
            })
            .expect("Thread spawn failed");
        handles.push(handle);
    }
}

pub fn stop_threads(sender: &Sender<Message>) {
    EXIT.store(true, Ordering::Relaxed);
    let mut handles = HANDLES.lock().expect("Mutex poisoned");
    for _ in 0..handles.len() {
        let _ = sender.send(Weak::new());
    }
    for handle in handles.drain(..) {
        let _ = handle.join();
    }
}

fn thread_main(receiver: Receiver<Message>, chunks: Arc<RwLock<Chunks>>) -> Result<()> {
    while !EXIT.load(Ordering::Relaxed) {
        let chunk = receiver.recv().context("Channel disconnected")?;
        if let Some(chunk) = chunk.upgrade() {
            let mut blocks = [BlockId::Air; BLOCKS_PER_CHUNK];
            for (i, block) in blocks.iter_mut().enumerate() {
                if i % 4 == 0 {
                    *block = BlockId::Block;
                }
            }
            let mut blocks_lock = chunk.blocks.lock().expect("Mutex poisoned");
            debug_assert!(blocks_lock.is_none());
            *blocks_lock = Some(blocks);
            drop(blocks_lock);

            chunks
                .read()
                .expect("Lock poisoned")
                .chunk_generated(&chunk);
        }
    }

    Ok(())
}
