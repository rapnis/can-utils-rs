use clap::{App, Arg};
use log::LevelFilter;
use std::process;
use socketcan::CanFrame;
use simple_logger::SimpleLogger;

const DEFAULT_INFLIGHT_COUNT: usize = 50;

fn increment_frame(frame: CanFrame) -> Option<CanFrame> {
    let frame_id: u32 = frame.id() + 1;
    let mut frame_data: Vec<u8> = vec![0; frame.data().len() as usize];
    frame_data[..].clone_from_slice(&frame.data());
    for i in 0..frame_data.len() {
        // handled attempt to add with overflow
        if frame_data[i] >= 255 {
            frame_data[i] = 0;
        } else {
            frame_data[i] += 1;
        }
    }
    match CanFrame::new(frame_id, &frame_data, false, false) {
        Ok(frame) => Some(frame),
        Err(_) => None,
    }
}

mod host {
    use log;
    use socketcan::{CanFrame, CanSocket};
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
        socket: CanSocket,
        inflight_count: usize,
        frame_count: usize,
    }

    impl Host {

        fn compare_frame(expected_frame: CanFrame, received_frame: CanFrame, increment: usize) -> Result<bool, HostError> {
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
                 let new_expected_frame: CanFrame = match super::increment_frame(expected_frame) {
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

        pub fn new(socket: &str, inflight_count: usize, frame_count: usize) -> Result<Host, HostError> {

            let can: CanSocket = match CanSocket::open(socket) {
                Ok(socket) => socket,
                Err(_) => return Err(HostError::new("Error opening socket!")),
            };
            //TODO: set sockopt to receive own frames
            if let Err(_) = can.set_recv_own_msgs(true) {
                return Err(HostError::new("Could not set socket option to receive own messages!"));
            }

            let host = Host {
                socket: can,
                inflight_count: inflight_count,
                frame_count: frame_count,
            };
            Ok(host)
        }

        pub fn run(self) {
            let mut byte_counter: u8 = 0;
            let mut _loop_count: usize = 0;
            let mut tx_frames: Vec<CanFrame> = Vec::with_capacity(self.inflight_count); 
            let mut response: Vec<bool> = Vec::with_capacity(self.inflight_count);

            loop {
                if tx_frames.len() < self.inflight_count {
                    let mut data_bytes: [u8;8] = [0; 8];
                    for i in 0..data_bytes.len() {
                        let counted_bytes: usize = byte_counter as usize;
                        let byte: u8 = if counted_bytes + i > 255 {
                            let result: usize = counted_bytes + i - 256;
                            result as u8
                        } else {
                            byte_counter + i as u8
                        };
                        data_bytes[i] = byte;
                    }
                    let frame: CanFrame = match CanFrame::new(CAN_MSG_ID, &data_bytes[..], false, false) {
                        Ok(f) => f,
                        Err(_) => {
                            log::error!("Could not create frame for sending! At index {}", &tx_frames.len());
                            break;
                        },
                    };
                    match self.socket.write_frame_insist(&frame) {
                        Ok(_) => {
                            tx_frames.push(frame);
                        },
                        Err(_) => {
                            log::error!("Could not send frame! Frame: {:x?} at index {}", &frame, &tx_frames.len());
                            break;
                        },
                    }
                    if byte_counter == 255 {
                        byte_counter = 0;
                    } else {
                        byte_counter += 1;
                    }
                    if byte_counter % 33 == 0 {
                        thread::sleep(Duration::from_millis(3));
                    } else {
                        thread::sleep(Duration::from_millis(1));
                    }
                } else {
                    let received_frame: CanFrame = match self.socket.read_frame() {
                        Ok(frame) => {
                            log::debug!{"Received frame: {:x?}", &frame};
                            frame
                        },
                        Err(e) => {
                            log::error!("Error receiving frame: {}", e);
                            break;
                        },
                    };

                    //TODO: receiving own frame is possible with version 2.0.0 of crate socketcan (but does not build)
                    if received_frame.id() == CAN_MSG_ID {
                        log::debug!("Received own frame.");
                        let tx_frame: CanFrame = tx_frames[response.len()];
                        match Host::compare_frame(tx_frame, received_frame, 0) {
                            Ok(result) => {
                                response.push(result)
                            },
                            Err(_) => break,
                        }
                    } else {
                        if response.remove(0) {
                            log::debug!("Received DUT frame.");
                            let expected_frame: CanFrame = tx_frames.remove(0);
                            match Host::compare_frame(expected_frame, received_frame, 1) {
                                Ok(result) => {
                                    if result {
                                        // loop_count += 1;
                                        log::debug!("Frame comparison passed.");
                                        continue;
                                    } else {
                                        log::error!("Frame comparison failed!");
                                        break;
                                    }
                                },
                                Err(_) => break,
                            }
                        } else {
                            log::error!("Did not receive own frame! Rx before Tx!");
                            break;
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_compare_self() {
        let test_frame: CanFrame = CanFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        assert!(Host::compare_frame(test_frame, test_frame, 0).unwrap());
    }

    #[test]
    fn test_compare_incremented_received() {
        let expected_frame: CanFrame = CanFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        let test_frame: CanFrame = CanFrame::new(0x78, &[2, 3, 4, 5, 6, 7, 8, 9], false, false)
            .unwrap();
        assert!(Host::compare_frame(expected_frame, test_frame, 1).unwrap());
    }

    impl fmt::Display for HostError {
        //TODO: todo
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.details)
        }
    }

    #[test]
    fn test_false_incremented_received() {
        let expected_frame: CanFrame = CanFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        let test_frame: CanFrame = CanFrame::new(0x78, &[2, 3, 4, 5, 7, 8, 9, 10], false, false)
            .unwrap();
        assert_eq!(false, Host::compare_frame(expected_frame, test_frame, 1).unwrap());
    }
}

mod dut {
    use log;
    use std::fmt;
    use socketcan::{CanFrame, CanSocket};

    pub struct Dut {
        socket: CanSocket,
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
    
    fn check_frame(frame: CanFrame) -> Result<bool, DutError> {
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

    impl Dut {
        pub fn new(socket_name: &str) -> Result<Dut, DutError> {
            let can: CanSocket = match CanSocket::open(socket_name) {
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
                let received_frame: CanFrame = match self.socket.read_frame() {
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
                            let frame: CanFrame = match super::increment_frame(received_frame) {
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
        let correct_frame: CanFrame = CanFrame::new(0x77, &[1, 2, 3], false, false)
            .unwrap();
        assert!(check_frame(correct_frame)
            .unwrap()
        );
    }

    #[test]
    fn test_false_id_frame_check() {
        let false_id_frame: CanFrame = CanFrame::new(0x123, &[1, 2, 3], false, false)
            .unwrap();
        assert_eq!(true, check_frame(false_id_frame)
            .is_err()
        );
    }

    #[test]
    fn test_false_data_frame_check() {
        let false_data_frame: CanFrame = CanFrame::new(0x77, &[1, 1, 3], false, false)
            .unwrap();
        assert_eq!(true, check_frame(false_data_frame)
            .is_err()
        );
    }

    #[test]
    fn test_frame_increment() {
        let host_frame: CanFrame = CanFrame::new(0x77, &[1, 2, 3, 4, 5, 6, 7, 8], false, false)
            .unwrap();
        let incremented_frame: CanFrame = super::increment_frame(host_frame)
            .unwrap();
        
        assert_eq!(0x78, incremented_frame.id());
        assert_eq!(&[2, 3, 4, 5, 6, 7, 8, 9], incremented_frame.data());
    }

    #[test]
    fn test_partial_frame_increment() {
        // This should not occur during normal echo test, but it doesn't hurt to test it
        let host_frame: CanFrame = CanFrame::new(0x77, &[1, 2, 3, 4], false, false)
            .unwrap();
        let incremented_frame: CanFrame = super::increment_frame(host_frame)
            .unwrap();
        
        assert_eq!(0x78, incremented_frame.id());
        assert_eq!(&[2, 3, 4, 5], incremented_frame.data());
    }

    #[test]
    fn test_overflow_increment() {
        // test for problem fixed in commit '1af70af034f7c4c20ad63a5e3127875b9bee6533'
        let host_frame: CanFrame = CanFrame::new(0x77, &[0xf9, 0xfa , 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0x00], false, false)
            .unwrap();
        let incremented_frame: CanFrame = super::increment_frame(host_frame)
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
                                Arg::with_name("frame_count")
                                .help("test frame count")
                                .short("l")
                                .takes_value(true)
                                .requires("generator"),
                            )
                            .get_matches();
    
    match arg_matches.occurrences_of("verbosity") {
        0 => SimpleLogger::new()
                .with_level(LevelFilter::Error)
                .init()
                .unwrap(),
        1 => SimpleLogger::new()
                .with_level(LevelFilter::Info)
                .init()
                .unwrap(),
        2 => SimpleLogger::new()
                .with_level(LevelFilter::Warn)
                .init()
                .unwrap(),
        3 => SimpleLogger::new()
                .with_level(LevelFilter::Debug)
                .init()
                .unwrap(),
        _ => SimpleLogger::new()
                .with_level(LevelFilter::Error)
                .init()
                .unwrap(),
    }
    let socket_name: &str = match arg_matches.value_of("socket") {
        Some(s) => s,
        None => {
            log::error!("No valid program argument for socket given!");
            process::exit(1);
        },
    };
    if arg_matches.is_present("generator") {
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
