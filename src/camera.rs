use errno::Errno;
mod usbio;

#[derive(Debug)]
pub struct Camera {
  handle : usbio::CameraHandle,
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
//const CAMERA_BG_BITMAP_0 : [ u8; 4] = [ 0x0e, 0x02, 0x00, 0x00 ]; // 2 bytes be
//const CAMERA_BG_BITMAP_1 : [ u8; 4] = [ 0x0e, 0x02, 0x00, 0x01 ]; // 2 bytes be
//const CAMERA_BG_BITMAP_2 : [ u8; 4] = [ 0x0e, 0x02, 0x00, 0x02 ]; // 2 bytes be

const CAMERA_SLEEP_S : [ u8; 2] = [ 0x0b, 0x02, ]; // +2 bytes le
//const CAMERA_SLEEP_30 : [ u8; 4] = [ 0x0b, 0x02, 0x1e, 0x00 ]; // 2 bytes le
//const CAMERA_SLEEP_120 : [ u8; 4] = [ 0x0b, 0x02, 0x78, 0x00 ]; // 2 bytes le
//const CAMERA_SLEEP_600 : [ u8; 4] = [ 0x0b, 0x02, 0x58, 0x02 ]; // 2 bytes le

// const CAMERA_NULL : [ u8; 0] = [];

impl Camera {
  pub fn new(hint : &str) -> Result<Self, errno::Errno> {
    return match usbio::open_camera(hint) {
      Ok(camera) => Ok(Self { handle : camera }),
      Err(err) => Err(err),
    }
  }

  pub fn info(&self) -> Result<(), Errno> {
    return self.handle.info();
  }

  pub fn dump(&self) -> Result<(), Errno> {
    let mut data = [ 0u8; 60 ];
    self.get_cur(0x6, 0x6, &mut data)?;
    hexdump::hexdump(&data);
    return Ok(());
  }

  fn send_cmd(&self, unit : u8, selector : u8, cmd : & [u8]) -> Result<(), errno::Errno> {
    let mut data = [ 0u8; 60 ];
    data[..cmd.len()].copy_from_slice(&cmd);

    return self.set_cur(unit, selector, &mut data);
  }

  fn get_cur(&self, unit : u8, selector : u8, data : &mut [ u8 ] ) -> Result<(), errno::Errno> {
    // always call get_len first
    match self.get_len(unit, selector) {
      Ok(size) => {
        if data.len() < size {
          println!("Got size {}", size); 
          return Err(errno::Errno(1)) 
        }
      },
      Err(err) => return Err(err)
    };

    // Why not &mut data here?
    return match self.io(unit, selector, usbio::UVC_GET_CUR, data) {
      Ok(_) => return Ok(()),
      Err(err) => Err(err)
    }
  }

  fn set_cur(&self, unit : u8, selector : u8, data : &mut [u8]) -> Result<(), errno::Errno> {
    match self.get_len(unit, selector) {
      Ok(size) => {
        if data.len() > size {
          println!("Got size {}", size); 
          return Err(errno::Errno(1)) 
        }
      },
      Err(err) => return Err(err)
    };

    println!("{:?}", data);

    return match self.io(unit, selector, usbio::UVC_SET_CUR, data) {
      Ok(_) => return Ok(()),
      Err(err) => Err(err)
    }
  }

  fn get_len(&self, unit: u8, selector: u8) -> Result<usize, Errno> {
    let mut data = [ 0u8; 2 ];

    return match self.io(unit, selector, usbio::UVC_GET_LEN, &mut data) {
      Ok(_) => Ok(u16::from_le_bytes(data).into()),
      Err(err) => Err(err)
    }
  }

  fn io(&self, unit: u8, selector: u8, query: u8,  data : &mut [u8]) -> Result<(), Errno> {
    return self.handle.io(unit, selector, query, data);
  }

  fn send_cmd_p(&self, unit : u8, selector : u8, cmd : & [u8], p : & [u8]) -> Result<(), Errno> {
    let mut data = [ 0u8; 60 ];
    data[..cmd.len()].copy_from_slice(&cmd);
    data[cmd.len()..cmd.len() + p.len()].copy_from_slice(&p);
    return self.set_cur(unit, selector, &mut data);
  }

  pub fn send_cmd_66(&self, cmd : & [u8]) -> Result<(), Errno> {
    self.send_cmd(0x6, 0x6, cmd)
  }

  fn send_cmd_66_p(&self, cmd : & [u8], p : & [u8]) -> Result<(), Errno> {
    self.send_cmd_p(0x6, 0x6, cmd, p)
  }

  pub fn effect_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_EFFECT_OFF)
  }

  pub fn effect_bg(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_EFFECT_BG)
  }

  pub fn effect_track(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_EFFECT_TRACK)
  }

  pub fn hdr_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_HDR_OFF)
  }

  pub fn hdr_on(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_HDR_ON)
  }

  pub fn face_ae_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_FACE_AE_OFF)
  }

  pub fn face_ae_on(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_FACE_AE_ON)
  }

  pub fn angle_65(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_ANGLE_65)
  }

  pub fn angle_78(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_ANGLE_78)
  }

  pub fn angle_85(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_ANGLE_85)
  }

  pub fn bg_solid(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID)
  }

  pub fn bg_solid_now(&self) -> Result<(), Errno> {
    self.bg_solid()?;
    self.effect_bg()
  }

  pub fn bg_bitmap(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_BITMAP)
  }

  pub fn bg_bitmap_now(&self) -> Result<(), Errno> {
    self.bg_bitmap()?;
    self.effect_bg()
  }

  pub fn bg_blur(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_BLUR)
  }

  pub fn bg_blur_now(&self) -> Result<(), Errno> {
    self.bg_blur()?;
    self.effect_bg()
  }

  pub fn blur_level(&self, level : u8) -> Result<(), Errno> {
    let level = std::cmp::min(64, level);
    self.send_cmd_66_p(&CAMERA_BG_BLUR_LEVEL, &level.to_le_bytes())
  }

  pub fn blur_level_now(&self, level : u8) -> Result<(), Errno> {
    self.blur_level(level)?;
    self.bg_blur()?;
    self.effect_bg()
  }

  pub fn button_default(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BUTTON_DEFAULT)
  }

  pub fn button_rotate(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BUTTON_ROTATE)
  }

  pub fn noise_reduction_off(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_NOISE_REDUCTION_OFF)
  }

  pub fn noise_reduction_on(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_NOISE_REDUCTION_ON)
  }

  pub fn bg_solid_blue(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_BLUE)
  }

  pub fn bg_solid_blue_now(&self) -> Result<(), Errno> {
    self.bg_solid_blue()?;
    self.bg_solid()?;
    self.effect_bg()
  }

  pub fn bg_solid_green(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_GREEN)
  }

  pub fn bg_solid_green_now(&self) -> Result<(), Errno> {
    self.bg_solid_green()?;
    self.bg_solid()?;
    self.effect_bg()
  }

  pub fn bg_solid_red(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_RED)
  }

  pub fn bg_solid_red_now(&self) -> Result<(), Errno> {
    self.bg_solid_red()?;
    self.bg_solid()?;
    self.effect_bg()
  }

  pub fn bg_solid_black(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_BLACK)
  }

  pub fn bg_solid_black_now(&self) -> Result<(), Errno> {
    self.bg_solid_black()?;
    self.bg_solid()?;
    self.effect_bg()
  }

  pub fn bg_solid_white(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_BG_SOLID_WHITE)
  }

  pub fn bg_solid_white_now(&self) -> Result<(), Errno> {
    self.bg_solid_white()?;
    self.bg_solid()?;
    self.effect_bg()
  }

  pub fn auto_frame_group(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_AUTO_FRAME_GROUP)
  }

  pub fn auto_frame_group_now(&self) -> Result<(), Errno> {
    self.auto_frame_group()?;
    self.effect_track()
  }

  pub fn auto_frame_face(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_AUTO_FRAME_FACE)
  }

  pub fn auto_frame_face_now(&self) -> Result<(), Errno> {
    self.auto_frame_face()?;
    self.effect_track()
  }

  pub fn auto_frame_body(&self) -> Result<(), Errno> {
    self.send_cmd_66(&CAMERA_AUTO_FRAME_BODY)
  }

  pub fn auto_frame_body_now(&self) -> Result<(), Errno> {
    self.auto_frame_body()?;
    self.effect_track()
  }

  pub fn bg_bitmap_n(&self, n : u16) -> Result<(), Errno> {
    self.send_cmd_66_p(&CAMERA_BG_BITMAP_N, &n.to_be_bytes())
  }

  pub fn bg_bitmap_n_now(&self, n : u16) -> Result<(), Errno> {
    self.bg_bitmap_n(n)?;
    self.bg_bitmap()?;
    self.effect_bg()
  }

  pub fn sleep(&self, n : u16) -> Result<(), Errno> {
    self.send_cmd_66_p(&CAMERA_SLEEP_S, &n.to_le_bytes())
  }
}

