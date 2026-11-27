#!/usr/bin/env python3
# Draws assets/hero.svg: a GNOME-style window holding a collage of the four savers' scenes.
#
# (c) Copyright 2026 Liminal HQ, Scott Morris
# SPDX-License-Identifier: Apache-2.0 OR MIT
#
# The banner rules are the Labs ones (scripts/check-banner.sh in liminal-installer checks them): 1200 x 320, SMIL only,
# a .motion and a .static copy of every animated part for reduced motion, data-loop on the root and animations whose
# dur divides it. The palette is this repo's own: deep plum ground, magenta, peach and aqua.
#
#   python3 art/gen-hero.py             rewrites assets/hero.svg
import math
import os
import sys

sys.dont_write_bytecode = True
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

PLUM_A, PLUM_B = "#140a24", "#2a1244"
MAGENTA, PEACH, AQUA, GOLD, PALE = "#ff4fa3", "#ffb38a", "#4fe3d4", "#ffd166", "#fff0dc"
LOOP = 8


def num(value):
	text = ("%.1f" % value).rstrip("0").rstrip(".")
	return "0" if text in ("-0", "") else text


def spiral(cx, cy, turns, start_degrees, start_radius, end_radius):
	points = []
	for step in range(41):
		fraction = step / 40
		angle = math.radians(start_degrees + 360 * turns * fraction)
		radius = start_radius + (end_radius - start_radius) * fraction
		points.append("%s %s" % (num(cx + radius * math.cos(angle)), num(cy + radius * math.sin(angle))))
	return "M" + " L".join(points)


def sine(x0, x1, mid, amplitude, period):
	"""A sine as cubic segments, one per half period."""
	path, x, sign = "M%s %s" % (num(x0), num(mid)), x0, -1
	half = period / 2
	while x < x1:
		path += " C%s %s %s %s %s %s" % (num(x + half * 0.36), num(mid + sign * amplitude * 1.33), num(x + half * 0.64), num(mid + sign * amplitude * 1.33), num(x + half), num(mid))
		x, sign = x + half, -sign
	return path


def tile(x, y, w, h, index, name, scene):
	"""One tile of the collage; scene(animated) returns its content in tile-local coordinates."""
	lines = ['<clipPath id="tile%d"><rect x="%d" y="%d" width="%d" height="%d" rx="8"/></clipPath>' % (index, x, y, w, h)]
	body = ['<g clip-path="url(#tile%d)">' % index, '<rect x="%d" y="%d" width="%d" height="%d" fill="url(#sky)"/>' % (x, y, w, h)]
	for animated in (True, False):
		body.append('<g class="%s" transform="translate(%d %d)">' % ("motion" if animated else "static", x, y))
		body.extend(scene(animated, w, h))
		body.append("</g>")
	body.append('<rect x="%d" y="%d" width="%d" height="%d" fill="url(#scrim)"/>' % (x, y + h - 26, w, 26))
	body.append("</g>")
	body.append('<rect x="%s" y="%s" width="%d" height="%d" rx="8" fill="none" stroke="#ffffff" stroke-opacity="0.14"/>' % (num(x + 0.5), num(y + 0.5), w - 1, h - 1))
	body.append('<text class="ui" x="%d" y="%d" font-size="12" font-weight="700" fill="%s">%s</text>' % (x + 12, y + h - 9, PALE, name))
	return lines, body


def maze(animated, w, h):
	cx, cy = w / 2, h / 2 - 4
	out = []
	for index, (half_w, half_h) in enumerate(((w * 0.46, h * 0.5), (w * 0.3, h * 0.34), (w * 0.17, h * 0.2), (w * 0.085, h * 0.1))):
		opacity = 0.35 + 0.18 * index
		out.append('<rect x="%s" y="%s" width="%s" height="%s" fill="none" stroke="%s" stroke-width="1.6" stroke-opacity="%s"/>' % (
			num(cx - half_w), num(cy - half_h), num(2 * half_w), num(2 * half_h), MAGENTA, num(opacity)))
	out.append('<path d="M0 0 L%s %s M%s 0 L%s %s M0 %s L%s %s M%s %s L%s %s" stroke="%s" stroke-opacity="0.45" stroke-width="1.4" fill="none"/>' % (
		num(cx - w * 0.085), num(cy - h * 0.1), w, num(cx + w * 0.085), num(cy - h * 0.1), h, num(cx - w * 0.085), num(cy + h * 0.1), w, h,
		num(cx + w * 0.085), num(cy + h * 0.1), MAGENTA))
	sun = '<path d="M%s %s v-9 h4 v-4 h6 v4 h4 v9 z" fill="%s"/>' % (num(cx - 7), num(cy + h * 0.1), GOLD)
	if animated:
		out.append('<g>%s<animate attributeName="opacity" dur="4s" repeatCount="indefinite" values="1;0.55;1"/></g>' % sun)
	else:
		out.append(sun)
	return out


def cracktro(animated, w, h):
	out = []
	for y, colour in ((16, MAGENTA), (30, PEACH), (44, GOLD)):
		out.append('<rect x="0" y="%d" width="%d" height="10" fill="%s"/><rect x="0" y="%d" width="%d" height="3" fill="#ffffff" fill-opacity="0.35"/>' % (y, w, colour, y + 3, w))
	period = 60
	wave = '<path d="%s" fill="none" stroke="%s" stroke-width="3" stroke-linecap="round"/>' % (sine(-period, w + period, h - 26, 16, period), PALE)
	if animated:
		out.append('<g>%s<animateTransform attributeName="transform" type="translate" values="0 0;%d 0;0 0" dur="8s" repeatCount="indefinite"/></g>' % (wave, period))
	else:
		out.append(wave)
	return out


def twilight(animated, w, h):
	cx, cy, radius = w / 2 - 20, h / 2 + 8, 25
	out = ['<circle cx="%s" cy="%s" r="%d" fill="#3b1d6b"/>' % (num(cx), num(cy), radius),
		'<clipPath id="planet"><circle cx="%s" cy="%s" r="%d"/></clipPath>' % (num(cx), num(cy), radius),
		'<g clip-path="url(#planet)"><circle cx="%s" cy="%s" r="%d" fill="%s"/><circle cx="%s" cy="%s" r="%d" fill="#3b1d6b"/></g>' % (
			num(cx), num(cy), radius, PEACH, num(cx - 9), num(cy + 7), radius)]
	for sx, sy in ((30, 16), (w - 40, 22), (w - 70, 60), (60, 56)):
		twinkle = '<circle cx="%d" cy="%d" r="1.5" fill="%s"%s' % (sx, sy, PALE, ">" if animated else "/>")
		if animated:
			twinkle += '<animate attributeName="opacity" dur="%ds" repeatCount="indefinite" values="1;0.3;1"/></circle>' % (2 + sx % 3 * 2 // 2 * 1 if False else 4)
		out.append(twinkle)
	moon = '<circle cx="%s" cy="%s" r="6" fill="%s"/>' % (num(cx + 52), num(cy - 20), PALE)
	if animated:
		out.append('<g transform-origin="0 0">%s<animateTransform attributeName="transform" type="rotate" values="0 %s %s;180 %s %s;0 %s %s" dur="8s" repeatCount="indefinite"/></g>' % (
			moon, num(cx), num(cy), num(cx), num(cy), num(cx), num(cy)))
	else:
		out.append(moon)
	return out


def ink(animated, w, h):
	out = []
	for cx, cy, start, colour, highlight in ((w / 2 - 22, h / 2 - 4, 20, PEACH, GOLD), (w / 2 + 24, h / 2 + 2, 200, MAGENTA, "#ff9fcf")):
		path = spiral(cx, cy, 1.6, start, 2.5, 26)
		strokes = '<path d="%s" fill="none" stroke="%s" stroke-width="6" stroke-linecap="round"/><path d="%s" fill="none" stroke="%s" stroke-width="2" stroke-linecap="round"/>' % (
			path, colour, path, highlight)
		if animated:
			direction = "120" if start == 20 else "-120"
			out.append('<g>%s<animateTransform attributeName="transform" type="rotate" values="0 %s %s;%s %s %s;0 %s %s" dur="8s" repeatCount="indefinite"/></g>' % (
				strokes, num(cx), num(cy), direction, num(cx), num(cy), num(cx), num(cy)))
		else:
			out.append(strokes)
	return out


def main():
	tiles = []
	scenes = ((maze, "Maze"), (cracktro, "Cracktro"), (twilight, "Twilight"), (ink, "Ink"))
	x0, y0, tile_w, tile_h, gap = 656, 90, 232, 82, 14
	for index, (scene, name) in enumerate(scenes):
		column, row = index % 2, index // 2
		tiles.append(tile(x0 + column * (tile_w + gap), y0 + row * (tile_h + 12), tile_w, tile_h, index, name, scene))
	defs = [d for lines, _ in tiles for d in lines]
	body = [line for _, lines in tiles for line in lines]
	svg = '''<?xml version="1.0" encoding="UTF-8"?>
<!--
	Afterglow Screensavers hero banner: a GNOME-style window holding the four savers side by side.

	(c) Copyright 2026 Liminal HQ, Scott Morris
	SPDX-License-Identifier: Apache-2.0 OR MIT

	Scene: the maze's sun window breathes, the cracktro's sine sways, Twilight's moon
	swings round its planet and Ink's two swirls turn against each other. Keep the 1200 x 320 viewBox,
	SMIL only, the .motion and .static pairs for reduced motion, data-loop on the root and
	animations whose dur divides it. Generated by art/gen-hero.py; do not edit by hand.
-->
<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="320" viewBox="0 0 1200 320" role="img" aria-labelledby="hero-title hero-desc" data-loop="%(loop)d">
	<title id="hero-title">Afterglow Screensavers</title>
	<desc id="hero-desc">Afterglow Screensavers: four screensavers and a collection for the Afterglow-themed ReactOS and any Windows that runs .scr files. A Liminal HQ Labs experiment. An animated banner shows a GNOME-style window holding the Maze, Cracktro, Twilight and Ink scenes: a corridor with a glowing sun window, copper bars over a scrolling sine wave, a planet with a moon, and two turning ink swirls.</desc>

	<style>
		.ui { font-family: ui-sans-serif, system-ui, -apple-system, 'Segoe UI', 'Adwaita Sans', Inter, sans-serif; }
		.mono { font-family: ui-monospace, 'JetBrains Mono', 'SFMono-Regular', Menlo, Consolas, monospace; }
		.bg-a { stop-color: %(a)s; }
		.bg-b { stop-color: %(b)s; }
		.static { display: none; }
		@media (prefers-color-scheme: dark) {
			.bg-a { stop-color: #0c0618; }
			.bg-b { stop-color: #1e0d35; }
		}
		@media (prefers-reduced-motion: reduce) {
			.motion { display: none; }
			.static { display: inline; }
		}
	</style>

	<defs>
		<linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
			<stop class="bg-a" offset="0"/>
			<stop class="bg-b" offset="1"/>
		</linearGradient>
		<linearGradient id="title" x1="0" y1="0" x2="1" y2="0">
			<stop offset="0" stop-color="%(pale)s"/>
			<stop offset="0.5" stop-color="%(peach)s"/>
			<stop offset="1" stop-color="%(magenta)s"/>
		</linearGradient>
		<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" stop-color="#1a1040"/>
			<stop offset="1" stop-color="#5a2160"/>
		</linearGradient>
		<linearGradient id="scrim" x1="0" y1="0" x2="0" y2="1">
			<stop offset="0" stop-color="#0c0618" stop-opacity="0"/>
			<stop offset="1" stop-color="#0c0618" stop-opacity="0.8"/>
		</linearGradient>
		<radialGradient id="wash">
			<stop offset="0" stop-color="%(magenta)s" stop-opacity="0.2"/>
			<stop offset="1" stop-color="%(magenta)s" stop-opacity="0"/>
		</radialGradient>
		<clipPath id="frame"><rect width="1200" height="320" rx="18"/></clipPath>
		%(defs)s
	</defs>

	<g clip-path="url(#frame)">
		<rect data-bg="" width="1200" height="320" fill="url(#bg)"/>
		<circle cx="1000" cy="330" r="320" fill="url(#wash)"/>

		<!-- Left: title, description, status chip -->
		<!-- hero-text:start (system font; outline it for the final wordmark) -->
		<text class="ui" x="56" y="140" font-size="44" font-weight="800" letter-spacing="-1" fill="url(#title)">Afterglow Screensavers</text>
		<!-- hero-text:end -->
		<text class="ui" x="58" y="180" font-size="15" font-weight="500" fill="#e8dcf2">Four screensavers and a collection, drawn in software</text>
		<g class="ui" font-size="13" font-weight="600" fill="#d6c6e6">
			<rect id="chip-status" data-box="" x="58" y="204" width="134" height="28" rx="14" fill="%(aqua)s" fill-opacity="0.1" stroke="%(aqua)s" stroke-opacity="0.5"/>
			<circle class="motion" cx="76" cy="218" r="4" fill="%(aqua)s">
				<animate attributeName="opacity" dur="4s" repeatCount="indefinite" calcMode="spline" keySplines="0.45 0 0.55 1;0.45 0 0.55 1" keyTimes="0;0.5;1" values="1;0.35;1"/>
			</circle>
			<circle class="static" cx="76" cy="218" r="4" fill="%(aqua)s"/>
			<text data-for="chip-status" x="88" y="223">experimental</text>
		</g>
		<text class="mono" x="58" y="262" font-size="12" fill="#a995c2">Liminal HQ Labs</text>

		<!-- Right: a GNOME-style window -->
		<rect x="644" y="42" width="500" height="246" rx="14" fill="#000000" opacity="0.4"/>
		<rect x="640" y="36" width="500" height="246" rx="14" fill="#241440"/>
		<path d="M640 50 a14 14 0 0 1 14 -14 H1126 a14 14 0 0 1 14 14 V74 H640 Z" fill="#33205a"/>
		<rect x="640" y="73" width="500" height="1" fill="#ffffff" fill-opacity="0.1"/>
		<text class="ui" x="890" y="60" font-size="13" font-weight="700" text-anchor="middle" fill="#f3e9fb">Afterglow Screensavers</text>
		<circle cx="1118" cy="55" r="9" fill="#ffffff" fill-opacity="0.12"/>
		<path d="M1114 51 L1122 59 M1122 51 L1114 59" stroke="#f3e9fb" stroke-width="1.6" stroke-linecap="round"/>
		<rect x="640.5" y="36.5" width="499" height="245" rx="13.5" fill="none" stroke="#ffffff" stroke-opacity="0.1"/>
		%(body)s
	</g>
</svg>
''' % dict(loop=LOOP, a=PLUM_A, b=PLUM_B, pale=PALE, peach=PEACH, magenta=MAGENTA, aqua=AQUA, defs="\n\t\t".join(defs), body="\n\t\t".join(body))
	os.makedirs(os.path.join(ROOT, "assets"), exist_ok=True)
	with open(os.path.join(ROOT, "assets", "hero.svg"), "w") as handle:
		handle.write(svg)


if __name__ == "__main__":
	main()
