use std::{
    mem::{align_of, size_of},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, RwLock, Weak,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use log::warn;
use vulkanalia::vk::{self, DeviceV1_0, SuccessCode};

use crate::{
    render::{create_fence, Buffer, CommandPool, StagingBuffer, Vertex, DEVICE, QUEUES},
    utils::try_init_array,
};

use super::{chunk::Chunk, chunks::Chunks, MAX_VERTICES_PER_CHUNK};

pub const THREADS_COUNT: usize = 10;
const IN_FLIGHT_COPIES: usize = 4;
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
        let r = handle.join();
        if let Err(e) = r {
            warn!("Failed to join chunk: {:?}", e);
        }
    }
}

fn thread_main(receiver: Receiver<Message>, chunks: Arc<RwLock<Chunks>>) -> Result<()> {
    let fences: [vk::Fence; IN_FLIGHT_COPIES] = try_init_array(|| create_fence(true))?;
    let mut staging_buffs: [StagingBuffer; IN_FLIGHT_COPIES] = try_init_array(|| {
        StagingBuffer::new(
            MAX_VERTICES_PER_CHUNK * size_of::<Vertex>(),
            align_of::<Vertex>(),
        )
    })
    .context("Staging buffer creation failed")?;
    let queue = QUEUES.fetch_queue(vk::QueueFlags::TRANSFER)?;
    let command_pool = CommandPool::new(queue.family)?;
    let mut command_buffs = command_pool
        .alloc_buffers(IN_FLIGHT_COPIES)
        .context("Command buffers alloc failed")?;
    const NONE_INIT: Option<(Arc<Chunk>, Buffer)> = None;
    let mut in_copy_chunks: [Option<(Arc<Chunk>, Buffer)>; IN_FLIGHT_COPIES] =
        [NONE_INIT; IN_FLIGHT_COPIES];

    let mut buff_idx = 0;
    let mut current_copies_count = 0_usize;

    while !EXIT.load(Ordering::Relaxed) {
        let mess = if current_copies_count == 0 {
            receiver.recv().context("Channel disconnected")?
        } else {
            let r = receiver.recv_timeout(Duration::from_millis(100));
            match r {
                Ok(mess) => mess,
                Err(RecvTimeoutError::Timeout) => Weak::new(),
                e => e.context("Channel disconnected")?,
            }
        };

        let (fence, staging_buff, command_buff) = {
            let r = get_first_signaled_fence(&fences, buff_idx)?;
            let signaled_fence = match r {
                Some(index) => index,
                None => {
                    unsafe { DEVICE.wait_for_fences(&fences, false, u64::MAX) }
                        .context("Failed to wait for fences")?;
                    get_first_signaled_fence(&fences, buff_idx)?
                        .expect("At least one fence should be signaled")
                }
            };

            buff_idx = signaled_fence;
            if let Some((finished_copy_chunk, vertex_buffer)) = in_copy_chunks[buff_idx].take() {
                *finished_copy_chunk
                    .vertex_buffer
                    .lock()
                    .expect("Mutex poisoned") = Some(vertex_buffer);
                current_copies_count -= 1;
            }

            (
                fences[buff_idx],
                &mut staging_buffs[buff_idx],
                &mut command_buffs[buff_idx],
            )
        };

        if let Some(chunk) = mess.upgrade() {
            let vertices = unsafe { staging_buff.data::<Vertex>() };
            let vertices_count = chunk.mesh(&chunks, vertices);
            if vertices_count == 0 {
                continue;
            }
            let vertices_size = vertices_count * size_of::<Vertex>();

            let mut vertex_buff = Buffer::new(
                vertices_size,
                vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::DEVICE_LOCAL,
                false,
                align_of::<Vertex>(),
            )
            .context("Vertex buffer creation failed")?;

            unsafe { DEVICE.reset_fences(&[fences[buff_idx]]) }.context("Failed to reset fence")?;
            staging_buff
                .copy_into(*queue, command_buff, fence, &mut vertex_buff, vertices_size)
                .context("Buffer copy failed")?;

            in_copy_chunks[buff_idx] = Some((chunk, vertex_buff));

            current_copies_count += 1;
        }
        buff_idx = (buff_idx + 1) % IN_FLIGHT_COPIES;
    }

    Ok(())
}

/// Return the index of the first signaled fence (starting to check from `start_at` and looping through in `fences`) or `None` if no fence is signaled.
fn get_first_signaled_fence(fences: &[vk::Fence], start_at: usize) -> Result<Option<usize>> {
    let mut checked_count = 0;
    let mut i = start_at;
    while checked_count < fences.len() {
        let signaled = unsafe { DEVICE.get_fence_status(fences[i]) }
            .context("Failed to get fence status")?
            == SuccessCode::SUCCESS;
        if signaled {
            return Ok(Some(i));
        }
        checked_count += 1;
        i = (i + 1) % fences.len();
    }

    Ok(None)
}
