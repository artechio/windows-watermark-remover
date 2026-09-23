# Windows Watermark Remover

Windows Watermark Remover is a portable Windows app that hides the Insider evaluation watermark in the desktop corner (`Evaluation copy. Build …`). It includes an app icon and a shadcn/ui window with a live activity log, so you can see every step while it runs.

## Download and run

1. Open the latest successful **Build** run on GitHub Actions.
2. Download the `wwr-windows-amd64` artifact.
3. Run `wwr.exe`.
4. Click **Remove watermark** and watch the log.

The app patches `shell32!CDesktopWatermark::s_DesktopBuildPaint` inside the running Explorer process (UWD2-style). It does not modify files under `C:\Windows`. It registers a current-user Run entry so the patch can be re-applied after logon.

## Develop

```sh
npm install
npm run build        # web UI only
npm run build:app    # Tauri desktop build (Windows)
```

Frontend: Vite + React + shadcn/ui.  
Backend: Rust / Tauri 2 with PDB download, pattern cache, and structural scan fallbacks.

## Notes

- Does **not** remove the separate “Activate Windows” watermark.
- Does **not** change Windows activation.
- First run may need internet access for Microsoft `shell32.pdb` symbols. If symbols are not published yet, the log will show the 404 and the structural scan attempt.
