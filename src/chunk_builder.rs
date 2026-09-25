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
pub struct ChunkBuilder{
    value_arrays: Vec<Vec<f64>>,
    channel_map: HashMap<String, usize>,
    packet_nums: Option<Vec<u64>>,
    sampling_rate: Option<u32>,
}

impl ChunkBuilder{
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
    pub fn set_packet_num_array(mut self, packet_num_array: Vec<u64>) -> Self{
        self.packet_nums = Some(packet_num_array);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_success() {
        let packet_nums = vec![1, 2, 3];
        let chunk = ChunkBuilder::new()
            .set_sampling_rate(250)
            .set_packet_num_array(packet_nums)
            .set_sample_buffer("Fp1".to_string(), vec![1.0, 2.0, 3.0])
            .set_sample_buffer("Fp2".to_string(), vec![4.0, 5.0, 6.0])
            .build();

        assert!(chunk.is_ok(), "Chunk should be built successfully");
    }

    #[test]
    fn test_build_missing_sampling_rate() {
        let packet_nums = vec![1, 2, 3];
        let err = ChunkBuilder::new()
            .set_packet_num_array(packet_nums)
            .set_sample_buffer("Fp1".to_string(), vec![1.0, 2.0, 3.0])
            .build()
            .unwrap_err();
        
        assert_eq!(err, ChunkBuildError::MissingSamplingRate);
    }

    #[test]
    fn test_build_missing_packet_nums() {
        let err = ChunkBuilder::new()
            .set_sampling_rate(250)
            .set_sample_buffer("Fp1".to_string(), vec![1.0, 2.0, 3.0])
            .build()
            .unwrap_err();
        
        assert_eq!(err, ChunkBuildError::MissingPacketNums);
    }

    #[test]
    fn test_build_empty_sample_buffers() {
        let packet_nums = vec![1, 2, 3];
        let err = ChunkBuilder::new()
            .set_sampling_rate(250)
            .set_packet_num_array(packet_nums)
            .build()
            .unwrap_err();
        
        assert_eq!(err, ChunkBuildError::EmptySampleBuffers);
    }
}
