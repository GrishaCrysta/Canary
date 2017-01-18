
//
//  MultibootInfo Information Struct Parsing
//

use core::ptr;

/// The multiboot struct header.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Header {
	total_size: u32,
	reserved: u32,
	first_tag: TagHeader,
}

/// The header that precedes a tag within the multiboot struct.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct TagHeader {
	kind: u32,
	size: u32,
}

/// The multiboot information struct.
pub struct MultibootInfo {
	header: &'static Header,
	memory_map_tag: &'static MemoryMapTag,
	sections_tag: &'static SectionsTag,
}

impl MultibootInfo {
	/// Create a new multiboot information struct from a pointer to the start
	/// of one.
	pub unsafe fn new(start: *const Header) -> MultibootInfo {
		let header = &*start;

		// Iterate over each tag header, looking for the memory map and sections
		// tags
		let mut memory_map_tag = ptr::null();
		let mut sections_tag = ptr::null();
		let mut tag = &header.first_tag as *const TagHeader;

		// The last tag in the list has a type of 0, so stop parsing tags when
		// we reach it
		while (*tag).kind != 0 {
			// Check if we're interested in this tag
			match (*tag).kind {
				6 => memory_map_tag = tag as *const MemoryMapTag,
				9 => sections_tag = tag as *const SectionsTag,
				_ => {},
			}

			// Move to the next tag by skipping over the size of the tag in
			// bytes
			let next_ptr = tag as usize + (*tag).size as usize;

			// Each tag within the multiboot information struct is 8 byte
			// aligned (ie. starts on an 8 byte boundary), so align the `tag`
			// pointer to 8 bytes
			let alignment = 8;
			let aligned_ptr = (next_ptr + alignment - 1) & !(alignment - 1);

			// Update the tag pointer
			tag = aligned_ptr as *const TagHeader;
		}

		MultibootInfo {
			header: header,
			memory_map_tag: &*memory_map_tag,
			sections_tag: &*sections_tag,
		}
	}

	/// The address of the start of the multiboot structure.
	pub fn start(&self) -> usize {
		self.header as *const _ as usize
	}

	/// The address of the end of the multiboot structure.
	pub fn size(&self) -> usize {
		self.header.total_size as usize
	}

	/// Return an iterator over all valid memory areas.
	pub fn memory_areas(&self) -> EntryIterator<MemoryArea> {
		// Calculate a pointer to the last entry
		let tag_ptr = self.memory_map_tag as *const MemoryMapTag as usize;
		let tag_size = self.memory_map_tag.tag_size as usize;
		let entry_size = self.memory_map_tag.entry_size as usize;
		let last_entry = tag_ptr + tag_size - entry_size;

		EntryIterator {
			current_entry: &self.memory_map_tag.first_area,
			last_entry: last_entry as *const MemoryArea,
			entry_size: entry_size,
		}
	}

	/// Return an iterator over all ELF sections in the kernel executable.
	pub fn sections(&self) -> EntryIterator<Section> {
		// Calculate a pointer to the last entry
		let tag_ptr = self.sections_tag as *const SectionsTag as usize;
		let tag_size = self.sections_tag.tag_size as usize;
		let entry_size = self.sections_tag.entry_size as usize;
		let last_entry = tag_ptr + tag_size - entry_size;

		EntryIterator {
			current_entry: &self.sections_tag.first_section,
			last_entry: last_entry as *const Section,
			entry_size: entry_size,
		}
	}
}


/// An iterator over a series of entries in a list within the multiboot
/// information struct.
#[derive(Clone)]
pub struct EntryIterator<T: 'static> {
	current_entry: *const T,
	last_entry: *const T,
	entry_size: usize,
}

impl<T: 'static> Iterator for EntryIterator<T> {
	type Item = &'static T;

	fn next(&mut self) -> Option<&'static T> {
		// Check if we've gone past the last entry
		if self.current_entry > self.last_entry {
			return None;
		}

		// Save the current entry
		let entry = self.current_entry;

		// Advance the entry pointer to the next entry
		let next_ptr = self.current_entry as usize + self.entry_size;
		self.current_entry = next_ptr as *const T;

		// Return the current entry
		Some(unsafe { &*entry })
	}
}


/// The memory map tag within the multiboot struct.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MemoryMapTag {
	tag_type: u32,
	tag_size: u32,
	entry_size: u32,
	version: u32,
	first_area: MemoryArea,
}

/// A valid memory area.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct MemoryArea {
	// These fields match the size and type of each field in a memory map entry
	// as specified in the multiboot specification
	address: usize,
	length: usize,
	kind: u32,
	reserved: u32,
}

/// The type of a memory area.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MemoryAreaType {
	Usable,
	Unusable,
}

impl MemoryArea {
	/// Returns the address of the start of the memory area.
	pub fn start(&self) -> usize {
		self.address
	}

	/// Returns the size of the memory address in bytes.
	pub fn size(&self) -> usize {
		self.length
	}

	/// Returns the type of the memory area. At this stage, only a distinction
	/// bewteen usable and unusable memory areas is made.
	pub fn kind(&self) -> MemoryAreaType {
		if self.kind == 1 {
			MemoryAreaType::Usable
		} else {
			MemoryAreaType::Unusable
		}
	}
}


/// The tag containing all sections.
#[derive(Clone, Copy, Debug, PartialEq)]
// Using `repr(C)` would add unwanted padding before `first_section`
#[repr(packed)]
pub struct SectionsTag {
	tag_type: u32,
	tag_size: u32,
	sections_count: u32,
	entry_size: u32,
	string_table: u32,
	first_section: Section,
}

/// A section header within an ELF file.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub struct Section {
	name_index: u32,
	kind: u32,
	pub flags: usize,
	address: usize,
	offset: usize,
	size: usize,
	link: u32,
	info: u32,
	address_alignment: usize,
	entry_size: usize,
}

impl Section {
	/// Returns the physical start address of the code within the section in
	/// memory.
	pub fn start(&self) -> usize {
		self.address
	}

	/// Returns the size of the section in bytes.
	pub fn size(&self) -> usize {
		self.size
	}
}
