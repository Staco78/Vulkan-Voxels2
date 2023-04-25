use std::{
    collections::{hash_map::Entry, HashMap},
    sync::{Arc, RwLock},
};

use anyhow::{Context, Result};
use crossbeam_channel::Sender;

use super::{chunk::Chunk, generator, meshing, ChunkPos};

#[derive(Debug)]
pub struct Chunks {
    data: HashMap<ChunkPos, Arc<Chunk>>,
    generator_sender: Sender<generator::Message>,
    meshing_sender: Sender<meshing::Message>,
}

impl Chunks {
    pub fn new() -> Arc<RwLock<Self>> {
        let (generator_sender, receiver) = generator::create_sender();
        let meshing_sender = meshing::start_threads();
        let chunks = Arc::new(RwLock::new(Self {
            data: HashMap::new(),
            generator_sender,
            meshing_sender,
        }));

        generator::start_threads(receiver, &chunks);

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
