#[derive(Debug, Clone, PartialEq)]
pub struct EegPacket{
    pub packet_num: u64,
    pub samples: Vec<f64>,
}