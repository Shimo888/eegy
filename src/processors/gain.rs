use crate::core::processor::{IOPort, IOType, Processor, ProcessorMeta};
use crate::core::buffer::{Buffer};

pub struct GainProcessor {
    gain: f64,
}

impl GainProcessor {
    pub fn new(gain: f64) -> Self{
        Self{
            gain,
        }
    }
}

impl Processor for GainProcessor {
    fn setup(&mut self, inputs: &[Buffer], outputs: &mut[Buffer]) -> Result<(), String> {
        let input = inputs.get(0).ok_or("input 0 is required")?;
        let output = outputs.get_mut(0).ok_or("output 0 is required")?;
        *output = input.clone();
        Ok(())
    }

    fn process(&mut self, inputs: &[Buffer], outputs: &mut [Buffer]) -> Result<(), String> {
        let input_signal = inputs.first()
            .and_then(|input| input.as_time())
            .ok_or("Input 0 is missing or not TimeDomain")?;
        
        let output_signal = outputs.first_mut()
            .and_then(|output| output.as_time_mut())
            .ok_or("Output buffer is not TimeDomain")?;
        
        output_signal.first_packet_num = input_signal.first_packet_num;
        
        for (input, output) in input_signal.data.iter().zip(output_signal.data.iter_mut()){
            *output = self.gain * input;
        } 
        Ok(())
    }
}

impl ProcessorMeta for GainProcessor {
    fn get_type(&self) -> &'static str {
        "Gain"
    }

    fn get_input_ports(&self) -> &'static [IOPort] {
        &[
            IOPort{
                name: "input", 
                optional: false,
                types: &[IOType::Time], 
            }
        ]
    }

    fn get_output_ports(&self) -> &'static [IOPort] {
        &[
            IOPort{
                name: "output",
                optional: false,
                types: &[IOType::Time],
            }
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::buffer::TimeDomainBuffer;

    #[test]
    fn test_gain_process() {
        let input_buffer = Buffer::Time(TimeDomainBuffer {
            first_packet_num: 100,
            num_samples: 3,
            channels: vec!["Fp1".to_string(), "Fp2".to_string()],
            sampling_rate: 250.0,
            data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        });

        let mut gain_node = GainProcessor::new(2.0);

        let inputs = vec![input_buffer];
        let mut outputs = vec![Buffer::None]; 
        
        assert!(gain_node.setup(&inputs, &mut outputs).is_ok());
        assert!(gain_node.process(&inputs, &mut outputs).is_ok());

        let out_time = outputs[0].as_time().unwrap();

        assert_eq!(out_time.first_packet_num, 100);
        assert_eq!(out_time.channels, vec!["Fp1", "Fp2"]);
        assert_eq!(out_time.data, vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0]);
    }
}