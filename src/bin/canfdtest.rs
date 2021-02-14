use clap::{App, Arg};
use log;
use std::process;
use socketcan::CANFrame;

const DEFAULT_INFLIGHT_COUNT: usize = 50;

fn increment_frame(frame: CANFrame) -> Option<CANFrame> {
    let frame_id: u32 = frame.id() + 1;
    let mut frame_data: Vec<u8> = vec![0; frame.data().len() as usize];
    frame_data[..].clone_from_slice(&frame.data());
    log::debug!("Frame data {:x?}", &frame_data);
    for i in 0..frame_data.len() {
        // handled attempt to add with overflow
        if frame_data[i] >= 255 {
            frame_data[i] = 0;
        } else {
            frame_data[i] += 1;
        }
    }
    match CANFrame::new(frame_id, &frame_data, false, false) {
        Ok(frame) => Some(frame),
        Err(_) => None,
    }
}

mod host {
    use log;
    use socketcan::{CANFrame, CANSocket, CANFilter};
    use std::fmt;
    use std::time::{Duration};
    use std::thread;

    const CAN_MSG_ID: u32 = 0x77;

    #[derive(Debug)]
    pub struct HostError {
        details: String
    }

    impl HostError {
        fn new(msg: &str) -> HostError {
            HostError{details: msg.to_string()}
        }
    }

    pub struct Host {
        socket: CANSocket,
        inflight_count: usize,
        loop_count: usize,
    }

    impl Host {

        fn compare_frame(expected_frame: CANFrame, received_frame: CANFrame, increment: usize) -> Result<bool, HostError> {
            if increment == 0 {
                if expected_frame.id() != received_frame.id() {
                    log::error!("Expected ID: {}, Received ID: {}", expected_frame.id(), received_frame.id());
                    return Err(HostError::new("ID mismatch!"));
                } else {
                    for i in 0..expected_frame.data().len() {
                        if expected_frame.data()[i] != received_frame.data()[i] {
                            log::error!("Expected data: {:x?}, Received data: {:x?}", expected_frame.data(), received_frame.data());
                            return Err(HostError::new("Data byte mismatch!"));
                        } else {
                            continue;
                        }
                    }
                }
                Ok(true)
             } else {
                 let new_expected_frame: CANFrame = match super::increment_frame(expected_frame) {
                     Some(f) => f,
                     None => return Err(HostError::new("Could not compare expteded receive from DUT!")),
                 };
                 if Host::compare_frame(new_expected_frame, received_frame, 0 as usize).is_err() {
                     Ok(false)
                 } else {
                     Ok(true)
                 }
             }
        }

        pub fn new(socket: &str, inflight_count: usize, loop_count: usize) -> Result<Host, HostError> {

            let can: CANSocket = match CANSocket::open(socket) {
                Ok(socket) => socket,
                Err(_) => return Err(HostError::new("Error opening socket!")),
            };

            //TODO: set sockopt to receive own frames
            let filter: CANFilter = match CANFilter::new(4, 1) {
                Ok(f) => {
                    log::debug!("Creating filter for redceiving own messages");
                    f
                },
                Err(e) => {
                    log::debug!("Could not create filter for receiving own messages!");
                    return Err(HostError::new("Failed to create filter for socket option"));
                },
            };
            match can.set_filter(&[filter]) {
                Ok(_) => {
                    log::debug!("Set up filter for receiving own messages");
                },
                Err(_) => {
                    log::error!("Error setting socket option for Host mode!");
                    return Err(HostError::new("Could not set filter for Host mode!"));
                },
            }

            let host = Host {
                socket: can,
                inflight_count: inflight_count,
                loop_count: loop_count,
            };
            Ok(host)
        }

        pub fn run(self) {
            let mut byte_counter: u8 = 0;
            let mut index: usize = 0;
            let mut tx_frames: Vec<CANFrame> = Vec::with_capacity(self.inflight_count);
            let mut response: Vec<bool> = Vec::with_capacity(self.inflight_count);
            // let mut unresponeded_count: usize = 0;

            loop {
                if response.len() < self.inflight_count {
                    response.push(false);
                    let mut data_bytes: [u8;8] = [0; 8];
                    for i in 0..data_bytes.len() {;
                        data_bytes[i] = byte_counter + i as u8;
                    }
                    let frame: CANFrame = match CANFrame::new(CAN_MSG_ID, &data_bytes[..], false, false) {
                        Ok(f) => f,
                        Err(_) => {
                            log::error!("Could not create frame for sending! At index {}", index);
                            break;
                        },
                    };
                    match self.socket.write_frame_insist(&frame) {
                        Ok(_) => {
                            tx_frames.push(frame);
                        },
                        Err(_) => {
                            log::error!("Could not send frame! Frame: {:x?} at index {}", &frame, index);
                            break;
                        },
                    }
                    // TODO: check this part particularly
                    if index + 1 == self.inflight_count {
                        index = 0;
                    } else {
                        index += 1;
                    }
                    byte_counter += 1;
                    if byte_counter % 33 == 0 {
                        thread::sleep(Duration::from_millis(3));
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }                  
                } else {
                    let received_frame: CANFrame = match self.socket.read_frame() {
                        Ok(frame) => {
                            log::debug!{"Received frame: {:x?}", &frame};
                            frame
                        },
                        Err(e) => {
                            log::error!("Error receiving frame: {}", e);
                            break;
                        },
                    };
                    // TODO: check own frame
                    if received_frame.id() == CAN_MSG_ID {
                        response[index] = match Host::compare_frame(tx_frames[index], received_frame, 0) {
                            Ok(result) => {
                                result
                            },
                            Err(_) => false,
                        };
                        if response[index] {
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        match Host::compare_frame(tx_frames[index], received_frame, 1) {
                            Ok(result) => {
                                if result {
                                    // loop_count += 1;
                                    continue;
                                } else {
                                    break;
                                }
                            },
                            Err(_) => break,
                        }
                        // if loop_count && self.loop_count >= loop_count {
                        //     break;
                        // } else {
                        //     index -= 1;
                        // }
                    }
                }
            }
        }
    }

    #[test]
    fn test_compare_self() {
        let test_frame: CANFrame = CANFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        assert!(Host::compare_frame(test_frame, test_frame, 0).unwrap());
    }

    #[test]
    fn test_compare_incremented_received() {
        let expected_frame: CANFrame = CANFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        let test_frame: CANFrame = CANFrame::new(0x78, &[2, 3, 4, 5, 6, 7, 8, 9], false, false)
            .unwrap();
        assert!(Host::compare_frame(expected_frame, test_frame, 1).unwrap());
    }

    impl fmt::Display for HostError {
        //TODO: todo
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.details)
        }
    }
}

mod dut {
    use log;
    use std::fmt;
    use socketcan::{CANFrame, CANSocket};

    pub struct Dut {
        socket: CANSocket,
    }
    
    const CAN_MSG_ID: u32 = 0x77;

    #[derive(Debug)]
    pub struct DutError {
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
                let byte: u8 = if frame.data()[i] == 0  {
                    0xff
                } else {
                    frame.data()[i] - 1
                };
                if frame.data()[i - 1] != byte {
                    log::debug!("Received data bytes: {:x?}", frame.data());
                    return Err(DutError::new("Received data byte mismatch!"));
                }
            }
            Ok(true)
        }
    }

    // Moved to parent module 'canfdtest'
    // fn increment_frame(frame: CANFrame) -> Option<CANFrame> {
    //     let frame_id: u32 = frame.id() + 1;
    //     let mut frame_data: Vec<u8> = vec![0; frame.data().len() as usize];
    //     frame_data[..].clone_from_slice(&frame.data());
    //     log::debug!("Frame data {:x?}", &frame_data);
    //     for i in 0..frame_data.len() {
    //         // handled attempt to add with overflow
    //         if frame_data[i] >= 255 {
    //             frame_data[i] = 0;
    //         } else {
    //             frame_data[i] += 1;
    //         }
    //     }
    //     match CANFrame::new(frame_id, &frame_data, false, false) {
    //         Ok(frame) => Some(frame),
    //         Err(_) => None,
    //     }
    // }

    impl Dut {
        pub fn new(socket_name: &str) -> Result<Dut, DutError> {
            let can: CANSocket = match CANSocket::open(socket_name) {
                Ok(socket) => socket,
                Err(_) => return Err(DutError::new("Could not open socket")),
            };
            let dut = Dut {
                socket: can,
            };
            Ok(dut)
        }

        pub fn run(self) {
            let mut frame_count: usize = 0;
            loop {
                let received_frame: CANFrame = match self.socket.read_frame() {
                    Ok(frame) => {
                        log::debug!{"Received frame: {:x?}", &frame};
                        frame_count += 1;
                        frame
                    },
                    Err(e) => {
                        log::error!("Error receiving frame: {}", e);
                        break;
                    },
                };
                match check_frame(received_frame) {
                    Ok(result) => {
                        if result {
                            let frame: CANFrame = match super::increment_frame(received_frame) {
                                None => {
                                    log::error!("Error incrementing frame for sending!");
                                    break;
                                },
                                Some(f) => f,
                            };
                            match self.socket.write_frame_insist(&frame) {
                                //TODO: implement wait time for interleaving mode, i.e. a wait time
                                Ok(_) => continue,
                                Err(e) => {
                                    log::error!("Error while writing frame! {}", e);
                                    break;
                                },
                            }
                        } else {
                            log::error!("Frame check did not pass!");
                            break;
                        }
                    },
                    Err(_) => log::error!("Error occured checking frame!"),
                };
            }
            log::info!("Received {} frames.", frame_count);
        }
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
        let incremented_frame: CANFrame = super::increment_frame(host_frame)
            .unwrap();
        
        assert_eq!(0x78, incremented_frame.id());
        assert_eq!(&[2, 3, 4, 5, 6, 7, 8, 9], incremented_frame.data());
    }

    #[test]
    fn test_partial_frame_increment() {
        // This should not occur during normal echo test, but it doesn't hurt to test it
        let host_frame: CANFrame = CANFrame::new(0x77, &[1, 2, 3, 4], false, false)
            .unwrap();
        let incremented_frame: CANFrame = super::increment_frame(host_frame)
            .unwrap();
        
        assert_eq!(0x78, incremented_frame.id());
        assert_eq!(&[2, 3, 4, 5], incremented_frame.data());
    }

    #[test]
    fn test_overflow_increment() {
        // test for problem fixed in commit '1af70af034f7c4c20ad63a5e3127875b9bee6533'
        let host_frame: CANFrame = CANFrame::new(0x77, &[0xf9, 0xfa , 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0x00], false, false)
            .unwrap();
        let incremented_frame: CANFrame = super::increment_frame(host_frame)
            .unwrap();

        assert_eq!(&[0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0x00, 0x01], incremented_frame.data());
    }
}

pub fn main() {
    let arg_matches = App::new("canfdtest")
                            .version("0.1.0")
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
                                    .takes_value(true)
                                    .requires("generator"),
                            )
                            .arg(
                                Arg::with_name("loop_count")
                                .help("test loop count")
                                .short("l")
                                .takes_value(true)
                                .requires("generator"),
                            )
                            .get_matches();
    
    //TODO: select logging framework... maybe fern or something else...
    match arg_matches.occurrences_of("verbosity") {
        0 => log::error!("I.O.U.  level setting"),
        1 => log::info!("I.O.U. logging level setting"),
        2 => log::warn!("I.O.U. logging level setting"),
        3 => log::debug!("I.O.U. logging level setting"),
        _ => log::error!("I.O.U. logging level setting"),
    }
    let socket_name: &str = match arg_matches.value_of("socket") {
        Some(s) => s,
        None => {
            log::error!("No valid program argument for socket given!");
            process::exit(1);
        },
    };
    if arg_matches.is_present("generator") {
        //TODO set socket options to receive own frames
        let host: host::Host = match host::Host::new(socket_name, DEFAULT_INFLIGHT_COUNT, 0) {
            Ok(h) => h,
            Err(e) => {
                log::error!("Could not instantiate Host/Generator! Reason: {}", e);
                process::exit(1);
            },
        };
        host.run();
        process::exit(0);
    } else {
        log::info!("Starting as DUT.");
        let dut: dut::Dut = match dut::Dut::new(socket_name) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Could not instantiate DUT! Reason: {}", e);
                process::exit(1);
            },
        };
        //TODO: give run Result as return type to exit program with exit code accordingly
        dut.run();
        process::exit(0);
    }
}
