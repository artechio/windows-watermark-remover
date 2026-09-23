# Windows Watermark Remover

**Windows Watermark Remover** is a free, portable Windows tool to **remove the Insider Preview evaluation copy watermark** from the desktop corner. It hides text such as `Windows 11 Pro Insider Preview` and `Evaluation copy. Build …` without an installer and without changing Windows activation.

Download the latest release, run `wwr.exe`, click **Remove watermark**, and follow the live activity log.

## Keywords

Windows watermark remover · Insider Preview watermark · Evaluation copy desktop watermark · remove build number from desktop · Windows 11 Insider Beta · portable no install · UWD2-style patcher

## Download

Get the latest portable build from **[Releases](https://github.com/artechio/wwr/releases/latest)**.

1. Download `wwr.exe` from the newest release.
2. Double-click the app (no setup).
3. Click **Remove watermark**.
4. Watch the log until it reports success.

You can also grab a CI artifact named `wwr-windows-amd64` from [Actions](https://github.com/artechio/wwr/actions).

## What it removes

| Watermark | Supported |
| --- | --- |
| Insider / Evaluation copy build string on the desktop | Yes |
| Optional desktop build paint (`PaintDesktopVersion`) | Yes (as part of the same shell paint path) |
| “Activate Windows” notice | No |

## How it works

The app finds `CDesktopWatermark::s_DesktopBuildPaint` in `shell32.dll` (Microsoft PDB when available, otherwise a structural scan), writes a single `ret` into the running `explorer.exe` process, and refreshes the desktop. System files under `C:\Windows` are not modified. A current-user Run entry re-applies the patch after logon.

## Features

- Portable Windows executable — no installer
- App icon and shadcn/ui window with a live activity log
- Works when Insider PDBs are late (404) by falling back to a shell32 scan
- No administrator account required for a normal same-user Explorer process

## Develop

```sh
npm install
npm run build        # web UI
npm run build:app    # Tauri desktop build on Windows
```

Frontend: Vite + React + shadcn/ui.  
Backend: Rust / Tauri 2.

## License

MIT. See [LICENSE](LICENSE).
