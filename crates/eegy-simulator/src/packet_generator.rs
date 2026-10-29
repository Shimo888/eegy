use crate::signal_simulator::{SignalSimulator, StepContext};

/// パケット
/// 各電極の電圧値を管理
pub struct Packet{
    pub packet_num: u64,
    pub samples: Vec<f64>,
}

pub struct PacketGenerator {
    channels: Vec<String>,
    simulators: Vec<Box<dyn SignalSimulator>>,
    sampling_rate: f64,
    current_packet_num: u64,
}

impl PacketGenerator {
    pub fn new(sampling_rate: f64) -> Self{
        Self{
            channels: Vec::new(),
            simulators: Vec::new(),
            sampling_rate,
            current_packet_num: 0,
        }
    }
    
    fn add_channel<T:SignalSimulator + 'static>(mut self, channel: &str, simulator: T) -> Self{
        self.channels.push(channel.to_string());
        self.simulators.push(Box::new(simulator));
        self
    }
}

impl Iterator for PacketGenerator {
    type Item = Packet;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.current_packet_num += 1;
        
        let mut packet = Self::Item{
            packet_num: self.current_packet_num,
            samples: vec![0.0; self.channels.len()], 
        };
        
        for (index, simulator) in self.simulators.iter_mut().enumerate() {
            let step_context = StepContext{ 
                packet_num: self.current_packet_num,
                sampling_rate: self.sampling_rate,
            };
            
            packet.samples[index] = simulator.step(&step_context);
        } 
        
        Some(packet)
    }
}