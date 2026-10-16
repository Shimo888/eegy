#[derive(Clone,Debug,PartialEq, Default)]
pub enum Buffer {
    #[default]
    None,
    Time(TimeDomainBuffer),
    Frequency(FrequencyDomainBuffer),
    TimeFrequency(TimeFrequencyDomainBuffer),
}

impl Buffer {
    pub fn as_time(&self) -> Option<&TimeDomainBuffer> {
        match self {
            Buffer::Time(time) => Some(time),
            _ => None,
        }
    }
    
    pub fn as_time_mut(&mut self) -> Option<&mut TimeDomainBuffer> {
        match self {
            Buffer::Time(time) => Some(time),
            _ => None,
        }
    }
}

#[derive(Clone,Debug,PartialEq)]
pub struct TimeDomainBuffer{
    pub first_packet_num: u64,
    pub channels: Vec<String>,
    pub sampling_rate: f64,
    pub data: Vec<f64>,
}

#[derive(Clone,Debug,PartialEq)]
pub struct FrequencyDomainBuffer{
}

#[derive(Clone,Debug,PartialEq)]
pub struct TimeFrequencyDomainBuffer{
}