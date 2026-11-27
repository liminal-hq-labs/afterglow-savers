# Scenes

The storyboard for the four savers and the collection: what the viewer sees, how the camera moves, what each scene is made of, and how it scales from a Pentium III to a modern machine. The look and the palette rules are in [art.md](art.md); behaviour and settings are in [SPEC.md](../SPEC.md).

## The shared idea

An XP-era screensaver protected a CRT by never holding still. These keep that duty and add a modern, cinematic finish: every scene is a moving picture in the Afterglow palettes, finished by the same post-processing chain (glow, anamorphic streaks, vignette, fine grain, optional letterbox and slow fades). Branding is only the palette and the shapes: stepped bands, round suns, soft horizons. No wordmark, except the credits the cracktro's scroller reads out.

What none of them is: a sunset or sunrise scene, phosphor trails, halo rings, bouncing suns, a card cascade, or a kaleidoscope.

## The burn-in rule

Every scene obeys one rule, checked by eye in the preview strips and by `burn_in` tests that compare frames ten seconds apart:

- **Nothing bright holds still.** No pixel keeps a luma above 40% for more than 8 seconds. Bright things travel: the scroller, the planet's lit limb, the maze's sun window (the camera always moves), the ink's hot filaments.
- **Dark is calm.** Most of the screen is dark most of the time (mean luma under about 35%), which is also what makes the glow read.
- **Grain keeps every pixel alive.** At its default strength the film grain changes every pixel by a level or two every frame, so even a dark corner is never static.
- **Overlays never park.** The letterbox bars are pure black (safe), and no logo, clock or text stays in one place.

## The quality ladder

Two settings set the cost, and both are in every saver's settings:

- **Render scale** (100%, 75%, 50%, 25%): the scene is drawn at that fraction of the screen's width and height and scaled up by the finishing pass. 25% of 800 × 600 is 200 × 150, which a Pentium III draws in a few milliseconds per scene.
- **Detail** (Low, Medium, High, Ultra): what each scene draws per pixel: octaves of noise, reflections, cloud layers, ink layers, bloom resolution, streak length. Each scene's ladder is in its section below.

**Targets** (SAV-D19): at 25% and Low, a Pentium III-class machine (QEMU `-cpu pentium3`) keeps at least 15 frames a second; on a modern virtual machine at 100% and Ultra, at least 30. The preview tool's `bench` times every rung on the host, and estimates the Pentium III from a calibrated reference loop (see [testing.md](testing.md)).

**Finishing pass.** Post-processing runs at render resolution, except the grain and the final scaling, which run at the output size, capped per detail level so a 1600 × 1200 screen does not cost four times a 800 × 600 one: the shell's `StretchDIBits` does the last enlargement.

## Ink

*Flowing ink on dark water: colour folds over itself like marbled paper, and now and then the folds stand up into aurora curtains.*

**How it works.** A marbling sheet is painted once at startup (and again for each new pour) as a tiling texture: drops of palette ink laid down as concentric rings, then combed. The screen holds, for each pixel, where on that sheet its ink came from. Each frame every pixel looks back along a slowly changing flow (a few drifting vortices and a large shear, all divergence-free, so the ink never piles up or thins out) and takes the sheet position from there. Ink folds into ever finer filaments without blurring, because the colour is read fresh from the sheet each frame and only the positions move. Two such layers run half a cycle apart and cross-fade, so the pattern renews itself before it frays into noise (texture advection with a dual phase).

**Second by second** (one 60-second cycle; the flow and the pour differ every run):

- **0–5 s.** The screen fades up from black on a still pour: rings of rose, orange and purple ink (Dusk) floating on the void, a soft vignette.
- **5–20 s.** Two vortices wake and drag the rings into commas and hooks. The bright orange band catches the glow and throws a thin horizontal streak.
- **20–35 s.** The shear strengthens; the hooks fold into feather patterns, the classic marbled-paper "nonpareil". A slow light sweep, like a lamp passing over wet paper, crosses diagonally and lifts every band it touches.
- **35–45 s.** **Wow:** an aurora phase. The flow turns mostly vertical, the filaments stretch into tall curtains, and a vertical ray texture shimmers through them, brightest at their lower edges, fading upwards.
- **45–60 s.** A new pour fades in through the old one (the second layer), with the palette's hues in a new order; the cycle continues without a cut.

**Camera.** None in the usual sense: a slow drift and a very slow zoom of the whole sheet (about 3% a minute) so even the large forms travel.

**Palette.** The palette's glow hues as ink, its neutral 900 as the water, a lifted neutral as a pale "paper white" ink line between colour bands. Light mode is marbled paper: paper 900 as the ground with the deep hues as ink.

**Settings.** Ink thickness (how wide each ink band is against the dark water), turbulence (vortex strength and count), seed (0 means a new one each run).

**Ladder.** Low: a 256-texel sheet read nearest, bloom at an eighth of the size, no curtains. Medium: a 512-texel sheet read bilinear, and the aurora curtains (light carried up each column from bright, saturated ink as a decaying maximum). High: adds the light sweep and the wet-ink relief (each pixel lit by how much brighter it is than its upper-left neighbour). Ultra: a finer 8-pixel flow grid.

## Twilight

*A tiny painted planet turning in a luminous atmosphere, its day and night sweeping round it, moons and satellites crossing, the camera drifting like a slow documentary.*

**How it works.** The planet is generated from a seed as soft colour fields rather than a photograph: an equirectangular map of continents (domain-warped noise cut into stepped terraces, each step with a lit rim and a shadowed foot like contour lines in a painting, dry and wet land from a second field, coastal shelves, brush strokes along the latitude lines), seas, ice and night lights; a separate cloud map in drifting bands. Each pixel of the disc turns its normal into the planet's frame (the camera's tilt and roll) and reads latitude (an arcsine table) and longitude (the core's `atan2`); the spin is an offset in longitude. Light is a colour multiplied into the ground: warm white by day, orange then rose at the terminator, purple dusk, indigo night, with warm lights on the night side. The atmosphere is a glow both inside the limb (a fresnel tint) and outside it (a soft falloff), brightest on the day side and pink at the terminator. Behind it: a painterly nebula in the palette's hues and a sparse twinkling starfield.

**Second by second** (one 90-second loop of four shots; the seed changes the planet, the moons and the shot order):

- **Shot 1, 0–25 s: the establishing shot.** The planet small and off-centre on the left third, slowly turning; the terminator crosses its face; one moon swings in front, its shadow a soft dot on the clouds.
- **Shot 2, 25–50 s: the push-in.** A slow dolly towards the terminator until the planet fills two thirds of the frame. The cloud bands drift faster than the ground; the atmosphere's rim glows rose where day meets night. **Wow:** as the camera passes the limb the rim brightens into a streak of light across the frame (the anamorphic streak catching the atmosphere).
- **Shot 3, 50–70 s: the satellite pass.** A wide shot from slightly above; a satellite with a blinking light crosses close to the camera, fast, then a second moon rises over the limb.
- **Shot 4, 70–90 s: the drift.** The camera rolls a few degrees and slides along the night side, city lights glinting, the nebula sliding behind with parallax, lit from behind; **wow:** the sun slides out from behind the limb. Fade to the next loop.

**The sun** (on by default, the `Sun` setting): a small white-hot disc in the palette's sun hue with a soft glow, never on a horizon. In the establishing shot it sits far off on the lit side, a star among the stars; in the night-side drift the light comes from behind, and the sun slides out from behind the limb through the atmosphere, a diamond-ring glimpse the bloom and streaks pick up. It is drawn behind the planet and moons, so they hide it as they pass.

**Palette.** Seas in the neutral 700–800 range tinted by the first glow hue, land in the glow hues at low saturation, highlands lifted towards neutral 300, ice in neutral 200, the atmosphere in the glow hues, the sunlight in the palette's sun hue.

**Settings.** Planet seed (0 means a new world each run), atmosphere strength, moons (0–2), show the sun.

**Ladder.** Low: 256 × 128 maps read nearest, no cloud shadows, no night lights, bloom at an eighth. Medium: 512 × 256 maps read bilinear, cloud shadows. High: the night-side lights. Ultra: the seas' sheen. Start-up paints the maps: about 50 ms at Low and 150 ms at Ultra on the development host.

## Cracktro

*A demoscene intro in Afterglow colours: copper bars, plasma, a rotozoomer, a stepped tunnel and vector bars, with a sine scroller greeting everyone who keeps old machines alive.*

**How it works.** Drawn at a low internal resolution by design (a 320-wide stage, scaled up crisply), in five parts that change every 20 to 40 seconds with transitions (a raster wipe, a pixel dissolve, a copper curtain). The scroller runs through every part along the lower third, in an original stroke font drawn at startup, filled with a copper gradient and bending on a sine wave. Bobbing sprites (small stepped suns and diamonds) ride sine paths over the copper bars.

**Parts** (the default order; the order and which parts appear are settings):

1. **Copper bars, 0–30 s.** Horizontal raster bars in the glow hues, rising and falling on sines, with soft highlights; a starfield scrolls behind; the sprites swing in a figure-of-eight. The scroller enters from the right.
2. **Plasma, 30–60 s.** Full-screen plasma, the classic sum of sines, mapped through a cycled palette ramp. **Wow:** the plasma's palette cycles in time with the scroller's wave.
3. **Rotozoomer, 60–90 s.** A tiled texture of stepped suns and inlaid tiles rotating and zooming, breathing in and out.
4. **Stepped-ring tunnel, 90–120 s.** A tunnel drawn from angle and distance tables, its walls stepped rings in the palette, the eye swinging round inside it. **Wow:** the tunnel's mouth glows white-hot and blooms.
5. **Vector bars, 120–150 s.** A twister: a square column twisting down the screen, flat-shaded in four hues, with rotating 3D bars sweeping across.

Then the cycle repeats. The default message reads, in part: "Afterglow for ReactOS ... greetings to the ReactOS team, the Wine project, everyone keeping a 486 humming, the DOS and Amiga sceners, retro OS archivists ... an unofficial remaster, made by Liminal HQ" and is editable in the settings.

**Palette.** The palette's glow hues and its sun hue as the copper ramp, the neutral 900 as the void, neutral 100 as the scroller's highlight.

**Settings.** Scroller text, which parts play and whether in order or shuffled, scroller speed (the speed slider).

**Ladder.** The stage is drawn small and enlarged with hard pixel edges: 240 pixels wide at Low, 320 at Medium, 400 at High (which adds the scroller's reflection in a strip of water), 480 at Ultra. It is the cheapest scene: 2 to 4 ms a frame on the development host at any rung.

## Maze

*A walk through dusk architecture: stepped stone and glass under an evening sky, polished floors, soft fog, and a glowing sun window at the end of every long corridor.*

**How it works.** A ray caster in the spirit of the classic 3D Maze, with better materials. A maze is generated from a seed (a recursive backtracker on a grid that carries straight on two times in three, for long corridors with a view down them), and its walls are stone blocks, brick with mortar, inlaid tile bands and glass panes, all generated as textures at startup. The tops of the walls are stepped like crenellations, cut against an open dusk sky with slow clouds and the first stars. The floor is polished stone that mirrors the walls (the wall column mirrored about its base, darkened and blended with the floor texture) or still water that ripples the reflection. Distance fog tints everything towards the sky's horizon colour. Sun windows, discs of stepped stained glass, glow at the ends of corridors and throw light shafts (a radial blur of the bright parts, from the window's centre on screen).

**Second by second** (each maze takes one to three minutes to walk, then fades to a new one):

- **0–4 s.** Fade up inside the maze, facing down a corridor. Fog, quiet.
- **4–30 s.** The camera glides forward along the solution path, easing into each turn on a smooth curve, looking slightly ahead into the turn. Crenellated wall tops step against the sky.
- **30–60 s.** A long corridor opens with a sun window at its end. **Wow:** the light shafts fan out of it across the polished floor and the reflection doubles the window.
- **Through the walk.** Now and then the path enters a dead end and turns round in a slow arc, or pauses at a junction and looks both ways. The camera never stops moving for more than a second.
- **The exit.** The last corridor ends in the largest window; the camera walks into the light, the screen blooms to the window's colour and fades into a new maze.

**Palette.** Stone in the neutrals lifted towards the first glow hue, brick in the warm glow hue darkened, glass in the cool hues, mortar dark, the sky from the palette's dusk gradient, the windows in the sun hue with the glow hues in the stepped rings.

**Settings.** Floor (polished stone, water or tiles), corridor style seed (0 means new each run), maze size.

**Ladder.** Low: 64-texel textures and a plain floor with fog. Medium: 128-texel textures and the mirrored walls in the floor. High: light shafts from the brightest window on screen. Ultra: every floor ripples a little (water always does).

## Collection

*All four in turn.*

Afterglow Collection runs the four savers one after another, each for its duration (90 seconds by default), with a two-second cross-fade between them (both scenes are drawn during the fade). Each scene starts 2.5 seconds into its own storyboard, so its fade-up is over before it appears, and the next scene is made five seconds before its fade, so its start-up does not fall in the fade. The order is Ink, Twilight, Cracktro, Maze, or shuffled; any saver can be left out. It uses its own render settings and each saver's own scene settings.

## Quill Ghostwriter (an Easter egg)

*A document that writes itself: a Quill window drifting across a dark desk while Quill types, makes mistakes, reflows, rewinds and changes eras.*

An extra file, `AGQUILL.SCR`, listed as "Quill Ghostwriter" once installed. It is **not** installed or selected by default, is in no default component list, and is left out of the Collection unless its "Include Quill Ghostwriter" setting is on (SAV-D20). It may later be started from Quill itself, for example by a click on the nib in Quill's About box (an open question for the Quill project; this repository does not touch Quill).

**How it works.** A Quill-style window in Quill's navy and gold (the Q-with-a-nib identity, a gold nib as the caret) is drawn as a textured card floating over a dark desk, with a slow cinematic camera (drift, a gentle tilt and dolly), so the window and its text always move on screen. The document is laid out by a small text engine (line breaking, justification, hyphenation), its text drawn with Quill's own bitmap fonts where practical (copied with provenance from the Quill sources, SAV-D21) and the core's stroke font otherwise. Lines come from an editable text bank stored in the saver's settings, with a built-in default bank.

**Acts** (each 20 to 40 seconds; which acts run is a setting):

1. **Typing, with human mistakes.** Characters arrive at a typist's uneven rhythm. A typo appears, a red spell-check squiggle wriggles under it, a suggestion popup fades up, the correction lands. The status bar counts like Quill's: `Pg 1 Ln 1 Col 88`, `Times New Roman 12`, `Modified`.
2. **The reflow ballet.** The margins drag in and out; the text reflows and justifies like water finding its level, words hyphenating at the edge.
3. **The time machine.** Undo runs like a tape rewinding: letters fly back into the caret, then redo rushes them forward again, while an overlay draws the **piece table** (Quill's real data structure) as coloured bars splitting and merging. **Wow:** the rewind's motion blur and the bars snapping together.
4. **Font eras.** The page morphs through the DOS 8 × 16 bitmap font, a Windows 3.x look and modern type, the window chrome changing with it.
5. **The real lore.** The document types project facts and jokes from the text bank: decision-log lines ("DOS-37: the QUILLDOS.EXE budget is 416 KB"), parody release notes ("v0.0.1: it types"), man-page text, a haiku about piece tables.
6. **The visit from the dialog.** Now and then "Save the changes to Untitled before Quill closes?" fades up with Yes, No and Cancel; Cancel's focus rectangle blinks; the saver never clicks.

**Palette.** Quill navy and gold by default, or any Afterglow palette (navy becomes the palette's neutral 800, gold its sun hue).

**Settings.** The common ones, plus the text bank editor and which acts run.

**Ladder.** Low: no camera tilt (a flat drift), no motion blur. Medium: tilt. High: the rewind's motion blur and soft shadows under the window. Ultra: depth of field on the desk.
