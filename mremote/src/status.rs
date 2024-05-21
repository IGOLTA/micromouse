pub const STATUS_SIZE:usize = 3*4 + 4 + 1;

#[derive(Clone, Copy)]
pub struct Status {
    kp: f32,
    ki: f32,
    kd: f32,
    feedback_loop_sample_time: u32,
    feedback_enabled: bool,
}

impl Status {
    pub fn from_bytes(bytes: [u8; STATUS_SIZE]) -> Self {
        Status {
            kp: f32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            ki: f32::from_le_bytes(bytes[4..8].try_into().unwrap()),
            kd: f32::from_le_bytes(bytes[8..12].try_into().unwrap()),
            feedback_loop_sample_time: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
            feedback_enabled: bytes[16] != 0
        }
    }


    pub fn print_in_console(&self) {
        println!("Kp: {}", self.kp);
        println!("Ki: {}", self.ki);
        println!("Kd: {}", self.kd);
        println!("Temps d'echantillonage des correcteurs et de la vitesse: {}", self.feedback_loop_sample_time);
        println!("Boucle de retour: {}", self.feedback_enabled);
    }
}