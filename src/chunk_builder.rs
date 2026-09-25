use std::collections::HashMap;
use crate::chunk::Chunk;

#[derive(Debug, PartialEq, Eq)]
pub enum ChunkBuildError {
    MissingSamplingRate, // サンプリングレートが未初期化
    MissingPacketNums, // パケット番号が未初期化
    EmptySampleBuffers, // サンプル値のバッファーが未初期化
}

impl std::fmt::Display for ChunkBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkBuildError::MissingSamplingRate => write!(f, "Sampling rate is required"),
            ChunkBuildError::MissingPacketNums => write!(f, "Packet numbers are required"),
            ChunkBuildError::EmptySampleBuffers => write!(f, "At least one sample buffer is required"),
        }
    }
}

impl std::error::Error for ChunkBuildError {}

/// Chunkのbuilder
/// チェーンメソッド形式で初期化できるようにしとく
pub struct ChunkBuilder<'a>{
    value_arrays: Vec<Vec<f64>>,
    channel_map: HashMap<String, usize>,
    packet_nums: Option<&'a [u64]>,
    sampling_rate: Option<u32>,
}

impl <'a> ChunkBuilder<'a>{
    pub fn new() -> Self{
        Self{
            value_arrays: Vec::new(),
            channel_map: HashMap::new(),
            packet_nums: None,
            sampling_rate: None,
        }
    }

    /// Chunkの作成
    pub fn build(self) -> Result<Chunk, ChunkBuildError>{
        let sampling_rate = self.sampling_rate.ok_or(ChunkBuildError::MissingSamplingRate)?;
        let packet_nums = self.packet_nums.ok_or(ChunkBuildError::MissingPacketNums)?;

        if self.value_arrays.is_empty() {
            return Err(ChunkBuildError::EmptySampleBuffers);
        }

        Ok(Chunk {
            channel_map: self.channel_map,
            sample_buffers: self.value_arrays,
            packet_nums: packet_nums.to_vec(),
            sampling_rate,
        })
    }

    /// SampleのBuffer設定
    pub fn set_sample_buffer(mut self, channel: String, sample_array: Vec<f64>) -> Self{
        let index = self.value_arrays.len();
        self.channel_map.insert(channel, index);
        self.value_arrays.push(sample_array);
        self
    }

    /// サンプリングレート(Hz)の設定
    pub fn set_sampling_rate(mut self, sample_rate: u32) -> Self{
        self.sampling_rate = Some(sample_rate);
        self
    }

    /// パケット番号のリストの設定
    pub fn set_packet_num_array(mut self, packet_num_array: &'a [u64]) -> Self{
        self.packet_nums = Some(packet_num_array);
        self
    }
}
