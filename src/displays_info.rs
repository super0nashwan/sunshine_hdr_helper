use windows::{
    Win32::{
        Graphics::Gdi::{
            EnumDisplayDevicesW,
            EnumDisplaySettingsW,
            DEVMODEW,
            ENUM_CURRENT_SETTINGS,
            ENUM_DISPLAY_SETTINGS_MODE,
            DISPLAY_DEVICEW,
            DISPLAY_DEVICE_PRIMARY_DEVICE,
        },
        Foundation::{LUID, ERROR_SUCCESS},
        Devices::Display::{
            GetDisplayConfigBufferSizes,
            QueryDisplayConfig,
            DISPLAYCONFIG_MODE_INFO,
            DISPLAYCONFIG_PATH_INFO,
            QDC_ONLY_ACTIVE_PATHS,
        },
    },
    core::{PCWSTR, PWSTR}
};
use log::{info, error};
use std::collections::HashSet;

pub struct DisplayDevice {
    pub device_index: u32,
    pub device_name: String,
    pub device_string: String,
    pub state_flags: u32,
    pub is_primary: bool,
    pub current_resolution: (u32, u32),
    pub current_refresh_rate: u32,
    pub adapter_id: LUID,
    pub source_id: u32,
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct DisplayMode {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
}

impl DisplayDevice {
    // Get supported modes for a display
    pub fn get_supported_modes(&self) -> Vec<DisplayMode> {
        let mut modes = HashSet::new();
        let mut mode_num: u32 = 0;

        loop {
            let mut dev_mode: DEVMODEW = unsafe { std::mem::zeroed() };
            dev_mode.dmSize = size_of::<DEVMODEW>() as u16;

            let success = unsafe {
                EnumDisplaySettingsW(
                    PCWSTR::from_raw(self.device_name.encode_utf16()
                        .chain(std::iter::once(0))
                        .collect::<Vec<u16>>()
                        .as_ptr()),
                    ENUM_DISPLAY_SETTINGS_MODE(mode_num),
                    &mut dev_mode,
                )
            };

            if !success.as_bool() {
                break;
            }

            modes.insert(DisplayMode {
                width: dev_mode.dmPelsWidth,
                height: dev_mode.dmPelsHeight,
                refresh_rate: dev_mode.dmDisplayFrequency,
            });

            mode_num += 1;
        }

        let mut result: Vec<DisplayMode> = modes.into_iter().collect();
        result.sort_by(|a, b| {
            b.width.cmp(&a.width)
                .then(b.height.cmp(&a.height))
                .then(b.refresh_rate.cmp(&a.refresh_rate))
        });
        result
    }
}

// Get primary display info with supported modes
pub fn get_primary_display_info() -> Option<(DisplayDevice, Vec<DisplayMode>)> {
    let displays = enumerate_displays();

    if let Some(primary) = displays.into_iter().find(|d| d.is_primary) {
        let modes = primary.get_supported_modes();
        info!("Found {} supported modes for primary display", modes.len());
        for mode in &modes {
            info!("  {}x{} @{}Hz", mode.width, mode.height, mode.refresh_rate);
        }
        Some((primary, modes))
    } else {
        error!("No primary display found");
        None
    }
}

pub fn enumerate_displays() -> Vec<DisplayDevice> {
    info!("Initiating displays enumeration");
    // First get display configuration information
    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    info!("Getting display configuration buffer sizes");
    let result = unsafe {
        GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            &mut num_modes,
        )
    };

    if result != ERROR_SUCCESS {
        error!("GetDisplayConfigBufferSizes failed with code: {:?}", result);
        return Vec::new();
    }

    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = vec![Default::default(); num_paths as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![Default::default(); num_modes as usize];

    let result = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            None,
        )
    };

    if result != ERROR_SUCCESS {
        error!("QueryDisplayConfig failed with code: {:?}", result);
        return Vec::new();
    }

    // Store the paths info for later matching
    let path_info: Vec<(u32, LUID)> = paths[..num_paths as usize]
        .iter()
        .map(|path| {
            info!("Path source ID: {}, Adapter ID: {:?}", path.sourceInfo.id, path.sourceInfo.adapterId);
            (path.sourceInfo.id, path.sourceInfo.adapterId)
        })
        .collect();

    // Now enumerate displays using EnumDisplayDevicesW
    let mut displays: Vec<DisplayDevice> = Vec::new();
    let mut device_index: u32 = 0;

    loop {
        let mut display_device: DISPLAY_DEVICEW = unsafe { std::mem::zeroed() };
        display_device.cb = size_of::<DISPLAY_DEVICEW>() as u32;

        let success = unsafe {
            EnumDisplayDevicesW(
                PWSTR::null(),
                device_index,
                &mut display_device,
                0,
            )
        };

        if !success.as_bool() {
            break;
        }

        let device_name = String::from_utf16_lossy(
            &display_device.DeviceName[..].iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c)
                .collect::<Vec<u16>>()
        );
        let device_string = String::from_utf16_lossy(
            &display_device.DeviceString[..].iter()
                .take_while(|&&c| c != 0)
                .map(|&c| c)
                .collect::<Vec<u16>>()
        );

        let state_flags = display_device.StateFlags;

        // If state is 0, no display is attached
        if state_flags == 0 {
            info!("Port {} exists but no display attached", device_name);
            device_index += 1;
            continue;
        }

        // Get current display settings
        let mut dev_mode: DEVMODEW = unsafe { std::mem::zeroed() };
        dev_mode.dmSize = size_of::<DEVMODEW>() as u16;

        let settings_success = unsafe {
            EnumDisplaySettingsW(
                PCWSTR::from_raw(display_device.DeviceName.as_ptr()),
                ENUM_CURRENT_SETTINGS,
                &mut dev_mode,
            )
        };

        if settings_success.as_bool() {
            let is_primary = (state_flags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0;

            // Find matching path info. WARNING: we assume that the source ID matches the device index. Not sure if this is always true.
            let (adapter_id, source_id) = path_info.iter()
                .find(|(id, _)| *id == device_index)
                .map(|(id, luid)| (*luid, *id))
                .unwrap_or((LUID { LowPart: 0, HighPart: 0 }, 0));

            let display = DisplayDevice {
                device_index,
                device_name: device_name.clone(),
                device_string: device_string.clone(),
                state_flags,
                is_primary,
                current_resolution: (dev_mode.dmPelsWidth, dev_mode.dmPelsHeight),
                current_refresh_rate: dev_mode.dmDisplayFrequency,
                adapter_id,
                source_id,
            };

            info!("Found display: {} ({}) - {}x{} @{}Hz{} [device_index: {}, source_id: {}, adapter: {:?}]",
                display.device_name,
                display.device_string,
                display.current_resolution.0,
                display.current_resolution.1,
                display.current_refresh_rate,
                if display.is_primary { " [Primary]" } else { "" },
                display.device_index,
                display.source_id,
                display.adapter_id
            );

            displays.push(display);
        } else {
            error!("Failed to get settings for display: {}", device_name);
        }

        device_index += 1;
    }

    displays
}


/*
pub fn test_query_display_config() -> Result<(), windows::core::Error> {
    info!("Getting display configuration buffer sizes");
    let mut num_paths: u32 = 0;
    let mut num_modes: u32 = 0;

    let result = unsafe {
        GetDisplayConfigBufferSizes(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            &mut num_modes,
        )
    };

    if result != ERROR_SUCCESS {
        error!("GetDisplayConfigBufferSizes failed with code: {:?}", result);
        return Err(windows::core::Error::from_win32());
    }

    info!("Buffer sizes - Paths: {}, Modes: {}", num_paths, num_modes);

    // Allocate the arrays
    let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = vec![Default::default(); num_paths as usize];
    let mut modes: Vec<DISPLAYCONFIG_MODE_INFO> = vec![Default::default(); num_modes as usize];

    // Get the actual configuration
    let result = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            None,
        )
    };

    if result != ERROR_SUCCESS {
        error!("QueryDisplayConfig failed with code: {:?}", result);
        return Err(windows::core::Error::from_win32());
    }

    info!("Display Configuration Information:");

    // Log paths
    info!("Paths:");
    for (i, path) in paths[..num_paths as usize].iter().enumerate() {
        info!("Path {}:", i);
        info!("  Flags: {:#010x}", path.flags);
        info!("  Source Info:");
        info!("    Id: {}", path.sourceInfo.id);
        info!("    Adapter LUID: {:?}", path.sourceInfo.adapterId);
        info!("  Target Info:");
        info!("    Id: {}", path.targetInfo.id);
        info!("    Adapter LUID: {:?}", path.targetInfo.adapterId);
    }

    // Log modes
    info!("Modes:");
    for (i, mode) in modes[..num_modes as usize].iter().enumerate() {
        info!("Mode {}:", i);
        info!("  Info Type: {:?}", mode.infoType);
        info!("  Id: {}", mode.id);
        info!("  Adapter LUID: {:?}", mode.adapterId);

        unsafe {
            match mode.infoType {
                DISPLAYCONFIG_MODE_INFO_TYPE_SOURCE => {
                    let source_mode = mode.Anonymous.sourceMode;
                    info!("  Source Mode:");
                    info!("    Width: {}", source_mode.width);
                    info!("    Height: {}", source_mode.height);
                    info!("    Position: ({}, {})",
                        source_mode.position.x,
                        source_mode.position.y);
                }
                DISPLAYCONFIG_MODE_INFO_TYPE_TARGET => {
                    let target_mode = mode.Anonymous.targetMode;
                    info!("  Target Mode:");
                    info!("    Video Signal Info available");
                }
                _ => info!("  Unknown mode type"),
            }
        }
    }

    Ok(())
}
*/
// Stubbed to remove compiler warnings
pub fn test_query_display_config() -> Result<(), windows::core::Error> {
    Ok(())
}
