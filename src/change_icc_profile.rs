use windows::Win32::Graphics::Gdi::{CreateDCW, DeleteDC};
use windows::Win32::Foundation::LPARAM;
use windows::core::{PCWSTR, Result};
use log::{info, error};
use std::path::PathBuf;

use crate::displays_info::{DisplayDevice, get_primary_display_info};

use windows::Win32::UI::ColorSystem::{
    ColorProfileSetDisplayDefaultAssociation,
    EnumICMProfilesW,
    WCS_PROFILE_MANAGEMENT_SCOPE_CURRENT_USER,
    CPT_ICC,
    CPST_RGB_WORKING_SPACE
};

pub struct IccProfile {
    pub name: String,
    pub path: PathBuf,
}

fn get_display_icc_profiles(display: &DisplayDevice) -> Vec<IccProfile> {
    info!("Retrieving ICC profiles for display: {} ({})", display.device_name, display.device_string);
    let mut profiles = Vec::new();

    unsafe {
        let dc = CreateDCW(
            PCWSTR::from_raw(display.device_name.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>().as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            None,
        );

        if dc.is_invalid() {
            error!("Failed to create DC for display {}", display.device_name);
            return profiles;
        }

        extern "system" fn enum_profiles_callback(profile_name: PCWSTR, param: LPARAM) -> i32 {
            unsafe {
                let profiles = &mut *(param.0 as *mut Vec<IccProfile>);
                let path_str = profile_name.to_string().unwrap_or_default();
                let path = PathBuf::from(&path_str);
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                info!("Found ICC profile - Name: '{}', Path: '{}'", name, path.display());
                profiles.push(IccProfile { name, path });
                1 // Return 1 for success
            }
        }

        let result = EnumICMProfilesW(
            dc,
            Some(enum_profiles_callback),
            LPARAM(&mut profiles as *mut _ as isize),
        );

        let _ = DeleteDC(dc);

        if result == -1 {
            info!("No ICC profiles found for display {}", display.device_name);
        } else if result == 0 {
            error!("Enumeration of ICC profiles was interrupted");
        }
    }

    info!("Retrieved {} ICC profile(s) for display {}", profiles.len(), display.device_name);
    profiles
}

// Set a display's default ICC profile



fn set_display_icc_profile(display: &DisplayDevice, profile_name: &str) -> Result<()> {
    info!("Attempting to set ICC profile '{}' for display: {}", profile_name, display.device_name);

    // Get available profiles and validate the requested profile
    let available_profiles = get_display_icc_profiles(display);
    let profile = available_profiles.iter().find(|p| p.name == profile_name);

    let profile = match profile {
        Some(p) => p,
        None => {
            error!("Profile '{}' not found in available profiles for display", profile_name);
            return Err(windows::core::Error::from_win32());
        }
    };

    // Convert path to wide string for Windows API
    let profile_path = profile.path.to_string_lossy();
    let profile_path_wide: Vec<u16> = profile_path.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        match ColorProfileSetDisplayDefaultAssociation(
            WCS_PROFILE_MANAGEMENT_SCOPE_CURRENT_USER,
            PCWSTR::from_raw(profile_path_wide.as_ptr()),
            CPT_ICC,
            CPST_RGB_WORKING_SPACE,
            display.adapter_id,
            display.source_id,
        ) {
            Ok(()) => {
                info!("Successfully set ICC profile '{}' for display", profile_name);
                Ok(())
            },
            Err(e) => {
                error!("Failed to set ICC profile: {}", e);
                Err(e)
            }
        }
    }
}




//==============================================================================
// Helper functions for CLI commands
//==============================================================================

// Primary display only right now (makes most sense for game streaming).
pub fn list_icc_profiles() -> Vec<(String, PathBuf)> {
    if let Some((primary_display, _)) = get_primary_display_info() {
        get_display_icc_profiles(&primary_display)
            .into_iter()
            .map(|p| (p.name, p.path))
            .collect()
    } else {
        Vec::new()
    }
}

pub fn change_primary_display_icc_profile(profile_name: &str) -> Result<()> {
    match get_primary_display_info() {
        Some((primary_display, _)) => {
            info!("Setting ICC profile '{}' for primary display", profile_name);
            set_display_icc_profile(&primary_display, profile_name)
        }
        None => {
            let error = windows::core::Error::from_win32();
            error!("Error setting primary display default ICC color profile: {}", error);
            Err(error)
        }
    }
}

