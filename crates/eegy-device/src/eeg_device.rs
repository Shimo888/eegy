use crate::eeg_packet::EegPacket;

pub trait EegDevice{
    fn name(&self) -> &str;
    fn channels(&self) -> &[String];
    fn sample_rate(&self) -> f64;
    fn start(&mut self) -> Result<(), String>;
    fn stop(&mut self) -> Result<(), String>;
    fn is_running(&self) -> bool;
    fn poll_packets(&mut self) -> Result<Vec<EegPacket>, String>;
}