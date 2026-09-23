# Windows Watermark Remover

Windows Watermark Remover is a portable no-install app that hides the desktop-corner build watermark, including the Insider Preview desktop watermark. Download `wwr.exe` and double-click it. There is no setup, no window, and no button to click. The corner build number is turned off, and the setting stays after a reboot.

## Download and run

1. Open the latest successful **Build** run on GitHub Actions and download the `wwr-windows-amd64` artifact, or take `wwr.exe` from a release that attached that file.
2. Double-click `wwr.exe`. Windows may ask you to confirm an unrecognized app the first time. The program itself shows nothing.
3. Explorer restarts. The taskbar flashes, open File Explorer windows close, and the desktop build text is gone.

No administrator account is required. Running the app again writes the same setting and restarts Explorer again. That is safe.

To build the portable executable yourself:

```sh
GOOS=windows GOARCH=amd64 CGO_ENABLED=0 go build -trimpath -ldflags "-H windowsgui -s -w" -o wwr.exe .
```

## What it changes

The app sets `PaintDesktopVersion` to `0` under `HKEY_CURRENT_USER\Control Panel\Desktop`.

Windows paints the bottom-right desktop version from that value. `1` shows the build text. `0` hides it. The text looks like “Windows 11 Pro Insider Preview Build …” or a plain Windows build number. The same string is what some Windows 11 24H2 installs show in the corner.

Nothing else is changed. The app does not edit system files, does not install a service, and does not change Windows activation or licensing.

## Remove the build number from the desktop

`wwr.exe` is a single file. It does not add a Start menu entry or leave a background process running. After it saves the setting, it restarts Explorer so the corner text disappears without a sign-out.

## Run it again

A second run sets `PaintDesktopVersion` to `0` again and restarts Explorer again. A normal reboot does not bring the corner text back. Run the file again if you want the same result on another Windows user account.

## Undo

Show the build number on the desktop again by setting `PaintDesktopVersion` back to `1`:

1. Press `Win + R`, type `regedit`, and press Enter.
2. Open `HKEY_CURRENT_USER\Control Panel\Desktop`.
3. Set `PaintDesktopVersion` to `1`. If it is missing, create it as a DWORD (32-bit) value.
4. Sign out and back in, or restart Explorer from Task Manager.

## FAQ

### Does this remove the Insider Preview desktop watermark?

It hides the desktop build string, including an Insider Preview build number painted from `PaintDesktopVersion`. That is the corner text on many Insider installs and on Windows 11 24H2 when the desktop version paint is on.

### Do I need an administrator account?

No. The value is stored for the current user.

### Does this activate Windows?

No. Activation and licensing are left as they are. The separate “Activate Windows” notice, and the Canary channel “evaluation copy” mark, are not this desktop build string. This app does not remove those notices.

### Is a portable no-install build available?

Yes. `wwr.exe` is one portable executable. There is no installer.
