#!/usr/bin/env python3
# Starts QUILLGW.SCR three ways in a copy of a ReactOS image under QEMU, headless, and saves screendumps and the command lines it saw.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Usage: python3 -I platforms/win16/tools/guest-reactos.py QUILLGW.SCR [--source IMG] [--work DIR] [--shots DIR]

The source image (default: the Quill remix, which carries otvdm in C:\\Program Files\\otvdm) is only read: it is copied (sparse), the saver
goes into `ReactOS/system32`, CONTROL.INI (in the Windows folder, the root, system32 and otvdm's folder, since where a 16-bit program looks differs) gets `Log=1` so the saver writes the command line of each start to `C:\\QGWn.LOG`, the registry
names it as the screensaver, and a start-up script runs it

  1. a copy named `C:\\QGW\\QUILLGW.EXE` through `otvdmw.exe` and `otvdm.exe`, then through ReactOS's own hand-off;
  2. Display Properties' Screen Saver tab (the list), left open for a screendump.

QEMU runs with KVM, no display and a monitor socket; the host only sends `screendump` and `quit`. Only this script's own QEMU is stopped.
"""

import argparse
import shutil
import socket
import subprocess
import sys
import time
from pathlib import Path

RL_CHECKOUTS = [
	Path.home() / "source/worktrees/reactos-liminal/full-look",
	Path.home() / "source/liminal-hq-labs/reactos-liminal",
]
STARTUP_DIRS = [
	"Documents and Settings/All Users/Start Menu/Programs/StartUp",
	"Documents and Settings/All Users/Start Menu/Programs/Startup",
]
SCRIPT = [
	"@echo off",
	"echo start %TIME% > C:\\qgw-log.txt",
	"regedit /s C:\\qgw.reg",
	"ping -n 41 127.0.0.1 > nul",
	"echo otvdmw %TIME% >> C:\\qgw-log.txt",
	'cd /d "C:\\Program Files\\otvdm"',
	"start \"\" otvdmw.exe C:\\QGW\\QUILLGW.EXE /s",
	"ping -n 41 127.0.0.1 > nul",
	"echo otvdm %TIME% >> C:\\qgw-log.txt",
	"start \"\" otvdm.exe C:\\QGW\\QUILLGW.EXE /s",
	"ping -n 41 127.0.0.1 > nul",
	"echo handoff %TIME% >> C:\\qgw-log.txt",
	"start \"\" C:\\QGW\\QUILLGW.EXE /s",
	"ping -n 31 127.0.0.1 > nul",
	"echo applet %TIME% >> C:\\qgw-log.txt",
	'start "" rundll32.exe shell32.dll,Control_RunDLL desk.cpl,,1',
	"ping -n 61 127.0.0.1 > nul",
	"echo done %TIME% >> C:\\qgw-log.txt",
]
REG = (
	"Windows Registry Editor Version 5.00\r\n\r\n[HKEY_CURRENT_USER\\Control Panel\\Desktop]\r\n"
	'"SCRNSAVE.EXE"="C:\\\\ReactOS\\\\system32\\\\QUILLGW.SCR"\r\n"ScreenSaveActive"="1"\r\n"ScreenSaveTimeOut"="3600"\r\n'
	'"ScreenSaverIsSecure"="0"\r\n'
)
INI = "[Screen Saver.Quill Ghostwriter]\r\nLog=1\r\nSpeed=2\r\n"


def fat_image(path: Path):
	for checkout in RL_CHECKOUTS:
		if (checkout / "reactos_liminal" / "mtools.py").is_file():
			sys.path.insert(0, str(checkout))
			from reactos_liminal.mtools import FatImage

			return FatImage(path)
	sys.exit("guest-reactos: no reactos-liminal checkout with reactos_liminal/mtools.py")


def monitor(sock_path: Path, command: str) -> None:
	with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
		connection.settimeout(10)
		connection.connect(str(sock_path))
		time.sleep(0.2)
		connection.recv(4096)
		connection.sendall(command.encode() + b"\n")
		time.sleep(0.5)


def main() -> int:
	parser = argparse.ArgumentParser()
	parser.add_argument("scr", type=Path)
	parser.add_argument("--source", type=Path, default=Path.home() / ".cache/quill-reactos-liminal-work/images/qagrr.img")
	parser.add_argument("--work", type=Path, default=Path.home() / ".cache/afterglow-savers/guest-win16")
	parser.add_argument("--shots", type=Path, default=Path.home() / ".cache/afterglow-savers/guest-win16/shots")
	parser.add_argument("--times", default="100,115,130,145,165,185,215,240", help="seconds after power-on to take a screendump at")
	args = parser.parse_args()

	if shutil.disk_usage(Path.home()).free < 6 * 1024**3:
		sys.exit("guest-reactos: less than 6 GB free on /home")
	args.work.mkdir(parents=True, exist_ok=True)
	args.shots.mkdir(parents=True, exist_ok=True)
	image = args.work / "win16.img"
	subprocess.run(["cp", "--sparse=always", "--reflink=auto", str(args.source), str(image)], check=True)
	fat = fat_image(image)
	staging = args.work / "staging"
	staging.mkdir(exist_ok=True)
	(staging / "qgw.reg").write_bytes(REG.encode("ascii"))
	(staging / "control.ini").write_bytes(INI.encode("ascii"))
	(staging / "qgw-test.cmd").write_bytes(("\r\n".join(SCRIPT) + "\r\n").encode("ascii"))
	fat.copy_in(args.scr, "ReactOS/system32/QUILLGW.SCR")
	fat.make_directory("QGW")
	fat.copy_in(args.scr, "QGW/QUILLGW.EXE")
	for place in ["ReactOS/control.ini", "control.ini", "Program Files/otvdm/control.ini", "ReactOS/system32/control.ini"]:
		fat.copy_in(staging / "control.ini", place)
	fat.copy_in(staging / "qgw.reg", "qgw.reg")
	startup = next((d for d in STARTUP_DIRS if fat.exists(d)), None)
	if startup is None:
		sys.exit("guest-reactos: no All Users StartUp folder in the image")
	fat.copy_in(staging / "qgw-test.cmd", f"{startup}/qgw-test.cmd")

	sock = args.work / "monitor.sock"
	if sock.exists():
		sock.unlink()
	qemu = subprocess.Popen(
		[
			"qemu-system-i386", "-machine", "pc", "-cpu", "pentium3", "-m", "768", "-enable-kvm", "-vga", "std",
			"-display", "none", "-monitor", f"unix:{sock},server,nowait", "-usb", "-device", "usb-tablet",
			"-drive", f"file={image},format=raw,if=ide,index=0",
		]
	)
	print(f"guest-reactos: QEMU pid {qemu.pid}")
	try:
		start = time.monotonic()
		for when in [int(t) for t in args.times.split(",")]:
			while time.monotonic() - start < when:
				time.sleep(1)
				if qemu.poll() is not None:
					sys.exit("guest-reactos: QEMU ended early")
			ppm = args.shots / f"t{when}.ppm"
			monitor(sock, f"screendump {ppm}")
			time.sleep(1)
			subprocess.run(["magick", str(ppm), str(ppm.with_suffix(".png"))], check=False)
			ppm.unlink(missing_ok=True)
			print(f"guest-reactos: shot at {int(time.monotonic() - start)} s")
		monitor(sock, "quit")
		qemu.wait(timeout=30)
	finally:
		if qemu.poll() is None:
			qemu.terminate()
			qemu.wait(timeout=30)
	for name in ["qgw-log.txt", "QGW0.LOG", "QGW1.LOG", "QGW2.LOG", "QGW3.LOG"]:
		try:
			print(f"--- {name}\n{fat.read(name).decode(errors='replace')}")
		except Exception:
			print(f"--- {name}: absent")
	image.unlink()
	return 0


if __name__ == "__main__":
	sys.exit(main())
