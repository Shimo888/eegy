use std::cell::RefCell;
use crate::core::buffer::Buffer;

pub struct BufferManager {
    pub buffers: Vec<RefCell<Buffer>>,
}

impl BufferManager{
    pub fn new() -> Self {
        Self{
            buffers: Vec::new(),
        }
    }
}