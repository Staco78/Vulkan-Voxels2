use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{atomic::Ordering, Arc, RwLock},
    time::SystemTime,
};

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};

use crate::{
    gui,
    render::{Buffer, RegionsManager, MAX_FRAMES_IN_FLIGHT},
};

use super::{chunk::Chunk, generator, meshing, ChunkPos};

#[derive(Debug)]
pub struct Chunks {
    data: HashMap<ChunkPos, Arc<Chunk>>,
    generator_sender: Sender<generator::Message>,
    generator_receiver: Receiver<generator::Message>,
    meshing_sender: Sender<meshing::Message>,
    meshing_receiver: Receiver<meshing::Message>,

    waiting_for_delete_buffers: WaitingForDeleteBuffers,
}

impl Chunks {
    pub fn new() -> Arc<RwLock<Self>> {
        let (generator_sender, generator_receiver) = generator::create_sender();
        let (meshing_sender, meshing_receiver) = meshing::create_sender();
        Arc::new(RwLock::new(Self {
            data: HashMap::new(),
            generator_sender,
            generator_receiver,
            meshing_sender,
            meshing_receiver,
            waiting_for_delete_buffers: Default::default(),
        }))
    }

    pub fn init(s: &Arc<RwLock<Self>>, regions: &Arc<RegionsManager>) {
        let chunks = s.read().expect("Lock poisoned");

        let seed = if cfg!(feature = "bench") {
            0
        } else {
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs() as u32
        };
        generator::start_threads(seed, chunks.generator_receiver.clone(), s);
        meshing::start_threads(chunks.meshing_receiver.clone(), s, regions);
    }

    /// Return `true` if the chunk has been successfully loaded.
    #[inline]
    pub fn load(&mut self, pos: ChunkPos) -> Result<bool> {
        if let Entry::Vacant(entry) = self.data.entry(pos) {
            let chunk = Chunk::new(pos);
            let data = gui::DATA.read().expect("Lock poisoned");
            data.created_chunks_total.fetch_add(1, Ordering::Relaxed);
            data.created_chunks.fetch_add(1, Ordering::Relaxed);
            let arc = Arc::new(chunk);
            let weak = Arc::downgrade(&arc);
            entry.insert(arc);
            self.generator_sender
                .send(weak)
                .context("Sender disconnected")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    #[inline]
    pub fn drain_filter<C>(&mut self, closure: C, regions: &RegionsManager)
    where
        C: FnMut(&ChunkPos, &mut Arc<Chunk>) -> bool,
    {
        let drained = self.data.drain_filter(closure);
        self.waiting_for_delete_buffers
            .tick(drained.filter_map(|(_, chunk)| {
                regions
                    .set_dirty(chunk.pos.region())
                    .expect("Region should exists");
                chunk.vertex_buffer.lock().expect("Mutex poisoned").take()
            }));
    }

    pub fn update_gui_data(&self) {
        let data = gui::DATA.read().expect("Lock poisoned");
        data.waiting_for_generate_chunks
            .store(self.generator_sender.len(), Ordering::Relaxed);
        data.waiting_for_mesh_chunks
            .store(self.meshing_sender.len(), Ordering::Relaxed);
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

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
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
