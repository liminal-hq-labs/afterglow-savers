# A private headless QEMU guest and the few things the savers' ReactOS check needs from it: a QMP socket of its own, screendumps, keys, the
# pointer, and a copy-on-write overlay over a read-only disk image.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""The guest has no display, no network and no host devices: QEMU runs with `-display none`, a QMP socket in the work folder, an
absolute pointer (usb-tablet) and, when asked, an AC97 card whose output goes to a WAV file. Only this process's own QEMU is ever stopped,
by the process number it started."""

import json
import socket
import subprocess
import time
from pathlib import Path


class Vm:
	def __init__(self, work: Path, base: Path, audio: Path | None = None, memory: int = 768):
		self.work = work
		self.overlay = work / "overlay.qcow2"
		self.sock_path = work / "qmp.sock"
		self.base = base
		self.audio = audio
		self.memory = memory
		self.process = None
		self.sock = None
		self.reader = None

	def start(self) -> None:
		self.work.mkdir(parents=True, exist_ok=True)
		self.overlay.unlink(missing_ok=True)
		self.sock_path.unlink(missing_ok=True)
		subprocess.run(
			["qemu-img", "create", "-q", "-f", "qcow2", "-b", str(self.base), "-F", "raw", str(self.overlay)], check=True
		)
		sound = ["-audiodev", f"wav,id=snd0,path={self.audio}", "-device", "AC97,audiodev=snd0"] if self.audio else []
		self.process = subprocess.Popen(
			[
				"qemu-system-i386", "-machine", "pc", "-cpu", "pentium3", "-m", str(self.memory), "-enable-kvm", "-vga", "std",
				"-display", "none", "-nic", "none", "-qmp", f"unix:{self.sock_path},server=on,wait=off",
				"-usb", "-device", "usb-tablet",
				"-drive", f"file={self.overlay},format=qcow2,if=ide,index=0",
				*sound,
			]
		)
		for _ in range(100):
			if self.sock_path.exists():
				break
			time.sleep(0.1)
		self.sock = socket.socket(socket.AF_UNIX)
		self.sock.settimeout(30)
		self.sock.connect(str(self.sock_path))
		self.reader = self.sock.makefile("rw")
		self.reader.readline()
		self.command("qmp_capabilities")

	def command(self, name: str, **arguments):
		self.reader.write(json.dumps({"execute": name, "arguments": arguments}) + "\n")
		self.reader.flush()
		while True:
			reply = json.loads(self.reader.readline())
			if "return" in reply:
				return reply["return"]
			if "error" in reply:
				raise RuntimeError(f"QMP {name}: {reply['error']}")

	def hmp(self, line: str) -> str:
		return self.command("human-monitor-command", **{"command-line": line})

	def shot(self, path: Path) -> None:
		self.command("screendump", filename=str(path), format="png")

	def key(self, *names: str) -> None:
		self.command("send-key", keys=[{"type": "qcode", "data": n} for n in names], **{"hold-time": 80})
		time.sleep(0.3)

	def type(self, text: str) -> None:
		special = {" ": "spc", ".": "dot", "\\": "backslash", ":": ("shift", "semicolon"), "/": "slash", ",": "comma", "-": "minus"}
		for char in text:
			if char in special:
				value = special[char]
				self.key(*(value if isinstance(value, tuple) else (value,)))
			elif char.isupper():
				self.key("shift", char.lower())
			else:
				self.key(char)

	def move(self, x: int, y: int, width: int = 1024, height: int = 768) -> None:
		events = [
			{"type": "abs", "data": {"axis": "x", "value": min(0x7FFF, x * 0x7FFF // (width - 1))}},
			{"type": "abs", "data": {"axis": "y", "value": min(0x7FFF, y * 0x7FFF // (height - 1))}},
		]
		self.command("input-send-event", events=events)
		time.sleep(0.25)

	def click(self, x: int, y: int, button: str = "left") -> None:
		self.move(x, y)
		self.command("input-send-event", events=[{"type": "btn", "data": {"down": True, "button": button}}])
		time.sleep(0.12)
		self.command("input-send-event", events=[{"type": "btn", "data": {"down": False, "button": button}}])
		time.sleep(0.4)

	def stop(self) -> None:
		if self.process is None:
			return
		if self.process.poll() is None:
			try:
				self.command("quit")
			except Exception:
				pass
		try:
			self.process.wait(timeout=20)
		except subprocess.TimeoutExpired:
			self.process.terminate()
			self.process.wait(timeout=20)
		self.process = None
		for handle in (self.reader, self.sock):
			try:
				handle.close()
			except Exception:
				pass
