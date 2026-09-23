// Windows Watermark Remover hides the desktop-corner build string.
// It has no window and no prompts. Double-click the executable and it exits.
package main

import (
	"os"
	"os/exec"
	"syscall"

	"golang.org/x/sys/windows/registry"
)

// createNoWindow stops taskkill from flashing a console window.
const createNoWindow = 0x08000000

func main() {
	if err := hideDesktopVersion(); err != nil {
		os.Exit(1)
	}
	if err := restartExplorer(); err != nil {
		os.Exit(1)
	}
}

// hideDesktopVersion turns off the optional desktop build paint.
// Windows reads PaintDesktopVersion from the current user: 1 shows the
// corner text, 0 hides it. The value persists across reboots.
func hideDesktopVersion() error {
	key, _, err := registry.CreateKey(
		registry.CURRENT_USER,
		`Control Panel\Desktop`,
		registry.SET_VALUE,
	)
	if err != nil {
		return err
	}
	defer key.Close()
	return key.SetDWordValue("PaintDesktopVersion", 0)
}

// restartExplorer applies the setting immediately. Explorer is stopped,
// then started again, so the user does not have to sign out.
func restartExplorer() error {
	kill := exec.Command("taskkill.exe", "/f", "/im", "explorer.exe")
	kill.SysProcAttr = &syscall.SysProcAttr{
		HideWindow:    true,
		CreationFlags: createNoWindow,
	}
	if err := kill.Run(); err != nil {
		return err
	}

	// Do not hide this process. Explorer is the Windows shell.
	return exec.Command("explorer.exe").Start()
}
