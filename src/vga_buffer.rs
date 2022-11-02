
use volatile::Volatile;
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]

pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15

}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);


impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8)  << 4 |(  foreground as u8))
    }
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode
}

const BUFFER_WIDTH: usize = 25;
const BUFFER_HEIGHT: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH];BUFFER_HEIGHT]
    //the compiler doesnt know we access the vga buffer located at 0xb8000
    //so the comiler optimizes and decides the writes are unnecessary
    //we use volatile writes so compilers doesnt optimizes
}


pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}


impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.new_line()
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH{
                    self.new_line();
                }
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;
                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar{
                    ascii_character: byte,
                    color_code
                });
                self.column_position += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' => {
                    self.write_byte(byte)
                }
                _ => self.write_byte(0xfe)
            }
        }
    }

    //we iterate over each char and move each char one row up
    fn new_line(&mut self ) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row-1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT-1);
        self.column_position = 0;

    }

    //method clears a row by overwriting its char's with space 
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }



    

}

//to print different types we can support rust's formatting macros such as write! and writeln!
//to support them we need core::fmt::Write
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

//We provide a global witer that can be used as a interface
//statics are initialized at compile time
//we cant convert raw pointer to references at compile time
//se we use lazy_static which initializes itself at runtime
//it will be useless since its immutable
//we can use mutable static with spinlock to prevent race conditions
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new( Writer { 
        column_position: 0, 
        color_code: ColorCode::new(Color::Yellow,Color::Black),
        buffer: unsafe {
            &mut *(0xb8000 as *mut Buffer)
        } 
    });
}


#[macro_export]
macro_rules! print {
    //tt tokentree matches anything.  
    ($($x: tt)*) => (
        $crate::vga_buffer::_print(format_args!($($x)*))
    );
}

#[macro_export]
macro_rules! println {
    //tt tokentree matches anything.
    () => ($crate::print!("\n"));  
    ($($x: tt)*) => (
        $crate::print!("{}\n",format_args!($($x)*))
    );
}


#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}