# Sunshine HDR Helper

## What is this?
This is a command line utility intended for Windows 11 users to help with some of the issues you might run into when using [Sunshine](https://github.com/LizardByte/Sunshine) / [Moonlight](https://github.com/moonlight-stream) to stream your games from your host's primary display to a client device. I made this to help with streaming to my Steam Deck OLED, but you might find it useful more generally.

More specifically this CLI utility is intended to use the stream start/stop hooks that can be configured in the Sunshine web interface, to help with some problems you might have with streaming from a HDR host display on Windows 11. These hooks can be found when you create or edit an entry on the `Applications` tab, under `Command Preparations`. See the Sunshine [Prep Commands](https://docs.lizardbyte.dev/projects/sunshine/en/latest/about/guides/app_examples.html#prep-commands) documentation for more details.

There are currently three available commands (for the primary display only):
- `change-primary-display-mode`, `cpdm`: Change the host resolution and refresh rate to another supported by the display. For example, you can set a resolution of 1280x800x90Hz to optimize streaming to a Steam Deck. This is particularly useful if you're finding Sunshine's downscaling from 2160p to be a bit "crunchy", or you have frame pacing issues because the host display refresh doesn't easily fit the client display. I know there are other utilities that do this, but I wanted everything in one place.

- `set-sdr-level`, `ssdrl`: Change the Windows SDR brightness boost for the primary display (normally found at Settings > Display > HDR). If you sometimes stream to HDR clients but also use SDR clients, setting the brightness boost to 0 should solve the client looking washed out. Then you can set it back to your normal setting when the stream is ended. Big thanks to Microsoft for not documenting this part of the Windows API at all. Credit to [this heroic StackOverflow user](https://stackoverflow.com/a/78435051) for sharing their findings!

- `set-icc-profile`, `sicc`: Change the default ICC profile to another one associated with the primary display. This is the most important optimisation for HDR streaming. Each HDR client needs to be set up with the [Windows HDR Calibration Tool](https://support.microsoft.com/en-gb/windows/calibrate-your-hdr-display-using-the-windows-hdr-calibration-app-f30f4809-3369-43e4-9b02-9eabebd23f19) (while streaming to Moonlight), to match the client's display capabilities. Without this, your client will inherit the HDR calibration of your host, with an incorrect gamma curve. For example, my Steam Deck OLED has a vastly different max luminance to my LG C2 OLED used on the host machine. Switching to the correctly calibrated ICC profile will make sure your shadows and highlights are properly rendered.

Example usage:
- `sunshine_helper.exe change-primary-display-mode 1920 1080 60`
- `sunshine_helper.exe set-sdr-level 50`
- `sunshine_helper.exe set-icc-profile "My awesome ICC profile.icc"`

Sunshine allows multiple commands to be set if you need to.

There is some limited help text available with the `--help` flag.

## Limitations
- It only targets the primary display. This works for my purposes and should be applicable to most gamers because of the way games like to choose where to render. If you want to target a secondary display, or switch the primary display when you start streaming (e.g. to a virtual display that advertises HDR support to your Steam Deck), the code should be extensible enough to make that easy to do, if you fork it. I might get around to supporting this in the future.
- Custom resolutions added in the Nvidia Control Panel do not seem to be reported through the Windows API, so will fail the validation check. If you're feeling brave, you can use the `--unsafe` flag with `change-primary-display-mode` if you're very sure the target resolution and framerate is supported.
- The utility is intended exclusively for Windows 11. It might work for Windows 10 in a limited way, but if you're using a HDR display you really should move to Windows 11 if you can bear it. Win10 HDR support is not great and IIRC you will also miss out on AutoHDR in games that support it.
- Error handling is very basic and incomplete. Sorry.
- Logging to a file is a bit spammy, but is disabled by default. Use the --log flag to enable it. There's a slim chance you might get a useful error message out of it, if you need one.
- I hard coded my ICC profile names for ease of use. If you want to do that too, you'll have to edit `main.rs` and build it yourself. You can still provide the name of any valid profile to the `set-icc-profile` command as a string, of course.
- This whole thing was made with copious amounts of AI assistance. I've never used Rust for a project before, nor made use of the Windows API. If the code looks bad, you should've seen it before I spent many hours bullying the AI into getting this just barely working. I share this only in the hope it will be useful to someone, somewhere.