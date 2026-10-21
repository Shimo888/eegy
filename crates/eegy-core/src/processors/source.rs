use std::cmp::max;
use std::sync::mpsc::channel;
use crate::core::buffer::{Buffer, TimeDomainBuffer};
use crate::core::processor::IOType::Time;
use crate::core::processor::{IOPort, Processor, ProcessorMeta};
use serde::{Deserialize, Serialize};

/// SourceProcessor
/// リングバッファに各電圧の値を保存する
/// 脳波データのパケットを受け取るたびにpush_packet()でリングバッファを更新する
/// process()ではデータの欠損を補完しつつ、outputにデータを詰める
#[derive(Serialize, Deserialize)]
pub struct SourceProcessor{
    channels: Vec<String>,
    sample_rate: f64,
    window_size: usize,
    
    /// 各電極の電圧値を格納
    /// size = window_size x (電極の種類数)
    /// [O1の電圧の配列..., O2の電圧の配列, ...]のように並ぶ
    #[serde(skip)]
    ring_buffer: Vec<f64>,
    
    /// サンプル番号の配列
    #[serde(skip)]
    sample_nums: Vec<u64>,
    
    /// 一番新しいパケット番号 
    #[serde(skip)]
    latest_packet_num: u64,
}

impl SourceProcessor{
    pub fn push_packet(&mut self, packet_num: u64, packet_data: &[f64]) -> Result<(), String> {
        if self.ring_buffer.len() == 0{
            return Err("Source processor is not initialized".to_string());
        }
        
        if packet_data.len() != self.channels.len(){
            return Err(format!("invalid number of packets received: {} != {}",packet_data.len(),self.channels.len()));
        }
        
        let index = (packet_num % self.window_size as u64) as usize;
        self.sample_nums[index] = packet_num;
        self.latest_packet_num = max(self.latest_packet_num, packet_num);
        for (ch_index, &data) in packet_data.iter().enumerate(){
            let buffer_index = ch_index * self.window_size + index;
            self.ring_buffer[buffer_index] = data;
        }
        Ok(())
    }
}

impl Default for SourceProcessor{
    fn default() -> Self {
        Self {
            channels: vec!["Ch1".to_string()],
            sample_rate: 250.0,
            window_size: 256,
            ring_buffer: Vec::new(),
            sample_nums: Vec::new(),
            latest_packet_num: 0,
        }
    }
}

impl ProcessorMeta for SourceProcessor {
    fn get_type(&self) -> &'static str
    {
        "Source"
    }
    
    fn get_input_ports(&self) -> &'static [IOPort] {
        &[]
    }

    fn get_output_ports(&self) -> &'static [IOPort] {
        &[IOPort{
            name: "samples",
            optional: false,
            types: &[Time],
        }]
    }
}

impl Processor for SourceProcessor {
    fn setup(&mut self, inputs: &[&Buffer], outputs: &mut [&mut Buffer]) -> Result<(), String> {
        if self.window_size == 0{
            return Err(String::from("window size must be greater than 0"));
        }
        
        self.ring_buffer = vec![0.0; self.channels.len() * self.window_size];
        self.sample_nums = vec![0; self.window_size];
        self.latest_packet_num = 0;
        
        let output = TimeDomainBuffer{
            latest_packet_num: 0,
            channels: self.channels.clone(),
            sampling_rate: self.sample_rate,
            data: vec![0.0; self.ring_buffer.len()], 
        };
        
        let output0 = outputs.get_mut(0).ok_or("Cant get output")?;
        **output0 = Buffer::Time(output);
        Ok(())
    }

    fn process(&mut self, inputs: &[&Buffer], outputs: &mut [&mut Buffer]) -> Result<(), String> {
        let output0 = outputs.get_mut(0)
            .and_then(|b| b.as_time_mut())
            .ok_or("Cant get output".to_string())?;
        
        output0.latest_packet_num = self.latest_packet_num;
        let latest_index = (self.latest_packet_num % self.window_size as u64) as usize;
        let oldest_index = (latest_index + 1) % self.window_size;
        let first_part_len = self.window_size - oldest_index;
        for ch_index in 0..self.channels.len() {
            let ring_slice = &self.ring_buffer[ch_index * self.window_size..(ch_index + 1) * self.window_size];
            let output_slice = &mut output0.data[ch_index * self.window_size..(ch_index + 1) * self.window_size];
            
            output_slice[..first_part_len].copy_from_slice(&ring_slice[oldest_index..]);
            output_slice[first_part_len..].copy_from_slice(&ring_slice[..oldest_index]);
        }
        
        Ok(())
    }
}