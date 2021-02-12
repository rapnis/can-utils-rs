# can-utils-rs

[can-utils](https://github.com/linux-can/can-utils "The famous original") rewritten in rust (mainly for learning purpose)

## cansend

use this command to send a frame via CAN with ```cansend <socket_name> <frame_id>#<data_bytes>```

#### Open Topics:  
- Source code documentation  
- Verbose outputs/Logging  

#### Tested on:  
- x86_64  

## canfdtest

Echoes frames between a host and a device under test. Sends frames with fixed length and continuous data bytes.  
Does not supprot CAN FD protocol.

Start as DUT: ```canfdtest <socke_name>```

#### Open Topics:
- Source code documentation  
- Verbose outputs/Logging  
- Host part of echo program  

#### DUT part tested on:
- x86_64 
