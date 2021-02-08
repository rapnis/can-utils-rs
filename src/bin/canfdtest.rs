use clap::{App, Arg};
use log;
use socketcan::{CANFrame, CANSocket};

static CAN_MSG_SIZE: u32 = 8;

mod host {
    use socketcan::{CANFrame, CANSocket};
    
    const DEFAULT_INFLIGHT_COUNT: u32 = 50;
    const CAN_MSG_ID: u32 = 0x77;

}

mod dut {
    use std::fmt;
    use socketcan::{CANFrame, CANSocket};
    
    const CAN_MSG_ID: u32 = 0x77;

    #[derive(Debug)]
    struct DutError {
        details: String
    }

    impl DutError {
        fn new(msg: &str) -> DutError {
            DutError{details: msg.to_string()
            }
        }
    }

    impl fmt::Display for DutError {
        //TODO: todo
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.details)
        }
    }
    
    fn check_frame(frame: CANFrame) -> Result<bool, DutError> {
        if  frame.id() != CAN_MSG_ID {
            Err(DutError::new("Received message ID mismatch!"))
        } else {
            for i in 1..frame.data().len() {
                if (frame.data()[i - 1] + 1) != frame.data()[i] {
                    log::debug!("Received data bytes: {:x?}", frame.data());
                    return Err(DutError::new("Received data byte mismatch!"));
                }
            }
            Ok(true)
        }
    }

    fn increment_frame(frame: CANFrame) -> Option<CANFrame> {
        let frame_id: u32 = frame.id() + 1;
        let mut frame_data = vec![0; 8];
        frame_data[..].clone_from_slice(&frame.data());
        for i in 0..frame_data.len() {
            frame_data[i] += 1;
        }
        match CANFrame::new(frame_id, &frame_data, false, false) {
            Ok(frame) => Some(frame),
            Err(_) => None,
        }
    }

    pub fn can_echo(socket: CANSocket) {
        let mut frame_count: u32 = 0;


    }

    #[test]
    fn test_correct_frame_check() {
        let correct_frame: CANFrame = CANFrame::new(0x77, &[1, 2, 3], false, false)
            .unwrap();
        assert!(check_frame(correct_frame)
            .unwrap()
        );
    }

    #[test]
    fn test_false_id_frame_check() {
        let false_id_frame: CANFrame = CANFrame::new(0x123, &[1, 2, 3], false, false)
            .unwrap();
        assert_eq!(true, check_frame(false_id_frame)
            .is_err()
        );
    }

    #[test]
    fn test_false_data_frame_check() {
        let false_data_frame: CANFrame = CANFrame::new(0x77, &[1, 1, 3], false, false)
            .unwrap();
        assert_eq!(true, check_frame(false_data_frame)
            .is_err()
        );
    }

    #[test]
    fn test_frame_increment() {
        let host_frame: CANFrame = CANFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        let incremented_frame: CANFrame = increment_frame(host_frame)
            .unwrap();
        
        assert_eq!(0x78, incremented_frame.id());
        assert_eq!(&[2, 3, 4, 5, 6, 7, 8, 9], incremented_frame.data());
    }

    #[test]
    fn test_partial_frame_increment() {
        // This should not occur during normal echo test, but it doesn't hurt to test it
        let host_frame: CANFrame = CANFrame::new(0x77, &[1, 2, 3, 4], false, false)
            .unwrap();
        let incremented_frame: CANFrame = increment_frame(host_frame)
            .unwrap();
        
        assert_eq!(0x78, incremented_frame.id());
        assert_eq!(&[2, 3, 4, 5], incremented_frame.data());
    }
}

pub fn main() {
    let arg_matches = App::new("canfdtest")
                            .version("0.0.1")
                            .author("Raphael Nissl")
                            .about("Echoing CAN frames between a host and a DUT")
                            .arg(
                                Arg::with_name("socket")
                                    .help("Name of CAN socket")
                                    .index(1)
                                    .required(true),
                            )
                            .arg(
                                Arg::with_name("generator")
                                    .help("host flag")
                                    .short("g")
                                    .long("generator"),
                            )
                            .arg(
                                Arg::with_name("verbosity")
                                    .help("sets verbose level")
                                    .short("v")
                                    .long("verbose")
                                    .multiple(true),
                            )
                            .arg(
                                Arg::with_name("inflight")
                                    .help("inflight count")
                                    .short("f")
                                    .takes_value(true),
                            )
                            .arg(
                                Arg::with_name("loop_count")
                                .help("test loop count")
                                .short("l")
                                .takes_value(true),
                            )
                            .get_matches();
}