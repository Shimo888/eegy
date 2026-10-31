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
    drop_rate : f64,
    
    // cache
    current_packet_num: u64,
}

impl PacketGenerator {
    pub fn new(sampling_rate: f64) -> Self{
        Self{
            channels: Vec::new(),
            simulators: Vec::new(),
            sampling_rate,
            drop_rate: 0.0,
            current_packet_num: 0,
        }
    }
    
    pub fn add_channel<T:SignalSimulator + 'static>(mut self, channel: &str, simulator: T) -> Self{
        self.channels.push(channel.to_string());
        self.simulators.push(Box::new(simulator));
        self
    }
    
    pub fn set_drop_rate(mut self, drop_rate: f64) -> Self{
        self.drop_rate = drop_rate.clamp(0.0, 1.0_f64.next_down());
        self
    }
    
    pub fn increment_step(&mut self){
        self.current_packet_num += 1;
        
        // 欠損確率が0のとのきは1インクリメントするだけ
        if self.drop_rate <= 0.0{
            return;
        }
        
        // 欠損率が0よりおきいときは、欠損確率を考慮してpacketをskipする
        while rand::random::<f64>() < self.drop_rate {
            self.current_packet_num += 1;
        } 
    }
}

impl Iterator for PacketGenerator {
    type Item = Packet;
    
    fn next(&mut self) -> Option<Self::Item> {
        let prev_packet_num = self.current_packet_num;
        self.increment_step();
        
        let mut packet = Self::Item{
            packet_num: self.current_packet_num,
            samples: vec![0.0; self.channels.len()], 
        };
        
        for (index, simulator) in self.simulators.iter_mut().enumerate() {
            let step_context = StepContext{ 
                packet_num: self.current_packet_num,
                sampling_rate: self.sampling_rate,
                step: (self.current_packet_num - prev_packet_num) as usize,
            };
            
            packet.samples[index] = simulator.step(&step_context);
        } 
        
        Some(packet)
    }
}