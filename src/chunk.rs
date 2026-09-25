use std::collections::HashMap;

/// EEGデータの実体の管理struct
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use crate::chunk_builder::ChunkBuilder;

    #[test]
    fn test_get_sample_success() {
        // テスト用のChunkをビルダー経由で作成
        let packet_nums = vec![1, 2, 3];
        let chunk = ChunkBuilder::new()
            .set_sampling_rate(250)
            .set_packet_num_array(packet_nums)
            .set_sample_buffer("Fp1".to_string(), vec![1.0, 2.0, 3.0])
            .set_sample_buffer("Fp2".to_string(), vec![4.0, 5.0, 6.0])
            .build()
            .unwrap();

        // 存在する電極名で正しくデータが取得できるか
        let fp1_data = chunk.get_sample("Fp1");
        assert_eq!(fp1_data, Some(vec![1.0, 2.0, 3.0]));

        let fp2_data = chunk.get_sample("Fp2");
        assert_eq!(fp2_data, Some(vec![4.0, 5.0, 6.0]));
    }

    #[test]
    fn test_get_sample_not_found() {
        let packet_nums = vec![1, 2, 3];
        let chunk = ChunkBuilder::new()
            .set_sampling_rate(250)
            .set_packet_num_array(packet_nums)
            .set_sample_buffer("Fp1".to_string(), vec![1.0, 2.0, 3.0])
            .build()
            .unwrap();

        // 存在しない電極名を指定した場合に None が返るか
        let missing_data = chunk.get_sample("O1");
        assert_eq!(missing_data, None);
    }

    #[test]
    fn test_getters() {
        let packet_nums = vec![1, 2, 3];
        let chunk = ChunkBuilder::new()
            .set_sampling_rate(500)
            .set_packet_num_array(packet_nums)
            .set_sample_buffer("Fp1".to_string(), vec![1.0, 2.0, 3.0])
            .build()
            .unwrap();

        assert_eq!(chunk.get_sampling_rate(), 500);
        assert_eq!(chunk.get_packet_num_array(), &vec![1, 2, 3]);
    }
}