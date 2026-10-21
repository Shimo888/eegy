use std::collections::BTreeMap;
use serde::de::DeserializeOwned;
use crate::core::processor::Processor;
use crate::core::registry;
use crate::processors::register_processors;

type NodeFactory = fn(serde_json::Value) -> Result<Box<dyn Processor>, String>;
type DefaultNodeFactory = fn() -> Box<dyn Processor>;

struct RegistryEntry {
    from_json: NodeFactory,
    create_default: DefaultNodeFactory,
}

pub struct Registry {
    entries: BTreeMap<String, RegistryEntry>,
}

impl Registry {
    pub fn new() -> Self {
        let mut registry =Self {
            entries: BTreeMap::new(),
        };
        register_processors(&mut registry);
        registry
    }

    pub fn register<T>(&mut self, processor_type: &str)
    where
        T: Processor + DeserializeOwned + Default + 'static,
    {
        let from_json: NodeFactory = |value| {
            let processor: T = serde_json::from_value(value).map_err(|e| e.to_string())?;
            Ok(Box::new(processor))
        };
        let create_default: DefaultNodeFactory = || Box::new(T::default());

        self.entries.insert(
            processor_type.to_string(),
            RegistryEntry {
                from_json,
                create_default,
            },
        );
    }

    pub fn create(&self, processor_type: &str, params: serde_json::Value) -> Result<Box<dyn Processor>, String> {
        let entry = self
            .entries
            .get(processor_type)
            .ok_or_else(|| format!("Unknown processor {}", processor_type))?;
        (entry.from_json)(params)
    }

    pub fn create_default(&self, processor_type: &str) -> Result<Box<dyn Processor>, String> {
        let entry = self
            .entries
            .get(processor_type)
            .ok_or_else(|| format!("Unknown processor {}", processor_type))?;
        Ok((entry.create_default)())
    }

    pub fn get_processor_types(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }
}