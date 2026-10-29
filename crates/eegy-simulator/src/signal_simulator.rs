pub trait SignalSimulator { 
    fn step(&mut self, step: &StepContext) -> f64;
}

#[derive(Clone)]
pub struct StepContext{
    pub packet_num: u64,
    pub sampling_rate: f64,
}

impl StepContext{
    /// 誤差がない正確な時間
    #[inline]
    pub fn time(&self) -> f64{
        self.packet_num as f64 / self.sampling_rate
    }
    
    /// 1ステップあたりの時間刻み幅
    #[inline]
    pub fn dt(&self) -> f64{
        1.0 / self.sampling_rate
    }
} 