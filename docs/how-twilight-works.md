# How Twilight is drawn

Afterglow Twilight is drawn entirely by the processor, in Rust, with no graphics card and no maths library: every picture is computed pixel by pixel into a block of memory, then handed to Windows to show. The code is `crates/savers-core/src/scenes/twilight.rs`; this is what it does, in plain English.

## Before the first frame: painting the world

When the saver starts it paints the planet once, from a seed number, so every seed is a different world.

- **The map.** The planet's surface is a flat rectangular map, like a world map, where left to right is longitude and top to bottom is latitude. For each point on the map, the program works out where that point sits on a ball and asks a **noise function** for a height there. Noise is a smooth, endlessly varied random landscape: the same inputs always give the same value, and nearby points give similar values. Several layers of it at finer and finer scales (fractal noise) give coastlines with bays inside bays. The positions are first nudged by another noise field (domain warping), which bends the continents into flowing, organic shapes.
- **Sea and land.** Below a chosen level is sea, deeper colours further from the coast; above it is land. A second noise field decides which land is dry and which is wet, and each gets a different set of palette colours.
- **Stepped terraces.** The land's height is cut into six steps, like layered paper or contour lines on a painting. Each step gets a lit rim and a shadowed foot, which is what gives the planet its painted, voxel-like relief.
- **The painterly pass.** A thin layer of noise stretched along the lines of latitude lightens and darkens the colours slightly, like brush strokes laid across the planet. Ice covers the poles; a sprinkle of lights is scattered along the coasts for the night side.
- **Clouds** are a second map: banded noise, swirled, kept as transparency.
- **The sky behind** is a nebula painted the same way in the palette's hues, dimmed, with dark dust lanes, plus a few hundred stars.

## Every frame: turning a flat map into a turning world

- **The camera.** Four shots loop over 90 seconds: a wide establishing shot, a slow push-in towards the line between day and night, a pass from above while a satellite crosses close by, and a drift along the night side. Each shot moves the planet's position, size, tilt and roll smoothly from its start to its end, with a fade at each cut.
- **Projection.** For each pixel inside the planet's circle, the program works out which point of the ball is under it: the x and y come from the pixel's position, and the depth from Pythagoras (the ball's surface is where x² + y² + z² = 1). It then turns that point by the camera's tilt and roll to find where it is on the planet, and reads its latitude (with an arcsine) and longitude (with an arctangent). The spin is simply an offset in longitude that grows with time, so the continents slide round while the light stays put. The clouds use a slightly faster offset, so they drift over the ground.
- **The terminator.** How brightly a point is lit depends on the angle between its surface and the direction of the sun. Instead of plain brightness, that angle picks a **colour of light** from a ramp: warm white at noon, then gold, then orange, then rose right at the line between day and night (the terminator), then purple dusk and deep indigo night. The ground's colour is multiplied by that light colour. That is why the edge of the day side glows rose and orange instead of just going dark: it is the planet's evening, painted.
- **Night.** On the dark side, the coastal lights glow gold.
- **The atmosphere.** Near the edge of the ball you look through more air, so a glow is added that grows towards the rim (inside the circle) and fades outwards just beyond it (outside the circle), brighter on the side facing the sun. Its colours come from the palette, so a Dusk planet has a rose-purple halo and an Aurora planet a teal one.
- **Moons and satellites.** Each moon is a small lit ball on its own tilted orbit, drawn before the planet when it is behind and after it when it is in front, with soft noise for craters. Satellites are single bright points with a blinking light.
- **Smooth edges.** The planet's and moons' outlines are blended over their last pixel, so the limb is clean, not jagged.

## The finish: making it look filmed

The same finishing chain runs over every Afterglow saver (`crates/savers-core/src/post.rs`):

- **Glow (bloom).** The brightest parts are copied into a picture a quarter of the size, blurred, and laid back over the frame, so bright things seem to give off light.
- **Light streaks.** The very brightest parts are also smeared far to the left and right, like the anamorphic lens flares of cinema.
- **Vignette.** The corners are gently darkened, like an old lens.
- **Film grain.** Every pixel gets a tiny random change every frame. It looks like film, and it means no pixel on the screen ever stays perfectly still, which is what a screensaver is for.
- **Fades and letterbox.** Slow fades at the cuts between shots, and optional black cinema bars.

## Why no graphics card

ReactOS's OpenGL is weak, and old machines have no 3D card worth the name, so everything here is plain arithmetic the processor can do anywhere, even a Pentium. The maths functions (sine, cosine, square root, arctangent) are the program's own, written out as short formulas, because the program has no maths library. The same scene can be drawn at a quarter of the screen's resolution and enlarged, which is how it keeps moving smoothly on a Pentium III. A graphics-card edition for 4K is designed in [gpu-twilight.md](gpu-twilight.md).
