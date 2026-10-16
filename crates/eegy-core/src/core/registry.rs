use std::collections::HashMap;
use serde::de::DeserializeOwned;
use crate::core::processor::Processor;

type NodeFactory = fn(serde_json::Value) -> Result<Box<dyn Processor>, String>; 

pub struct Registry { 
    factories: HashMap<String, NodeFactory>
}

impl Registry{
    pub fn new() -> Self{
        Self{
            factories: HashMap::new(),
        }
    }
    
    pub fn register<T>(&mut self, processor_type: &str) 
    where 
        T: Processor + DeserializeOwned + 'static,
    {
        let factory : NodeFactory = |value| {
            let processor: T = serde_json::from_value(value).map_err(|e| e.to_string())?;
            Ok(Box::new(processor))
        };
        
        self.factories.insert(processor_type.to_string(), factory);
    }
    
    pub fn create(&self, processor_type: &str, params: serde_json::Value) -> Result<Box<dyn Processor>, String> {
        let factory = self.factories.get(processor_type).ok_or(format!("Unknown processor {}", processor_type))?;
        factory(params)
    }
}