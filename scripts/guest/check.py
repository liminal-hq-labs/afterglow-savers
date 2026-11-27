#!/usr/bin/env python3
# Runs every Afterglow saver in a ReactOS guest, headless, and records what it did: the Screen Saver list, each saver's monitor preview,
# its settings dialog, a full-screen run ended by a key and again by the mouse, its frame rate and its memory, and the Collection starting
# by itself after the idle timeout.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Usage: python3 -I scripts/guest/check.py --scr DIR --base IMG --out DIR [--work DIR] [--audio WAV]

IMG is a raw disk image of a ReactOS remix (the Quill remix's `qagrr.img`, for one); it is only read. The guest runs on a copy-on-write
overlay of a reflinked copy of it in the work folder, with the `.scr` files and `fpscheck.exe` written in with reactos-liminal's mtools
wrapper (imported read-only from a reactos-liminal checkout) and a start-up script that measures frame rates, opens Display Properties
on its Screen Saver tab and logs the processes' memory. The host sends only QMP commands to this script's own QEMU, on a socket in the
work folder; nothing touches the host screen. Screendumps and a report go to OUT.
"""

import argparse
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from vm import Vm  # noqa: E402

RL_CHECKOUTS = [
	Path.home() / "source/liminal-hq-labs/reactos-liminal",
	Path.home() / "source/worktrees/reactos-liminal/full-look",
]
SCR_FILES = ["AGMAZE.SCR", "AGCRACK.SCR", "AGTWILIT.SCR", "AGINK.SCR", "AGCOLLCT.SCR", "AGQUILL.SCR"]
STARTUP = "Documents and Settings/All Users/Start Menu/Programs/Startup/pack.cmd"

# What the Screen Saver list shows, in the order the guest lists it (the combo box's rows are 13 pixels apart, the first at y 253).
SAVERS = [
	("collection", "Afterglow Collection", "AGCOLLCT.SCR"),
	("cracktro", "Afterglow Cracktro", "AGCRACK.SCR"),
	("ink", "Afterglow Ink", "AGINK.SCR"),
	("maze", "Afterglow Maze", "AGMAZE.SCR"),
	("twilight", "Afterglow Twilight", "AGTWILIT.SCR"),
	("ghostwriter", "Quill Ghostwriter", "AGQUILL.SCR"),
]
ROW = {"collection": 266, "cracktro": 279, "ink": 292, "maze": 305, "twilight": 318, "ghostwriter": 357}
# Display Properties opens in a different place from one boot to the next, so these are for the dialog with its top left corner at the
# screen's, and `find_dialog` measures how far this boot's dialog is from there by matching the tab row (`tabs.png`).
COMBO, SETTINGS, PREVIEW, OK_BUTTON = (110, 234), (229, 234), (312, 234), (185, 425)
MONITOR = (137, 80, 124, 86)
TABS_AT = (10, 35)

STARTUP_SCRIPT = (
	"@echo off\r\n"
	"echo boot %TIME% > C:\\PACK.LOG\r\n"
	"ping -n 11 127.0.0.1 > nul\r\n"
	"C:\\FPSCHECK.EXE\r\n"
	"echo fps-done %TIME% >> C:\\PACK.LOG\r\n"
	'start "" rundll32.exe shell32.dll,Control_RunDLL desk.cpl,,1\r\n'
	":loop\r\n"
	"echo T %TIME% >> C:\\PACK.LOG\r\n"
	"tasklist | find /i \".SCR\" >> C:\\PACK.LOG\r\n"
	"ping -n 4 127.0.0.1 > nul\r\n"
	"goto loop\r\n"
)


def fat_image(path: Path):
	for checkout in RL_CHECKOUTS:
		if (checkout / "reactos_liminal" / "mtools.py").is_file():
			sys.path.insert(0, str(checkout))
			from reactos_liminal.mtools import FatImage

			return FatImage(path)
	sys.exit("check: no reactos-liminal checkout with reactos_liminal/mtools.py")


def rmse(first: Path, second: Path, crop: str | None = None) -> float:
	"""The root-mean-square difference of two screendumps (0 to 1), of a region when `crop` is given as WxH+X+Y."""
	if crop:
		args = ["magick", str(first), "-crop", crop, "+repage", str(second), "-crop", crop, "+repage"]
	else:
		args = ["magick", str(first), str(second)]
	done = subprocess.run([*args, "-metric", "RMSE", "-compare", "-format", "%[distortion]", "info:"], capture_output=True, text=True)
	try:
		return float(done.stdout.strip())
	except ValueError:
		return float("nan")


def match_tabs(screen: Path) -> tuple[float, int, int]:
	"""Where the Display Properties tab row best matches in a screendump, and how well (0 is exact, about 0.3 or more is not there)."""
	tabs = Path(__file__).resolve().parent / "tabs.png"
	done = subprocess.run(
		["magick", "compare", "-metric", "RMSE", "-subimage-search", str(screen), str(tabs), "null:"], capture_output=True, text=True
	)
	match = re.search(r"\(([\d.e+-]+)\) @ (-?\d+),(-?\d+)", done.stderr)
	if not match:
		sys.exit("check: could not search the screendump for Display Properties")
	return float(match.group(1)), int(match.group(2)) - TABS_AT[0], int(match.group(3)) - TABS_AT[1]


def find_dialog(screen: Path) -> tuple[int, int]:
	"""How far Display Properties is from the top left of the screen: its tab row is searched for in the screendump."""
	distance, dx, dy = match_tabs(screen)
	if distance > 0.1:
		sys.exit(f"check: Display Properties is not on the screen (best match {distance})")
	return dx, dy


def main() -> int:
	parser = argparse.ArgumentParser()
	parser.add_argument("--scr", required=True, type=Path)
	parser.add_argument("--base", required=True, type=Path)
	parser.add_argument("--out", required=True, type=Path)
	parser.add_argument("--work", type=Path, default=Path.home() / ".cache/afterglow-savers/pack-guest")
	parser.add_argument("--audio", type=Path)
	parser.add_argument("--only", action="append", default=[])
	parser.add_argument("--boot", type=int, default=100, help="seconds from power-on until Display Properties is up")
	args = parser.parse_args()

	if shutil.disk_usage(Path.home()).free < 12 * 1024**3:
		sys.exit("check: under 12 GB free on /home")
	args.work.mkdir(parents=True, exist_ok=True)
	args.out.mkdir(parents=True, exist_ok=True)
	disk = args.work / "disk.img"
	disk.unlink(missing_ok=True)
	subprocess.run(["cp", "--reflink=auto", "--sparse=always", str(args.base), str(disk)], check=True)
	fat = fat_image(disk)
	for name in SCR_FILES:
		fat.copy_in(args.scr / name, f"ReactOS/system32/{name}")
	fat.copy_in(args.scr / "fpscheck.exe", "FPSCHECK.EXE")
	script = args.work / "pack.cmd"
	script.write_bytes(STARTUP_SCRIPT.encode("ascii"))
	fat.copy_in(script, STARTUP)

	vm = Vm(args.work, disk, audio=args.audio)
	events = []
	begin = time.monotonic()

	def note(what: str, **extra) -> None:
		events.append({"at": round(time.monotonic() - begin, 1), "what": what, **extra})
		print(f"check: {events[-1]}", flush=True)

	def shot(name: str) -> Path:
		path = args.out / f"{name}.png"
		vm.shot(path)
		return path

	results = {}
	vm.start()
	try:
		note("power on", pid=vm.process.pid)
		time.sleep(args.boot)
		desktop = shot("guest-desktop-and-display")
		ox, oy = find_dialog(desktop)
		note("Display Properties found", dx=ox, dy=oy)
		combo, settings_button, preview_button, ok_button = ((x + ox, y + oy) for x, y in (COMBO, SETTINGS, PREVIEW, OK_BUTTON))
		monitor = f"{MONITOR[2]}x{MONITOR[3]}+{MONITOR[0] + ox}+{MONITOR[1] + oy}"
		vm.click(*combo)
		time.sleep(1)
		shot("guest-list")
		# Choosing "(None)" closes the list; Escape would close Display Properties as well.
		vm.click(110 + ox, 253 + oy)
		time.sleep(1)
		wanted = [s for s in SAVERS if not args.only or s[0] in args.only]
		for key, title, scr in wanted:
			result = results[key] = {"title": title, "file": scr}
			note(f"{key}: select")
			vm.click(*combo)
			time.sleep(0.8)
			vm.click(110 + ox, ROW[key] + oy)
			time.sleep(4)
			first = shot(f"work-{key}-p1")
			time.sleep(1.5)
			second = shot(f"guest-preview-{key}")
			result["preview_changes"] = round(rmse(first, second, monitor), 4)
			result["preview_nonblank"] = round(rmse(second, args.out / "guest-desktop-and-display.png", monitor), 4)
			first.unlink()
			# The settings dialog: opens, draws its banner, then Cancel brings back the preview.
			note(f"{key}: settings")
			vm.click(*settings_button)
			time.sleep(7)
			shot(f"guest-dialog-{key}")
			time.sleep(2)
			shot_two = shot(f"work-{key}-d2")
			result["dialog_animates"] = round(rmse(args.out / f"guest-dialog-{key}.png", shot_two), 4)
			shot_two.unlink()
			vm.key("esc")
			time.sleep(4)
			after = shot(f"work-{key}-after-cancel")
			result["tabs_after_cancel"] = round(match_tabs(after)[0], 4)
			after.unlink()
			# Preview full screen; a key ends it.
			for how in ("key", "mouse"):
				note(f"{key}: preview, end with {how}")
				vm.click(*preview_button)
				time.sleep(5)
				full_a = shot(f"guest-fullscreen-{key}" if how == "key" else f"work-{key}-m1")
				time.sleep(1.5)
				full_b = shot(f"work-{key}-fb-{how}")
				result[f"tabs_in_fullscreen_{how}"] = round(match_tabs(full_a)[0], 4)
				result[f"fullscreen_changes_{how}"] = round(rmse(full_a, full_b), 4)
				if how == "mouse":
					full_a.unlink()
				full_b.unlink()
				if how == "key":
					vm.key("spc")
				else:
					vm.move(400, 300)
					time.sleep(0.5)
					vm.move(700, 500)
				time.sleep(3.5)
				ended = shot(f"guest-after-{how}-{key}")
				result[f"tabs_after_{how}"] = round(match_tabs(ended)[0], 4)
		# The Cracktro's words (WP-SCROLL): tick the box in the guest, type words, OK, and run it; then put the box back.
		if not args.only or "cracktro" in args.only:
			note("cracktro: scrolling words on")
			vm.click(*combo)
			time.sleep(0.8)
			vm.click(110 + ox, ROW["cracktro"] + oy)
			time.sleep(3)
			vm.click(*settings_button)
			time.sleep(7)
			box, words, dialog_ok = (497 + ox, 483 + oy), (612 + ox, 532 + oy), (631 + ox, 643 + oy)
			vm.click(*box)
			time.sleep(1.5)
			shot("guest-dialog-cracktro-words-on")
			vm.click(*words)
			vm.type("Hello from ReactOS")
			time.sleep(2)
			shot("guest-dialog-cracktro-own-words")
			vm.click(*dialog_ok)
			time.sleep(4)
			vm.click(*preview_button)
			time.sleep(7)
			shot("guest-fullscreen-cracktro-words")
			time.sleep(10)
			shot("guest-fullscreen-cracktro-words-later")
			vm.key("spc")
			time.sleep(3)
			vm.click(*settings_button)
			time.sleep(7)
			vm.click(*box)
			time.sleep(1.5)
			shot("guest-dialog-cracktro-words-off-again")
			vm.click(*dialog_ok)
			time.sleep(4)
		# The Collection starts by itself: select it, one minute of idle time, OK, and wait.
		if not args.only or "collection" in args.only:
			note("collection: idle timeout")
			vm.click(*combo)
			time.sleep(0.8)
			vm.click(110 + ox, ROW["collection"] + oy)
			time.sleep(2)
			vm.click(*ok_button)
			time.sleep(3)
			time.sleep(75)
			shot("guest-idle-collection")
			time.sleep(3)
			later = shot("work-idle-b")
			results["idle"] = {"changes": round(rmse(args.out / "guest-idle-collection.png", later), 4)}
			later.unlink()
			vm.key("spc")
			time.sleep(3)
			shot("guest-idle-ended")
		note("power down")
		vm.command("system_powerdown")
		for _ in range(60):
			if vm.process.poll() is not None:
				break
			time.sleep(1)
		note("guest ended" if vm.process.poll() is not None else "guest still up")
	finally:
		vm.stop()

	# The log and the frame rates, read back from the guest's disk.
	raw = args.work / "after.img"
	subprocess.run(["qemu-img", "convert", "-q", "-f", "qcow2", "-O", "raw", str(vm.overlay), str(raw)], check=True)
	after = fat_image(raw)
	for name, target in (("PACK.LOG", "pack.log"), ("FPS.TXT", "fps.txt")):
		try:
			after.copy_out(name, args.out / target)
		except Exception as error:
			print(f"check: no {name} ({error})")
	raw.unlink()
	# Memory: the largest working set a task list showed for each file; frame rates: what fpscheck wrote.
	memory = {}
	log = args.out / "pack.log"
	if log.exists():
		for line in log.read_text(errors="replace").splitlines():
			match = re.match(r"(\S+\.SCR)\s+\d+\s+\d+\s+([\d,]+) K", line.strip(), re.I)
			if match:
				memory[match.group(1).upper()] = max(memory.get(match.group(1).upper(), 0), int(match.group(2).replace(",", "")))
	results["memory_peak_kb"] = memory
	rates = {}
	if (args.out / "fps.txt").exists():
		for line in (args.out / "fps.txt").read_text().splitlines():
			parts = line.split()
			if len(parts) >= 7:
				rates[parts[0]] = {"start_ms": int(parts[2]), "frames_per_second": int(parts[6]) / 10}
	results["frame_rate_flat_out"] = rates
	(args.out / "results.json").write_text(json.dumps({"results": results, "events": events}, indent="\t"))
	vm.overlay.unlink(missing_ok=True)
	disk.unlink(missing_ok=True)
	return 0


if __name__ == "__main__":
	sys.exit(main())
