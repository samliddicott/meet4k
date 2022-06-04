use rusb::{Device as UsbDevice, Context as UsbContext, DeviceDescriptor, DeviceHandle, Direction, TransferType, RequestType, Recipient};
use std::time::{Duration};
use std::thread::sleep;
use std::io;
use std::fs::File;
use nix::{ioctl_read_buf,ioctl_readwrite_buf};
use nix::errno::{errno};
use errno::Errno;
use std::os::unix::io::AsRawFd;
//use std::fs::OpenOptions;
//use std::os::unix::fs::OpenOptionsExt;
use std::env;
use std::str;
use nix::libc::{c_int};
//use nix::libc;
use hex;
use glob::glob_with;
use glob::MatchOptions;
//use crate::errno::Errno;
use hexdump;

fn main() {
  println!("OBSBOT Meet 4K controller");
  let args: Vec<_> = env::args().collect();

  let camera = match Camera::new(&args[1]) {
    Ok(camera) => camera,
    Err(err) => panic!("Can't find camera {:?}", err)
  };
  println!("Opened {:?}", camera);

  cmds(&camera, &args[2..]);
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

fn info(camera : & Camera) -> Result<(), Errno> {
  let dev = &camera.dev;
  match v4l2_capability::new(&dev) {
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

fn dump(camera : & Camera) -> Result<(), Errno> {
  let dev = &camera.dev;
  let result = dump_cur(&dev, 0x6, 0x6);
  match result {
    Ok(_) => return Ok(()),
    Err(error) => panic!("Error {:?}", error)
  }
}

fn cmds(camera : & Camera, cmds : & [ String ]) -> Result<(), Errno> {
  for c in cmds {
    match cmd(camera, c) {
      Ok(_) => (),
      Err(error) => return Err(error),
    }
  }
  return Ok(());
}

fn cmd(camera : & Camera, cmd : &str) -> Result<(), Errno> {
  let mut parts = cmd.splitn(2, '=');
  let cmd = parts.next().unwrap();
  let arg = parts.next();

  let result = match cmd {
    "info" => info(&camera),
    "get" => dump(&camera),

    "effect-off" => camera.effect_off(),
    "effect-bg" => camera.effect_bg(),
    "effect-track" => camera.effect_track(),
    "hdr-off" => camera.hdr_off(),
    "hdr-on" => camera.hdr_on(),
    "face-ae-off" => camera.face_ae_off(),
    "face-ae-on" => camera.face_ae_on(),
    "angle-65" => camera.angle_65(),
    "angle-78" => camera.angle_78(),
    "angle-85" => camera.angle_85(),
    "bg-solid" => camera.bg_solid(),
    "bg-solid!" => camera.bg_solid_now(),
    "bg-bitmap" => camera.bg_bitmap(),
    "bg-bitmap!" => camera.bg_bitmap_now(),
    "bg-blur" => camera.bg_blur(),
    "bg-blur!" => camera.bg_blur_now(),
    "bg-blur-level" => {
      match arg {
        Some(arg) => match arg.parse::<u8>() {
          Ok(arg) => camera.blur_level(arg),
          Err(err) => {
            eprintln!("{:?}", err);
            return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
          }
        },
        None => {
          eprintln!("Missing argument");
          return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
        }
      }
    },
    "bg-blur-level!" => {
      match arg {
        Some(arg) => match arg.parse::<u8>() {
          Ok(arg) => camera.blur_level_now(arg),
          Err(err) => {
            eprintln!("{:?}", err);
            return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
          }
        },
        None => {
          eprintln!("Missing argument");
          return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
        }
      }
    },
    "button-default"      => camera.button_default(),
    "button-rotate"       => camera.button_rotate(),
    "noise-reduction-off" => camera.noise_reduction_off(),
    "noise-reduction-on"  => camera.noise_reduction_on(),
    "bg-solid-blue"       => camera.bg_solid_blue(),
    "bg-solid-blue!"      => camera.bg_solid_blue_now(),
    "bg-solid-green"      => camera.bg_solid_green(),
    "bg-solid-green!"     => camera.bg_solid_green_now(),
    "bg-solid-red"        => camera.bg_solid_red(),
    "bg-solid-red!"       => camera.bg_solid_red_now(),
    "bg-solid-black"      => camera.bg_solid_black(),
    "bg-solid-black!"     => camera.bg_solid_black_now(),
    "bg-solid-white"      => camera.bg_solid_white(),
    "bg-solid-white!"     => camera.bg_solid_white_now(),
    "auto-frame-group"    => camera.auto_frame_group(),
    "auto-frame-group!"   => camera.auto_frame_group_now(),
    "auto-frame-face"     => camera.auto_frame_face(),
    "auto-frame-face!"    => camera.auto_frame_face_now(),
    "auto-frame-body"     => camera.auto_frame_body(),
    "auto-frame-body!"    => camera.auto_frame_body_now(),
    "bg-bitmap-n"    => {
      match arg {
        Some(arg) => match arg.parse::<u16>() {
          Ok(arg) => camera.bg_bitmap_n(arg),
          Err(err) => {
            eprintln!("{:?}", err);
            return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
          }
        },
        None => {
          eprintln!("Missing argument");
          return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
        }
      }
    },
    "bg-bitmap-n!" => {
      match arg {
        Some(arg) => match arg.parse::<u16>() {
          Ok(arg) => camera.bg_bitmap_n_now(arg),
          Err(err) => {
            eprintln!("{:?}", err);
            return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
          }
        },
        None => {
          eprintln!("Missing argument");
          return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
        }
      }
    },
    "sleep" => {
      match arg {
        Some(arg) => match arg.parse::<u16>() {
          Ok(arg) => camera.sleep(arg),
          Err(err) => {
            eprintln!("{:?}", err);
            return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
          }
        },
        None => {
          eprintln!("Missing argument");
          return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
        }
      }
    },

    "hex" => {
      match arg {
        Some(arg) => match hex::decode(&arg) {
          Ok(data) => camera.send_cmd_66(&data),
          Err(err) => { 
            eprintln!("Command `{}` not recognized", &cmd);
            return Err(errno::Errno(nix::errno::Errno::EINVAL as i32));
          }
        },
        None => {
          eprintln!("Missing argument");
          return Err(errno::Errno(nix::errno::Errno::EINVAL as i32))
        }
      }
    },

    _ => {
      eprintln!("Unrecognized command {}", &cmd);
      return Err(errno::Errno(nix::errno::Errno::EINVAL as i32));
    }
  };

  match result {
    Ok(_) => return Ok(()),
    Err(error) => panic!("Error {:?}", error)
  };
}

fn set(camera : & Camera, params : & [ String ]) -> Result<(), io::Error> {
  let dev = &camera.dev;
  let mut data = [ 0u8; 60 ];

  //hex::decode_to_slice(&params[0][..], &mut data).expect("Decoding failed");
  let decoded = hex::decode(&params[0][..]).expect("Decoding failed");

  data[..decoded.len()].copy_from_slice(&decoded);

  let result = set_cur(&dev, 0x6, 0x6, &mut data);
  match result {
    Ok(_) => return Ok(()),
    Err(error) => panic!("Error {:?}", error)
  }
}

const CAMERA_EFFECT_OFF : [ u8 ; 3] = [ 0x0, 0x01, 0x0 ];
const CAMERA_EFFECT_BG : [ u8 ; 3] = [ 0x0, 0x01, 0x1 ];
const CAMERA_EFFECT_TRACK : [ u8 ; 3] = [ 0x0, 0x01, 0x2 ];

const CAMERA_HDR_OFF : [ u8 ; 3] = [ 0x01, 0x01, 0x00];
const CAMERA_HDR_ON  : [ u8 ; 3] = [ 0x01, 0x01, 0x01];

const CAMERA_FACE_AE_OFF : [ u8 ; 3] = [ 0x03,  0x01, 0x00 ];
const CAMERA_FACE_AE_ON  : [ u8 ; 3] = [ 0x03,  0x01, 0x01 ];

const CAMERA_ANGLE_65 : [ u8 ; 3] = [ 0x04, 0x01, 0x02 ];
const CAMERA_ANGLE_78 : [ u8 ; 3] = [ 0x01, 0x01, 0x01 ];
const CAMERA_ANGLE_85 : [ u8 ; 3] = [ 0x01, 0x01, 0x00 ];

const CAMERA_BG_SOLID : [ u8 ; 3] = [ 0x05, 0x01, 0x01 ];
const CAMERA_BG_BITMAP : [ u8 ; 3] = [ 0x05, 0x01, 0x11 ];
const CAMERA_BG_BLUR : [ u8 ; 3] = [ 0x05, 0x01, 0x12 ];

const CAMERA_BG_BLUR_LEVEL : [ u8; 2] = [ 0x06, 0x01, ];

const CAMERA_BUTTON_DEFAULT : [ u8; 3] = [ 0x07, 0x01, 0x00 ];
const CAMERA_BUTTON_ROTATE : [ u8; 3] = [ 0x07, 0x01, 0x01 ];

const CAMERA_NOISE_REDUCTION_OFF : [ u8; 3] = [ 0x0a, 0x01, 0x00 ];
const CAMERA_NOISE_REDUCTION_ON : [ u8; 3] = [ 0x0a, 0x01, 0x01 ];

const CAMERA_BG_SOLID_BLUE : [ u8; 3] = [ 0x10, 0x01, 0x00 ];
const CAMERA_BG_SOLID_GREEN : [ u8; 3] = [ 0x10, 0x01, 0x01 ];
const CAMERA_BG_SOLID_RED : [ u8; 3] = [ 0x10, 0x01, 0x02 ];
const CAMERA_BG_SOLID_BLACK : [ u8; 3] = [ 0x10, 0x01, 0x03 ];
const CAMERA_BG_SOLID_WHITE : [ u8; 3] = [ 0x10, 0x01, 0x04 ];

const CAMERA_AUTO_FRAME_GROUP : [ u8; 4] = [ 0x0d, 0x02, 0x00, 0xff ];
const CAMERA_AUTO_FRAME_FACE : [ u8; 4] = [ 0x0d, 0x02, 0x01, 0x00 ];
const CAMERA_AUTO_FRAME_BODY : [ u8; 4] = [ 0x0d, 0x02, 0x01, 0x01 ];

const CAMERA_BG_BITMAP_N : [ u8; 2] = [ 0x0e, 0x02, ]; // +2 bytes be
const CAMERA_BG_BITMAP_0 : [ u8; 4] = [ 0x0e, 0x02, 0x00, 0x00 ]; // 2 bytes be
const CAMERA_BG_BITMAP_1 : [ u8; 4] = [ 0x0e, 0x02, 0x00, 0x01 ]; // 2 bytes be
const CAMERA_BG_BITMAP_2 : [ u8; 4] = [ 0x0e, 0x02, 0x00, 0x02 ]; // 2 bytes be

const CAMERA_SLEEP_S : [ u8; 2] = [ 0x0b, 0x02, ]; // +2 bytes le
const CAMERA_SLEEP_30 : [ u8; 4] = [ 0x0b, 0x02, 0x1e, 0x00 ]; // 2 bytes le
const CAMERA_SLEEP_120 : [ u8; 4] = [ 0x0b, 0x02, 0x78, 0x00 ]; // 2 bytes le
const CAMERA_SLEEP_600 : [ u8; 4] = [ 0x0b, 0x02, 0x58, 0x02 ]; // 2 bytes le

const CAMERA_NULL : [ u8; 0] = [];
//  let seconds : u16 = 30;
//  data[..cmd.len()].copy_from_slice(seconds.to_le_bytes());

#[derive(Debug)]
pub struct Camera {
  dev : std::fs::File
}

impl Camera {
  fn new(dev : &str) -> Result<Self, errno::Errno> {
    return match open_camera(dev) {
      Ok(camera) => Ok(Self { dev: camera }),
      Err(err) => Err(err),
    };
  }

  fn send_cmd_66(&self, cmd : & [u8]) -> Result<(), Errno> {
    send_cmd(&self.dev, 0x6, 0x6, cmd)
  }

  fn send_cmd_66_p(&self, cmd : & [u8], p : & [u8]) -> Result<(), Errno> {
    send_cmd_p(&self.dev, 0x6, 0x6, cmd, p)
  }

  fn effect_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_EFFECT_OFF)
  }

  fn effect_bg(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_EFFECT_BG)
  }

  fn effect_track(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_EFFECT_TRACK)
  }

  fn hdr_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_HDR_OFF)
  }

  fn hdr_on(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_HDR_ON)
  }

  fn face_ae_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_FACE_AE_OFF)
  }

  fn face_ae_on(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_FACE_AE_ON)
  }

  fn angle_65(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_ANGLE_65)
  }

  fn angle_78(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_ANGLE_78)
  }

  fn angle_85(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_ANGLE_85)
  }

  fn bg_solid(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID)
  }

  fn bg_solid_now(&self) -> Result<(), Errno> {
    self.bg_solid();
    self.effect_bg()
  }

  fn bg_bitmap(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_BITMAP)
  }

  fn bg_bitmap_now(&self) -> Result<(), Errno> {
    self.bg_bitmap();
    self.effect_bg()
  }

  fn bg_blur(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_BLUR)
  }

  fn bg_blur_now(&self) -> Result<(), Errno> {
    self.bg_blur();
    self.effect_bg()
  }

  fn blur_level(&self, level : u8) -> Result<(), Errno> {
    let level = std::cmp::min(64, level);
    self.send_cmd_66_p(&CAMERA_BG_BLUR_LEVEL, &level.to_le_bytes())
  }

  fn blur_level_now(&self, level : u8) -> Result<(), Errno> {
    self.blur_level(level);
    self.bg_blur();
    self.effect_bg()
  }

  fn button_default(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BUTTON_DEFAULT)
  }

  fn button_rotate(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BUTTON_ROTATE)
  }

  fn noise_reduction_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_NOISE_REDUCTION_OFF)
  }

  fn noise_reduction_on(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_NOISE_REDUCTION_ON)
  }

  fn bg_solid_blue(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_BLUE)
  }

  fn bg_solid_blue_now(&self) -> Result<(), Errno> {
    self.bg_solid_blue();
    self.bg_solid();
    self.effect_bg()
  }

  fn bg_solid_green(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_GREEN)
  }

  fn bg_solid_green_now(&self) -> Result<(), Errno> {
    self.bg_solid_green();
    self.bg_solid();
    self.effect_bg()
  }

  fn bg_solid_red(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_RED)
  }

  fn bg_solid_red_now(&self) -> Result<(), Errno> {
    self.bg_solid_red();
    self.bg_solid();
    self.effect_bg()
  }

  fn bg_solid_black(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_BLACK)
  }

  fn bg_solid_black_now(&self) -> Result<(), Errno> {
    self.bg_solid_black();
    self.bg_solid();
    self.effect_bg()
  }

  fn bg_solid_white(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_WHITE)
  }

  fn bg_solid_white_now(&self) -> Result<(), Errno> {
    self.bg_solid_white();
    self.bg_solid();
    self.effect_bg()
  }

  fn auto_frame_group(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_AUTO_FRAME_GROUP)
  }

  fn auto_frame_group_now(&self) -> Result<(), Errno> {
    self.auto_frame_group();
    self.effect_track()
  }

  fn auto_frame_face(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_AUTO_FRAME_FACE)
  }

  fn auto_frame_face_now(&self) -> Result<(), Errno> {
    self.auto_frame_face();
    self.effect_track()
  }

  fn auto_frame_body(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_AUTO_FRAME_BODY)
  }

  fn auto_frame_body_now(&self) -> Result<(), Errno> {
    self.auto_frame_body();
    self.effect_track()
  }

  fn bg_bitmap_n(&self, n : u16) -> Result<(), Errno> {
    self.send_cmd_66_p(&CAMERA_BG_BITMAP_N, &n.to_be_bytes())
  }

  fn bg_bitmap_n_now(&self, n : u16) -> Result<(), Errno> {
    self.bg_bitmap_n(n);
    self.bg_bitmap();
    self.effect_bg()
  }

  fn sleep(&self, n : u16) -> Result<(), Errno> {
    self.send_cmd_66_p(&CAMERA_SLEEP_S, &n.to_le_bytes())
  }
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

fn get_cur(dev : & std::fs::File, unit : u8, selector : u8, data : &mut [ u8 ] ) -> Result<(), Errno> {
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

  println!("{:?}", data);

  return match uvc_io(&dev, unit, selector, UVC_SET_CUR, data) {
    Ok(_) => return Ok(()),
    Err(err) => Err(err)
  }
}

fn send_cmd(dev : &std::fs::File, unit : u8, selector : u8, cmd : & [u8]) -> Result<(), Errno> {
  let mut data = [ 0u8; 60 ];
  data[..cmd.len()].copy_from_slice(&cmd);

  return set_cur(dev, unit, selector, &mut data);
}

fn send_cmds(dev : &std::fs::File, unit : u8, selector : u8, cmds : & [& [ u8]]) -> Result<(), Errno> {
  for cmd in cmds {
    match send_cmd(dev, unit, selector, cmd) {
      Ok(_) => (),
      Err(error) => return Err(error)
    }
  }
  return Ok(())
}

fn send_cmd_p(dev : &std::fs::File, unit : u8, selector : u8, cmd : & [u8], p : & [u8]) -> Result<(), Errno> {
  let mut data = [ 0u8; 60 ];
  data[..cmd.len()].copy_from_slice(&cmd);
  data[cmd.len()..cmd.len() + p.len()].copy_from_slice(&p);
  return set_cur(dev, unit, selector, &mut data);
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
            println!("Config desc: {:?}", config_desc);

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
