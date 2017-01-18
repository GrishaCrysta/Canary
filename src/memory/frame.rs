
//
//  Physical Memory Management (Frames)
//

use multiboot::{MultibootInfo, EntryIterator, MemoryArea, Section};

/// The size of a single frame, in bytes. This is a physical constant of the
/// architecture.
const FRAME_SIZE: usize = 4096;

/// A section of size 4096 bytes of physical memory, called a Frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
	/// Each frame is identified by an ID number, which is simply its index in
	/// memory, starting at the very first frame (ie. its starting address
	/// divided by the frame size).
	id: usize,
}

impl Frame {
	/// Create a new frame that contains the given address.
	fn containing(address: usize) -> Frame {
		Frame {
			id: address / FRAME_SIZE,
		}
	}
}


/// A region of frame-aligned memory.
struct Region {
	start: Frame,
	end: Frame,
}

impl Region {
	/// Returns a region of memory that encases the multiboot information
	/// struct.
	fn from_multiboot_info(info: &MultibootInfo) -> Region {
		Region {
			start: Frame::containing(info.start()),
			end: Frame::containing(info.start() + info.size()),
		}
	}

	/// Returns a region of memory that encases the kernel's code using a list
	/// of ELF sections derived from the multiboot information struct.
	fn from_kernel_sections(sections: EntryIterator<Section>) -> Region {
		// Get the starting and ending memory address of the kernel's code
		let kernel_start = sections.clone().map(|s| s.start()).min().unwrap();
		let kernel_end = sections.map(|s| s.start() + s.size()).max().unwrap();

		// Create a region from the kernel code loaded into memory
		Region {
			start: Frame::containing(kernel_start),
			end: Frame::containing(kernel_end),
		}
	}
}


/// A trait implemented by all possible frame allocators, so that we can easily
/// interchange allocators later.
pub trait FrameAllocator {
	/// Allocates a new free frame for use, marking it as used. Returns None if
	/// there are no more free frames.
	fn allocate(&mut self) -> Option<Frame>;

	/// Deallocates a previously allocated frame. The allocator is guaranteed
	/// that the frame was previously allocated through a call to `allocate`.
	fn deallocate(&mut self, frame: Frame);
}

/// A simple "bump" frame allocator, which simply maintains an index to the
/// first available frame, incrementing it every time a new frame is allocated.
///
/// To deallocate a frame, it pushes the frame onto a "free frames" stack, which
/// is first checked before allocating a frame through incrementing the frame
/// counter.
pub struct BumpAllocator {
	/// The next free frame to return when `allocate` is called.
	next_free_frame: Frame,

	/// An iterator over all valid memory areas, determined from the multiboot
	/// information struct. These areas exclude any memory mapped devices such
	/// as VGA.
	memory_areas: EntryIterator<MemoryArea>,

	/// The current memory area which we've split into frames and are using to
	/// allocate memory.
	memory_area: Option<&'static MemoryArea>,

	/// A list of all invalid memory areas which we can't use to allocate frames
	/// since they contain important information (eg. the code for the kernel
	/// and the multiboot information struct).
	invalid_regions: [Region; 2],
}

impl BumpAllocator {
	/// Create a new bump allocator using information contained in the multiboot
	/// information struct.
	pub fn new(info: &MultibootInfo) -> BumpAllocator {
		let mut memory_areas = info.memory_areas();
		let first_area = memory_areas.next();

		BumpAllocator {
			next_free_frame: Frame::containing(0),
			memory_areas: memory_areas,
			memory_area: first_area,

			// The only two invalid regions of memory so far are the kernel's
			// code and the multiboot struct. All other invalid regions are
			// described by the `MemoryArea` iterator above
			invalid_regions: [
				Region::from_multiboot_info(&info),
				Region::from_kernel_sections(info.sections()),
			],
		}
	}
}
