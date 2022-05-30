use rusb::{Device as UsbDevice, Context as UsbContext, DeviceDescriptor, DeviceHandle, Direction, TransferType, RequestType, Recipient};
use std::time::{Duration};
use std::thread::sleep;
use std::io;
use std::fs::File;
use nix::ioctl_readwrite_buf;
use nix::errno::Errno;
use std::os::unix::io::AsRawFd;
//use crate::errno::Errno;
use hexdump;

fn main() {
  println!("OBSBOT Meet 4K controller");
  //list_usb().expect("Failed");
  v4l_ioctl("/dev/video2").expect("Failed");
}

const CAMERA_BG_SOLID : [ u8 ; 3] = [ 0x05, 0x01, 0x01 ];
const CAMERA_BG_BITMAP : [ u8 ; 3] = [ 0x05, 0x01, 0x11 ];

const CAMERA_EFFECT_OFF : [ u8 ; 3] = [ 0x0, 0x01, 0x0 ];
const CAMERA_EFFECT_BG : [ u8 ; 3] = [ 0x0, 0x01, 0x1 ];
const CAMERA_EFFECT_TRACK : [ u8 ; 3] = [ 0x0, 0x01, 0x2 ];



#[repr(C)]
pub struct uvc_xu_control_query {
        unit :     u8,
        selector : u8,
        query :    u8,             /* Video Class-Specific Request Code, */
                                   /* defined in linux/usb/video.h A.8.  */
        size : u16,
        data : *mut u8,
}

const UVCIOC_CTRL_MAGIC: u8 = b'u';        // Defined in linux/uvcvideo.h
const UVCIOC_CTRL_QUERY_MESSAGE: u8 = 0x21; // Defined in linux/uvcvideo.h
ioctl_readwrite_buf!(uvcioc_ctrl_query, UVCIOC_CTRL_MAGIC, UVCIOC_CTRL_QUERY_MESSAGE, uvc_xu_control_query);

fn set_cur(dev : std::fs::File, _data : & [u8]) -> Result<i32, Errno> {
  let mut data = [ 0u8; 60 ];
  
  let mut query = uvc_xu_control_query { 
    unit: 0x6,
    selector: 0x6,
//    query: 0x81, // UVC_GET_CUR
    query: 0x85, // UVC_GET_LEN
    size: 2,
//    size: 0x60,
    data: data.as_mut_ptr()
  };
  unsafe {
    let result = uvcioc_ctrl_query(dev.as_raw_fd(), &mut [query]);
    println!("{} {}", data[0], data[1]);
  }

  data[.._data.len()].copy_from_slice(&_data);

  let mut query = uvc_xu_control_query { 
    unit: 0x6,
    selector: 0x6,
    query: 0x1, // UVC_GET_CUR
    size: 60,
    data: data.as_mut_ptr()
  };
  unsafe {
    let result = uvcioc_ctrl_query(dev.as_raw_fd(), &mut [query]);
    println!("{} {}", data[0], data[1]);
    result
  }

}

fn v4l_ioctl(video : &str) -> Result<u8, io::Error> {
  let mut camera = File::open(video)?;

//  let result = set_cur(camera, &CAMERA_BG_SOLID);
  let result = set_cur(camera, &CAMERA_EFFECT_BG);
  match result {
    Ok(n) => return Ok(u8::try_from(n).ok().unwrap()),
    Err(error) => panic!("Error {:?}", error)
  }
}

struct CameraConfig {
    data : [ u8; 64 ]
}

fn list_usb() -> Result<u8, rusb::Error> {
    for device in rusb::devices().unwrap().iter() {
        let device_desc = device.device_descriptor().unwrap();

        if device_desc.vendor_id() == 0x6e30 &&
           device_desc.product_id() ==  0xfef3 {
            println!("Bus {:03} Device {:03} ID {:04x}:{:04x}",
                device.bus_number(),
                device.address(),
                device_desc.vendor_id(),
                device_desc.product_id());

            let mut camera = match device.open() {
              Ok(handle) => handle,
              Err(error) => panic!("Can't open USB device: {:?}", error),
              // Err(error) => return Err(error),
            };

            println!("Opened device {:}, {:}", camera.read_product_string_ascii(&device_desc)?, 
                                               camera.read_manufacturer_string_ascii(&device_desc)?);
            println!("Version {}", device_desc.device_version());
//            println!("Serial: {:}",            camera.read_serial_number_string_ascii(&device_desc)?);
//            println!("Serial: {:}", device_desc.serial_number_string_index().ok_or(rusb::Error::NotSupported)?);

//            for index in 1..u8::MAX {
//                match camera.read_string_descriptor_ascii(index) {
//                  Ok(text) => println!(">> {:} = {:}", index, text),
//                  Err(_) => break,
//                }
//            }

/////            camera.reset()?;
            let active_config = camera.active_configuration()?;
            println!("Active configuration: {}", active_config);
            let config_desc = device.config_descriptor(0)?;
            println!("Config desc: {}", active_config);

            camera.set_auto_detach_kernel_driver(true)?;
            println!("Claim");
            camera.claim_interface(0)?;
            println!("Active");

//            sleep(Duration::from_millis(10000));
//            get_config(&camera);
            let mut buff = [0x00u8, 0x00u8 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };
            println!("Size {}, data {:?}", size, buff);
            let mut data = [ 0u8; 64 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x81, 0x600, 0x600, &mut data, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };
            hexdump::hexdump(&data);
            println!("Size {}, data {:?}", size, data);


            println!("Setting.....");
            sleep(Duration::from_millis(1000));

            let mut buff = [0x00u8, 0x00u8 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };

            let mut data = [ 0u8; 60 ];

            data[0]=4;
            data[1]=1;
            data[2]=2;
//            data[3]=1;
// 5,1,1 is solid background
// 5,1,17 is picture bg
// 5,1,18 is blur

// 16,1,4 is white
// 16,1,3 is black
// 16,1,2 is red
// 16,1,1 is green
// 16,1,0 is blue

// 0,1,0 is disable effects
// 0,1,1 is enable bg
// 0,1,2 is zoom

// e 2 0 1 pic 2
// e 2 0 0 pic 1

// 050112 blur
// 050101 solid 
// 050111 replace
// 100100 blue
// 100101 green
// 0e020000 pic 1
// 0e020001 pic 2
// 0e020002 pic 3
// 000100 disable replacement
// 000101 enable replacement
// 000102 enabe autoframeing
// 0d0200ff auto-frame group
// 0d020100 auto-frame single close up
// 0d020101 auto-frame single upper-body
// 040102 65 degree view
// 040101 78 degree view
// 040100 85 degree view
// 010100 HDR off
// 010101 HRD on
// 030100 FACE AE OFF
// 030101 FACE AE ON
// 070100 button default
// 070101 button rotation
// 0a0100 noise reduction off
// 0a0101 noise reduction on
// 0b021e00 autosleep 30s
// 0b027800 autosleep 2 min
// 0b025802 autosleep 10 min
// 0601xx xx->00-64 (100) blur level

const BG_VIRTUAL_SOLID: [ u8; 60 ] = [ 0x05, 0x01, 0x01, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const BG_VIRTUAL_BG: [ u8; 60 ] = [ 0x05, 0x01, 0x11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const BG_VIRTUAL_BLUR: [ u8; 60 ] = [ 0x05, 0x01, 0x12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

//let data = BG_VIRTUAL_BLUR;


// zoom is set with control selector b entity 1 wlength 2 for 2 byte number (zoom absolute)
// 0904 ff 02 ff 02 centre
// control unit 4 used for bg

            println!("Size {}, data {:?}", size, buff);
            let config_request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
            let size = match camera.write_control(config_request_type, 0x1, 0x600, 0x600, &data, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't set config: {:?}", error),
            };
            println!("Size {}, data {:?}", size, data);

            let mut buff = [0x00u8, 0x00u8 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };
            println!("Size {}, data {:?}", size, buff);
            let mut data = [ 0u8; 64 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x81, 0x600, 0x600, &mut data, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };
            hexdump::hexdump(&data);
            println!("Size {}, data {:?}", size, data);

            camera.reset()?;
        }
        // open_device_with_vid_pid     6e30 fef3
    }
    return Ok(0)
}

fn get_config(camera : &DeviceHandle<rusb::Context>) {
            let mut buff = [0x00u8, 0x00u8 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };
            println!("Size {}, data {:?}", size, buff);

            let mut data = [ 0u8; 64 ];
            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
            let size = match camera.read_control(config_request_type, 0x81, 0x600, 0x600, &mut data, Duration::from_millis(1000)) {
              Ok(size) => size,
              Err(error) => panic!("Can't get config: {:?}", error),
            };
            println!("Size {}, data {:?}", size, data);
}


/*
/// Internal endpoint representations
#[derive(Debug, PartialEq, Clone)]
struct Endpoint {
    config: u8,
    iface: u8,
    setting: u8,
    address: u8
}
        let (mut write, mut read) = (None, None);

        for interface in config_desc.interfaces() {
            for interface_desc in interface.descriptors() {
                for endpoint_desc in interface_desc.endpoint_descriptors() {

                    // Create an endpoint container
                    let e = Endpoint {
                        config: config_desc.number(),
                        iface: interface_desc.interface_number(),
                        setting: interface_desc.setting_number(),
                        address: endpoint_desc.address(),
                    };

                    println!("Endpoint: {:?}", e);

                    // Find the relevant endpoints
                    match (endpoint_desc.transfer_type(), endpoint_desc.direction()) {
                        (TransferType::Bulk, Direction::In) => read = Some(e),
                        (TransferType::Bulk, Direction::Out) => write = Some(e),
                        (_, _) => continue,
                    }
                }
            }
        }
*/
