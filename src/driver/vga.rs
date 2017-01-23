
//
//  VGA Graphics Driver
//

use volatile::Volatile;
use spin::Mutex;

use core::fmt;
use core::ptr::Unique;

/// The width of the terminal window, in cells.
const TERM_WIDTH: usize = 80;

/// The height of the terminal window, in cells.
const TERM_HEIGHT: usize = 25;

/// The static Writer used to output characters to the terminal.
pub static WRITER: Mutex<Writer> = Mutex::new(Writer::vga());

/// All possible foreground and background colors we can use.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Color {
	Black      = 0,
	Blue       = 1,
	Green      = 2,
	Cyan       = 3,
	Red        = 4,
	Magenta    = 5,
	Brown      = 6,
	LightGray  = 7,
	DarkGray   = 8,
	LightBlue  = 9,
	LightGreen = 10,
	LightCyan  = 11,
	LightRed   = 12,
	Pink       = 13,
	Yellow     = 14,
	White      = 15,
}

/// Stores a combined foreground and background color for a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CombinedColor(u8);

impl CombinedColor {
	/// Create a new combined cell color from a lone foreground and background
	/// color.
	const fn new(foreground: Color, background: Color) -> CombinedColor {
		CombinedColor((background as u8) << 4 | (foreground as u8))
	}
}

/// Stores a cell's foreground color, background color, and ASCII character.
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
struct Cell {
	character: u8,
	color: CombinedColor,
}

/// Stores all cells on a terminal window.
struct Buffer {
	cells: [[Volatile<Cell>; TERM_WIDTH]; TERM_HEIGHT],
}

/// Stores all information associated with the cursor while writing to the
/// terminal.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Cursor {
	x: usize,
	y: usize,
	color: CombinedColor,
}

/// Writes text to the screen in a terminal-style fashion, moving the contents
/// of the screen up when we reach the end of the terminal.
pub struct Writer {
	cursor: Cursor,

	/// A `Unique` is a wrapper around a raw mutable pointer which indicates
	/// that we own the pointer.
	buffer: Unique<Buffer>,
}

impl Writer {
	/// Create a new writer for the kernel's VGA buffer.
	const fn vga() -> Writer {
		Writer {
			cursor: Cursor {
				x: 0,
				y: 0,
				color: CombinedColor::new(Color::White, Color::Black),
			},
			buffer: unsafe { Unique::new(0xb8000 as *mut _) },
		}
	}

	/// Returns a safe, mutable pointer to the writer's buffer.
	fn buffer(&mut self) -> &mut Buffer {
		// It's safe to use the unsafe call here because it's an invariant of
		// the `Buffer` struct that it always points to a valid memory location
		unsafe { self.buffer.get_mut() }
	}

	/// Clears a single row, replacing each character in the row with spaces,
	/// using the cursor's current foreground and background colors.
	pub fn clear_row(&mut self, y: usize) {
		// Iterate over each cell in the row
		for x in 0 .. TERM_WIDTH {
			// Set the cell at (x, y)
			let color = self.cursor.color;
			self.buffer().cells[y][x].write(Cell {
				character: b' ',
				color: color,
			});
		}
	}

	/// Clear the entire terminal to the cursor's current background color.
	pub fn clear_screen(&mut self) {
		// Iterate over each row
		for y in 0 .. TERM_HEIGHT {
			// Clear this row
			self.clear_row(y);
		}
	}

	/// Sets the cursor's position.
	pub fn set_cursor(&mut self, x: usize, y: usize) {
		self.cursor.x = x;
		self.cursor.y = y;
	}

	/// Sets the character of the cell under the cursor to the given character,
	/// sets its foreground and background color to the cursor's current color,
	/// and advances the cursor one cell right.
	fn write_byte(&mut self, character: u8) {
		// If there's a `\n`, or the cursor is on the last cell of the line,
		// then move the cursor to the next line
		if character == b'\n' || self.cursor.x >= TERM_WIDTH - 1 {
			self.newline();
			return;
		}

		// Set the cursor's current cell
		// Use a volatile write so that the compiler doesn't optimise out our
		// write to the buffer
		let cursor = self.cursor;
		self.buffer().cells[cursor.y][cursor.x].write(Cell {
			character: character,
			color: cursor.color,
		});

		// Move the cursor right by 1. We don't need to check if the cursor is
		// at the end of a column because we've already done that with the
		// opening `if` condition in this function
		self.cursor.x += 1;
	}

	/// Scroll the contents of the screen up by a certain amount.
	///
	/// Extra lines are created using the cursor's current color configuration,
	/// using a space as the character for each empty cell.
	///
	/// The terminal's cursor is moved up with the rest of the screen, leaving
	/// it in the same location relative to the text around it.
	fn scroll_up(&mut self, amount: usize) {
		// Iterate over every row that will still exist when the terminal
		// screen has been scrolled
		for y in amount .. TERM_HEIGHT {
			// Iterate over every character in the row
			for x in 0 .. TERM_WIDTH {
				// Replace the character `amount` rows up with this character
				let buffer = self.buffer();
				let character = buffer.cells[y][x].read();
				buffer.cells[y - amount][x].write(character);
			}
		}

		// Clear each empty row at the bottom of the screen
		for y in (TERM_HEIGHT - amount) .. TERM_HEIGHT {
			self.clear_row(y);
		}

		// Move the cursor up by `amount` so that it stays in the same location
		// relative to the text around it
		self.cursor.y -= amount;
	}

	/// Advances the cursor to the next line, and moves it to the start of this
	/// next line. If the cursor is at the bottom of the screen, then shifts
	/// all existing lines up by 1.
	fn newline(&mut self) {
		// Check if the cursor is on the last line of the terminal, in which
		// case we need to scroll the contents of the terminal up by 1
		if self.cursor.y >= TERM_HEIGHT - 1 {
			self.scroll_up(1);
		}

		// Move the cursor to the start of the next line
		self.cursor.y += 1;
		self.cursor.x = 0;
	}
}

impl fmt::Write for Writer {
	fn write_str(&mut self, string: &str) -> fmt::Result {
		for byte in string.bytes() {
			self.write_byte(byte);
		}

		// Writing using VGA can't really generate any errors, so always return
		// OK here
		Ok(())
	}
}


/// Initialise the VGA module.
///
/// Clears the screen and moves the cursor to the origin.
pub fn init() {
	// Clear the screen and set the cursor position to the origin, since the
	// bootloader would've printed a bunch of messages before us
	let mut writer = WRITER.lock();
	writer.clear_screen();
	writer.set_cursor(0, 0);
}


/// A macro to print a format string and arguments to the terminal.
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::driver::vga::print(format_args!($($arg)*));
    });
}

/// Prints a string to the terminal, appending a newline after it.
macro_rules! println {
    ($fmt:expr) => (print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => (print!(concat!($fmt, "\n"), $($arg)*));
}

/// Prints a series of format arguments to the terminal.
pub fn print(args: fmt::Arguments) {
	// This is required (instead of just inlining this in the `print!` macro) to
	// avoid a deadlock of the spin mutex around the VGA writer. Eg. in the code
	// `println!("something {}", { println!("else"); 3 })`, we'd call the
	// writer's `lock()` function twice, causing a deadlock. By moving the call
	// to the mutex's lock function into a separate function, we avoid this.
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
