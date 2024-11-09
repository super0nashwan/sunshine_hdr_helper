use log::{info, error};
use windows::Win32::Graphics::Gdi::{
    DEVMODEW,
    ChangeDisplaySettingsExW,
    CDS_UPDATEREGISTRY,
    DISP_CHANGE_SUCCESSFUL,
    DISP_CHANGE_BADMODE,
    DISP_CHANGE_FAILED,
    DISP_CHANGE_RESTART,
    DM_PELSWIDTH,
    DM_PELSHEIGHT,
    DM_DISPLAYFREQUENCY,
};
use windows::core::PCWSTR;
use std::{thread, time::Duration};
use std::ffi::{OsStr};
use std::os::windows::ffi::{OsStrExt};

use crate::displays_info::{self};

pub fn change_primary_display_mode(width: u32, height: u32, refresh_rate: u32, unsafe_mode: bool) -> bool {
    info!("Attempting to change primary display mode to {}x{} @{}Hz (unsafe: {})",
        width, height, refresh_rate, unsafe_mode);

    let Some((primary, supported_modes)) = displays_info::get_primary_display_info() else {
        error!("Failed to get primary display information");
        return false;
    };

    // Early return if mode validation is required and the mode isn't supported
    if !unsafe_mode && !supported_modes.iter().any(|mode|
        mode.width == width &&
            mode.height == height &&
            mode.refresh_rate == refresh_rate
    ) {
        error!("Requested mode {}x{} @{}Hz is not reported by the display as supported.",
            width, height, refresh_rate);
        return false;
    }

    // Create and initialize DEVMODE structure
    let mut dev_mode = DEVMODEW::default();
    dev_mode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;
    dev_mode.dmPelsWidth = width;
    dev_mode.dmPelsHeight = height;
    dev_mode.dmDisplayFrequency = refresh_rate;
    dev_mode.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;

    // Convert device name to wide string and keep it in scope
    let device_name: Vec<u16> = OsStr::new(&primary.device_name)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let pcwstr = PCWSTR::from_raw(device_name.as_ptr());

    // Attempt to change the display settings
    unsafe {
        let result = ChangeDisplaySettingsExW(
            pcwstr,
            Some(&dev_mode),
            None,
            CDS_UPDATEREGISTRY,
            None,
        );

        match result {
            DISP_CHANGE_SUCCESSFUL => {
                thread::sleep(Duration::from_millis(3000));
                info!("Successfully changed display mode to {}x{} @{}Hz",
                    width, height, refresh_rate);
                true
            }
            error_code => {
                let error_msg = match error_code {
                    DISP_CHANGE_BADMODE => "The graphics mode is not supported",
                    DISP_CHANGE_FAILED => "The display driver failed the specified graphics mode",
                    DISP_CHANGE_RESTART => "The computer must be restarted for the graphics mode to work",
                    _ => "Unknown error occurred"
                };
                error!("Failed to change display mode: {}. Error code: {}", error_msg, error_code.0);
                false
            }
        }
    }
}