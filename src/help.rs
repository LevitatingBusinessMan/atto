use crate::buffer::Buffer;

pub const HELP_BUFFER_CONTENT: &'static str = include_str!("./docs/help.txt");

pub fn help_buffer() -> Buffer {
    let mut buffer = Buffer::from_string(HELP_BUFFER_CONTENT.to_owned());
    buffer.set_readonly(true);
    buffer
}
