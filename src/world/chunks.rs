use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, RwLock},
    time::SystemTime,
};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;

use crate::render::{Buffer, MAX_FRAMES_IN_FLIGHT};

use super::{chunk::Chunk, generator, meshing, ChunkPos};

#[derive(Debug)]
pub struct Chunks {
    data: HashMap<ChunkPos, Arc<Chunk>>,
    generator_sender: Sender<generator::Message>,
    meshing_sender: Sender<meshing::Message>,

    waiting_for_delete_buffers: WaitingForDeleteBuffers,
}

impl Chunks {
    pub fn new() -> Arc<RwLock<Self>> {
        let (generator_sender, generator_receiver) = generator::create_sender();
        let (meshing_sender, meshing_receiver) = meshing::create_sender();
        let chunks = Arc::new(RwLock::new(Self {
            data: HashMap::new(),
            generator_sender,
            meshing_sender,
            waiting_for_delete_buffers: Default::default(),
        }));

        generator::start_threads(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as u32,
            generator_receiver,
            &chunks,
        );
        meshing::start_threads(meshing_receiver, &chunks);

        chunks
    }

    #[inline]
    pub fn load(&mut self, pos: ChunkPos) -> Result<()> {
        if let Entry::Vacant(entry) = self.data.entry(pos) {
            let chunk = Chunk::new(pos);
            let arc = Arc::new(chunk);
            let weak = Arc::downgrade(&arc);
            entry.insert(arc);
            self.generator_sender
                .send(weak)
                .context("Sender disconnected")?;
        }
        Ok(())
    }

    #[inline]
    pub fn drain_filter<C>(&mut self, closure: C)
    where
        C: FnMut(&ChunkPos, &mut Arc<Chunk>) -> bool,
    {
        let drained = self.data.drain_filter(closure);
        self.waiting_for_delete_buffers.tick(
            drained.filter_map(|(_, chunk)| {
                chunk.vertex_buffer.lock().expect("Mutex poisoned").take()
            }),
        )
    }

    #[inline]
    pub fn get(&self, pos: &ChunkPos) -> Option<&Arc<Chunk>> {
        self.data.get(pos)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (&ChunkPos, &Arc<Chunk>)> {
        self.data.iter()
    }

    #[inline]
    pub fn chunk_generated(&self, chunk: &Arc<Chunk>) {
        self.meshing_sender
            .send(Arc::downgrade(chunk))
            .expect("Sender disconnected");
    }
}

impl Drop for Chunks {
    fn drop(&mut self) {
        generator::stop_threads(&self.generator_sender);
        meshing::stop_threads(&self.meshing_sender);
    }
}

#[derive(Debug, Default)]
struct WaitingForDeleteBuffers {
    buffers: [Vec<Buffer>; MAX_FRAMES_IN_FLIGHT],
    index: usize,
}

impl WaitingForDeleteBuffers {
    #[inline]
    fn tick<I: Iterator<Item = Buffer>>(&mut self, new_buffs: I) {
        self.buffers[self.index].clear();
        self.buffers[self.index].extend(new_buffs);
        self.index = (self.index + 1) % MAX_FRAMES_IN_FLIGHT;
    }
}
