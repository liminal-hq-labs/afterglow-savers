#!/usr/bin/env python3
# Runs the savers in a copy of the Afterglow ReactOS image under QEMU, headless, and takes screendumps through the monitor.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Usage: python3 -I scripts/guest/guest.py --scr DIR [--source IMG] [--work DIR] [--shots DIR] [--seconds N] [--audio OUT.wav] [--only NAME]...

The source image is only read: it is copied (sparse) into the work folder, the `.scr` files from DIR are written into
`ReactOS/system32` with reactos-liminal's mtools wrapper (imported read-only from a reactos-liminal checkout), and a start-up script is
put in the All Users StartUp folder. That script sets the screensaver in the registry, opens Display Properties on its Screen Saver tab
(the list and the monitor preview), then runs each saver full screen with /s for a while and ends it, writing a log with times. QEMU
runs with KVM, a Pentium III CPU model, no display and a monitor socket; the host only sends `screendump` and `quit`. Only this
script's own QEMU process is ever stopped. With --audio, the guest gets an AC97 sound card whose output QEMU writes to that WAV file (no speakers), for checking the savers' sound.
The copy is deleted at the end unless --keep is given.
"""

import argparse
import os
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
SAVERS = ["AGINK.SCR", "AGTWILIT.SCR", "AGCRACK.SCR", "AGMAZE.SCR", "AGCOLLCT.SCR", "AGQUILL.SCR"]
STARTUP_DIRS = [
	"Documents and Settings/All Users/Start Menu/Programs/StartUp",
	"Documents and Settings/All Users/Start Menu/Programs/Startup",
]


def fat_image(path: Path):
	for checkout in RL_CHECKOUTS:
		if (checkout / "reactos_liminal" / "mtools.py").is_file():
			sys.path.insert(0, str(checkout))
			from reactos_liminal.mtools import FatImage

			return FatImage(path)
	sys.exit("guest: no reactos-liminal checkout with reactos_liminal/mtools.py")


def startup_script(savers: list[str], dwell: int) -> str:
	"""The batch file the guest runs at logon. Lines end in CRLF; `ping -n N 127.0.0.1` is the sleep that every ReactOS has."""
	lines = [
		"@echo off",
		"echo start %TIME% > C:\\savers-log.txt",
		"regedit /s C:\\savers.reg",
		"ping -n 21 127.0.0.1 > nul",
		"echo applet %TIME% >> C:\\savers-log.txt",
		'start "" rundll32.exe shell32.dll,Control_RunDLL desk.cpl,,1',
		"ping -n 41 127.0.0.1 > nul",
		"taskkill /f /im rundll32.exe >> C:\\savers-log.txt 2>&1",
	]
	for name in savers:
		lines += [
			f"echo {name} %TIME% >> C:\\savers-log.txt",
			f'start "" C:\\ReactOS\\system32\\{name} /s',
			f"ping -n {dwell + 1} 127.0.0.1 > nul",
			f"taskkill /f /im {name} >> C:\\savers-log.txt 2>&1",
			"ping -n 4 127.0.0.1 > nul",
		]
	lines.append("echo done %TIME% >> C:\\savers-log.txt")
	return "\r\n".join(lines) + "\r\n"


REG = """Windows Registry Editor Version 5.00\r
\r
[HKEY_CURRENT_USER\\Control Panel\\Desktop]\r
"SCRNSAVE.EXE"="C:\\\\ReactOS\\\\system32\\\\AGTWILIT.SCR"\r
"ScreenSaveActive"="1"\r
"ScreenSaveTimeOut"="3600"\r
"ScreenSaverIsSecure"="0"\r
"""


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
	parser.add_argument("--scr", required=True, type=Path)
	parser.add_argument("--source", type=Path, default=Path.home() / ".cache/reactos-liminal-fulllook/afterglow-full-new.img")
	parser.add_argument("--work", type=Path, default=Path.home() / ".cache/afterglow-savers/guest")
	parser.add_argument("--shots", type=Path, default=Path.home() / ".cache/afterglow-savers/guest/shots")
	parser.add_argument("--dwell", type=int, default=30)
	parser.add_argument("--keep", action="store_true")
	parser.add_argument("--audio", type=Path, help="write the guest's AC97 output to this WAV file")
	parser.add_argument("--only", action="append", default=[], help="run only this .scr (repeatable), e.g. AGINK.SCR")
	args = parser.parse_args()

	free = shutil.disk_usage(Path.home()).free
	if free < 12 * 1024**3:
		sys.exit(f"guest: only {free // 1024**3} GB free on /home; need 12")
	args.work.mkdir(parents=True, exist_ok=True)
	args.shots.mkdir(parents=True, exist_ok=True)
	image = args.work / "afterglow-savers.img"
	subprocess.run(["cp", "--sparse=always", "--reflink=auto", str(args.source), str(image)], check=True)
	fat = fat_image(image)
	savers = [name for name in SAVERS if (args.scr / name).is_file() and (not args.only or name in args.only)]
	if not savers:
		sys.exit(f"guest: no .scr files in {args.scr}")
	for name in savers:
		fat.copy_in(args.scr / name, f"ReactOS/system32/{name}")
	staging = args.work / "staging"
	staging.mkdir(exist_ok=True)
	(staging / "savers.reg").write_bytes(REG.encode("ascii"))
	(staging / "savers-test.cmd").write_bytes(startup_script(savers, args.dwell).encode("ascii"))
	fat.copy_in(staging / "savers.reg", "savers.reg")
	startup = next((d for d in STARTUP_DIRS if fat.exists(d)), None)
	if startup is None:
		sys.exit("guest: no All Users StartUp folder in the image")
	fat.copy_in(staging / "savers-test.cmd", f"{startup}/savers-test.cmd")
	print(f"guest: {len(savers)} savers and the start-up script are in {image}")

	sock = args.work / "monitor.sock"
	if sock.exists():
		sock.unlink()
	sound = ["-audiodev", f"wav,id=snd0,path={args.audio}", "-device", "AC97,audiodev=snd0"] if args.audio else []
	qemu = subprocess.Popen(
		[
			"qemu-system-i386", "-machine", "pc", "-cpu", "pentium3", "-m", "768", "-enable-kvm", "-vga", "std",
			"-display", "none", "-monitor", f"unix:{sock},server,nowait", "-usb", "-device", "usb-tablet",
			"-drive", f"file={image},format=raw,if=ide,index=0",
			*sound,
		]
	)
	print(f"guest: QEMU pid {qemu.pid}")
	try:
		# Measured in the Afterglow image with KVM: the start-up script begins about 50 s after power-on; it waits 20 s, shows the
		# applet for 40 s, then runs each saver for `dwell` seconds with 3 s between (about dwell + 4.5 s each, with taskkill).
		schedule = [("desktop", 62), ("applet", 85)]
		at = 110
		for name in savers:
			schedule.append((name.lower().replace(".scr", "") + "-a", at + 10))
			schedule.append((name.lower().replace(".scr", "") + "-b", at + args.dwell - 6))
			at += args.dwell + 4.5
		start = time.monotonic()
		for label, when in schedule:
			while time.monotonic() - start < when:
				time.sleep(1)
				if qemu.poll() is not None:
					sys.exit("guest: QEMU ended early")
			ppm = args.shots / f"{label}.ppm"
			monitor(sock, f"screendump {ppm}")
			time.sleep(1)
			subprocess.run(["magick", str(ppm), str(ppm.with_suffix(".png"))], check=False)
			ppm.unlink(missing_ok=True)
			print(f"guest: {label} at {int(time.monotonic() - start)} s")
		time.sleep(20)
		monitor(sock, "quit")
		qemu.wait(timeout=30)
	finally:
		if qemu.poll() is None:
			qemu.terminate()
			qemu.wait(timeout=30)
	log = args.work / "savers-log.txt"
	try:
		fat.copy_out("savers-log.txt", log)
		print(log.read_text(errors="replace"))
	except Exception as error:  # the log is a help, not a requirement
		print(f"guest: no log ({error})")
	if not args.keep:
		image.unlink()
	return 0


if __name__ == "__main__":
	sys.exit(main())
