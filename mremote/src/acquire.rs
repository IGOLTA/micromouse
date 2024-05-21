use std::{collections::HashMap, time::Duration};

use clap::builder::Str;

use crate::FLOAT_SIZE;

pub const MOTOR_COUNT: usize = 2;
pub const ACQ_BLOCK_SIZE: usize = 4;

pub struct AcqResult {
    pub samples: Vec<(Duration, Sample)> 
}

impl AcqResult {
    pub fn from_bytes(sample_time: u32, bytes: &[u8]) -> Self {
        let floats: Vec<f32> = bytes
            .chunks_exact(FLOAT_SIZE)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
            .collect();
        let untimed_samples: Vec<Sample> = floats
            .chunks_exact(MOTOR_COUNT * ACQ_BLOCK_SIZE)
            .map(|floats| Sample::from_floats(floats.try_into().unwrap()))
            .collect();

        let samples:Vec<(Duration, Sample)> = untimed_samples
                .iter()
                .enumerate()
                .map(|(index, sample)| (Duration::from_micros(sample_time as u64 * index as u64), *sample))
                .collect();

        AcqResult {
            samples,
        }
    }

    pub fn as_regressi_format(&self) -> String {
        let mut regressi: String = String::new();
        regressi += "t CVM1 VM1 EM1 IM1 CVM2 VM2 EM2 IM2";
        for (time, sample) in &self.samples {
            regressi += "\n";
            regressi += time.as_secs_f64().to_string().as_str();
            regressi += " ";
            regressi += sample.motor_1_input_speed.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_1_speed.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_1_error.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_1_input.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_2_input_speed.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_2_speed.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_2_error.to_string().as_str();
            regressi += " ";
            regressi += sample.motor_2_input.to_string().as_str();

        }

        regressi
    }
}

#[derive(Clone, Copy)]
pub struct Sample {
    pub motor_1_input_speed:f32,
    pub motor_1_speed:f32,
    pub motor_1_error:f32,
    pub motor_1_input:f32,
    pub motor_2_input_speed:f32,
    pub motor_2_speed:f32,
    pub motor_2_error:f32,
    pub motor_2_input:f32,
}

impl Sample {
    pub fn from_floats(floats: [f32;8]) -> Self {
        Sample{
            motor_1_input_speed: floats[0],
            motor_1_speed: floats[1],
            motor_1_error: floats[2],
            motor_1_input: floats[3],
            motor_2_input_speed: floats[4],
            motor_2_speed: floats[5],
            motor_2_error: floats[6],
            motor_2_input: floats[7],
        }
    }
}