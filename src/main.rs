mod displays_info;
mod change_display_mode;
mod set_sdr_level;
mod change_icc_profile;


use clap::{Parser, Subcommand, value_parser};
use log::{info, LevelFilter};
use std::fs::OpenOptions;
use env_logger::{Builder, Target};
use std::io::{Write};
use chrono::Local;
use std::str::FromStr;

use displays_info::{enumerate_displays};

//==============================================================================
// CLI setup
//==============================================================================
#[derive(Parser)]
#[command(name = "sunshine-helper")]
#[command(about = "Windows display API utility", long_about = None)]
struct Cli {
    #[arg(short, long, help = "Enable logging to file")]
    log: bool,

    #[command(subcommand)]
    command: Commands,
}

// Hard coding some ICC profile strings. Don't use these! I mean, you can if you really want.
#[derive(Debug)]
enum PresetString {
    LgOled,
    SteamDeck,
    TestProfile,
}

impl PresetString {
    fn as_str(&self) -> &'static str {
        match self {
            PresetString::LgOled => "HDR LG OLED.icc", // Update these to your own ICC profile names if you want to save some typing.
            PresetString::SteamDeck => "HDR Steam Deck.icc",
            PresetString::TestProfile => "HDR bad test.icc",
        }
    }

    fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::LgOled),
            1 => Some(Self::SteamDeck),
            2 => Some(Self::TestProfile),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct StringOrPreset(String);

impl FromStr for StringOrPreset {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Try to parse as number first
        if let Ok(num) = s.parse::<u8>() {
            let preset = PresetString::from_index(num)
                .ok_or_else(|| "Invalid preset number".to_string())?;
            Ok(StringOrPreset(preset.as_str().to_string()))
        } else {
            // If not a number, use the string as-is
            Ok(StringOrPreset(s.to_string()))
        }
    }
}


#[derive(Subcommand)]
enum Commands {
    // Test suite
    Test {
        #[command(subcommand)]
        subcommand: TestCommands,
    },
    #[command(
        alias = "cpdm",
        about = "Change the primary display mode (must be a mode reported by the display)"
    )]
    ChangePrimaryDisplayMode { // Positional arguments
        #[arg(help = "Width of the display resolution")]
        width: u32,
        #[arg(help = "Height of the display resolution")]
        height: u32,
        #[arg(help = "Refresh rate of the display resolution")]
        refresh_rate: u32,
    },
    #[command(
        alias = "ssdrl",
        about = "Set the SDR white level of the primary display"
    )]
    SetSdrLevel {
        #[arg(
            value_parser = value_parser!(u32).range(0..=100),
            help = "SDR white level (0-100, matches Windows SDR content brightness slider)"
        )]
        level: u32,
    },
    #[command(
        alias = "sicc",
        about = "Set the ICC profile for the primary display"
    )]
    SetICCProfile {
        #[arg(help = "Name of the ICC profile to set. Remember to include the *.icc extension! You can also enter a preset number here, but the names are hardcoded, so that's only if you built this yourself and changed the enum.")]
        profile_name: StringOrPreset,
    },
}

#[derive(Subcommand)]
enum TestCommands {
    Echo {
        // Message to echo
        #[arg(short, long)]
        message: String,
    },
    #[command(alias = "ed")]
    EnumerateDisplays,
    #[command(alias = "pdm")]
    PrimaryDisplayModes,
    #[command(alias = "licc")]
    ListICCProfiles,
    #[command(alias = "qdc")]
    QueryDisplayConfig, //TODO: Remove this test command
}

//==============================================================================
// Logger setup
//==============================================================================

fn setup_logger(logging_enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !logging_enabled {
        return Ok(());
    }

    let log_file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open("sunshine-helper.log")?;

    Builder::new()
        .format(|buf, record| {
            writeln!(buf,
                     "{} [{}] - {}",
                     Local::now().format("%Y-%m-%d %H:%M:%S"),
                     record.level(),
                     record.args()
            )
        })
        .filter_level(LevelFilter::Info)
        .target(Target::Pipe(Box::new(log_file)))
        .init();

    Ok(())
}

//==============================================================================
// Main function - CLI parsing
//==============================================================================

fn main() {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Setup logger
    if let Err(e) = setup_logger(cli.log) {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    }

    match cli.command {
        Commands::Test { subcommand } => match subcommand {
            TestCommands::Echo { message } => {
                info!("Echo test command received with message: {}", message);
                println!("Echo: {}", message);
            }
            TestCommands::EnumerateDisplays => {
                info!("Display enumeration test initiated");
                let displays = enumerate_displays();

                match displays.len() {
                    0 => {
                        println!("Error: No displays found!");
                    }
                    _ => {
                        println!("\nDisplay Information:");
                        println!("-------------------");

                        for display in displays {
                            println!("\nDevice Index: {}", display.device_index);
                            println!("Name: {}", display.device_name);
                            println!("Description: {}", display.device_string);
                            println!("Primary Display: {}", if display.is_primary { "Yes" } else { "No" });
                            println!("Current Resolution: {}x{}",
                                     display.current_resolution.0,
                                     display.current_resolution.1);
                            println!("Refresh Rate: {}Hz", display.current_refresh_rate);
                            println!("State Flags: {:#010x}", display.state_flags);
                        }
                    }
                }
            }
            TestCommands::PrimaryDisplayModes => {
                match displays_info::get_primary_display_info() {
                    Some((primary, modes)) => {
                        println!("\nPrimary Display Information:");
                        println!("-------------------------");
                        println!("Name: {} ({})", primary.device_name, primary.device_string);
                        println!("Current: {}x{} @{}Hz",
                                 primary.current_resolution.0,
                                 primary.current_resolution.1,
                                 primary.current_refresh_rate
                        );
                        println!("\nSupported Modes:");
                        for mode in &modes {
                            println!("  {}x{} @{}Hz", mode.width, mode.height, mode.refresh_rate);
                        }
                    }
                    None => {
                        println!("Error: Failed to get primary display information");
                    }
                }
            }
            TestCommands::ListICCProfiles => {
                info!("ICC profile enumeration test initiated");
                if let Some((primary, _)) = displays_info::get_primary_display_info() {
                    let profiles = change_icc_profile::list_icc_profiles();

                    match profiles.len() {
                        0 => println!("No ICC profiles found for primary display"),
                        _ => {
                            println!("\nICC Profiles for Primary Display:");
                            println!("--------------------------------");
                            println!("Display: {} ({})", primary.device_name, primary.device_string);

                            for (profile_name, profile_path) in profiles {
                                println!("\nProfile Name: {}", profile_name);
                                println!("Profile Path: {}", profile_path.display());
                            }
                        }
                    }
                } else {
                    println!("Error: Failed to get primary display information");
                }
            }
            TestCommands::QueryDisplayConfig => { //TODO: Remove this test command
                info!("QueryDisplayConfig test initiated");
                if let Err(e) = displays_info::test_query_display_config() {
                    println!("Error querying display config: {}", e);
                }
            }
        }
        Commands::ChangePrimaryDisplayMode { width, height, refresh_rate } => {
            info!("Change primary display mode command received with parameters: {}x{} @{}Hz", width, height, refresh_rate);
            if change_display_mode::change_primary_display_mode(width, height, refresh_rate) {
                println!("Successfully changed primary display mode to {}x{} @{}Hz", width, height, refresh_rate);
            } else {
                println!("Failed to change primary display mode to {}x{} @{}Hz", width, height, refresh_rate);
            }
        }
        Commands::SetSdrLevel { level } => {
            match set_sdr_level::set_primary_display_sdr_white(level) {
                Ok(()) => println!("Successfully set SDR white level to {}", level),
                Err(e) => {
                    println!("Failed to set SDR white level: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::SetICCProfile { profile_name } => {
            info!("Set ICC profile command received with profile name: {}", profile_name.0);
            match change_icc_profile::change_primary_display_icc_profile(&profile_name.0) {
                Ok(()) => println!("Successfully set ICC profile to '{}'", profile_name.0),
                Err(e) => {
                    println!("Failed to set ICC profile: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}
