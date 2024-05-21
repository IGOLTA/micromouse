pub enum Request {
    SetSampleCount,
    SetAcqSampleRate,
    SetInputSpeed ,
    LaunchAq,
    SetFeedbackSampleRate,
    SetP,
    SetI,
    SetD,
    GetStatus,
    SetFeedback,
}

impl Request {
    pub fn value(&self) -> u8 {
        match *self {
            Request::SetSampleCount => 0x00,
            Request::SetAcqSampleRate => 0x01,
            Request::SetInputSpeed => 0x02,
            Request::LaunchAq => 0x03,
            Request::SetFeedbackSampleRate => 0x04,
            Request::SetP => 0x05,
            Request::SetI => 0x06,
            Request::SetD => 0x07,
            Request::GetStatus => 0x08,
            Request::SetFeedback => 0x09
        }
    }
}