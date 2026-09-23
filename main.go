// Windows Watermark Remover hides the Insider evaluation watermark.
// It has no window and no prompts. Double-click the executable and it exits.
package main

import (
	"os"
	"os/exec"
	"syscall"

	"golang.org/x/sys/windows/registry"
)

const (
	// createNoWindow stops taskkill from flashing a console window.
	createNoWindow = 0x08000000

	// SPI_SETDISABLEOVERLAPPEDCONTENT is the Ease of Access switch
	// "Remove background images (where available)". Windows uses it to
	// turn off desktop background images and watermarks. TRUE disables them.
	spiSetDisableOverlappedContent = 0x1041
	spifUpdateIniFile              = 0x01
	spifSendChange                 = 0x02
)

func main() {
	if err := hideDesktopVersion(); err != nil {
		os.Exit(1)
	}
	if err := hideEvaluationWatermark(); err != nil {
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

// hideEvaluationWatermark turns on Remove background images.
// The Insider "Evaluation copy" line is painted with the desktop background.
// PaintDesktopVersion does not control that line. This switch does, and it
// also clears the desktop picture. The plain background remains after reboot.
func hideEvaluationWatermark() error {
	user32 := syscall.NewLazyDLL("user32.dll")
	systemParametersInfo := user32.NewProc("SystemParametersInfoW")
	r, _, callErr := systemParametersInfo.Call(
		uintptr(spiSetDisableOverlappedContent),
		0,
		uintptr(1),
		uintptr(spifUpdateIniFile|spifSendChange),
	)
	if r == 0 {
		if callErr != nil && callErr != syscall.Errno(0) {
			return callErr
		}
		return syscall.EINVAL
	}
	return nil
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
