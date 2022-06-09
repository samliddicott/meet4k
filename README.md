# meet4k
Control extra features of OBSBOT Meet 4K Camera under Linux. See also guvcview fork for GTK3 UI controls: https://github.com/samliddicott/guvciew-meet4k/tree/meet4k

The first release is in rust, because I wanted something to make me learn rust, but I'll plan to code it as a C library with wxWidget UI app.

Features:

* Open camera using v4l2 UVC interface while camera is in use
* Open camera using libusb (seizes control of the camera)

The direct USB interface is not fully tested.

## Warning

I've been at this for about a week, it's a bit of a mess, as I'm learning rust at the same time, but it works as a command line tool that you can bind to hotkeys.

## Building

Have rust installed:

    cargo build

## Running

UVC requires root access right now:

    sudo target/debug/meet4k <camera-reference> <cmd[=arg]> ...

camera-reference can be the pathname, e.g. /dev/video10 or a sequence of text that will be matched against the camera *card name* or PCI *bus-info* fields as reported by the UVC interface, or the USB vid:pid, e.g. 6e30:fef3 or text matching the USB product or manufacturer text, e.g. "Remo Tech"

I find that OBSBOT is enough to match against the *card name* for the UVC interface. To find suitable values try running the `info` command on all of your devices: `meet4k /dev/video/0 info`

### examples

This will enable blur mode at level 30 (max level is 64) with a zoom view-angle of 65 degrees

    sudo env RUST_BACKTRACE=1 target/debug/meet4k OBSBOT angle-65 bg-blur bg-blur-leve=30

This will set the background colour to green and activate it

    sudo env RUST_BACKTRACE=1 target/debug/meet4k OBSBOT bg-solid-green bg-solid effect-bg

The shorter version uses an exclamation mark, and is:

    sudo env RUST_BACKTRACE=1 target/debug/meet4k OBSBOT 'bg-solid-green!'

Depending on your shell settings you may need to enclose the commands in *single quotes* '...' to protect the exclamation mark from shell interpolation

Any command that depends on other commands has a version with a trailing exclamation mark to automatically invoke the other commands and make the setting active. Normally setting the background colour to green while in blur mode is *valid* but has no effect until solid-colour mode is active.

## Supported commands

Dump card info and PCI info

    meet4k OBSBOT info

e.g.

    $ sudo env RUST_BACKTRACE=1 target/debug/meet4k /dev/video10 info
    OBSBOT Meet 4K controller
    Opened Camera { dev: File { fd: 3, path: "/dev/video10", read: true, write: false } }
    Card: OBSBOT Meet 4K: OBSBOT Meet 4K 
    Bus : usb-0000:00:14.0-8.3.1.1

Show unit 6 selector 6 memory dump -- good for debugging. (You can't simply write this back with changes, an RPC mechanism is used)

    meet4k OBSBOT get

e.g.

    $ sudo env RUST_BACKTRACE=1 target/debug/meet4k /dev/video10 info
    OBSBOT Meet 4K controller
    Opened Camera { dev: File { fd: 3, path: "/dev/video10", read: true, write: false } }
    |01000100 02114001 0f000102 01000100| ......@......... 00000000
    |00003c00 00010101 01060000 00000000| ..<............. 00000010
    |00000000 00000000 00000000 00000000| ................ 00000020
    |00000000 00000000 00000000|          ............     00000030
                                                           0000003c

Send an RPC to set some unit 6 selector 6 configuration. The first byte sometimes corresponds to an offset in the configuration. The second byte is the number of bytes to set. The third byte (and maybe 4th byte) are data -- sometimes little endian, sometimes big endian. 

This example turns off camera effects

    meet4k OBSBOT hex=000100

This example enables tracking mode

    meet4k OBSBOT hex=000102

You cannot concatenate hex sequences together for multiple commands, instead send multiple hex= commands (on the same command line):

    meet4k OBSBOT hex=000102 hex=0d020101

But there are nicer commands to control camera effects, and they can be chained together

    meet4k OBSBOT effect-off        # act like a dumb webcam with no effects
    meet4k OBSBOT effect-bg         # background replacement, see sub-commands bg-solid, bg-bitmap, bg-blur for effect to use
    meet4k OBSBOT effect-track      # face tracking, see auto-frame-group, auto-frame-face, auto-frame-body

    meet4K OBSBOT button-default    # make the camera button do nothing
    meet4K OBSBOT button-rotate     # make the camera button switch between effect-bg, effect-off and effect-track

    meet4k OBSBOT noise-reduction-off
    meet4k OBSBOT noise-reduction-on

    meet4k OBSBOT hdr-off
    meet4k OBSBOT hdr-on

    meet4k OBSBOT face-ae-off
    meet4k OBSBOT face-ae-on

    meet4k OBSBOT angle-65          # field of view of camera, consider to be "zoom" setting
    meet4k OBSBOT angle-78
    meet4k OBSBOT angle-85

    meet4k OBSBOT bg-solid          # enable solid-colour background, when effect-bg is enabled
    meet4k OBSBOT bg-solid-blue     # set solid-colour backgroud to blue, effective when bg-solid and effect-bg are enabled
    meet4k OBSBOT bg-solid-blue!    # set solid-colour backgroud to blue, and enable bg-solid and effect-bg
    meet4k OBSBOT bg-solid-green    # set solid-colour backgroud to green, effective when bg-solid and effect-bg are enabled
    meet4k OBSBOT bg-solid-green!   # set solid-colour backgroud to green, and enable bg-solid and effect-bg
    meet4k OBSBOT bg-solid-red      # set solid-colour backgroud to red, effective when bg-solid and effect-bg are enabled
    meet4k OBSBOT bg-solid-red!     # set solid-colour backgroud to red, and enable bg-solid and effect-bg
    meet4k OBSBOT bg-solid-black    # set solid-colour backgroud to black, effective when bg-solid and effect-bg are enabled
    meet4k OBSBOT bg-solid-black!   # set solid-colour backgroud to black, and enable bg-solid and effect-bg
    meet4k OBSBOT bg-solid-white    # set solid-colour backgroud to white, effective when bg-solid and effect-bg are enabled
    meet4k OBSBOT bg-solid-white!   # set solid-colour backgroud to white, and enable bg-solid and effect-bg

    meet4k OBSBOT bg-bitmap         # enable image background replacement, when effect-bg is enabled
    meet4k OBSBOT bg-bitmap-n=<n>   # set image background replacement to image n, when effect-bg is enabled and bg-bitmap is enabledd
    meet4k OBSBOT bg-bitmap-n!=<n>  # set image background replacement to image n, and enable effect-bg bg-bitmap

    meet4k OBSBOT bg-blur           # enable image blurring, when effect-bg is enabled
    meet4k OBSBOT bg-blur-level=n   # n=0-64 set blur level when bg-blur is enabled and effect-bg is enabled
    meet4k OBSBOT bg-blur-level!=n  # n=0-64 set blur level and enable bg-glur and effect-bg

    meet4k OBSBOT auto-frame-group  # track multiple people, when effect-track is enabled
    meet4k OBSBOT auto-frame-group! # track multiple people, and enable effect-track
    meet4k OBSBOT auto-frame-face   # track single face, when effect-track is enabled
    meet4k OBSBOT auto-frame-face!  # track single face, and enable effect-track
    meet4k OBSBOT auto-frame-body   # track single upper body, when effect-track is enabled
    meet4k OBSBOT auto-frame-body!  # track single upper body, and enable effect-track

    meet4k OBSBOT sleep=<s>         # turn on auto-sleep after s seconds, up to 65535 (I guess, it's 2 bytes)


## Hacking Info

Information was gained using wireshark on windows to capture USB packets used to control the camera.

Initially libusb control was used, but then I discovered kernel UVC support which allows configuration while the webcam is in use.

Currently support for these options under unit 6, selector 6. Support for uploading images and pan will be added (I have the USB controls captured, I haven't coded it yet)

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

Other settings are found capturing different selectors, I still need to list them. They include getting and setting background images, pan and zoom.
