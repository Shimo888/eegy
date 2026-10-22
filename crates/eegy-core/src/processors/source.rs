use std::cmp::max;
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
    interpolation_strategy: InterpolationStrategy,
    
    /// 各電極の電圧値を格納
    /// size = window_size x (電極の種類数)
    /// [O1の電圧の配列..., O2の電圧の配列, ...]のように並ぶ
    #[serde(skip)]
    ring_buffer: Vec<f64>,
    
    /// パケット番号の配列
    #[serde(skip)]
    packet_nums: Vec<u64>,
    
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
        self.packet_nums[index] = packet_num;
        self.latest_packet_num = max(self.latest_packet_num, packet_num);
        for (ch_index, &data) in packet_data.iter().enumerate(){
            let buffer_index = ch_index * self.window_size + index;
            self.ring_buffer[buffer_index] = data;
        }
        Ok(())
    }
    
    /// 欠損パケットの補完
    fn interpolate_missing_packet<F>(
        & self,
        output: &mut TimeDomainBuffer,
        strategy: F
    )
    where F: Fn(&mut [f64], usize, usize){
        let mut start_index = None;
        for i in 0..self.window_size{
            let ring_index = ((self.latest_packet_num + 1 + i as u64) % (self.window_size as u64)) as usize;
            let real_packet_num = self.packet_nums[ring_index];
            let ideal_packet_num = output.latest_packet_num.saturating_sub((self.window_size - 1 - i) as u64);

            if real_packet_num != ideal_packet_num{
                start_index = if start_index.is_none() { Some(i) } else { start_index };
                continue;
            }
            
            if start_index.is_some(){ 
                for ch_index in 0..self.channels.len(){
                    let slice = &mut output.data[self.window_size * ch_index .. self.window_size * (ch_index + 1)];
                    strategy(slice, start_index.unwrap(), i);
                }
                start_index = None;
            }
        }
    }
    
    /// ゼロ補完
    fn interpolate_strategy_zero(data: &mut [f64], start_index: usize, end_index: usize){
        data[start_index..end_index].fill(0.0);
    }
    
    /// 前回値保持
    fn interpolate_strategy_forward_fill(data: &mut [f64], start_index: usize, end_index: usize){
        // index=0のときは参照できる前回値がないので、次の値を参考にする
        if start_index == 0{
            let right_val = data[end_index];
            data[start_index..end_index].fill(right_val);
            return;
        }
        
        let left_val = data[start_index - 1];
        data[start_index..end_index].fill(left_val);
    }
    
    /// 線形補完
    fn interpolate_strategy_linear(data: &mut [f64], start_index: usize, end_index: usize){
        // index=0のときは参照できる前回値がないので、次の値を参考にする
        if start_index == 0{
            let right_val = data[end_index];
            data[start_index..end_index].fill(right_val);
            return;
        }
        
        let k = (data[end_index] - data[start_index -1]) / (end_index - start_index + 1) as f64;
        let start = data[start_index -1];
        data[start_index..end_index]
            .iter_mut()
            .enumerate()
            .for_each(|(i,v)| {
                *v = start + k * (i + 1) as f64;
            })
    }
}

impl Default for SourceProcessor{
    fn default() -> Self {
        Self {
            channels: vec!["Ch1".to_string()],
            sample_rate: 250.0,
            window_size: 256,
            interpolation_strategy: InterpolationStrategy::default(),
            ring_buffer: Vec::new(),
            packet_nums: Vec::new(),
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
        self.packet_nums = vec![0; self.window_size];
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
        
        // リングバッファのスライスコピー
        for ch_index in 0..self.channels.len() {
            let ring_slice = &self.ring_buffer[ch_index * self.window_size..(ch_index + 1) * self.window_size];
            let output_slice = &mut output0.data[ch_index * self.window_size..(ch_index + 1) * self.window_size];
            
            output_slice[..first_part_len].copy_from_slice(&ring_slice[oldest_index..]);
            output_slice[first_part_len..].copy_from_slice(&ring_slice[..oldest_index]);
        }
        
        // データ欠損を補完
        match self.interpolation_strategy{
            InterpolationStrategy::Zero => self.interpolate_missing_packet(output0, Self::interpolate_strategy_zero),
            InterpolationStrategy::ForwardFill => self.interpolate_missing_packet(output0, Self::interpolate_strategy_forward_fill),
            InterpolationStrategy::Linear => self.interpolate_missing_packet(output0, Self::interpolate_strategy_linear),
        }
        
        Ok(())
    }
}

/// データ欠損時の補完方法
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
enum InterpolationStrategy {
    /// ゼロ埋め
    Zero,
    
    /// 前回値保持 
    #[default]
    ForwardFill,
    
    /// 線形補完
    Linear,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1-1: setup() によって出力バッファが正しく初期化されること
    #[test]
    fn test_setup_initializes_output_buffer() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string(), "O2".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];

        assert!(source.setup(&[], &mut outputs).is_ok());

        let time_buf = out_buf.as_time().expect("Output should be TimeDomainBuffer");
        assert_eq!(time_buf.latest_packet_num, 0);
        assert_eq!(time_buf.channels, vec!["O1", "O2"]);
        assert_eq!(time_buf.sampling_rate, 250.0);
        assert_eq!(time_buf.data, vec![0.0; 8]);
    }

    /// 1-2: window_size が 0 の場合、setup() がエラーを返すこと
    #[test]
    fn test_setup_zero_window_size_fails() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string()],
            sample_rate: 250.0,
            window_size: 0,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];

        assert!(source.setup(&[], &mut outputs).is_err());
    }

    /// 1-3: setup() 前に push_packet() を呼んだ場合、パニックせずエラーを返すこと
    #[test]
    fn test_push_packet_before_setup_fails() {
        let mut source = SourceProcessor::default();
        assert!(source.push_packet(1, &[10.0]).is_err());
    }

    /// 1-4: チャンネル数と異なる要素数のパケットを push_packet() した場合、エラーを返すこと
    #[test]
    fn test_push_packet_channel_mismatch_fails() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string(), "O2".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];
        source.setup(&[], &mut outputs).unwrap();

        // 1要素（少なすぎる）
        assert!(source.push_packet(1, &[10.0]).is_err());
        // 3要素（多すぎる）
        assert!(source.push_packet(1, &[10.0, 20.0, 30.0]).is_err());
    }

    /// 2-1: ウィンドウサイズ未満のパケット投入時に、末尾に時系列順で格納されること
    #[test]
    fn test_process_partial_window() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string(), "O2".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];
        source.setup(&[], &mut outputs).unwrap();

        source.push_packet(1, &[10.0, 100.0]).unwrap();
        source.push_packet(2, &[20.0, 200.0]).unwrap();

        source.process(&[], &mut outputs).unwrap();

        let time_buf = out_buf.as_time().unwrap();
        assert_eq!(time_buf.latest_packet_num, 2);
        // O1: [0.0, 0.0, 10.0, 20.0], O2: [0.0, 0.0, 100.0, 200.0]
        assert_eq!(
            time_buf.data,
            vec![0.0, 0.0, 10.0, 20.0, 0.0, 0.0, 100.0, 200.0]
        );
    }

    /// 2-2: ちょうどウィンドウサイズ分（1周分）のパケット投入時に時系列順で出力されること
    #[test]
    fn test_process_exact_window() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string(), "O2".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];
        source.setup(&[], &mut outputs).unwrap();

        source.push_packet(1, &[10.0, 100.0]).unwrap();
        source.push_packet(2, &[20.0, 200.0]).unwrap();
        source.push_packet(3, &[30.0, 300.0]).unwrap();
        source.push_packet(4, &[40.0, 400.0]).unwrap();

        source.process(&[], &mut outputs).unwrap();

        let time_buf = out_buf.as_time().unwrap();
        assert_eq!(time_buf.latest_packet_num, 4);
        assert_eq!(
            time_buf.data,
            vec![10.0, 20.0, 30.0, 40.0, 100.0, 200.0, 300.0, 400.0]
        );
    }

    /// 2-3: リングバッファが1周を超えて折り返した際、直近 window_size 分が古い順に出力されること
    #[test]
    fn test_process_wrap_around() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string(), "O2".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];
        source.setup(&[], &mut outputs).unwrap();

        for pkt in 1..=6 {
            let v = pkt as f64;
            source.push_packet(pkt, &[v * 10.0, v * 100.0]).unwrap();
        }

        source.process(&[], &mut outputs).unwrap();

        let time_buf = out_buf.as_time().unwrap();
        assert_eq!(time_buf.latest_packet_num, 6);
        // 直近4個（pkt 3, 4, 5, 6）が時系列順に並んでいること
        assert_eq!(
            time_buf.data,
            vec![30.0, 40.0, 50.0, 60.0, 300.0, 400.0, 500.0, 600.0]
        );
    }

    /// 2-5: 再 setup() 時に内部のリングバッファと最新パケット番号がリセットされること
    #[test]
    fn test_setup_resets_previous_state() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];

        source.setup(&[], &mut outputs).unwrap();
        source.push_packet(1, &[10.0]).unwrap();
        source.push_packet(2, &[20.0]).unwrap();

        // 再セットアップでリセットされること
        source.setup(&[], &mut outputs).unwrap();
        source.process(&[], &mut outputs).unwrap();

        let time_buf = out_buf.as_time().unwrap();
        assert_eq!(time_buf.latest_packet_num, 0);
        assert_eq!(time_buf.data, vec![0.0; 4]);
    }

    /// 3-1: パケット番号が前後して到着した場合（1 -> 3 -> 2）でも正しい位置に格納されること
    #[test]
    fn test_push_packet_out_of_order() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];
        source.setup(&[], &mut outputs).unwrap();

        source.push_packet(1, &[10.0]).unwrap();
        source.push_packet(3, &[30.0]).unwrap();
        source.push_packet(2, &[20.0]).unwrap();

        source.process(&[], &mut outputs).unwrap();

        let time_buf = out_buf.as_time().unwrap();
        assert_eq!(time_buf.latest_packet_num, 3);
        assert_eq!(time_buf.data, vec![0.0, 10.0, 20.0, 30.0]);
    }

    /// 3-2: パケット欠損（1周後に番号が飛んだ場合）に1周前の古いデータが残らないこと
    #[test]
    fn test_push_packet_with_gap_does_not_keep_stale_data() {
        let mut source = SourceProcessor {
            channels: vec!["O1".to_string()],
            sample_rate: 250.0,
            window_size: 4,
            ..Default::default()
        };
        let mut out_buf = Buffer::None;
        let mut outputs = vec![&mut out_buf];
        source.setup(&[], &mut outputs).unwrap();

        // 1周目: パケット 1, 2, 3, 4 を投入
        source.push_packet(1, &[10.0]).unwrap();
        source.push_packet(2, &[20.0]).unwrap();
        source.push_packet(3, &[30.0]).unwrap();
        source.push_packet(4, &[40.0]).unwrap();

        // パケット 5 が欠損し、パケット 6 (60.0) が到着
        source.push_packet(6, &[60.0]).unwrap();
        source.process(&[], &mut outputs).unwrap();

        let time_buf = out_buf.as_time().unwrap();
        assert_eq!(time_buf.latest_packet_num, 6);
        // 期待されるウィンドウはパケット [3, 4, 5(欠損), 6]
        // インデックス 2（パケット5の位置）に1周前のパケット1の値（10.0）が残っていないこと
        assert_ne!(
            time_buf.data[2], 10.0,
            "Stale data from packet 1 remained at dropped packet 5's slot"
        );
    }
}