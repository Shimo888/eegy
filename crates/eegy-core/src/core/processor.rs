use erased_serde::Serialize;
use serde::de::DeserializeOwned;
use crate::core::buffer::Buffer;

pub trait Processor: ProcessorMeta + ProcessorParams + Serialize {
    fn setup(&mut self, inputs: &[&Buffer], outputs: &mut[&mut Buffer]) -> Result<(), String>;
    fn process(&mut self, inputs: &[&Buffer], outputs: &mut[&mut Buffer]) -> Result<(), String>;
}

erased_serde::serialize_trait_object!(Processor);

pub trait ProcessorParams {
    fn set_params(&mut self, params: serde_json::Value) -> Result<(), String>;
}

impl<T: DeserializeOwned> ProcessorParams for T {
    fn set_params(&mut self, params: serde_json::Value) -> Result<(), String> {
        *self = serde_json::from_value(params).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub trait ProcessorMeta{
    fn get_type(&self) -> &'static str; fn get_input_ports(&self) -> &'static [IOPort]; 
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
    None,
    Time,
    Frequency,
    TimeFrequency,
}