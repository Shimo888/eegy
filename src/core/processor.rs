use crate::core::buffer::Buffer;

pub trait Processor : ProcessorMeta{
    fn setup(&mut self, inputs: &[&Buffer]) -> Result<(), String>;
    fn process(&mut self, inputs: &[&Buffer]) -> Result<(), String>;
    fn get_outputs(&self, port: usize) -> Option<&Buffer>;
}

pub trait ProcessorMeta{
    fn get_type(&self) -> &'static str;
    fn get_input_ports(&self) -> &'static [IOPort]; 
    fn get_output_ports(&self) -> &'static [IOPort];
}

#[derive(Clone,Debug,PartialEq)]
pub struct IOPort{
    pub name: &'static str,
    pub optional: bool, 
    pub types: &'static [IOType]
}

#[derive(Clone,Debug,PartialEq)]
pub enum IOType {
    Time,
    Frequency,
    TimeFrequency,
}