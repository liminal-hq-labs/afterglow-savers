#!/usr/bin/env python3
# Draws the six screensaver icons as SVG masters: the 48 px drawings and the hand-hinted 16 px frames.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The language is Afterglow's icon set E (afterglow-themes, desktop/icons/gen_e.py): a softly raised dusk plate,
# faces that are a flat two-tone base with a gentle ramp, a lit band on the top and left and a thin inner edge in
# a darker tone of the same hue, a soft contact shadow, and no ink outline. The plate carries the set's keyline:
# an opaque edge in its own hue at a luminance of 0.137, which reaches 3:1 against the classic cream window and
# the Dusk desktop. Deterministic: no randomness, no clock, no environment input.
#
#   python3 art/gen-icons.py            rewrites art/masters/<name>.svg and art/masters/<name>-16.svg
import colorsys
import math
import os
import sys

sys.dont_write_bytecode = True
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "masters")

HEADER = (
	"<!--\n\t%s\n\n\t(c) Copyright 2026 Liminal HQ, Scott Morris\n\tSPDX-License-Identifier: Apache-2.0 OR MIT\n-->\n"
)

# ---- colour --------------------------------------------------------------------------------------------------------


def to_rgb(colour):
	return tuple(int(colour[index:index + 2], 16) for index in (1, 3, 5))


def to_hex(rgb):
	return "#%02x%02x%02x" % tuple(max(0, min(255, round(part))) for part in rgb)


def mix(first, second, amount):
	a, b = to_rgb(first), to_rgb(second)
	return to_hex(tuple(a[i] + (b[i] - a[i]) * amount for i in range(3)))


def luminance(colour):
	def channel(value):
		value /= 255
		return value / 12.92 if value <= 0.03928 else ((value + 0.055) / 1.055) ** 2.4
	red, green, blue = (channel(part) for part in to_rgb(colour))
	return 0.2126 * red + 0.7152 * green + 0.0722 * blue


def contrast(first, second):
	high, low = sorted((luminance(first), luminance(second)), reverse=True)
	return (high + 0.05) / (low + 0.05)


def retune(colour, target=0.137):
	"""The same hue and saturation at the lightness whose luminance is the target."""
	hue, lightness, saturation = colorsys.rgb_to_hls(*(part / 255 for part in to_rgb(colour)))
	low, high = 0.0, 1.0
	for _ in range(40):
		middle = (low + high) / 2
		candidate = to_hex(tuple(part * 255 for part in colorsys.hls_to_rgb(hue, middle, saturation)))
		if luminance(candidate) < target:
			low = middle
		else:
			high = middle
	return to_hex(tuple(part * 255 for part in colorsys.hls_to_rgb(hue, high, saturation)))


WHITE, VOID = "#ffffff", "#050507"


def ramp(anchor):
	return {100: mix(anchor, WHITE, 0.45), 200: anchor, 300: mix(anchor, VOID, 0.18), 400: mix(anchor, VOID, 0.36),
		500: mix(anchor, VOID, 0.52), 600: mix(anchor, VOID, 0.67)}


ORANGE, RED, PURPLE, BLUE = ramp("#ffaa40"), ramp("#f43f5e"), ramp("#a78bfa"), ramp("#60a5fa")
INDIGO = {100: "#f1eee6", 200: "#e4e2ec", 300: "#b8b6c8", 400: "#8f8da6", 500: "#5f5d78", 600: "#3a384f", 700: "#2a2840", 800: "#1a1828"}
SKY1, SKY7, HORIZON, TILE = "#1f2554", "#7a4562", "#4a4580", "#0a1230"
BRAND_GOLD, PALE, SUN = "#ffd08e", "#ffe7c4", "#fbbf24"
NAVY, NAVY_LIT = "#16204a", "#2a3a78"
CREAM, DUSK_DESKTOP = "#d4d0c8", "#090e2a"

KEYLINE = retune(HORIZON)
assert contrast(KEYLINE, CREAM) >= 3 and contrast(KEYLINE, DUSK_DESKTOP) >= 3, "keyline below 3:1"


def number(value):
	text = ("%.2f" % value).rstrip("0").rstrip(".")
	return "0" if text in ("-0", "") else text


class Mat:
	"""A surface: base, lit tone, next ramp step and the darker edge tone of the same hue."""

	def __init__(self, base, lit, below, edge):
		self.base, self.lit, self.edge = base, lit, edge
		self.top, self.bot = mix(base, lit, 0.3), mix(base, below, 0.18)


GOLD = Mat(ORANGE[200], ORANGE[100], ORANGE[300], ORANGE[500])
GOLD_LIGHT = Mat(BRAND_GOLD, PALE, ORANGE[200], ORANGE[500])
ROSE = Mat(RED[200], RED[100], RED[300], RED[500])
VIOLET = Mat(PURPLE[300], PURPLE[200], PURPLE[400], PURPLE[600])
VIOLET_DEEP = Mat(PURPLE[500], PURPLE[400], PURPLE[600], mix(PURPLE[600], VOID, 0.4))
VIOLET_DARK = Mat(PURPLE[600], PURPLE[500], mix(PURPLE[600], VOID, 0.4), mix(PURPLE[600], VOID, 0.6))
FLOOR = Mat(INDIGO[500], INDIGO[400], INDIGO[600], INDIGO[700])
PAGE = Mat(NAVY, NAVY_LIT, mix(NAVY, VOID, 0.4), mix(NAVY, VOID, 0.6))
MOON = Mat(PALE, WHITE, BRAND_GOLD, ORANGE[400])

# ---- shapes --------------------------------------------------------------------------------------------------------


def rounded(x, y, w, h, r):
	return "M%s %s H%s a%s %s 0 0 1 %s %s V%s a%s %s 0 0 1 %s %s H%s a%s %s 0 0 1 %s %s V%s a%s %s 0 0 1 %s %s Z" % (
		number(x + r), number(y), number(x + w - r), number(r), number(r), number(r), number(r), number(y + h - r),
		number(r), number(r), number(-r), number(r), number(x + r), number(r), number(r), number(-r), number(-r),
		number(y + r), number(r), number(r), number(r), number(-r))


def circle(cx, cy, radius):
	return "M%s %s a%s %s 0 1 0 %s 0 a%s %s 0 1 0 %s 0 Z" % (
		number(cx - radius), number(cy), number(radius), number(radius), number(2 * radius), number(radius), number(radius), number(-2 * radius))


def polygon(points):
	return "M" + " L".join("%s %s" % (number(x), number(y)) for x, y in points) + " Z"


def rotate(points, degrees, cx=24, cy=24):
	cosine, sine = math.cos(math.radians(degrees)), math.sin(math.radians(degrees))
	return [(cx + (x - cx) * cosine - (y - cy) * sine, cy + (x - cx) * sine + (y - cy) * cosine) for x, y in points]


def spiral(cx, cy, turns, start_degrees, start_radius, end_radius, steps=40):
	points = []
	for step in range(steps + 1):
		fraction = step / steps
		angle = math.radians(start_degrees + 360 * turns * fraction)
		radius = start_radius + (end_radius - start_radius) * fraction
		points.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
	return "M" + " L".join("%s %s" % (number(x), number(y)) for x, y in points)


PLATE = rounded(5, 5, 38, 38, 8)

# ---- the pen: collects defs and paints faces at the 48 px profile ----------------------------------------------------


class Pen:
	def __init__(self, prefix):
		self.prefix, self.counter = prefix, 0
		self.defs, self.clips, self.out = {}, {}, []

	def clip(self, path):
		if path not in self.clips:
			self.counter += 1
			clip_id = "%s-c%d" % (self.prefix, self.counter)
			self.clips[path] = clip_id
			self.defs[clip_id] = '<clipPath id="%s"><path d="%s" clip-rule="evenodd"/></clipPath>' % (clip_id, path)
		return self.clips[path]

	def gradient(self, top, bottom):
		gradient_id = "%s-g%s%s" % (self.prefix, top[1:], bottom[1:])
		self.defs[gradient_id] = (
			'<linearGradient id="%s" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="%s"/><stop offset="1" stop-color="%s"/></linearGradient>'
			% (gradient_id, top, bottom))
		return "url(#%s)" % gradient_id

	def blur(self):
		self.defs["%s-blur" % self.prefix] = (
			'<filter id="%s-blur" x="-50%%" y="-50%%" width="200%%" height="200%%"><feGaussianBlur stdDeviation="1"/></filter>' % self.prefix)
		return "url(#%s-blur)" % self.prefix

	def shadow(self, x0, x1, y, opacity=0.3):
		self.out.append('<ellipse cx="%s" cy="%s" rx="%s" ry="1.5" fill="%s" opacity="%s" filter="%s"/>' % (
			number((x0 + x1) / 2 + 1), number(y + 0.4), number((x1 - x0) * 0.43), TILE, opacity, self.blur()))

	def face(self, path, mat, band=True, edge=True):
		"""Ramp, a lit band on the top and left, and an inner edge clipped to the shape so the silhouette never grows."""
		clip_id = self.clip(path)
		paint = self.gradient(mat.top, mat.bot)
		if band:
			self.out.append('<path d="%s" fill="%s" fill-rule="evenodd"/><g clip-path="url(#%s)"><path d="%s" fill="%s" fill-rule="evenodd" transform="translate(1 1)"/></g>' % (
				path, mat.lit, clip_id, path, paint))
		else:
			self.out.append('<path d="%s" fill="%s" fill-rule="evenodd"/>' % (path, paint))
		if edge:
			self.out.append('<g clip-path="url(#%s)"><path d="%s" fill="none" stroke="%s" stroke-width="2" stroke-opacity="0.3"/></g>' % (clip_id, path, mat.edge))

	def fill(self, path, colour):
		self.out.append('<path d="%s" fill="%s" fill-rule="evenodd"/>' % (path, colour))

	def line(self, path, colour, width, mat=None):
		if mat is not None:
			self.out.append('<path d="%s" fill="none" stroke="%s" stroke-opacity="0.45" stroke-width="%s" stroke-linecap="round" stroke-linejoin="round"/>' % (
				path, mat.edge, number(width + 2)))
		self.out.append('<path d="%s" fill="none" stroke="%s" stroke-width="%s" stroke-linecap="round" stroke-linejoin="round"/>' % (path, colour, number(width)))

	def raw(self, text):
		self.out.append(text)

	def plate(self):
		"""The dusk plate: sky ramp with the horizon's lit band, a contact shadow and the keyline edge."""
		self.out.append('<ellipse cx="25" cy="43.4" rx="16" ry="1.5" fill="%s" opacity="0.3" filter="%s"/>' % (TILE, self.blur()))
		clip_id = self.clip(PLATE)
		self.out.append('<path d="%s" fill="%s"/><g clip-path="url(#%s)"><path d="%s" fill="%s" transform="translate(1 1)"/></g>' % (
			PLATE, HORIZON, clip_id, PLATE, self.gradient(SKY1, SKY7)))
		self.out.append('<g clip-path="url(#%s)"><path d="%s" fill="none" stroke="%s" stroke-width="1.5"/></g>' % (clip_id, PLATE, KEYLINE))

	def clipped(self, body):
		"""Content that stays inside the plate."""
		self.out.append('<g clip-path="url(#%s)">' % self.clip(rounded(5.75, 5.75, 36.5, 36.5, 7.25)))
		body()
		self.out.append("</g>")

	def document(self, title):
		defs = "".join("\n\t\t" + text for text in self.defs.values())
		body = "".join("\n\t" + text for text in self.out)
		return '<svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 48 48">\n\t<defs>%s\n\t</defs>%s\n</svg>\n' % (defs, body), title


# ---- the six drawings (48 px) ----------------------------------------------------------------------------------------


def maze(p):
	left = polygon([(5, 5), (18.5, 18.5), (18.5, 29.5), (5, 43)])
	right = polygon([(43, 5), (29.5, 18.5), (29.5, 29.5), (43, 43)])
	ceiling = polygon([(5, 5), (43, 5), (29.5, 18.5), (18.5, 18.5)])
	floor = polygon([(5, 43), (18.5, 29.5), (29.5, 29.5), (43, 43)])
	p.face(ceiling, VIOLET_DARK, band=False, edge=False)
	p.face(left, VIOLET_DEEP, edge=False)
	p.face(right, VIOLET, edge=False)
	p.face(floor, FLOOR, edge=False)
	# floor and wall joints, and the stepped crenellations on the left wall
	p.line("M5 43 L18.5 29.5 M43 43 L29.5 29.5 M5 5 L18.5 18.5 M43 5 L29.5 18.5", PURPLE[600], 1.2)
	p.fill(polygon([(9, 20), (13, 24), (13, 28), (9, 25)]), PURPLE[300])
	# the stepped sun window at the end of the corridor
	p.fill("M18.5 29.5 V18.5 H29.5 V29.5 Z", mix(ORANGE[500], SKY1, 0.4))
	p.fill("M19.5 29.5 V23 H22 V20.5 H26 V23 H28.5 V29.5 Z", ORANGE[200])
	p.fill("M21 29.5 V25.5 H23 V22.5 H25 V25.5 H27 V29.5 Z", BRAND_GOLD)
	p.fill("M22.8 29.5 V26.5 H25.2 V29.5 Z", PALE)


def cracktro(p):
	bars = ((13.5, ROSE), (20.5, GOLD), (27.5, GOLD_LIGHT))
	for y, mat in bars:
		p.face(rounded(5, y, 38, 6, 0.01), mat, edge=False)
		p.fill("M5 %s H43 V%s H5 Z" % (number(y + 2.2), number(y + 3.8)), mat.lit)
	p.line("M5 33 C10 22 14 22 19 33 S29 44 34 33 S42 22 46 30", PALE, 2.4, VIOLET)


def twilight(p):
	cx, cy, radius = 22.5, 26, 12
	p.fill(circle(35.5, 11.5, 0.8) + " " + circle(11, 12.5, 0.7) + " " + circle(14.5, 19, 0.6), INDIGO[200])
	p.shadow(10, 36, 40)
	p.face(circle(cx, cy, radius), VIOLET_DARK, band=False)
	clip_id = p.clip(circle(cx, cy, radius))
	p.raw('<g clip-path="url(#%s)"><path d="%s" fill="%s"/><path d="%s" fill="%s"/></g>' % (
		clip_id, circle(cx, cy, radius), ORANGE[200], circle(cx - 4.5, cy + 3.5, radius), PURPLE[600]))
	p.raw('<g clip-path="url(#%s)"><path d="%s" fill="none" stroke="%s" stroke-width="1.6"/></g>' % (clip_id, circle(cx, cy, radius), ORANGE[100]))
	p.face(circle(36, 14.5, 3.8), MOON)


def ink(p):
	p.line(spiral(31, 29, 1.55, 200, 2.5, 11), RED[200], 3.6, ROSE)
	p.line(spiral(19, 20, 1.55, 20, 2.5, 11), ORANGE[200], 3.6, GOLD)
	p.line(spiral(19, 20, 1.55, 20, 2.5, 11), BRAND_GOLD, 1.2)
	p.line(spiral(31, 29, 1.55, 200, 2.5, 11), RED[100], 1.2)


def collection_tiles(p):
	"""Four quarter tiles, each one of the other motifs at tile scale in its own small plate."""
	tiles = ((7.5, 7.5, maze), (25.5, 7.5, cracktro), (7.5, 25.5, twilight), (25.5, 25.5, ink))
	scale = 15 / 38
	for index, (x, y, motif) in enumerate(tiles):
		inner = Pen("t%d" % index)
		motif(inner)
		tile_clip = "%s-t%d" % (p.prefix, index)
		shape = rounded(x, y, 15, 15, 3.2)
		p.defs[tile_clip] = '<clipPath id="%s"><path d="%s"/></clipPath>' % (tile_clip, shape)
		for key, text in inner.defs.items():
			p.defs["t%d-%s" % (index, key)] = text
		p.raw('<path d="%s" fill="%s"/>' % (shape, SKY1))
		# the motif's own shadows would fall outside a tile, so they are left out
		body = "".join(piece for piece in inner.out if "<ellipse" not in piece)
		for key in inner.defs:
			body = body.replace("#" + key + ")", "#t%d-%s)" % (index, key))
		p.raw('<g clip-path="url(#%s)"><g transform="translate(%s %s) scale(%s) translate(-5 -5)">%s</g></g>' % (
			tile_clip, number(x), number(y), number(scale), body))
		p.raw('<path d="%s" fill="none" stroke="%s" stroke-width="1" stroke-opacity="0.9"/>' % (rounded(x + 0.5, y + 0.5, 14, 14, 2.7), mix(HORIZON, PALE, 0.15)))


def quill(p):
	page = rounded(9, 5, 30, 38, 3)
	p.shadow(9, 39, 43)
	p.face(page, PAGE)
	p.raw('<g clip-path="url(#%s)"><path d="%s" fill="none" stroke="%s" stroke-width="1.5"/></g>' % (p.clip(page), page, retune(NAVY_LIT)))
	for index, width in enumerate((14, 18, 10)):
		p.fill(rounded(13, 12 + index * 5, width, 2, 1), INDIGO[500])
	p.fill(rounded(13, 33, 8, 2, 1), INDIGO[500])
	nib = rotate([(24, 42), (19, 33), (17, 24), (20, 18), (28, 18), (31, 24), (29, 33)], 40, 24, 26)
	body = "M%s %s" % (number(nib[0][0]), number(nib[0][1]))
	for index in range(1, len(nib)):
		body += " L%s %s" % (number(nib[index][0]), number(nib[index][1]))
	p.face(body + " Z", GOLD_LIGHT)
	slit = rotate([(24, 42), (24, 26)], 40, 24, 26)
	p.line("M%s %s L%s %s" % tuple(number(v) for pair in slit for v in pair), NAVY, 1.2)
	hole = rotate([(24, 25)], 40, 24, 26)[0]
	p.fill(circle(hole[0], hole[1], 1.9), NAVY)
	shoulder = rotate([(24, 16)], 40, 24, 26)[0]
	p.line("M%s %s L%s %s" % tuple(number(v) for v in (*shoulder, *rotate([(24, 12)], 40, 24, 26)[0])), ORANGE[300], 3.2, GOLD)


ICONS = {"maze": (maze, True), "cracktro": (cracktro, True), "twilight": (twilight, True), "ink": (ink, True),
	"collection": (collection_tiles, True), "quill": (quill, False)}


def master(name):
	draw, on_plate = ICONS[name]
	pen = Pen(name)
	if on_plate:
		pen.plate()
		if name == "collection":
			draw(pen)
		else:
			pen.clipped(lambda: draw(pen))
	else:
		draw(pen)
	return pen.document(name)


# ---- the hinted 16 px frames: pixel-aligned, drawn on a 14 x 14 interior inside the plate's edge ------------------------


class Frame:
	def __init__(self):
		self.pixels = {}

	def rect(self, x, y, w, h, colour):
		for row in range(y, y + h):
			for column in range(x, x + w):
				if 0 <= row < 14 and 0 <= column < 14:
					self.pixels[(column, row)] = colour

	def polygon(self, points, colour):
		"""Pixels whose centres fall inside the polygon."""
		for row in range(14):
			for column in range(14):
				x, y, inside, count = column + 0.5, row + 0.5, False, len(points)
				for index in range(count):
					x1, y1 = points[index]
					x2, y2 = points[(index + 1) % count]
					if (y1 > y) != (y2 > y) and x < (x2 - x1) * (y - y1) / (y2 - y1) + x1:
						inside = not inside
				if inside:
					self.pixels[(column, row)] = colour

	def disc(self, cx, cy, radius, colour):
		for row in range(14):
			for column in range(14):
				if (column + 0.5 - cx) ** 2 + (row + 0.5 - cy) ** 2 <= radius ** 2:
					self.pixels[(column, row)] = colour

	def svg(self, with_plate=True, interior=None):
		cells = []
		edge = KEYLINE
		if with_plate:
			for row in range(16):
				for column in range(16):
					corner = (column in (0, 15)) and (row in (0, 15))
					if corner:
						continue
					border = column in (0, 15) or row in (0, 15)
					near_corner = (column in (1, 14)) and (row in (1, 14))
					if border:
						colour = edge
					elif near_corner:
						colour = edge
					else:
						colour = mix(SKY1, SKY7, (row - 1) / 13)
					cells.append((column, row, colour))
		lookup = {(x, y): c for x, y, c in cells}
		for (column, row), colour in self.pixels.items():
			lookup[(column + 1, row + 1)] = colour
		return lookup


def frame_maze():
	f = Frame()
	f.polygon([(0, 0), (14, 0), (9, 4), (5, 4)], PURPLE[600])
	f.polygon([(0, 0), (5, 4), (5, 9), (0, 14)], PURPLE[500])
	f.polygon([(14, 0), (9, 4), (9, 9), (14, 14)], PURPLE[300])
	f.polygon([(0, 14), (5, 9), (9, 9), (14, 14)], INDIGO[500])
	f.rect(5, 4, 4, 5, mix(ORANGE[500], SKY1, 0.4))
	f.rect(5, 6, 4, 3, ORANGE[200])
	f.rect(6, 5, 2, 1, ORANGE[200])
	f.rect(6, 7, 2, 2, BRAND_GOLD)
	f.rect(6, 8, 2, 1, PALE)
	return f


def frame_cracktro():
	f = Frame()
	for y, mat in ((2, ROSE), (5, GOLD), (8, GOLD_LIGHT)):
		f.rect(0, y, 14, 2, mat.base)
		f.rect(0, y, 14, 1, mat.lit)
	for column, row in ((0, 11), (1, 10), (2, 9), (3, 10), (4, 11), (5, 12), (6, 12), (7, 11), (8, 10), (9, 10), (10, 11), (11, 12), (12, 12), (13, 11)):
		f.rect(column, row, 1, 1, PALE)
	return f


def frame_twilight():
	f = Frame()
	for column, row in ((11, 6), (2, 2), (3, 5)):
		f.rect(column, row, 1, 1, INDIGO[200])
	cx, cy, radius = 5.6, 8.6, 4.6
	f.disc(cx, cy, radius, ORANGE[200])
	for row in range(14):
		for column in range(14):
			inside = (column + 0.5 - cx) ** 2 + (row + 0.5 - cy) ** 2 <= radius ** 2
			shadowed = (column + 0.5 - cx + 2.2) ** 2 + (row + 0.5 - cy - 1.8) ** 2 <= (radius + 0.2) ** 2
			if inside and shadowed:
				f.rect(column, row, 1, 1, PURPLE[600])
	f.disc(11, 2.8, 1.6, PALE)
	return f


def frame_ink():
	f = Frame()
	rose = [(8, 6), (9, 6), (10, 7), (10, 8), (10, 9), (9, 10), (8, 10), (7, 10), (6, 9), (6, 8), (7, 8)]
	gold = [(7, 5), (6, 4), (5, 4), (4, 5), (3, 6), (3, 7), (4, 8), (5, 8), (6, 7), (5, 6), (6, 6)]
	for column, row in rose:
		f.rect(column, row, 1, 1, RED[200])
	for column, row in gold:
		f.rect(column, row, 1, 1, ORANGE[200])
	f.rect(8, 6, 1, 1, RED[100])
	f.rect(7, 5, 1, 1, BRAND_GOLD)
	return f


def frame_collection():
	f = Frame()
	for x, y in ((1, 1), (8, 1), (1, 8), (8, 8)):
		f.rect(x, y, 5, 5, SKY1)
	f.rect(1, 1, 5, 5, PURPLE[500])
	f.rect(2, 2, 3, 3, PURPLE[300])
	f.rect(3, 3, 1, 1, ORANGE[200])
	f.rect(8, 2, 5, 1, RED[200])
	f.rect(8, 3, 5, 1, ORANGE[200])
	f.rect(8, 4, 5, 1, BRAND_GOLD)
	f.rect(2, 9, 3, 3, PURPLE[600])
	f.rect(3, 9, 2, 1, ORANGE[200])
	f.rect(4, 10, 1, 1, ORANGE[200])
	f.rect(10, 9, 1, 3, RED[200])
	f.rect(9, 10, 3, 1, ORANGE[200])
	return f


def frame_quill():
	f = Frame()
	f.rect(2, 1, 10, 12, retune(NAVY_LIT))
	f.rect(3, 2, 8, 10, NAVY)
	f.rect(3, 2, 8, 1, NAVY_LIT)
	f.rect(3, 2, 1, 10, NAVY_LIT)
	f.rect(4, 4, 4, 1, INDIGO[500])
	f.rect(4, 6, 3, 1, INDIGO[500])
	nib = [(9, 3), (11, 5), (8, 9), (4, 12), (5, 9)]
	f.polygon(nib, BRAND_GOLD)
	f.rect(9, 3, 2, 1, PALE)
	f.rect(8, 5, 1, 1, PALE)
	f.rect(5, 11, 1, 1, ORANGE[400])
	f.rect(7, 7, 1, 1, NAVY)
	return f


FRAMES = {"maze": frame_maze, "cracktro": frame_cracktro, "twilight": frame_twilight, "ink": frame_ink,
	"collection": frame_collection, "quill": frame_quill}


def frame_document(name):
	cells = FRAMES[name]().svg(with_plate=name != "quill")
	rects = []
	for (column, row), colour in sorted(cells.items(), key=lambda item: (item[0][1], item[0][0])):
		rects.append('<rect x="%d" y="%d" width="1" height="1" fill="%s"/>' % (column, row, colour))
	return ('<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16" shape-rendering="crispEdges">\n\t'
		+ "\n\t".join(rects) + "\n</svg>\n")


def main():
	os.makedirs(OUT, exist_ok=True)
	for name in ICONS:
		document, _ = master(name)
		with open(os.path.join(OUT, name + ".svg"), "w") as handle:
			handle.write('<?xml version="1.0" encoding="UTF-8"?>\n' + HEADER % ("The %s icon, 48 px master." % name) + document)
		with open(os.path.join(OUT, name + "-16.svg"), "w") as handle:
			handle.write('<?xml version="1.0" encoding="UTF-8"?>\n' + HEADER % ("The %s icon, hinted 16 px frame." % name) + frame_document(name))


if __name__ == "__main__":
	main()
