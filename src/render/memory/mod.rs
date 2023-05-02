#[cfg(not(feature = "dumb_allocator"))]
mod allocator;
#[cfg(not(feature = "dumb_allocator"))]
pub use allocator::*;
#[cfg(feature = "dumb_allocator")]
mod dumb_allocator;
#[cfg(feature = "dumb_allocator")]
pub use dumb_allocator::*;

use anyhow::{anyhow, Result};
use vulkanalia::vk;

use std::sync::OnceLock;

static ALLOCATOR: OnceLock<Allocator> = OnceLock::new();

#[inline(always)]
pub fn allocator() -> &'static Allocator {
    ALLOCATOR.get().expect("Allocator not initialized")
}

#[inline(always)]
pub fn init_allocator(physical_device: vk::PhysicalDevice) {
    ALLOCATOR.get_or_init(|| Allocator::new(physical_device));
}

fn get_memory_type_index(
    memory: vk::PhysicalDeviceMemoryProperties,
    properties: vk::MemoryPropertyFlags,
    requirements: vk::MemoryRequirements,
) -> Result<u32> {
    (0..memory.memory_type_count)
        .find(|i| {
            let suitable = (requirements.memory_type_bits & (1 << i)) != 0;
            let memory_type = memory.memory_types[*i as usize];
            suitable && memory_type.property_flags.contains(properties)
        })
        .ok_or_else(|| anyhow!("Failed to find suitable memory type."))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use super::*;
    use anyhow::Result;

    const MEMS: &[vk::MemoryPropertyFlags] = &[
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
        vk::MemoryPropertyFlags::HOST_VISIBLE,
    ];
    const SIZES: &[usize] = &[10, 16, 20, 32, 64, 112, 511, 512, 1024];
    const ALIGNMENTS: &[usize] = &[1, 4, 8, 16, 32, 64, 128, 1024, 4096];

    fn test_alloc(
        size: usize,
        alignment: usize,
        properties: vk::MemoryPropertyFlags,
        mapped: bool,
    ) -> Result<Allocation> {
        let requirements = vk::MemoryRequirements {
            size: size as u64,
            alignment: alignment as u64,
            memory_type_bits: u32::MAX, // this should accept all memory types
        };

        let mut alloc = allocator().alloc(properties, requirements, mapped)?;

        assert_eq!(alloc.size(), size);
        assert_eq!(
            alloc.offset() % alignment,
            0,
            "Allocation alignment mismatch (alloc offset: {}, alignment: {}, size: {})",
            alloc.offset(),
            alignment,
            alloc.size()
        );

        if mapped {
            let data = alloc.data().unwrap();
            let ptr = data.as_ptr();
            assert_eq!(ptr as usize % alignment, 0);

            assert_eq!(data.len(), size);
        } else {
            assert!(alloc.data().is_none());
        }

        Ok(alloc)
    }

    #[test]
    fn simple_allocs() -> Result<()> {
        let mut allocations = Vec::new();

        for &mem in MEMS {
            for &size in SIZES {
                for &alignment in ALIGNMENTS {
                    let alloc = test_alloc(size, alignment, mem, false)?;
                    allocations.push(alloc);
                }
            }
        }

        Ok(())
    }

    #[test]
    fn mapped_allocs() -> Result<()> {
        let mut allocations = Vec::new();

        for &size in SIZES {
            for &alignment in ALIGNMENTS {
                let mut alloc =
                    test_alloc(size, alignment, vk::MemoryPropertyFlags::HOST_VISIBLE, true)?;
                let data = alloc.data().unwrap();

                let mut hasher = DefaultHasher::new();
                (size, alignment).hash(&mut hasher);
                let id = hasher.finish();

                for val in data {
                    *val = id as u8;
                }

                allocations.push((id, alloc));
            }
        }

        for (id, mut alloc) in allocations {
            let data = alloc.data().unwrap();
            for &mut val in data {
                assert_eq!(val, id as u8);
            }
        }

        Ok(())
    }
}
