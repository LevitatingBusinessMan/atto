#[derive(Clone, Debug)]
pub struct Buffer {
    pub name: String,
    pub content: Vec<u8>,
}

impl Buffer {
    pub fn empty() -> Self {
        return Self {
            name: "Unknown".to_string(),
            content: vec![]
        }
    }
}