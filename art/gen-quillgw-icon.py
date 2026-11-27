#!/usr/bin/env python3
# Draws QUILLGW.SCR's icon masters (a gold nib writing on a navy page, in the sixteen standard Windows colours) and builds the .ico.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# usage: python3 -I art/gen-quillgw-icon.py [--check]
#
# Writes art/masters/quillgw.svg (32 px) and art/masters/quillgw-16.svg (16 px), both drawn as pixels in the sixteen standard Windows
# colours in the pixel style of Quill's own Windows 3.x icon (QUILL31.ICO), then renders them with rsvg-convert and packs them into
# platforms/win16/res/quillgw.ico: a 32 × 32 and a 16 × 16 image at 4 bits, the form Windows 3.x reads, with a 1-bit mask. Every
# rendered pixel takes the nearest of the sixteen colours, so a stray blend can never reach the icon. Needs rsvg-convert and
# ImageMagick (to read a render back as raw RGBA); no Python packages. With --check nothing is written: the masters and the icon are
# made in memory and compared with the committed files. Deterministic.
import os
import struct
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
MASTER = os.path.join(HERE, "masters", "quillgw.svg")
MASTER_16 = os.path.join(HERE, "masters", "quillgw-16.svg")
ICO = os.path.join(ROOT, "platforms", "win16", "res", "quillgw.ico")

# The sixteen standard Windows colours, in palette order.
WIN16 = [(0, 0, 0), (128, 0, 0), (0, 128, 0), (128, 128, 0), (0, 0, 128), (128, 0, 128), (0, 128, 128), (192, 192, 192),
	(128, 128, 128), (255, 0, 0), (0, 255, 0), (255, 255, 0), (0, 0, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255)]
NAVY, OLIVE, SILVER, GREY, YELLOW, WHITE = "#000080", "#808000", "#c0c0c0", "#808080", "#ffff00", "#ffffff"

HEADER = "<!--\n\t%s\n\n\t(c) Copyright 2026 Liminal HQ, Scott Morris\n\tSPDX-License-Identifier: Apache-2.0 OR MIT\n-->\n"

# The 32 px drawing, as cells: the navy page with a folded corner and three written lines, and the gold nib along the diagonal, its
# tip on the line being written. The nib is described in its own axes (a along the nib from the tip, b across it, both in units of
# a diagonal step), so its edges are clean 45-degree staircases.
TIP = (9, 24)


def nib_cell(x, y):
	"""The nib's code at a cell, or None outside it."""
	px, py = x + 0.5 - TIP[0], y + 0.5 - TIP[1]
	along, across = (px - py) / 2, (px + py) / 2
	if not 0 <= along <= 12:
		return None
	half = min(0.4 + along * 0.42, 2.6)
	if abs(across) > half:
		return None
	if abs(across) < 0.3 and 0.8 <= along <= 7.0:
		return "n"  # the slit
	if (along - 7.8) ** 2 + across * across < 0.8:
		return "n"  # the breather hole
	if 9.5 <= along <= 11.5 and across < -half + 0.9:
		return "w"  # the glint on the lit edge
	return "o" if across > half - 0.9 else "y"


def grid_32():
	cells = [["t"] * 32 for _ in range(32)]
	for y in range(2, 30):
		for x in range(4, 28):
			cells[y][x] = "n"
	for x, y in ((4, 2), (4, 29), (27, 29)):
		cells[y][x] = "t"
	# The folded corner: the flap's underside below the fold line, nothing above it.
	for y in range(2, 8):
		for x in range(22, 28):
			fold = (x - 22) - (y - 2)
			cells[y][x] = "t" if fold > 0 else ("g" if fold == 0 else "s")
	for y, start, end in ((7, 7, 19), (12, 7, 22), (17, 7, 15), (22, 6, 9)):
		for row in (y, y + 1):
			for x in range(start, end):
				cells[row][x] = "s"
	for y in range(32):
		for x in range(32):
			code = nib_cell(x, y)
			if code:
				cells[y][x] = code
	return ["".join(row) for row in cells]


# The 16 px frame, by hand: t transparent, n navy, s silver, g grey, y yellow, o olive, w white (the 32 px drawing uses the same codes).
HAND_16 = [
	"tttttttttttttttt",
	"ttnnnnnnnnngtttt",
	"ttnnnnnnnnnsgttt",
	"ttnssssssnnssgtt",
	"ttnnnnnnnnnwyntt",
	"ttnnnnnnnnyyontt",
	"ttnsssssnynonntt",
	"ttnnnnnnyyonnntt",
	"ttnnnnnyyonnnntt",
	"ttnssnyonnnnnntt",
	"ttnnnyonnnnnnntt",
	"ttnsynnnnnnnnntt",
	"ttnnnnnnnnnnnntt",
	"ttnnnnnnnnnnnntt",
	"tttnnnnnnnnnnttt",
	"tttttttttttttttt",
]
CODES = {"n": NAVY, "s": SILVER, "g": GREY, "y": YELLOW, "o": OLIVE, "w": WHITE}


def pixel_master(rows, summary):
	size = len(rows)
	cells = []
	for y, row in enumerate(rows):
		for x, code in enumerate(row):
			if code != "t":
				cells.append(f'\t<rect x="{x}" y="{y}" width="1" height="1" fill="{CODES[code]}"/>')
	return (HEADER % summary) + (
		f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {size} {size}" shape-rendering="crispEdges">\n'
		+ "\n".join(cells) + "\n</svg>\n")


def render(svg_text, size):
	"""The drawing at size by size pixels as a list of rows of (r, g, b, a)."""
	with tempfile.TemporaryDirectory() as scratch:
		path = os.path.join(scratch, "in.svg")
		with open(path, "w", encoding="utf-8") as handle:
			handle.write(svg_text)
		png = subprocess.run(["rsvg-convert", "-w", str(size), "-h", str(size), path], check=True, capture_output=True).stdout
		raw = subprocess.run(["magick", "png:-", "-depth", "8", "rgba:-"], input=png, check=True, capture_output=True).stdout
	return [[tuple(raw[(y * size + x) * 4:(y * size + x) * 4 + 4]) for x in range(size)] for y in range(size)]


def nearest(rgb):
	weights = (2, 4, 3)
	return min(range(16), key=lambda index: sum(weights[c] * (rgb[c] - WIN16[index][c]) ** 2 for c in range(3)))


def indexed(svg_text, size):
	"""(indices into WIN16, opaque) for each pixel of the rendered drawing."""
	samples = render(svg_text, size)
	index_rows = [[nearest(pixel[:3]) if pixel[3] >= 128 else 0 for pixel in row] for row in samples]
	opaque_rows = [[pixel[3] >= 128 for pixel in row] for row in samples]
	return index_rows, opaque_rows


def dib(index_rows, opaque_rows):
	"""A 4-bit icon image: the header with the doubled height, the sixteen colours, the XOR bits and the AND mask, bottom row first."""
	size = len(index_rows)
	xor = bytearray()
	mask = bytearray()
	mask_stride = (size + 31) // 32 * 4
	for y in range(size - 1, -1, -1):
		row = [index_rows[y][x] if opaque_rows[y][x] else 0 for x in range(size)]
		data = bytes((row[i] << 4) | row[i + 1] for i in range(0, size, 2))
		xor += data + bytes(-len(data) % 4)
		bits = bytearray(mask_stride)
		for x in range(size):
			if not opaque_rows[y][x]:
				bits[x >> 3] |= 0x80 >> (x & 7)
		mask += bits
	head = struct.pack("<IiiHHIIiiII", 40, size, size * 2, 1, 4, 0, len(xor) + len(mask), 0, 0, 16, 0)
	palette = b"".join(bytes([b, g, r, 0]) for r, g, b in WIN16)
	return head + palette + bytes(xor) + bytes(mask)


def ico(images):
	out = struct.pack("<HHH", 0, 1, len(images))
	offset = 6 + 16 * len(images)
	body = b""
	for size, data in images:
		out += struct.pack("<BBBBHHII", size, size, 16, 0, 1, 4, len(data), offset + len(body))
		body += data
	return out + body


def main():
	check = "--check" in sys.argv[1:]
	big = pixel_master(grid_32(), "Quill Ghostwriter for Windows 3.x: a gold nib writing on a navy page, drawn as 32 px in the sixteen standard Windows colours.")
	small = pixel_master(HAND_16, "Quill Ghostwriter for Windows 3.x: the 16 px frame, drawn as pixels in the sixteen standard Windows colours.")
	icon = ico([(32, dib(*indexed(big, 32))), (16, dib(*indexed(small, 16)))])
	wanted = {MASTER: big.encode(), MASTER_16: small.encode(), ICO: icon}
	if check:
		stale = [path for path, data in wanted.items() if not os.path.exists(path) or open(path, "rb").read() != data]
		if stale:
			print("gen-quillgw-icon: out of date: " + ", ".join(os.path.relpath(path, ROOT) for path in stale), file=sys.stderr)
			return 1
		print("gen-quillgw-icon: ok: the masters and quillgw.ico are current")
		return 0
	for path, data in wanted.items():
		with open(path, "wb") as handle:
			handle.write(data)
	print(f"gen-quillgw-icon: wrote {os.path.relpath(ICO, ROOT)} ({len(icon)} bytes) and the two masters")
	return 0


if __name__ == "__main__":
	sys.exit(main())
