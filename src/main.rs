use rusb::{Device as UsbDevice, Context as UsbContext, DeviceDescriptor, DeviceHandle, Direction, TransferType, RequestType, Recipient};
use std::time::{Duration};
use std::thread::sleep;
use std::io;
use std::fs::File;
use nix::{ioctl_read_buf,ioctl_readwrite_buf};
use nix::errno::{errno};
use errno::Errno;
use std::os::unix::io::AsRawFd;
use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::env;
use std::str;
use nix::libc::{c_int};
use nix::libc;
use hex;
use glob::glob_with;
use glob::MatchOptions;
//use crate::errno::Errno;
use hexdump;

fn main() {
  println!("OBSBOT Meet 4K controller");
  let mut data = [ 0x0u8 ];
  let args: Vec<_> = env::args().collect();

  let mut camera = match open_camera(&args[2]) {
    Ok(camera) => camera,
    Err(err) => panic!("Can't find camera {:?}", err)
  };
  println!("Opened {:?}", camera);

  match &args[1][..] {
    "info" => info(&camera).expect("Failed"),
    "get" => dump(&camera).expect("Failed"),
    "cmd" => cmd(&camera, &args[3..]).expect("Failed"),
    "set" => set(&camera, &args[3..]).expect("Failed"),
    _ => panic!("Unknown command {:?}", args)
  }
}


// hint can be a name, e.g. /dev/video1
// or a partial name, e.g. video1
// or a partial string matching the driver or bus_info or card.
// The first match will be selected
// dev : & std::fs::File
fn open_camera(hint : &str) -> Result<std::fs::File, errno::Errno> {
//  match OpenOptions::new().nonblock(true).open(hint) {
//  match OpenOptions::new().custom_flags(nix::libc::O_NONBLOCK).open(hint) {
  match File::open(hint) {
    Ok(file) => return Ok(file),
    Err(err) => 0 // Why do we even need this line?
  };

  match File::open("/dev/".to_owned() + hint) {
    Ok(file) => return Ok(file),
    Err(err) => 0 // Why do we even need this line?
  };

  // enumerate all cameras and check for match
  let options = MatchOptions {
    case_sensitive: true,
    require_literal_separator: true,
    require_literal_leading_dot: true,
  };
  for entry in glob_with("/dev/video*", options).unwrap() {
    if let Ok(path) = entry {
      if let Ok(device) = File::open(&path) {
        if let Ok(video_info) = v4l2_capability::new(&device) {
          // println!("Info: {}\nCard: {:?}\nBus:  {:?}\ndc {:#X}", , video_info.card, video_info.bus_info, video_info.device_caps & 0x800000);
          if (str::from_utf8(&video_info.card).unwrap().find(&hint).is_some() ||
              str::from_utf8(&video_info.bus_info).unwrap().find(&hint).is_some()) &&
             (video_info.device_caps & 0x800000 == 0) 
          {
            return Ok(device);
          }
        }
      }
    }
  }
  return Err(errno::Errno(errno())); // Why do we even need this line?
}

fn info(camera : & std::fs::File) -> Result<(), Errno> {
  let mut query = [v4l2_capability { ..Default::default() }];

  unsafe {
    match ioctl_videoc_querycap(camera.as_raw_fd(), &mut query) {
      Ok(result) => {
        return Ok(());
      },
      _ => {
        println!("Failed");
        return Err(errno::Errno(errno()))
      }
    }
  }
}

fn dump(camera : & std::fs::File) -> Result<(), io::Error> {
  let result = dump_cur(&camera, 0x6, 0x6);
  match result {
    Ok(_) => return Ok(()),
    Err(error) => panic!("Error {:?}", error)
  }
}

fn cmd(camera : & std::fs::File, params : & [ String ]) -> Result<(), io::Error> {
  for cmd in params.iter() {
    let decoded = hex::decode(&cmd[..]).expect("Decoding failed");

    let result = send_cmd(&camera, 0x6, 0x6, &decoded);
    match result {
      Ok(_) => (),
      Err(error) => panic!("Error {:?}", error)
    }
  }
  return Ok(());
}

fn set(camera : & std::fs::File, params : & [ String ]) -> Result<(), io::Error> {
  let mut data = [ 0u8; 60 ];

  //hex::decode_to_slice(&params[0][..], &mut data).expect("Decoding failed");
  let mut decoded = hex::decode(&params[0][..]).expect("Decoding failed");

  data[..decoded.len()].copy_from_slice(&decoded);

  let result = set_cur(&camera, 0x6, 0x6, &mut data);
  match result {
    Ok(_) => return Ok(()),
    Err(error) => panic!("Error {:?}", error)
  }
}


// 00 01 00 disable replacement
// 00 01 01 enable replacement
// 00 01 02 enabe autoframeing

// 01 01 00 HDR off
// 01 01 01 HRD on

// 03 01 00 FACE AE OFF
// 03 01 01 FACE AE ON

// 04 01 02 //65 degree view
// 04 01 01 //78 degree view
// 04 01 00 //85 degree view

// 05 01 12 blur
// 05 01 01 solid 
// 05 01 11 replace

// 06 01xx xx->00-64 (100) blur level

// 07 01 00 button default
// 07 01 01 button rotation

// 0a 01 00 noise reduction off
// 0a 01 01 noise reduction on

// 10 01 00 blue
// 10 01 01 green

// 0d 02 00 ff auto-frame group
// 0d 02 01 00 auto-frame single close up
// 0d 02 01 01 auto-frame single upper-body

// 0e 02 00 00 pic 1
// 0e 02 00 01 pic 2
// 0e 02 00 02 pic 3

// 0b 02 1e00 autosleep 30s (seconds small byte first)
// 0b 02 7800 autosleep 2 min
// 0b 02 5802 autosleep 10 min


const CAMERA_BG_SOLID : [ u8 ; 3] = [ 0x05, 0x01, 0x01 ];
const CAMERA_BG_BITMAP : [ u8 ; 3] = [ 0x05, 0x01, 0x11 ];

const CAMERA_EFFECT_OFF : [ u8 ; 3] = [ 0x0, 0x01, 0x0 ];
const CAMERA_EFFECT_BG : [ u8 ; 3] = [ 0x0, 0x01, 0x1 ];
const CAMERA_EFFECT_TRACK : [ u8 ; 3] = [ 0x0, 0x01, 0x2 ];

#[repr(C)]
#[derive(Copy, Clone)]
#[derive(Default)]
#[derive(Debug)]
pub struct v4l2_capability {
  driver       : [u8; 16],
  card         : [u8; 32],
  bus_info     : [u8; 32],
  version      : u32,
  capabilities : u32,
  device_caps  : u32,
  reserved     :[u32 ; 3],
}

impl v4l2_capability {
  fn new(dev : & std::fs::File) -> Result<Self, errno::Errno> {
    let mut query = [v4l2_capability { ..Default::default() }];

    unsafe {
      match ioctl_videoc_querycap(dev.as_raw_fd(), &mut query) {
        Ok(result) => {
          return Ok(query[0]);
        },
        _ => {
          return Err(errno::Errno(errno()));
        }
      }
    }
  }
}

const VIDIOC_QUERYCAP_MAGIC: u8 = b'V';
const VIDIOC_QUERYCAP_QUERY_MESSAGE: u8 = 0x00;
ioctl_read_buf!(ioctl_videoc_querycap, VIDIOC_QUERYCAP_MAGIC, VIDIOC_QUERYCAP_QUERY_MESSAGE, v4l2_capability);

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
ioctl_read_buf!(uvcioc_ctrl_query_read, UVCIOC_CTRL_MAGIC, UVCIOC_CTRL_QUERY_MESSAGE, uvc_xu_control_query);

/* A.8. Video Class-Specific Request Codes */
const UVC_RC_UNDEFINED : u8  = 0x00;
const UVC_SET_CUR  : u8      = 0x01;
const UVC_GET_CUR  : u8      = 0x81;
const UVC_GET_MIN  : u8      = 0x82;
const UVC_GET_MAX  : u8      = 0x83;
const UVC_GET_RES  : u8      = 0x84;
const UVC_GET_LEN  : u8      = 0x85;
const UVC_GET_INFO : u8      = 0x86;
const UVC_GET_DEF  : u8      = 0x87;

fn uvc_io(dev : & std::fs::File, unit: u8, selector: u8, query: u8,  data : &mut [u8]) -> Result<c_int, Errno>  {
  let mut query = uvc_xu_control_query { 
    unit: unit,
    selector: selector,
    query: query,
    size: data.len() as u16,
    data: data.as_mut_ptr()
  };

  unsafe {
    match uvcioc_ctrl_query(dev.as_raw_fd(), &mut [query]) {
      Ok(result) => return Ok(result),
      _ => return Err(errno::Errno(errno()))
    }
  }
}

fn uvcioc_get_len(dev : & std::fs::File, unit: u8, selector: u8) -> Result<usize, Errno> {
  let mut data = [ 0u8; 2 ];

  return match uvc_io(dev, unit, selector, UVC_GET_LEN, &mut data) {
    Ok(_) => Ok(u16::from_le_bytes(data).into()),
    Err(err) => Err(err)
  }
}

fn get_cur(dev : &std::fs::File, unit : u8, selector : u8, data : &mut [ u8 ] ) -> Result<(), Errno> {
  // always call get_len first
  match uvcioc_get_len(&dev, unit, selector) {
    Ok(size) => {
      if data.len() < size {
        println!("Got size {}", size); 
        return Err(errno::Errno(1)) 
      }
    },
    Err(err) => return Err(err)
  };

  // Why not &mut data here?
  return match uvc_io(&dev, unit, selector, UVC_GET_CUR, data) {
    Ok(_) => return Ok(()),
    Err(err) => Err(err)
  }
}

fn dump_cur(dev : &std::fs::File, unit : u8, selector : u8) -> Result<(), Errno> {
  let mut data = [ 0u8; 60 ];
  get_cur(dev, unit, selector, &mut data)?;
  hexdump::hexdump(&data);
  return Ok(());
}

// Need non-mut version
fn set_cur(dev : &std::fs::File, unit : u8, selector : u8, data : &mut [u8]) -> Result<(), Errno> {
  match uvcioc_get_len(&dev, unit, selector) {
    Ok(size) => {
      if data.len() > size {
        println!("Got size {}", size); 
        return Err(errno::Errno(1)) 
      }
    },
    Err(err) => return Err(err)
  };

  return match uvc_io(&dev, unit, selector, UVC_SET_CUR, data) {
    Ok(_) => return Ok(()),
    Err(err) => Err(err)
  }
}

// Need non-mut version
fn send_cmd(dev : &std::fs::File, unit : u8, selector : u8, cmd : & [u8]) -> Result<(), Errno> {
  let mut data = [ 0u8; 60 ];
  data[..cmd.len()].copy_from_slice(&cmd);

  return set_cur(dev, unit, selector, &mut data);
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
