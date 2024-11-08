/*
We are implementing a part of the Windows API that appears undocumented and not directly supported by the windows Rust crate.
The function we are targeting adjusts the SDR white level on HDR displays. It takes a value of 1000 to 6000, where each 1000 is 80 nits.
The Windows UI slider in the Settings app maps this range to 0-100 (i.e. 80-480 nits).
Documentation here (only the GET, missing the SET we need): https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ne-wingdi-displayconfig_device_info_type
We need to:
- Create a struct that represents the parameters for the function [DONE]
- Create a struct that represents the display information [DONE]
- Enumerate the displays, storing the needed information [DONE]
- Pick the primary display, knowing for our personal use that this will be our HDR monitor with HDR enabled. [DONE]
- Take a user input for the SDR white level and apply it to the primary display. [DONE]
- Log events and errors to a file. [IN PROGRESS, TODO]
Considerations:
- We must be memory safe when marking code as unsafe, knowing we are working at a low level with the Windows API.
- We will deal with robust error handling when have successfully tested a working implementation. [TODO]
- Now this is confirmed working, we need to refactor to prevent overlap with our utility functions in displays_info.rs. [TODO]
*/


use std::mem::size_of;
use windows::Win32::{
    Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_HEADER,
        DISPLAYCONFIG_PATH_INFO,
        DisplayConfigSetDeviceInfo,
        QueryDisplayConfig,
        GetDisplayConfigBufferSizes,
        DISPLAYCONFIG_MODE_INFO,
        QDC_ONLY_ACTIVE_PATHS,
        DISPLAYCONFIG_DEVICE_INFO_TYPE,
    },
    Foundation::{ERROR_SUCCESS, ERROR_INSUFFICIENT_BUFFER},
    Graphics::Gdi::{EnumDisplayDevicesW, DISPLAY_DEVICEW, DISPLAY_DEVICE_PRIMARY_DEVICE},
};
use log::info;

const DISPLAYCONFIG_DEVICE_INFO_SET_SDR_WHITE_LEVEL: DISPLAYCONFIG_DEVICE_INFO_TYPE = DISPLAYCONFIG_DEVICE_INFO_TYPE(-18i32);

//==============================================================================
// Structs
//==============================================================================

#[repr(C)]
#[derive(Copy, Clone)]
struct DisplayconfigSetSdrWhiteLevel {
    header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    sdr_white_level: u32,
    final_value: u8,
}

struct DisplayInfo {
    path_info: DISPLAYCONFIG_PATH_INFO,
    is_primary: bool,
}

//==============================================================================
// Get displays information
//==============================================================================

impl DisplayInfo {
    fn is_primary_display(&self) -> bool {
        let mut display_device = DISPLAY_DEVICEW::default();
        display_device.cb = size_of::<DISPLAY_DEVICEW>() as u32;

        unsafe {
            let mut device_index = 0;
            while EnumDisplayDevicesW(None, device_index, &mut display_device, 0).as_bool() {
                if (display_device.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0 {
                    return true;
                }
                device_index += 1;
            }
        }
        false
    }
}

fn enumerate_displays() -> windows::core::Result<Vec<DisplayInfo>> {
    let mut path_count: u32 = 0;
    let mut mode_count: u32 = 0;
    let flags = QDC_ONLY_ACTIVE_PATHS;

    unsafe {
        let result = GetDisplayConfigBufferSizes(flags, &mut path_count, &mut mode_count);
        if result != ERROR_SUCCESS {
            return Err(windows::core::Error::from_win32());
        }
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

    unsafe {
        let mut result = QueryDisplayConfig(
            flags,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        );

        if result == ERROR_INSUFFICIENT_BUFFER {
            paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
            modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

            result = QueryDisplayConfig(
                flags,
                &mut path_count,
                paths.as_mut_ptr(),
                &mut mode_count,
                modes.as_mut_ptr(),
                None,
            );
        }

        if result != ERROR_SUCCESS {
            return Err(windows::core::Error::from_win32());
        }
    }

    let mut displays = Vec::new();
    for path in paths.iter().take(path_count as usize) {
        let mut display_info = DisplayInfo {
            path_info: *path,
            is_primary: false,
        };
        display_info.is_primary = display_info.is_primary_display();
        displays.push(display_info);
    }

    Ok(displays)
}

//==============================================================================
// Set SDR white level
//==============================================================================

fn set_sdr_white_level(path_info: &DISPLAYCONFIG_PATH_INFO, level: u32) -> windows::core::Result<()> {
    // Map 0-100 directly to 1000-6000
    let api_value = 1000 + (level * 50);

    let params = DisplayconfigSetSdrWhiteLevel {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_SET_SDR_WHITE_LEVEL,
            size: size_of::<DisplayconfigSetSdrWhiteLevel>() as u32,
            adapterId: path_info.targetInfo.adapterId,
            id: path_info.targetInfo.id,
        },
        sdr_white_level: api_value,
        final_value: 1,
    };

    let result = unsafe { DisplayConfigSetDeviceInfo(&params.header) };
    if result == ERROR_SUCCESS.0 as i32 {
        Ok(())
    } else {
        Err(windows::core::Error::from_win32())
    }
}

//==============================================================================
// Set primary display SDR white level helper for CLI command
//==============================================================================
pub fn set_primary_display_sdr_white(level: u32) -> windows::core::Result<()> {
    if level > 100 {
        return Err(windows::core::Error::from_win32());
    }

    info!("Setting SDR white level to {}", level);
    let displays = enumerate_displays()?;

    if let Some(primary_display) = displays.iter().find(|d| d.is_primary) {
        set_sdr_white_level(&primary_display.path_info, level)
    } else {
        Err(windows::core::Error::from_win32())
    }
}