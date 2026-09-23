# Windows Watermark Remover

Windows Watermark Remover is a portable no-install app that hides the Windows Insider evaluation watermark in the bottom-right corner of the desktop. That line reads “Evaluation copy. Build …” under “Windows 11 Pro Insider Preview”. Download `wwr.exe` and double-click it. There is no setup, no window, and no button to click.

The desktop picture is cleared too. Windows hides this watermark by turning off desktop background images, so the desktop becomes a solid color. The change stays after a reboot.

## Download and run

1. Open the latest successful **Build** run on GitHub Actions and download the `wwr-windows-amd64` artifact, or take `wwr.exe` from a release that attached that file.
2. Double-click `wwr.exe`. Windows may ask you to confirm an unrecognized app the first time. The program itself shows nothing.
3. Explorer restarts. The taskbar flashes, open File Explorer windows close, the wallpaper is removed, and the evaluation watermark is hidden.

No administrator account is required. Running the app again writes the same settings and restarts Explorer again.

To build the portable executable yourself:

```sh
GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build -trimpath -ldflags "-H windowsgui -s -w" -o wwr.exe .
```

## What it changes

The Insider evaluation watermark on builds such as Windows 11 Beta `26220` is not the optional build-number paint. Setting `PaintDesktopVersion` to `0` leaves “Evaluation copy. Build …” on the desktop. Windows draws that line with the desktop background.

The app turns on the Windows setting **Remove background images (where available)** from Ease of Access. That setting is `SPI_SETDISABLEOVERLAPPEDCONTENT`. Windows documents it as the switch for background images and watermarks. The desktop picture is removed and the corner evaluation text goes with it.

The app also sets `PaintDesktopVersion` to `0` under `HKEY_CURRENT_USER\Control Panel\Desktop`, which hides the separate optional build string.

Nothing is installed. The app does not edit system files, does not install a service, and does not change Windows activation or licensing.

## Remove the Insider evaluation watermark

`wwr.exe` is a single file. It does not add a Start menu entry or leave a background process running. After it saves the settings, it restarts Explorer so the corner text disappears without a sign-out.

## Run it again

A second run applies the same settings and restarts Explorer again. A normal reboot does not bring the evaluation watermark or the old wallpaper back. Run the file again on another Windows user account if that account should get the same result.

## Undo

Bring the wallpaper and the evaluation watermark back from Ease of Access:

1. Open Control Panel and go to Ease of Access, then Ease of Access Center.
2. Open **Make the computer easier to see**.
3. Clear **Remove background images (where available)** and choose Apply.
4. Set the desktop background again in Settings.

To show the optional build number as well, set `PaintDesktopVersion` under `HKEY_CURRENT_USER\Control Panel\Desktop` back to `1`, then sign out and back in.

## FAQ

### Does this remove the Insider Preview evaluation copy watermark?

Yes. It hides the “Evaluation copy. Build …” desktop watermark, including the one on Windows 11 Insider Beta builds such as `26220`, by turning off desktop background images. The desktop picture is removed with it.

### Do I need an administrator account?

No. Both settings belong to the current user.

### Does this activate Windows?

No. Activation and licensing stay as they are. The separate “Activate Windows” notice is not this evaluation watermark.

### Is a portable no-install build available?

Yes. `wwr.exe` is one portable executable. There is no installer.
