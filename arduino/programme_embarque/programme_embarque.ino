#include <WiFi.h>
#include <ESP32Encoder.h>
#include "request.h"
#include "status.h"

// Timeout for communication in milliseconds
const unsigned long timeout = 1000;

// Block size for data transmission in bytes
const unsigned long block_size = 1024;

// Wi-Fi credentials
const char* ssid     = "Micromouse_TIPE";
const char* password = "tipelouis";

// Communication port
const int port = 50000;

// Number of motors and encoder parameters
const int motors_amount = 2;
const int incs_per_rotation = 200;

// Encoder pin configurations
const int encoder_A[] = {4, 13};
const int encoder_B[] = {5, 14};

// Motor pin configurations
const int motor_P[] = {23, 18};
const int motor_N[] = {25, 19};

// PWM settings
const int pwm_freq = 1000;
const int pwm_resolution = 16;
const int32_t max_pwm = int32_t(pow(2, pwm_resolution) - 1);

const uint8_t acq_block_size = 4;

// Encoder instances
ESP32Encoder encoders[2];

// Motor speed and control variables
float motor_speed[] = {0, 0};
float last_error[] = {0, 0};
float integral_error[] = {0, 0};
float motor_req_speed[] = {0, 0};
float current_input[] = {0, 0};

// Network settings
IPAddress local_ip(192, 168, 0, 1);
IPAddress gateway(192, 168, 0, 1);
IPAddress subnet(255, 255, 255, 0);

// Wi-Fi server and client
WiFiServer server(port);
WiFiClient client;

// Hardware timers for feedback loop and acquisition interrupt
hw_timer_t* feedback_loop_timer = nullptr;
hw_timer_t* acq_timer = nullptr;

// Flags for acquisition state
bool in_acq = false;
bool do_acq = false;

// Acquisition parameters
uint32_t sample_count = 1;
uint32_t sample_rate = 0;
uint32_t acq_progress = 0;

// Buffer for input speed during acquisition
float* input_speed = nullptr; // [B1 input speed][B2 input_speed]
char* send_buffer = nullptr;

// Flag for feedback loop execution
bool do_feedback = false;

Status status = {.kp=1, .feedback_loop_delay = 1000, .feedback_enabled = true};

// Interrupt service routine for feedback loop
void IRAM_ATTR feedback_loop() {
  do_feedback = true;
}

// Interrupt service routine for acquisition
void IRAM_ATTR acq_interrupt() {
  do_acq = true;
}

// Setup function
void setup() {
  Serial.begin(115200);

  // Configure Wi-Fi as an access point
  WiFi.mode(WIFI_AP);
  WiFi.softAPConfig(local_ip, gateway, subnet);
  WiFi.softAP(ssid, password);
  server.begin();

  // Initialize motors, encoders, and timers
  for(int motor_id = 0; motor_id < motors_amount; motor_id++) {
     ledcSetup(motor_id * 2, pwm_freq, pwm_resolution);
     ledcSetup(motor_id * 2 + 1, pwm_freq, pwm_resolution);
     ledcAttachPin(motor_P[motor_id], motor_id * 2);
     ledcAttachPin(motor_N[motor_id], motor_id * 2 + 1);

     encoders[motor_id].attachFullQuad(encoder_A[motor_id], encoder_B[motor_id]);
  }

  // Setup feedback loop timer
  feedback_loop_timer = timerBegin(0, 80, true);
  timerAttachInterrupt(feedback_loop_timer, &feedback_loop, true);
  timerAlarmWrite(feedback_loop_timer, status.feedback_loop_delay, true);
  timerAlarmEnable(feedback_loop_timer);

  // Setup acquisition timer
  acq_timer = timerBegin(1, 80, true);
  timerAttachInterrupt(acq_timer, &acq_interrupt, true);
}

// Main loop
void loop() {
  // Handle incoming client requests
  if(!in_acq) {
    if(client.available() > 0) {
      handle_request();
    } else {
      client = server.available();
    }
  }


  // Check for feedback loop update
  if(do_feedback) {
    do_feedback = false;
    update_motor_input();
  }

  // Check for acquisition interrupt
  if(do_acq) {
    do_acq = false;
    if(in_acq) acq();
  }
}

// Function to perform acquisition
void acq() {
  if(acq_progress >= sample_count) {
    in_acq = false;
    Serial.println("Acq finished. Sending data");
    power_motor(0, 0);
    write_bytes(send_buffer, sample_count * motors_amount * acq_block_size * sizeof(float));
    integral_error[0] = 0;
    integral_error[1] = 0;
    Serial.println("Data sent");
    return;
  }

  // Send motor speeds and requested speeds to the client
  for(int i = 0; i < motors_amount; i++) {
    motor_req_speed[i] = input_speed[acq_progress * motors_amount + i];
    uint32_t offset = acq_progress * motors_amount * acq_block_size * sizeof(float) +  i * acq_block_size * sizeof(float);
    memcpy(&send_buffer[offset], &motor_req_speed[i], sizeof(float));
    offset += sizeof(float);
    memcpy(&send_buffer[offset], &motor_speed[i], sizeof(float));
    offset += sizeof(float);
    memcpy(&send_buffer[offset], &last_error[i], sizeof(float));
    offset += sizeof(float);
    memcpy(&send_buffer[offset], &current_input[i], sizeof(float));
  }

  acq_progress++;
}

// Function to handle incoming requests from the client
void handle_request() {
  if(in_acq) {
    Serial.println("ERROR: Unable to process requests during acquisitions");
  }

  // Read the first byte of the request
  char first_byte = client.read();
  switch(first_byte) {
    case Request::SET_ACQ_SAMPLE_RATE:
      if(client.available() >= sizeof(int32_t)) {
        // Update sample rate based on client request
        read_bytes((char*)&sample_rate, sizeof(int32_t));
        timerAlarmWrite(acq_timer, sample_rate, true);
        timerAlarmEnable(acq_timer);
        Serial.print("Acquire mode sample rate changed to ");
        Serial.println(sample_rate);
      }
      break;
    case Request::SET_SAMPLE_COUNT:
      if(client.available() >= sizeof(int32_t)) {
        // Update sample count and allocate memory for input speed
        read_bytes((char*)&sample_count, sizeof(int32_t));
        if(input_speed) free(input_speed);
        input_speed = (float*)malloc(sample_count * motors_amount * sizeof(float));
        Serial.print("Acquire mode sample count changed to ");
        Serial.println(sample_count);
      }
      break;
    case Request::SET_INPUT_SPEED:
      {
      Serial.println("Changing input speed");
      unsigned long received = 0;
      unsigned long start_time = millis();

      // Receive input speed data in blocks
      while(true) {
        if(client.available() >= block_size) {
          read_bytes((char*)&input_speed[received], block_size);
          received += block_size / sizeof(float);
          start_time = millis();
          Serial.println("Received block");
        } else if(client.available() >= (sample_count * motors_amount - received) * sizeof(float)) {
          // Receive the remaining data
          read_bytes((char*)&input_speed[received], (sample_count * motors_amount - received) * sizeof(float));
          Serial.println("Received end block");
          break;
        } else if(millis() > start_time + timeout) {
          break;
        }
      }

      Serial.println("Input speed changed");
      }
      break;
    case Request::LAUNCH_AQ:
      // Start acquisition
      in_acq = true;
      acq_progress = 0;
      if(send_buffer) free(send_buffer);
      send_buffer = (char*) malloc(sample_count * motors_amount * acq_block_size * sizeof(float));
      Serial.println("Starting acquisition");
      break;
    case Request::SET_FEEDBACK_SAMPLE_RATE:
      if(client.available() >= sizeof(int32_t)) {
        // Update sample rate based on client request
        read_bytes((char*)&status.feedback_loop_delay, sizeof(int32_t));
        timerAlarmWrite(feedback_loop_timer, status.feedback_loop_delay, true);
        timerAlarmEnable(feedback_loop_timer);
        Serial.print("Feedback sample rate changed to ");
        Serial.println(status.feedback_loop_delay);
      }
      break;
    case Request::SET_P:
      if(client.available() >= sizeof(float)) {
        read_bytes((char*)&status.kp, sizeof(float));
        Serial.print("status.kp changed to ");
        Serial.println(status.kp);
      }
      break;
    case Request::SET_I:
      if(client.available() >= sizeof(float)) {
        read_bytes((char*)&status.ki, sizeof(float));
        Serial.print("status.ki changed to ");
        Serial.println(status.ki);
      }
      break;
    case Request::SET_D:
      if(client.available() >= sizeof(float)) {
        read_bytes((char*)&status.kd, sizeof(float));
        Serial.print("status.kd changed to ");
        Serial.println(status.kd);
      }
      break;
    case Request::SET_FEEDBACK:
      if(client.available() >= sizeof(uint8_t)) {
        read_bytes((char*)&status.feedback_enabled, sizeof(uint8_t));
        Serial.print("status.feedback_enabled changed to ");
        Serial.println(status.feedback_enabled);
        for(int i = 0; i < motors_amount; i++) integral_error[i] = 0;
      }
      break;
    case Request::GET_STATUS:
      char* status_bytes_buffer = (char*) malloc(STATUS_SIZE);
      status_to_bytes(status_bytes_buffer, status);
      write_bytes(status_bytes_buffer, STATUS_SIZE);
      Serial.println("Status dent");
      break;
  }
}

// Function to update motor inputs using feedback control
void update_motor_input() {
  for(int motor_id = 0; motor_id < motors_amount; motor_id++) {
    // Get motor speed
    motor_speed[motor_id] = -2.0f * 3.1415f * 1000000.0f * (float)(encoders[motor_id].getCount()) / ((float)status.feedback_loop_delay * (float)(incs_per_rotation));
    encoders[motor_id].clearCount();

    // Compute error, derivative, and integral
    float err = motor_req_speed[motor_id] - motor_speed[motor_id] * status.feedback_enabled;
    float der = err - last_error[motor_id];
    last_error[motor_id] = err;
    integral_error[motor_id] += err;

    // Calculate motor input using PID control
    current_input[motor_id]  = (status.kp * err + status.kd * der + status.ki * integral_error[motor_id]);
    

    // Clamp motor input within the allowed range
    if( current_input[motor_id] < -max_pwm)     current_input[motor_id]  = -max_pwm;
    if( current_input[motor_id] > max_pwm)     current_input[motor_id]  = max_pwm;

    // Power the motor
    power_motor(motor_id,     current_input[motor_id] );
  }
}

// Function to power a motor based on the input value
void power_motor(int motor_id, int32_t input_value) {
  if(input_value > 0) {
    ledcWrite(motor_id * 2, input_value);
    ledcWrite(motor_id * 2 + 1, 0);
  } else {
    ledcWrite(motor_id * 2, 0);
    ledcWrite(motor_id * 2 + 1, -input_value);
  }
}

// Function to read a specified number of bytes from the client
void read_bytes(char* buffer, uint32_t n) {
    for(uint32_t i = 0; i < n; i++) {
      buffer[i] = client.read();
    }
}

// Function to write a specified number of bytes to the client
void write_bytes(char* bytes, uint32_t n) {
    for(uint32_t i = 0; i < n; i++) {
      client.write(bytes[i]);
    }
}
