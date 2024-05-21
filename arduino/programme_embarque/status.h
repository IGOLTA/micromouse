const uint32_t STATUS_SIZE = 3*sizeof(float) + sizeof(uint32_t) + sizeof(uint8_t);

typedef struct Status {
  float kp;
  float kd;
  float ki;
  uint32_t feedback_loop_delay;
  uint8_t feedback_enabled; // 0 or 1
} Status;

void status_to_bytes(char* bytes_buffer, Status status) {
  uint32_t offset = 0;
  memcpy(&bytes_buffer[offset], &status.kp, sizeof(float));
  offset += sizeof(float);
  memcpy(&bytes_buffer[offset], &status.kd, sizeof(float));
  offset += sizeof(float);
  memcpy(&bytes_buffer[offset], &status.ki, sizeof(float));
  offset += sizeof(float);
  memcpy(&bytes_buffer[offset], &status.feedback_loop_delay, sizeof(uint32_t));
  offset += sizeof(uint32_t);
  memcpy(&bytes_buffer[offset], &status.feedback_enabled, sizeof(uint8_t));
}