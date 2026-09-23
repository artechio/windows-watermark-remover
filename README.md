# Windows Watermark Remover

Windows Watermark Remover is a portable no-install app that hides the Windows Insider evaluation watermark in the bottom-right corner of the desktop. That text looks like:

```text
Windows 11 Pro Insider Preview
Evaluation copy. Build 26220....
```

Download `wwr.exe` and double-click it. There is no setup, no window, and no button to click. The watermark disappears from the live desktop. The app also adds itself to the current user’s logon Run key so the patch is applied again after you sign in.

## Download and run

1. Open the latest successful **Build** run on GitHub Actions and download the `wwr-windows-amd64` artifact.
2. Double-click `wwr.exe`. Windows may warn about an unrecognized app once. The program itself shows nothing.
3. The desktop refreshes and the evaluation watermark is gone.

No administrator account is required for a normal same-user Explorer process. The first run may need internet access so Microsoft debug symbols for `shell32.dll` can be downloaded. Later runs use a local cache. If symbols are not published yet for a new Insider build, the app falls back to a structural scan of `shell32.dll` on disk.

Build it yourself on Windows:

```sh
cargo build --release
```

The binary is `target\release\wwr.exe`.

## How it works

This uses the same idea as [UWD2](https://github.com/machineonamission/uwd2):

1. Locate `CDesktopWatermark::s_DesktopBuildPaint` inside `shell32.dll`.
2. Prefer Microsoft public symbols (PDB) when they are available, and cache the RVA.
3. If the symbol server returns 404 for a brand-new Insider build, fall back to a structural scan (GDI `SetTextColor` call sites with white color) and to saved function byte patterns from an earlier successful run.
4. Write a single `ret` instruction into the matching code in the running `explorer.exe` process.
5. Refresh the desktop shell so the watermark is redrawn (and therefore skipped).

The patch lives only in Explorer’s memory. It does not modify files under `C:\Windows`. After Explorer or Windows restarts, run `wwr.exe` again, or rely on the logon Run entry the app creates.

## What it does not do

- It does **not** remove the separate “Activate Windows” watermark.
- It does **not** change Windows activation or licensing.
- It does **not** install a driver or a service.

## Undo

1. Open Task Manager and restart Windows Explorer, or sign out and back in without letting `wwr.exe` run.
2. Remove the logon entry: open `regedit`, go to `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`, and delete `WindowsWatermarkRemover`.

## FAQ

### Why did UWD2 stop working on new Insider builds?

New Insider builds often publish `shell32` PDBs days later. UWD2 needed that PDB on first run. This app still uses PDBs when present, and uses a no-PDB structural scan plus pattern cache when the symbol server returns 404.

### Does the watermark stay gone after reboot?

The memory patch does not survive Explorer restart. `wwr.exe` registers itself under the current user’s Run key so it re-applies the patch at logon with no clicks.

### Is a portable no-install build available?

Yes. `wwr.exe` is one portable executable. There is no installer.
