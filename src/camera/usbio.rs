use enum_dispatch::enum_dispatch;
use rusb::{Direction, RequestType, Recipient, DeviceHandle};
use std::time::{Duration};
use std::fs::File;
use nix::{ioctl_read_buf,ioctl_readwrite_buf};
use nix::errno::{errno};
use errno::Errno;
use std::os::unix::io::AsRawFd;
//use std::fs::OpenOptions;
//use std::os::unix::fs::OpenOptionsExt;
use std::str;
use glob::glob_with;
use glob::MatchOptions;

#[enum_dispatch(CameraHandleType)]
pub trait UvcUsbIo {
  fn info(&self) -> Result<(), Errno>;
  fn io(&self, unit: u8, selector: u8, query: u8,  data : &mut [u8]) -> Result<(), Errno>;
}

#[derive(Debug)]
pub struct UvcCameraHandle {
  handle : std::fs::File,
}

impl UvcUsbIo for UvcCameraHandle {
  fn info(&self) -> Result<(), Errno> {
    match v4l2_capability::new(&self.handle) {
      Ok(video_info) => {
        println!("Card: {}\nBus : {}",
                 str::from_utf8(&video_info.card).unwrap(),
                 str::from_utf8(&video_info.bus_info).unwrap());
        return Ok(());
      },
      _ => {
        println!("Failed");
        return Err(errno::Errno(errno()))
      }
    }
  }

  fn io(&self, unit: u8, selector: u8, query: u8,  data : &mut [u8]) -> Result<(), Errno>  {
    let dev = &self.handle;

    let query = uvc_xu_control_query {
      unit: unit,
      selector: selector,
      query: query,
      size: data.len() as u16,
      data: data.as_mut_ptr()
    };

    unsafe {
      match uvcioc_ctrl_query(dev.as_raw_fd(), &mut [query]) {
        Ok(_) => return Ok(()),
        _ => return Err(errno::Errno(errno()))
      }
    }
  }
}

#[derive(Debug)]
pub struct UsbCameraHandle {
  handle : DeviceHandle<rusb::GlobalContext>,
}

impl UvcUsbIo for UsbCameraHandle {
  fn info(&self) -> Result<(), Errno> {
    let camera = &self.handle;

    let device_desc=camera.device().device_descriptor().unwrap();
    let product = match camera.read_product_string_ascii(&device_desc) {
      Ok(text) => text,
      Err(_) => "unknown".to_string()
    };
    let manufacturer = match camera.read_manufacturer_string_ascii(&device_desc) {
      Ok(text) => text,
      Err(_) => "unknown".to_string()
    };
    println!("Opened device {:04x}:{:04x} {:}, {:}", device_desc.vendor_id(), device_desc.product_id(),
                                             product, manufacturer);
    return Ok(())
  }

  fn io(&self, unit: u8, selector: u8, query: u8,  data : &mut [u8]) -> Result<(), Errno>  {
    let camera = &self.handle;
    let unit : u16 = (unit as u16) << 8;
    let selector : u16 = (selector as u16) << 8;

    if query < 128 {
      let config_request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
      let size = match camera.write_control(config_request_type, query, unit, selector, &data, Duration::from_millis(1000)) {
        Ok(size) => size,
        Err(error) => panic!("Can't set config: {:?}", error),
      };
      println!("Size {}, data {:?}", size, data);
    } else {
      let config_request_type =  rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
      match camera.read_control(config_request_type, query, unit, selector, data, Duration::from_millis(1000)) {
        Ok(size) => size,
        Err(error) => panic!("Can't set config: {:?}", error),
      };
    }
    Ok(())
  }
}
#[derive(Debug)]
#[enum_dispatch]
pub enum CameraHandleType {
 UvcCameraHandle(UvcCameraHandle),
 UsbCameraHandle(UsbCameraHandle)
}

impl CameraHandle {
  pub fn info(&self) -> Result<(), Errno> {
    match &self.camera_handle {
      CameraHandleType::UvcCameraHandle(handle) => handle.info(),
      CameraHandleType::UsbCameraHandle(handle) => handle.info(),
    }
  }

  pub fn io(&self, unit: u8, selector: u8, query: u8,  data : &mut [u8]) -> Result<(), Errno> {
    match &self.camera_handle {
      CameraHandleType::UvcCameraHandle(handle) => handle.io(unit, selector, query, data),
      CameraHandleType::UsbCameraHandle(handle) => handle.io(unit, selector, query, data),
    }
  }
}

#[derive(Debug)]
pub struct CameraHandle {
  pub camera_handle : CameraHandleType
}

// hint can be a name, e.g. /dev/video1
// or a partial name, e.g. video1
// or a partial string matching the driver or bus_info or card.
// The first match will be selected
pub fn open_camera(hint: &str) -> Result<CameraHandle, errno::Errno> {
  match uvc_open_camera(& hint) {
    Ok(file) => {
      return Ok(CameraHandle { camera_handle : CameraHandleType::UvcCameraHandle( UvcCameraHandle { handle: file } ) });
    },
    Err(_) => match usb_open_camera(& hint) {
      Ok(dev) => return Ok(CameraHandle { camera_handle : CameraHandleType::UsbCameraHandle( UsbCameraHandle { handle: dev } ) }),
      Err(error) => panic!("Can't open {} {:?}", hint, error),
    }
  }
}

fn uvc_open_camera(hint : &str) -> Result<std::fs::File, errno::Errno> {
  match File::open(hint) {
    Ok(file) => return Ok(file),
    Err(_) => 0 // Why do we even need this line?
  };

  match File::open("/dev/".to_owned() + hint) {
    Ok(file) => return Ok(file),
    Err(_) => 0 // Why do we even need this line?
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
        Ok(_) => {
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
#[allow(dead_code)] const UVC_RC_UNDEFINED : u8  = 0x00;
#[allow(dead_code)] pub const UVC_SET_CUR  : u8      = 0x01;
#[allow(dead_code)] pub const UVC_GET_CUR  : u8      = 0x81;
#[allow(dead_code)] const UVC_GET_MIN  : u8      = 0x82;
#[allow(dead_code)] const UVC_GET_MAX  : u8      = 0x83;
#[allow(dead_code)] const UVC_GET_RES  : u8      = 0x84;
#[allow(dead_code)] pub const UVC_GET_LEN  : u8      = 0x85;
#[allow(dead_code)] const UVC_GET_INFO : u8      = 0x86;
#[allow(dead_code)] const UVC_GET_DEF  : u8      = 0x87;

/*
fn send_cmds(dev : &std::fs::File, unit : u8, selector : u8, cmds : & [& [ u8]]) -> Result<(), Errno> {
  for cmd in cmds {
    match send_cmd(dev, unit, selector, cmd) {
      Ok(_) => (),
      Err(error) => return Err(error)
    }
  }
  return Ok(())
}


fn send_cmds_p(dev : &std::fs::File, unit : u8, selector : u8, cmds : & [& [ u8]], p : & [u8]) -> Result<(), Errno> {
  match send_cmd_p(dev,  unit, selector, &cmds[0], &p) {
      Ok(_) => (),
      Err(error) => return Err(error)
  }

  match send_cmds(dev,  unit, selector, &cmds[1..]) {
      Ok(_) => (),
      Err(error) => return Err(error)
  }
  return Ok(())
}
*/

pub fn usb_open_camera(hint : &str) -> Result<DeviceHandle<rusb::GlobalContext>, rusb::Error> {
  for device in rusb::devices().unwrap().iter() {
    let device_desc = device.device_descriptor().unwrap();

    match device.open() {
      Ok(mut camera) => {
        let product = match camera.read_product_string_ascii(&device_desc) {
          Ok(text) => text,
          Err(_) => "unknown".to_string()
        };
        let manufacturer = match camera.read_manufacturer_string_ascii(&device_desc) {
          Ok(text) => text,
          Err(_) => "unknown".to_string()
        };

        if format!("{:04x}:{:04x}", device_desc.vendor_id(), device_desc.product_id()).eq(hint) || product.find(&hint).is_some() || manufacturer.find(&hint).is_some()
        {
          camera.set_auto_detach_kernel_driver(true)?;
          camera.claim_interface(0)?;

          return Ok(camera);
        }
      },
      _ => (),
    }
  }
  return Err(rusb::Error::NoDevice);
}

//            println!("Serial: {:}",            camera.read_serial_number_string_ascii(&device_desc)?);
//            println!("Serial: {:}", device_desc.serial_number_string_index().ok_or(rusb::Error::NotSupported)?);

//            for index in 1..u8::MAX {
//                match camera.read_string_descriptor_ascii(index) {
//                  Ok(text) => println!(">> {:} = {:}", index, text),
//                  Err(_) => break,
//                }
//            }

/////            camera.reset()?;
//            let active_config = camera.active_configuration()?;
//            println!("Active configuration: {}", active_config);
//            let config_desc = device.config_descriptor(0)?;
//            println!("Config desc: {:?}", config_desc);

//            camera.set_auto_detach_kernel_driver(true)?;
//            println!("Claim");
//            camera.claim_interface(0)?;
//            println!("Active");

//            sleep(Duration::from_millis(10000));
//            get_config(&camera);
            //let mut buff = [0x00u8, 0x00u8 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };
//            println!("Size {}, data {:?}", size, buff);
//            let mut data = [ 0u8; 64 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x81, 0x600, 0x600, &mut data, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };
//            hexdump::hexdump(&data);
//            println!("Size {}, data {:?}", size, data);


//            println!("Setting.....");
//            sleep(Duration::from_millis(1000));

//            let mut buff = [0x00u8, 0x00u8 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };



// zoom is set with control selector b entity 1 wlength 2 for 2 byte number (zoom absolute)
// 0904 ff 02 ff 02 centre
// control unit 4 used for bg
//
//            println!("Size {}, data {:?}", size, buff);
//            let config_request_type = rusb::request_type(Direction::Out, RequestType::Class, Recipient::Interface);
//            let size = match camera.write_control(config_request_type, 0x1, 0x600, 0x600, &data, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't set config: {:?}", error),
//            };
//            println!("Size {}, data {:?}", size, data);
//
//            let mut buff = [0x00u8, 0x00u8 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };
//            println!("Size {}, data {:?}", size, buff);
//            let mut data = [ 0u8; 64 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x81, 0x600, 0x600, &mut data, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };
//            hexdump::hexdump(&data);
//            println!("Size {}, data {:?}", size, data);
//
//            camera.reset()?;
//        }
//        // open_device_with_vid_pid     6e30 fef3
//    }
//    return Ok(0)
//}

//fn get_config(camera : &DeviceHandle<rusb::Context>) {
//            let mut buff = [0x00u8, 0x00u8 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x85, 0x600, 0x600, &mut buff, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };
//            println!("Size {}, data {:?}", size, buff);
//
//            let mut data = [ 0u8; 64 ];
//            let config_request_type = rusb::request_type(Direction::In, RequestType::Class, Recipient::Interface);
//            let size = match camera.read_control(config_request_type, 0x81, 0x600, 0x600, &mut data, Duration::from_millis(1000)) {
//              Ok(size) => size,
//              Err(error) => panic!("Can't get config: {:?}", error),
//            };
//            println!("Size {}, data {:?}", size, data);
//}


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
