use clap::{App, Arg};
use log::LevelFilter;
use socketcan::{CANFrame, CANSocket};
use std::{convert::TryInto, process};
use simple_logger::SimpleLogger;

const CAN_MSG_SIZE: usize = 8;

//TODO implement own error values to return
fn data_string_to_hex(input: String) -> Option<Vec<u8>> {
    let str: &str  = &input[..];
    if str.len() % 2 != 0 || str.len() > (CAN_MSG_SIZE * 2) {
        None
    } else {
        let result: Vec<u8> = (0..str.len())
            .step_by(2)
            .map(|o| u8::from_str_radix(&str[o..o + 2], 16)
            .unwrap())
            .collect();
        Some(result)
    }
}

fn id_string_to_hex(input: String) -> Option<u32> {
    let id_string: String;
    if input.len() % 2 != 0 {
        id_string = format!("{}{}", "0", input);
    } else {
        id_string = input;
    }
    let mut result: Vec<u8> = (0..id_string.len())
        .step_by(2)
        .map(|o| u8::from_str_radix(&id_string[o..o + 2], 16)
        .unwrap())
        .collect();
    if result.len() != 4 {
        for _i in 0..(4 - result.len()) {
            result.insert(0, 0);
        }
        log::debug!("Result id parsing: {:x?}", result);
    } else {
        log::debug!("Result id parsing: {:x?}", result);
    }
    Some(u32::from_be_bytes(result[..].try_into().unwrap()))
}

//TODO implement own error values to return
fn parse_frame_string(frame_string: String) -> Option<CANFrame> {
    let frame_tokens: Vec<String> = frame_string
        .split("#")
        .map(|s| s.to_string())
        .collect();
    log::debug!("Frame tokens: {:?}", frame_tokens);
    if frame_tokens.len() != 2 {
        return None;
    }
    let frame_id: u32 = id_string_to_hex(frame_tokens[0].to_owned()).unwrap();
    let frame_data: String = frame_tokens[1].to_owned();
    if frame_data == "R" {
        // set RTR flag in frame
        let frame: CANFrame = 
            CANFrame::new(frame_id, &[], true, false)
                .expect("Error creating CAN-Remote-Frame");
        Some(frame)
    } else {
        let data_bytes: &[u8] = &(data_string_to_hex(frame_data).unwrap())[..];
        log::debug!("Frame bytes: {:x?}", data_bytes);
        let frame: CANFrame =
            CANFrame::new(frame_id, data_bytes, false, false)
                .expect("Error creating CAN-Frame!");
        Some(frame)
    }
}

#[test]
fn test_frame_parsing() {
    let test_frame: String = "123#cafe"
        .to_owned();
    let exptected_frame: CANFrame = CANFrame::new(0x123, &[0xca, 0xfe], false, false)
        .unwrap();
    let created_frame: CANFrame = parse_frame_string(test_frame)
        .unwrap();
    assert_eq!(exptected_frame.id(), created_frame.id());
    assert_eq!(exptected_frame.is_extended(), created_frame.is_extended());
    assert_eq!(exptected_frame.is_rtr(), created_frame.is_rtr());
    for i in 0..exptected_frame.data().len() {
        assert_eq!(exptected_frame.data()[i], created_frame.data()[i]);
    }
}

#[test]
fn test_frame_parsing_remote() {
    let test_frame: String = "444#R"
        .to_owned();
    let exptected_frame: CANFrame = CANFrame::new(0x444, &[], true, false)
        .unwrap();
    let created_frame: CANFrame = parse_frame_string(test_frame)
        .unwrap();
    assert_eq!(exptected_frame.id(), created_frame.id());
    assert_eq!(exptected_frame.is_rtr(), created_frame.is_rtr());
    assert_eq!(exptected_frame.data().len(), created_frame.data().len());
}

#[test]
fn test_frame_parsing_extended() {
    let test_frame: String = "111fff#0102030405060708"
        .to_owned();
    let exptected_frame: CANFrame = CANFrame::new(0x111fff, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08], false, false)
        .unwrap();
    let created_frame: CANFrame = parse_frame_string(test_frame)
        .unwrap();
    assert_eq!(exptected_frame.id(), created_frame.id());
    assert_eq!(exptected_frame.is_extended(), created_frame.is_extended());
    assert!(created_frame.is_extended());
    assert_eq!(exptected_frame.is_rtr(), created_frame.is_rtr());
    for i in 0..exptected_frame.data().len() {
        assert_eq!(exptected_frame.data()[i], created_frame.data()[i]);
    }
}

#[test]
fn test_id_parsing_even() {
    let frame_id: u32 = id_string_to_hex("ff7123".to_owned()).unwrap();
    assert_eq!(0xff7123, frame_id);
}

#[test]
fn test_id_parsing_odd() {
    let frame_id: u32 = id_string_to_hex("1ff7123".to_owned()).unwrap();
    assert_eq!(0x1ff7123, frame_id);
}


/// Sets a single CAN frame on a given bus (does not support CAN-FD)
/// # Arguments
/// * 'args' - program arguments
///
/// # Examples
/// ```
/// cansend  can0 123#cafe
/// ```
///
fn main() {
    let arg_matches = App::new("cansend")
                            .version("0.1.1")
                            .author("Raphael Nissl")
                            .about("Program sets a CAN-Frame on a bus with given ID and data (does not support CAN FD protocol)")
                            .arg(
                                Arg::with_name("socket")
                                    .help("name of CAN socket")
                                    .index(1)
                                    .requires("frame")
                                    .required(true),
                            )
                            .arg(
                                Arg::with_name("frame")
                                    .help("Frame consisting of ID and data")
                                    .index(2)
                                    .required(true),
                            )
                            .get_matches();

    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let can_socket_name: &str = match arg_matches.value_of("socket") {
        Some(s) => s,
        None => {
            log::error!("No valid socket-name given!");
            process::exit(1);
        },
    };
    let can_socket: CANSocket = match CANSocket::open(can_socket_name) {
        Ok(socket) => socket,
        Err(error) => {
            log::debug!("Given name of socket: {}", can_socket_name);
            log::error!("Could not open socket! Error: {}", error);
            process::exit(1);
        }
    };
    let frame_string: String = arg_matches
        .value_of("frame")
        .unwrap()
        .to_owned();

    let frame: CANFrame = match parse_frame_string(frame_string) {
        Some(frame) => frame,
        None => {
            log::error!("Error parsing frame-string!");
            process::exit(1);
        },
    };
    // blocking write function
    match can_socket.write_frame_insist(&frame) {
        Ok(()) => {
            // socket will be closed on deallocation so nothing to do here
            process::exit(0)
        }
        Err(error) => {
            log::error!("Error sending frame! Error: {}", error);
            process::exit(1)
        }
    }
}
