
//
//  Multiboot Information Struct Parsing
//

use core::ptr;

/// A simple byte reader, which maintains a cursor position within a piece of
/// memory, with utilities to advance the cursor and read integers of various
/// sizes.
struct ByteReader {
	cursor: *const u8,
}

impl ByteReader {
	/// Returns a new byte reader starting at the given location in memory.
	fn new(start: *const u8) -> ByteReader {
		ByteReader {
			cursor: start,
		}
	}

	/// Moves the cursor forward by a certain number of bytes.
	unsafe fn skip(&mut self, amount: usize) {
		self.cursor = self.cursor.offset(amount as isize);
	}

	/// Aligns the cursor to the next byte boundary of the given size. `align`
	/// must be a power of 2.
	///
	/// For example, if `align` is 8, then this moves the cursor forward such
	/// that it lies on the start of an 8 byte boundary.
	unsafe fn align(&mut self, align: usize) {
		let cursor = self.cursor as usize;
		let aligned = (cursor + align - 1) & !(align - 1);
		self.cursor = aligned as *const u8;
	}

	/// Reads a u8 value from memory and advances the cursor by 1 byte.
	unsafe fn read_u8(&mut self) -> u8 {
		let value = *self.cursor;
		self.skip(1);
		value
	}

	/// Reads a u32 value from memory, advancing the cursor by 4 bytes.
	///
	/// Assumes we're allowed to read the memory (ie. won't generate a page
	/// fault), and that the memory contains something valid and useful.
	unsafe fn read_u32(&mut self) -> u32 {
		// Since we're on x86, and all x86 platforms are little-endian, the
		// u32 value is represented in the multiboot structure as little-endian
		// (this is also stated in the multiboot specification)
		self.read_u8() as u32 | (self.read_u8() as u32) << 8 |
			(self.read_u8() as u32) << 16 | (self.read_u8() as u32) << 24
	}

	/// Reads a u64 value from memory, advancing the cursor by 8 bytes.
	unsafe fn read_u64(&mut self) -> u64 {
		// Use a loop and let the compiler unroll it during optimisation
		// I'm too lazy to write out all 8 or statements explicitly
		let mut result = 0;
		for i in 0 .. 8 {
			result |= (self.read_u8() as u64) << (i << 3);
		}
		result
	}
}

/// The multiboot information struct.
pub struct Multiboot {
	/// A pointer to the start of the multiboot structure.
	start: *const u8,

	// Pointers to the start of relevant tags.
	memory_map: *const u8,
}

impl Multiboot {
	/// Create a new multiboot information struct from a pointer to the start
	/// of one.
	pub fn new(start: *const u8) -> Multiboot {
		let mut info = Multiboot {
			start: start,
			memory_map: ptr::null(),
		};

		// As long as the given pointer is a pointer to a valid multiboot
		// information struct (an invariant of this function), then this parse
		// function is safe
		unsafe { info.parse() };
		info
	}

	/// Parse the start of relevant tags from a pointer to a multiboot
	/// information struct.
	unsafe fn parse(&mut self) {
		// Read the starting two fields of the struct
		let mut reader = ByteReader::new(self.start);
		reader.read_u32(); // total size
		reader.read_u32(); // reserved

		// Iterate over each tag
		loop {
			// Read the tag's type
			let cursor = reader.cursor;
			let kind = reader.read_u32();
			let size = reader.read_u32();

			// Skip over the tag, subtracting 8 for the 2 u32s we've already
			// read
			reader.skip(size as usize - 8);

			// Each tag is aligned on an 8 byte boundary, so align the cursor
			// for the next tag to be read
			reader.align(8);

			// Depending on the tag's type, set the relevant pointer
			match kind {
				6 => self.memory_map = cursor,
				// 9 => self.elf_symbols = cursor,

				// Stop when we've reached the end of all tags
				0 => break,
				_ => {},
			}
		}
	}

	/// Return an iterator over all valid memory areas.
	pub fn memory_areas(&self) -> MemoryAreas {
		MemoryAreas::new(ByteReader::new(self.memory_map))
	}
}

/// A valid memory area.
pub struct MemoryArea {
	pub base: u64,
	pub size: u64,
}

/// An iterator over all valid memory areas.
///
/// These areas exclude any memory mapped devices (such as VGA), but include
/// the loaded kernel and multiboot information struct.
pub struct MemoryAreas {
	reader: ByteReader,

	/// The size of each entry in the memory map, given in the memory map tag
	/// header, used for compatability with future multiboot versions.
	entry_size: u32,

	/// The number of entries in the memory map.
	entry_count: usize,

	/// The index of the current entry that we're up to.
	current_entry: usize,
}

impl MemoryAreas {
	/// Create a new memory area iterator using a byte reader that points to the
	/// start of the memory map tag in the multiboot information struct.
	fn new(mut reader: ByteReader) -> MemoryAreas {
		// Read the tag header
		let total_size; let entry_size;
		unsafe {
			reader.read_u32(); // type
			total_size = reader.read_u32(); // size
			entry_size = reader.read_u32(); // entry size
			reader.read_u32(); // entry version, always 0
		}

		MemoryAreas {
			reader: reader,
			entry_size: entry_size,
			entry_count: ((total_size - 16) / entry_size) as usize,
			current_entry: 0,
		}
	}
}

impl Iterator for MemoryAreas {
	type Item = MemoryArea;

	fn next(&mut self) -> Option<MemoryArea> {
		// Check if we've read all entries
		if self.current_entry >= self.entry_count {
			return None;
		}
		self.current_entry += 1;

		// Read the memory map entry
		let base; let size; let kind;
		unsafe {
			base = self.reader.read_u64(); // base address
			size = self.reader.read_u64(); // size
			kind = self.reader.read_u32(); // type
			self.reader.read_u32(); // reserved

			// Skip to the end of the entry
			self.reader.skip(self.entry_size as usize - 24);
		}

		// Only produce a valid memory area if `kind` is 1 (ie. the given area
		// of memory is available for us to use)
		if kind == 1 {
			Some(MemoryArea {
				base: base,
				size: size,
			})
		} else {
			// Read the next memory area
			self.next()
		}
	}
}
