use std::env;
use std::process;
use socketcan::{CANSocket, CANFrame};

//TODO: write help screen function
fn help_screen() {
    println!("This program sends a frame via given CAN socket");
}

/// Sets a single CAN frame on a given bus (does not support CAN-FD)
/// # Arguments
/// * 'args' - program arguments
///
/// # Examples
/// ```
/// // cansend  can0 123#cafe
/// ```
///
fn main() {
    //TODO: write lib-function for parsing the arguments, perhaps use 'claps'
    let args: Vec<String> = env::args().collect(); 
    if args.len() != 3 {
        println!("Incorrect number of arguments given!");
        help_screen();
        process::exit(1);
    } 
    let frame_string: Vec<String> = args[2].split("#")
            .map(|s| s.to_string()).collect();
    
    if frame_string.len() != 3 {
        println!("Something went wrong parsing the frame argument!");
        process::exit(1);
    }
    let can_socket_name: &String = &args[1];
    let can_socket: CANSocket = match CANSocket::open(can_socket_name) {
        Ok(socket) => socket,
        Err(error) => {
            println!("Given name of socket: {}", can_socket_name);
            println!("Could not open socket! Error: {}", error);
            help_screen();
            process::exit(1);
        }
    };
    let frame_id: u32 = frame_string[0].parse().unwrap();
    let mut rtr: bool = false;
    let mut eff: bool = false;
    if u32::pow(2, 11) - 1 < frame_id {
        // set extended ID flag in frame
        eff = true;
    }
    let frame_data: String = frame_string[2].to_owned();
    if frame_data == "R" {
        // set RTR flag in frame
        rtr = true;
    }
    let frame_data: &[u8] = frame_data.as_bytes();
    if frame_data.len() > 8 {
        println!("Too many data bytes given!");
        // can_socket.close();
        process::exit(1);
    }
    let frame: CANFrame = CANFrame::new(
            frame_id, frame_data, rtr, eff).expect("Error creating CANFrame!");

    // blocking write function
    match can_socket.write_frame_insist(&frame) {
        Ok(()) => {
            // can_socket.close();
            process::exit(0)
        }
        Err(error) => {
            println!("Error sending frame! Error: {}", error);
            process::exit(1)
        },
    }
}
