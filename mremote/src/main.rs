use std::{net::TcpStream, io::{Write, Read, ErrorKind}, time::Duration, iter, collections::{binary_heap::Iter, HashMap}};

use acquire::{ACQ_BLOCK_SIZE, MOTOR_COUNT, AcqResult};
use args::{Cli, AcquireCommand, StepCommand, SetCommand, SineCommand, BodeCommand};
use clap::Parser;
use requests::Request;
use status::{Status, STATUS_SIZE};

mod args;
mod requests;
mod acquire;
mod status;

pub const  FLOAT_SIZE:usize = 4;

const ADRESS: &str = "192.168.0.1:50000";
const TIMEOUT: Duration = Duration::from_secs(1);

fn main() {
    let cli = Cli::parse();
    match cli.action_type {
        args::ActionType::Set(command) => set(command),
        args::ActionType::Acquire(command) => {acquire(command, true).expect("Acquire failed");},
        args::ActionType::Status {  } => status(),
        args::ActionType::Bode(command) => bode(command)
    }
}

fn acquire(command: AcquireCommand, do_print: bool) -> std::io::Result<AcqResult> {
    let sample_count = command.acquire_duration / command.sample_time;
    let sample_time = command.sample_time;

    if sample_count == 0 {
        panic!("Le temps d'acquisition doit etre superieur a la duree d'echantillonage");
    }


    let mut stream: TcpStream= TcpStream::connect(ADRESS)?;
    stream.write(&[Request::SetSampleCount.value()])?;
    stream.write(&u32::to_le_bytes(sample_count))?;

    stream.write(&[Request::SetAcqSampleRate.value()])?;
    stream.write(&u32::to_le_bytes(sample_time))?;

    let input_size = sample_count as usize * MOTOR_COUNT;
    let mut motor_input: Vec<f32> = match command.acq_type {
        args::AcqType::Step(StepCommand { value }) =>  vec![value; input_size],
        args::AcqType::Sine(SineCommand{amplitude, pulsation}) => 
            (0..input_size)
                .map(|i| amplitude * f32::sin(i as f32 * Duration::from_micros(sample_time as u64).as_secs_f32()  * pulsation))
                .collect(),
    };

    let input_len = motor_input.len();
    motor_input[input_len-2] = 0.;
    motor_input[input_len-1] = 0.;
    let bytes: Vec<u8> = motor_input
        .iter()
        .flat_map(|&v| f32::to_le_bytes(v))
        .collect();

    stream.write(&[Request::SetInputSpeed.value()])?;
    stream.write(&bytes)?;

    stream.write(&[Request::LaunchAq.value()])?;
    stream.shutdown(std::net::Shutdown::Write)?;
    
    let buff_len = sample_count as usize * ACQ_BLOCK_SIZE * MOTOR_COUNT * FLOAT_SIZE;
    let mut buffer: Vec<u8> = Vec::with_capacity(buff_len);
    while buffer.len() < buff_len {
        let mut sub_buffer: Vec<u8> = Vec::new();
        stream.read_to_end(& mut sub_buffer)?;
        buffer.append(&mut sub_buffer);
    }
    let acq_result = AcqResult::from_bytes(sample_time, buffer.as_slice());
    if do_print {
        println!("{}", acq_result.as_regressi_format());
    }
    std::io::Result::<AcqResult>::Ok(acq_result)
}

fn status() {
    let mut stream: TcpStream= TcpStream::connect(ADRESS).expect("Failed to connect to the micromouse");
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    stream.write(&[Request::GetStatus.value()]).unwrap();
    let mut buffer: Vec<u8> = Vec::with_capacity(STATUS_SIZE);
    while (stream.read_to_end(& mut buffer).unwrap() == 0) {}
    let status = Status::from_bytes(buffer[0..STATUS_SIZE].try_into().unwrap());
    status.print_in_console();
}

fn set(command: SetCommand) {

    let mut stream: TcpStream= TcpStream::connect(ADRESS).expect("Failed to connect to the micromouse");
    stream.set_read_timeout(Some(TIMEOUT)).unwrap();
    if let Some(kp) = command.proportional {
        stream.write(&[Request::SetP.value()]).unwrap();
        stream.write(&kp.to_le_bytes()).unwrap();
    }
    if let Some(ki) = command.integral {
        stream.write(&[Request::SetI.value()]).unwrap();
        stream.write(&ki.to_le_bytes()).unwrap();
    }
    if let Some(kd) = command.derivative {
        stream.write(&[Request::SetD.value()]).unwrap();
        stream.write(&kd.to_le_bytes()).unwrap();
    }
    if let Some(feedback_sample_time) = command.feedback_sample_time {
        stream.write(&[Request::SetFeedbackSampleRate.value()]).unwrap();
        stream.write(&feedback_sample_time.to_le_bytes()).unwrap();
    }
    if let Some(feedback_enabled) = command.feedback_enabled {
        stream.write(&[Request::SetFeedback.value()]).unwrap();
        stream.write(&[feedback_enabled]).unwrap();
    }
}

fn bode(command: BodeCommand) {
    let ws = (0..command.sample_count)
        .map(|i| {
            let norm_i = i as f32/(command.sample_count - 1) as f32;
            let scaled_i = norm_i * (command.max_w - command.min_w);
            let shifted_i = scaled_i + command.min_w;
            f32::powf(10., shifted_i)
        });
    
    let mut results_m1  = Vec::<(f32, Vec<(Duration, f32, f32)>)>::with_capacity(command.sample_count);
    let mut results_m2  = Vec::<(f32, Vec<(Duration, f32, f32)>)>::with_capacity(command.sample_count);
    
    for w in ws {
        let period_duration = 2.0 * std::f32::consts::PI/w;
        let period_duration_micros: u32 = Duration::from_secs_f32(period_duration).as_micros() as u32;
        println!("Lancement de l'acquisition pout w = {w} rad/s");
        let acq = acquire(AcquireCommand { 
            acq_type: args::AcqType::Sine(SineCommand { 
                amplitude: command.amplitude, 
                pulsation: w 
            }), 
            acquire_duration: period_duration_micros * command.steady_state_period_count + period_duration_micros * command.period_count, 
            sample_time: Duration::from_secs_f32(period_duration/ command.sine_sample_count as f32).as_micros() as u32
        }, true);
        match acq {
            Ok(acq_result) => {
                results_m1.push((w,
                    acq_result
                    .samples
                    .iter()
                    .map(|(time, sample)| (*time, sample.motor_1_input_speed, sample.motor_1_speed))
                    .collect()
                ));
                results_m2.push((w,
                    acq_result
                    .samples
                    .iter()
                    .map(|(time, sample)| (*time, sample.motor_2_input_speed, sample.motor_2_speed))
                    .collect()
                ));
            }
            Err(err) => {
                println!("{}", err.to_string());
                std::thread::sleep(TIMEOUT);
                println!("Retrying");
                return bode(command);
            }
        }
    }
    
}