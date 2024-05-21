typedef enum Request {
  SET_SAMPLE_COUNT = 0x00,
  SET_ACQ_SAMPLE_RATE = 0x01,
  SET_INPUT_SPEED = 0x02,
  LAUNCH_AQ = 0x03,
  SET_FEEDBACK_SAMPLE_RATE = 0x04,
  SET_P = 0x05,
  SET_I = 0x06,
  SET_D = 0x07,
  GET_STATUS = 0x08,
  SET_FEEDBACK = 0x09
}Request;