# can-utils-rs
[can-utils](https://github.com/linux-can/can-utils "The famous original") rewritten in rust (mainly for learning purpose)

## NIGHTLY
Uses version 2.0.0 of `socketcan-rs` repository. Said version is not available via published crate, uses also a fork of `netlink-rs` repository that does not build due missing type annotations.  
This branch uses forks of said projects to be buildable.

## cansend

use this command to send a frame via CAN with ```cansend <socket_name> <frame_id>#<data_bytes>```  
Examples: ```cansend can0 008#R``` ```cansend can0 10000#cafeaffe```

#### Open Topics:  
- Source code documentation  
- Verbose outputs/Logging  
- Parse frame-id part as hex not as decimal

#### Tested on:  
- x86_64  

## canfdtest

Echoes frames between a host and a device under test. Sends frames with fixed length and continuous data bytes.  
Does not supprot CAN FD protocol.
DEVIATION TO ORIGINAL: Host does not receive own messages after sending! (This is currently not supported)

Start as DUT: ```canfdtest <socket_name>```
Start as Host: ```canfdtest <socket_name> -g``` (other flags are not supported as of now)

#### Open Topics:
- Source code documentation
- Progress printing

#### DUT part tested on:
- x86_64 
