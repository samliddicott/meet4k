use errno::Errno;
use std::env;
use std::str;
use hex;

mod camera;

fn main() {
  println!("OBSBOT Meet 4K controller\n");
  let args: Vec<_> = env::args().collect();

  let camera = match camera::Camera::new(&args[1]) {
    Ok(camera) => camera,
    Err(err) => panic!("Can't find camera {:?}", err)
  };

  match cmds(&camera, &args[2..]) {
    Ok(_) => return (),
    _ => std::process::exit(1),
  };
}

pub fn cmds(camera : & camera::Camera, cmds : & [ String ]) -> Result<(), Errno> {
  for c in cmds {
    match cmd(&camera, c) {
      Ok(_) => (),
      Err(error) => return Err(error),
    }
  }
  return Ok(());
}

fn cmd(camera : & camera::Camera, cmd : &str) -> Result<(), Errno> {
  let mut parts = cmd.splitn(2, '=');
  let cmd = parts.next().unwrap();
  let arg = parts.next();

  let result = match cmd {
    "info" => camera.info(),
    "get" => camera.dump(),

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
          Err(_) => { 
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
