use std::collections::HashMap;

/// EEGデータの実体の管理struct
pub struct Chunk{
    /// 電極名 -> sample_arraysのindexを取得するためのHashMap 
    pub(crate) channel_map: HashMap<String, usize>,
    
    /// サンプルデータのバッファー
    /// 電極ごとにEEGデータの取得データ(電圧等)を管理する 
    pub(crate) sample_buffers: Vec<Vec<f64>>,
    
    /// パケット番号 
    pub(crate) packet_nums: Vec<u64>,
    
    /// サンプリングレート(Hz) 
    pub(crate) sampling_rate: u32,
}

impl Chunk{
    /// 電極を指定してその電極のデータを返す
    pub fn get_sample(&self, channel_name: &str) -> Option<Vec<f64>> {
        let index = self.channel_map.get(channel_name)?;
        self.sample_buffers.get(*index).cloned()
    }
   
    /// パケット番号のリストの取得
    pub fn get_packet_num_array(&self) -> &Vec<u64>{
        &self.packet_nums
    }
    
    /// サンプリングレート(Hz)の取得
    pub fn get_sampling_rate(&self) -> u32{
        self.sampling_rate
    } 
}