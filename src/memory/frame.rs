
//
//  Physical Memory Management (Frames)
//

use multiboot::MemoryAreas;

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
	memory_areas: MemoryAreas,
}
