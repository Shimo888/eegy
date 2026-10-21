use crate::core::registry::Registry;
use crate::processors::gain::GainProcessor;
use crate::processors::source::SourceProcessor;

pub(crate) mod gain;
pub(crate) mod source;

pub(crate) fn register_processors(registry: &mut Registry){
    registry.register::<GainProcessor>();
    registry.register::<SourceProcessor>()
}