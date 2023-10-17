#[no_mangle]
static mut TEXT_BUFFER: [u8; 2048] = [0; 2048];
static mut TEXT_SIZE: u32 = 0;

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::write(std::format_args!($($arg)*))
    };
}

#[no_mangle]
fn erase_text() {
    unsafe {
        TEXT_BUFFER = [0; 2048];
        TEXT_SIZE = 0;
    };
}

#[no_mangle]
fn get_text_size() -> i32 {
    unsafe { TEXT_SIZE as i32 }
}

pub fn write(args: std::fmt::Arguments) {
    let mut str_buf = String::new();
    let _ = std::fmt::write(&mut str_buf, args).unwrap();
    str_buf.push('\n');
    let starting_point = unsafe { TEXT_SIZE };
    let str_buf = str_buf.as_bytes().to_vec();

    for i in starting_point..starting_point + str_buf.len() as u32 {
        unsafe { TEXT_BUFFER[i as usize] = str_buf[(i - starting_point) as usize] };
    }
    unsafe { TEXT_SIZE += str_buf.len() as u32 }
}
