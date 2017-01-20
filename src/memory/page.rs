
//
//  Virtual Memory Management (Pages)
//

use memory::frame;
use memory::frame::{Frame, FrameAllocator, PhysicalAddr};

use core::ptr::Unique;
use core::marker::PhantomData;

/// The size of a single page. This should be the same size as the size of a
/// physical frame.
pub const PAGE_SIZE: usize = frame::FRAME_SIZE;

/// A virtual memory address. On the x86_64 architecture, only the lowest 48
/// bits are used - the remaining upper bits are copies of the highest bit in
/// use (ie. sign extended). Following this, four sets of 9 bits are used to
/// index each of the four page tables (since 2^9 = 512 entries). The final 12
/// bits are the offset into the final page itself (since 2^12 = 4096).
pub type VirtualAddr = usize;

/// A 4096 byte section of a process' virtual memory, called a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
	/// Each page is identified by 4 separate indices into each of the 4 page
	/// tables, combined into a single number. The lowest 9 bits is the index
	/// into the P1 table, the next 9 bits the index into the P2 table, and so
	/// on.
	id: usize,
}

impl Page {
	/// Create a new page that contains the given virtual address.
	fn containing(address: VirtualAddr) -> Page {
		Page {
			id: address / PAGE_SIZE,
		}
	}

	/// Returns the starting address of the page.
	pub fn start(&self) -> VirtualAddr {
		// The lowest 12 bits of a virtual address refer to the offset into the
		// referenced page. Leave them as 0 to specify the start of the page
		self.id << 12
	}

	/// Each virtual address has 4 page table indices, one for each page table,
	/// (each 9 bits) and one 12 bit offset into the page. This returns the
	/// page table index for the corresponding page table.
	///
	/// The `level` argument should probably be a constant (hence why this is
	/// inlined always, because the compiler should be able to compute the
	/// multiplication and combine the two bitshifts at compile time).
	#[inline(always)]
	pub fn page_table_index(&self, level: usize) -> usize {
		// First shift right to get rid of the offset into the page itself,
		// then shift further based on the level we're interested in
		((self.id) >> (level * 9)) & 0x1ff
	}
}


/// The present flag bit on a page table entry, set if the page is present in
/// memory.
const ENTRY_PRESENT: u64 = 1;

/// The huge flag bit on a page table entry, indicating if the referenced page
/// is "huge" (ie. 2 MB on a P2 entry, 1 GB on a P3 entry).
const ENTRY_HUGE: u64 = 1 << 7;

/// The writable flag bit on a page table entry, set if the page can be written
/// to.
const ENTRY_WRITABLE: u64 = 1 << 1;

/// An entry within a page table, which is 8 bytes long (u64).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry(u64);

impl Entry {
	/// Set the pointed to frame and flags for this page table entry.
	pub fn set(&mut self, frame: Frame, flags: u64) {
		self.0 = frame.start() as u64 | flags;
	}

	/// We define an unused page table entry as completely 0. There are a
	/// number of bits in an entry which are free to be used by the OS (bits
	/// 9 - 11 and 52 - 62).
	pub fn is_unused(&self) -> bool {
		self.0 == 0
	}

	/// Sets the entry to unused.
	pub fn set_unused(&mut self) {
		self.0 = 0;
	}

	/// Returns true if the page table entry is present in memory (ie. the
	/// present bit is set, bit 0).
	pub fn is_present(&self) -> bool {
		(self.0 & ENTRY_PRESENT) == ENTRY_PRESENT
	}

	/// Returns true if the huge page table flags is set (ie. the page is 2 MB
	/// big if the entry is in the P2 table, or 1 GB in the P3 table).
	pub fn is_huge(&self) -> bool {
		(self.0 & ENTRY_HUGE) == ENTRY_HUGE
	}

	/// Returns the physical frame that this page table entry points to, only if
	/// the page this entry points to is present in memory.
	pub fn pointed_frame(&self) -> Option<Frame> {
		// Check if the frame that this entry points to is actually present in
		// memory
		if self.is_present() {
			// Mask out the flag bits and extract just the physical address,
			// and then create a frame from this
			Some(Frame::containing(self.0 as PhysicalAddr & 0x000ffffffffff000))
		} else {
			None
		}
	}
}


/// We use Rust's type system to statically guarantee, when accessing a page
/// table entry, whether we're accessing another page table or a page itself.
///
/// All possible table levels implement the level trait.
pub trait Level {}

/// All tables which can be indexed to return a lower level table implement
/// heirarchical level.
pub trait HierarchicalLevel: Level {
	/// The next level below the current table level. For example, the level
	/// below a P4 table is P3.
	type Next: Level;
}

pub struct Level1;
pub struct Level2;
pub struct Level3;
pub struct Level4;
impl Level for Level1 {}
impl Level for Level2 {}
impl Level for Level3 {}
impl Level for Level4 {}
impl HierarchicalLevel for Level2 { type Next = Level1; }
impl HierarchicalLevel for Level3 { type Next = Level2; }
impl HierarchicalLevel for Level4 { type Next = Level3; }

/// The number of page table entries within a single page table.
///
/// A page table must fit within 1 page (4096 bytes), and each entry is an
/// unsigned 64 bit integer (8 bytes) which acts as a physical address to
/// another page. Thus there are 4096 / 8 = 512 entries per page table.
const ENTRY_COUNT: usize = PAGE_SIZE / 8;

/// A page table, consisting of 512 page table entries, each of which points to
/// the physical address of another page.
pub struct Table<L: Level> {
	entries: [Entry; ENTRY_COUNT],
	level: PhantomData<L>,
}

impl<L: Level> Table<L> {
	/// Sets every entry within the table to unused.
	pub fn set_all_unused(&mut self) {
		for entry in self.entries.iter_mut() {
			entry.set_unused();
		}
	}
}

impl<L: HierarchicalLevel> Table<L> {
	/// Returns a virtual address that can be used to access the page table
	/// referenced by the page table entry at `index` within this parent page
	/// table.
	///
	/// This function does not check if the referenced page table actually
	/// exists in memory, so dereferencing this virtual address may result in
	/// a page fault (since the PRESENT flag on the page table entry at `index`
	/// may not be set).
	fn index_addr_unchecked(&self, index: usize) -> VirtualAddr {
		// Convert the self pointer into an address, which will be the virtual
		// address of this page table
		let table_address = self as *const _ as VirtualAddr;

		// Bit shift the table address left by 9 gets rid of the lowest
		// recursive mapping in the virtual address, freeing up space to
		// include an actual index into the last page table
		//
		// Bit shift the next table's index in the current table by 12 so
		// we place it directly after the offset (which makes up the lowest
		// 12 bits in a virtual address)
		(table_address << 9) | (index << 12)
	}

	/// Returns a virtual address that can be used to access the page table
	/// referenced by the page table entry at `index` within this parent page
	/// table.
	fn index_addr(&self, index: usize) -> Option<VirtualAddr> {
		// We can only return the address of a page table entry if it actually
		// exists in memory (ie. the entry is mapped to a physical frame)
		let entry = self.entries[index];
		if entry.is_present() && !entry.is_huge() {
			// Get the virtual address used to modify the page table referenced
			// by `index` within this parent page table
			Some(self.index_addr_unchecked(index))
		} else {
			None
		}
	}

	/// Access a page table entry within a P2 table or higher, returning a
	/// pointer to another page table at a lower level (eg. indexing a P2 table
	/// returns a P1 table).
	pub fn index(&self, index: usize) -> Option<&Table<L::Next>> {
		self.index_addr(index).map(|addr| unsafe { &*(addr as *const _) })
	}

	/// Access a page table entry mutably (see `get` for more information).
	pub fn index_mut(&mut self, index: usize) -> Option<&mut Table<L::Next>> {
		self.index_addr(index).map(|addr| unsafe { &mut *(addr as *mut _) })
	}

	/// Access a page table entry within a P2 table or higher, and if the
	/// corresponding lower page table at this index doesn't yet exist in
	/// memory, allocate a new frame to store it in and zero it.
	pub fn create<A: FrameAllocator>(&mut self, index: usize, allocator: &mut A)
			-> &mut Table<L::Next> {
		// Check if the entry at the given index has already been mapped to a
		// physical address or not
		if let Some(addr) = self.index_addr(index) {
			// Already mapped to an address (which we assume is another lower
			// level page table), so convert the address to a pointer
			unsafe { &mut *(addr as *mut _) }
		} else {
			// Allocate a new frame to hold the new page table we're about to
			// create
			let frame = allocator.allocate().expect("out of memory");

			// Map the entry at the given index to the newly allocated page
			// table
			self.entries[index].set(frame, ENTRY_PRESENT | ENTRY_WRITABLE);

			// Get a pointer to the page table now at `index`
			//
			// We can use the unchecked version of this function since we know
			// the page table entry we're referencing is present in memory
			// (since we just created it in the previous line)
			let addr = self.index_addr_unchecked(index);
			let table = unsafe { &mut *(addr as *mut Table<L::Next>) };

			// Currently, the table contains a bunch of garbage from whatever
			// was using this frame previously, so zero the new page table
			table.set_all_unused();

			// Return the new page table
			table
		}
	}
}


/// The active page directory is the top level page table (P4 table) which is
/// currently being used by the OS
pub struct ActiveDirectory {
	p4: Unique<Table<Level4>>,
}

impl ActiveDirectory {
	/// Create an active directory struct from whatever page table is currently
	/// in use by the OS when this function is called.
	///
	/// We can do this by exploiting the recursive page table mapping we've set
	/// up.
	pub fn current() -> ActiveDirectory {
		ActiveDirectory {
			// We access the P4 table by accessing the recursive mapping in the
			// P4 table 4 times. The recursive mapping (ie. the mapping in the
			// P4 table which references the physical address of the P4 table
			// itself) is the last entry in the P4 table, so we set each of
			// the page table indices in the virtual address to 0x1ff (ie. all
			// bits set).
			p4: unsafe { Unique::new(0xfffffffffffff000 as *mut _) },
		}
	}

	/// Returns a pointer to the currently active page table.
	pub fn p4(&self) -> &Table<Level4> {
		unsafe { self.p4.get() }
	}

	/// Returns a mutable pointer to the currently active page table.
	pub fn p4_mut(&mut self) -> &mut Table<Level4> {
		unsafe { self.p4.get_mut() }
	}

	/// Maps a given page (ie. virtual address) to a physical frame with the
	/// given set of flags. The `PRESENT` flag is added by default.
	///
	/// The allocator is used to create the physical frame required to hold any
	/// new page tables that are needed for the mapping to be valid.
	pub fn map<A: FrameAllocator>(&self, page: Page, frame: Frame, flags: u64,
			allocator: &mut A) {

	}

	/// Maps a given page to a new, free physical frame using the given set of
	/// flags.
	pub fn map_to_any<A: FrameAllocator>(&self, page: Page, flags: u64,
			allocator: &mut A) {

	}

	/// Given a physical frame, this function maps the corresponding identity
	/// virtual page to the grame.
	pub fn identity_map<A: FrameAllocator>(&self, frame: Frame, flags: u64,
			allocator: &mut A) {

	}

	/// Removes the mapping between the given page and whatever physical frame
	/// it is mapped to. The allocator is used to free the underlying physical
	/// frame so it can be used again in the future.
	pub fn unmap<A: FrameAllocator>(&self, page: Page, allocator: &mut A) {

	}
}
