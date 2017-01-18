
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
			// `info.size()` is always greater than 0, so this can't overflow
			end: Frame::containing(info.start() + info.size() - 1),
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

	/// Returns true if this region contains another frame.
	fn contains(&self, frame: Frame) -> bool {
		frame >= self.start && frame <= self.end
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
	current_area: Option<&'static MemoryArea>,

	/// A list of all invalid memory areas which we can't use to allocate frames
	/// since they contain important information (eg. the code for the kernel
	/// and the multiboot information struct).
	invalid_regions: [Region; 2],
}

impl BumpAllocator {
	/// Create a new bump allocator using information contained in the multiboot
	/// information struct.
	pub fn new(info: &MultibootInfo) -> BumpAllocator {
		let mut allocator = BumpAllocator {
			// Start allocating frames at physical address 0x0
			next_free_frame: Frame::containing(0),
			memory_areas: info.memory_areas(),
			current_area: None,

			// The only two invalid regions of memory so far are the kernel's
			// code and the multiboot struct. All other invalid regions are
			// described by the `MemoryArea` iterator above
			invalid_regions: [
				Region::from_multiboot_info(&info),
				Region::from_kernel_sections(info.sections()),
			],
		};

		// Manually determine the first memory area to use
		allocator.advance_memory_area();
		allocator
	}

	/// Advances the current memory area that we're allocating frames within.
	fn advance_memory_area(&mut self) {
		// Find the next memory area that starts after `self.next_free_frame`
		//
		// We can't just keep consuming the iterator in order, since the list
		// of areas isn't necessarily guaranteed to be sorted by start address
		self.current_area = self.memory_areas.clone().filter(|area| {
			let end_address = area.start() + area.size() - 1;
			Frame::containing(end_address) >= self.next_free_frame
		}).min_by_key(|area| area.start());

		// If we successfully found another memory area to use, then we need
		// to update the next free frame to point to the start of the memory
		// area
		if let Some(area) = self.current_area {
			// Get the first frame of the memory area
			let start_frame = Frame::containing(area.start());

			// The next free frame can't be outside the current memory area, so
			// if it's currently below the start of the area, update it
			if self.next_free_frame < start_frame {
				self.next_free_frame = start_frame;
			}
		}
	}

	/// Checks whether the next free frame is within an invalid area. If it is,
	/// then it is set to the first frame after the invalid area and `true` is
	/// returned.
	fn within_invalid_region(&mut self) -> bool {
		// Iterate over all invalid regions
		for region in self.invalid_regions.iter() {
			// Check if the region contains the next free frame
			if region.contains(self.next_free_frame) {
				// Use the frame after the end of the invalid region
				self.next_free_frame = Frame {
					id: region.end.id + 1,
				};
				return true;
			}
		}

		// If we reached here, then the current frame wasn't in any of the
		// invalid regions
		false
	}
}

impl FrameAllocator for BumpAllocator {
	fn allocate(&mut self) -> Option<Frame> {
		// Check if we've got a free memory area
		if let Some(current_area) = self.current_area {
			// Get the last frame in the current memory area
			let area_end = current_area.start() + current_area.size() - 1;
			let area_limit = Frame::containing(area_end);

			// Check if the frame is beyond it
			if self.next_free_frame > area_limit {
				// All frames inside this area have been used, so use the next
				// memory area
				self.advance_memory_area();
				self.allocate()
			} else if self.within_invalid_region() {
				// The `next_free_frame` has been adjusted to point to the frame
				// after the invalid region it was in, so just allocate another
				// frame
				self.allocate()
			} else {
				// If we reach here, the current frame is valid, so use it
				let frame = self.next_free_frame;
				self.next_free_frame.id += 1;
				Some(frame)
			}
		} else {
			// We've run out of memory
			None
		}
	}

	fn deallocate(&mut self, _: Frame) {
		unimplemented!();
	}
}
