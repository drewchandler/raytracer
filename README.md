# Raytracer

A small, recursive ray tracer in Rust. This builds on the original `Camera`,
`Ray`, `Shape`, `Object`, `Scene`, and `Raytracer` design. Shapes still find ray
intersections; objects supply surface colors and materials; the scene organizes
objects and lights; the ray tracer shades pixels and follows secondary rays.

## Run in the prepared cloud environment

From `/workspace/raytracer`:

```sh
./scripts/cargo.sh test --locked
./scripts/cargo.sh run --release --locked -- --output out.png
```

The default scene is a classic ray-tracing composition: large chrome and glass
spheres, smaller marble, wood, and stone spheres, and a subtly bumped checkerboard
floor beneath a procedural cloudy sky. The glass is lightly frosted and reflections
can be rough; `--scene classic` keeps the smooth colored-sphere composition. A low, centered camera emphasizes the reflections. It
uses an area light for soft shadows and a directional fill light. The executable
renders a PNG and exits; no server or graphics window is needed. Save failures produce a message
and a nonzero exit status.

```sh
# Quick preview
./scripts/cargo.sh run --release --locked -- --width 320 --height 240 --samples 1

# Higher-quality edges and soft shadows (samples is per axis)
./scripts/cargo.sh run --release --locked -- --samples 4 --light-samples 6 --depth 7

# Depth of field: larger lens radius gives stronger blur
./scripts/cargo.sh run --release --locked -- --aperture 0.08 --focus 11 --samples 8

# The original two-sphere image, without antialiasing
./scripts/cargo.sh run --release --locked -- --scene original --samples 1

./scripts/cargo.sh run --release --locked -- --help
```

The default sky uses seeded 3D Perlin gradient noise, a daylight gradient, and shaded cloud
banks. It is continuous around the entire sky dome, including the reflected
view. A light distance haze fades the far checkerboard into the horizon without
softening the foreground. It needs no external image files. To use your own sky texture instead:

```sh
./scripts/cargo.sh run --release --locked -- --sky clouds.png --sky-rotation 45
```

Use an equirectangular latitude/longitude panorama (usually a 2:1 PNG or JPEG).
The top row is the zenith and the bottom row is the nadir. Rotation is in degrees
around the vertical axis; negative angles are allowed. Panoramas wrap
horizontally, clamp at the poles, and use bilinear filtering in linear light.
The sky appears in the background, mirrors, and glass. Scene lights still provide
direct illumination; the environment does not add path-traced indirect lighting.
The original scene keeps its flat background unless `--sky` is supplied.

Defaults are 640×480, 2×2 pixel samples, 3×3 area-light samples, and five
reflection/refraction bounces. The gallery uses 4×4 pixel samples for smoother
rough surfaces. Samples are deterministic, so repeated renders
with identical settings produce the same pixels. At depth zero, direct lighting
still works, but no secondary rays are followed. Use an existing output
directory; the renderer does not create parent directories.

### Toolchain

No dependencies were added or upgraded. The locked `image 0.7.0` dependency tree
requires an old compiler: modern Rust rejects parts of its `rayon` and
`rustc-serialize` dependencies. The prepared environment provides Rust 1.8.0,
Cargo from Rust 1.20.0, and checksum-verified vendored crates. The helper script
selects them explicitly; it does not install tools, fetch dependencies, or change
the environment configuration.

`/workspace/.raytracer-tools/build/Cargo.toml` is a helper manifest with a `src`
symlink to this checkout. Cargo can migrate the old lockfile's metadata there
without modifying the repository's manifest or lockfile; all 23 dependency
versions are retained. Source changes are used immediately. After intentionally
changing dependency declarations, refresh that helper and the vendored crates
using the environment's setup script before building. `RAYTRACER_TOOLING` can
select another prepared tooling directory.

`./scripts/cargo.sh check --locked` runs a full build because Rust 1.8 predates
Cargo's metadata-only check mode.

## GitHub Pages gallery

`.github/workflows/pages.yml` builds and tests the renderer, renders an 800×600
scene from the checked-out commit, and generates an HTML gallery with the PNG,
render settings, and a link to its source commit. The page uses relative asset
links, so it works at a project URL such as
`https://drewchandler.github.io/raytracer/`.

Pushes to `master` deploy the result using the official GitHub Pages actions.
Pull requests build a downloadable `raytracer-gallery` artifact for review and
never deploy. The workflow can also be run manually from the Actions tab;
deployment is limited to the `master` branch.

Before the first deployment, enable **Settings → Pages → Build and deployment →
Source: GitHub Actions** in the repository. The workflow's deploy job uses the
`github-pages` environment and the standard `pages: write` and `id-token: write`
permissions. It does not need a personal access token. Branch/environment
protection rules may require approval before a deployment.

The workflow prepares the old toolchains on a clean Ubuntu runner with
`scripts/prepare.sh`; no cloud snapshot or `/workspace` directory is required.
Installed tools and verified dependencies are cached separately from the source.
For a fresh local setup, choose a directory outside the checkout:

```sh
export RAYTRACER_TOOLING=/tmp/raytracer-tools
./scripts/prepare.sh
./scripts/cargo.sh test --locked
./scripts/cargo.sh build --release --locked
python3 scripts/build_site.py \
  --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer" --output _site --extras
python3 scripts/test_site.py \
  --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer"
```

Open `_site/index.html` locally, or use `python3 -m http.server --directory _site`
for a local HTTP check. The generator uses Python 3.9+ with no third-party
packages. Its `--width`, `--height`, `--samples`, `--light-samples`, and `--depth`
options control the rendered image. `GITHUB_SHA` and `GITHUB_REPOSITORY` supply
the source link automatically on Actions; local runs show “Local render”.
With `--extras`, the gallery also contains a Utah teapot render, volumetric clouds,
four editable example scenes with downloadable JSON, and a quick animation preview.
Animations start on a still poster and have
play/pause controls; the preview is displayed at its native size. The Pages
workflow enables these views automatically. A separate quality workflow later
replaces the preview with the full-quality animation from the same commit. The generated site is ignored by Git.

## Features

Scenes can also be edited as JSON, with named materials, cameras, lights,
transforms, procedural skies, and relative OBJ/panorama assets. See the
[scene format and examples](scenes/README.md). Render one with:

```sh
./scripts/cargo.sh run --release --locked -- \
  --scene-file scenes/glass-garden.json --output renders/garden.png
```

- Ambient, Lambert diffuse, and Phong specular shading with multiple colored
  lights. Point lights have inverse-square falloff; directional lights have
  constant illumination. Rectangular area lights use an N×N sample grid, with
  total intensity independent of the sample count.
- Hard shadows and sampled soft shadows, with light-distance limits and surface
  offsets to avoid self-shadowing. Transparent objects filter shadow rays.
- Bounded mirror recursion, Snell refraction, nested refractive media, Schlick
  Fresnel blending, and total internal reflection. Rays can start inside glass.
- Spheres, infinite planes, axis-aligned boxes, and two-sided triangles with
  normals. Spheres and boxes have entry and exit intersections.
- Translation, nonuniform scaling, and rotation around all three axes through
  a `Transformed` shape wrapper. Normals use the inverse transpose; local rays
  retain their length so intersection distances remain consistent.
- Object-space stripes, 2D checkerboards, and 3D checkers. Textures follow
  transformed shapes. The 2D checker uses x/z and is stable on a horizontal floor.
- An orthonormal perspective camera with look-at positioning, horizontal field
  of view, subpixel sampling, and optional thin-lens depth of field.
- Floating-point linear-light accumulation for shading and antialiasing, with
  sRGB decoding for scene colors and sRGB encoding/clamping only at output.
- Environment textures: a deterministic cloudy sky or panoramic image textures,
  with rotation and seamless sampling in backgrounds, reflections, and refractions.
- CLI controls for scene, output, resolution, antialiasing, area-light samples,
  recursion depth, aperture, and focus distance.

This is a Whitted-style renderer with direct lighting. It does not implement
path-traced indirect illumination, caustics, or image textures on object surfaces.
Object lookup is linear in the small scene object count; imported triangle meshes
use a bounding volume hierarchy. The sky volume uses single scattering and
self-shadowing, rather than a general multiple-scattering volume integrator. Glass shadows use
straight filtered rays rather than solving refracted light paths; tint is
applied at each interface rather than using volume absorption.

## Noise materials, bumps, and rough surfaces

`Pattern::Marble(a, b, size)`, `Pattern::Wood(a, b, size)`, and
`Pattern::Stone(a, b, size)` use the shared seeded 3D gradient-noise module.
Marble uses turbulent veins; wood uses distorted growth rings; stone combines
noise at several scales. Colors blend in linear light. Sphere patterns are
centered on the sphere, and patterns follow shape transforms.

`material.bump = Some(Bump { scale: 8.0, strength: 0.02, seed: 7 })` perturbs the
shading normal using noise derivatives. The underlying intersections and
geometric normals stay unchanged. Bump gradients use the inverse transpose under
nonuniform transforms. Keep strength small for fine surface relief.

`roughness` controls reflected-ray spread, and `transmission_roughness` controls
frosted glass. Both range from zero (perfect) to one. Sampling is deterministic;
the first rough bounce averages four lobe samples, and later rough bounces use
one per camera sample to avoid exponential sample growth. Increase `--samples`
to 4–8 for smooth rough reflections. This is a bounded Whitted sampling model,
not a microfacet path tracer.

## Animated and volumetric clouds

```sh
# A reproducible moment in the moving Perlin sky
./scripts/cargo.sh run --release --locked -- --time 12 --output clouds-at-12.png

# Ray-marched 3D cloud layer with sunlight scattering and cloud self-shadowing
./scripts/cargo.sh run --release --locked -- \
  --scene classic --clouds volume --cloud-steps 32 --samples 2 --time 8

# A clearly labeled, seamless quick preview
python3 scripts/render_animation.py \
  --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer" \
  --preset preview --output renders/preview/clouds.png

# A full-quality, 24 FPS seamless animation (CPU intensive)
python3 scripts/render_animation.py \
  --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer" \
  --preset quality --jobs 4 --cache-dir renders/.animation-cache \
  --output renders/quality/clouds.png
```

For the prepared cloud environment, the renderer path is
`/workspace/.raytracer-tools/build/target/release/raytracer` if
`RAYTRACER_TOOLING` is unset. No third-party Python packages are required.

| Setting | Quick preview | Quality render |
| --- | --- | --- |
| Resolution | 320×240 | 640×480 |
| Pixel samples | 2×2 (4) | 4×4 (16) |
| Area-light samples | 2×2 (4) | 2×2 (4) |
| Reflection/refraction bounces | 5 | 6 |
| Frame rate | 12 FPS | 24 FPS |
| Frames | 24 | 48 |
| Playback duration | 2 seconds | 2 seconds |

The clouds follow a closed orbit through the 3D noise field, including their
reflections and refractions. Position and velocity match at the loop boundary.
Frames sample the cycle at uniform intervals and exclude the duplicate endpoint,
so playback does not pause or jump at the seam. `--cloud-loop SECONDS` enables the
same periodic motion for individual renderer invocations. `--loop-duration` can
choose a different noise-cycle duration; by default it matches frames/FPS.
The optional legacy `--step` flag explicitly selects linear, non-looping wind.

The quality preset uses small 1/24-second time steps, rather than the original
preview's three simulated seconds per frame. `--frames 96` at 24 FPS gives a
four-second loop. Resolution, pixel/light samples, bounce depth, cloud integration
steps, frame rate, and worker count can all be overridden on the command line.

The exporter renders frames in parallel and checks PNG checksums, dimensions,
and timing. A persistent cache is keyed by the renderer's SHA-256 and all render
settings, including exact frame times. A retry reuses only matching, complete
frames; corrupt frames and changed settings trigger rerenders. The output is
replaced only after every frame succeeds. It also writes a still poster and JSON
metadata with the actual quality settings and source commit. APNG is lossless;
high-quality files can be large and take minutes to render. The gallery keeps the
poster visible until the user chooses to load/play the animation, and supports
pausing it again.

### Separate quality workflow

`.github/workflows/animation-quality.yml` starts after a successful `master`
gallery build, or can be run manually from the Actions tab. It renders the quality
preset separately, with two workers and a resumable frame cache; it does not
block the quick gallery build or PR checks. The workflow offers 48- or 96-frame
renders and uploads `raytracer-quality-animation` as a downloadable artifact.

For publication, it downloads the quick gallery for the exact same source SHA,
validates the animation's dimensions, timing, quality settings, and source SHA,
and replaces only the animation and its poster/metadata. Existing still images
are retained. The gallery is republished through GitHub Pages. Both Pages
publishers serialize deployments and check that `master` still points to the
rendered SHA; obsolete jobs skip publication. A manual run on a feature branch,
or without a matching successful gallery build, produces an artifact only.

You can attach a completed local render without rerendering the still images:

```sh
python3 scripts/attach_animation.py --site _site \
  --animation renders/quality/clouds.png --require-quality
```

Published builds additionally require `--expected-commit` to bind the animation
to its matching gallery. The quick build stays labeled “Quick preview” until a
validated quality render is attached, at which point it shows “High quality” and
the actual resolution, frame rate, and sample count.

The volume is a world-space cloud layer between y=20 and y=32, sampled along
camera and secondary rays with Beer–Lambert extinction and single sunlight
scattering. Four light-march samples estimate cloud self-shadowing. It also
attenuates finite surfaces seen through the layer. Clouds do not cast additional
shadows onto the ground lights. `--cloud-steps` accepts 8–128 (default 24);
more steps and pixel samples reduce integration aliasing. A grazing-distance
limit keeps horizon rays bounded. Volume mode cannot be combined with a custom
panoramic `--sky`. Ordinary procedural clouds remain the faster default.

## OBJ meshes and the Utah teapot

```sh
./scripts/cargo.sh run --release --locked -- --scene teapot
./scripts/cargo.sh run --release --locked -- --scene teapot --obj my-model.obj
```

The importer accepts vertices, normals, triangle/convex-polygon faces,
`v/vt/vn` and `v//vn` syntax, and negative indices. It reports invalid indices,
non-finite coordinates, and degenerate faces with line numbers. Polygons are fan
triangulated; triangulate concave polygons in your modeler before exporting.
Provided vertex normals are interpolated for smooth shading; otherwise faces
are flat. UV/material directives are ignored and one scene material is used.

Models are centered, placed on the floor, and uniformly fitted to the scene.
A median-split BVH prunes ray/triangle tests; smooth shading normals and geometric normals are
kept separate so ray offsets remain reliable. The bundled 4,032-triangle Utah teapot loads
without a download. Its MIT-licensed source data, attribution, license, and
reproduction instructions are in [assets/README.md](assets/README.md).

## Extend a scene

Edit `src/demo.rs` to add objects and lights. An object's existing `color` remains
its base color. `Object::new` uses the standard lit material;
`Object::with_material` accepts explicit material properties. `Material::unlit()`
preserves the original flat-color behavior. For example, inside a scene builder:

```rust
scene.objects.push(Object::with_material(
    Box::new(Sphere::new(Point::new(0.0, 1.0, 0.0), 1.0)),
    Color::new(255, 255, 255),
    Material::glass(1.5)
));
scene.lights.push(Light::point(
    Point::new(-3.0, 5.0, 2.0), Color::new(255, 245, 230), 50.0
));
```

Coefficients are in [0, 1], shininess is at least one, and refractive indices are
positive. `reflective` allocates a mirror fraction first; `transparency` allocates
a fraction of the remainder to glass. Fresnel splits that glass component into
reflection and transmission. Specular highlights are added separately. Use
closed shapes (spheres/boxes) for glass volumes; a transparent infinite plane is
a half-space boundary, and individual triangles do not form a closed volume.

To transform a shape, wrap it in `Transformed::new(shape, transform)` before
passing it to an object. `Transform::identity().scale(...).rotate_y(...).translate(...)`
applies operations in that written order; rotation angles are radians. Zero
scale and degenerate camera/geometry inputs are rejected.

## Tests

```sh
./scripts/cargo.sh test --locked
```

Tests exercise geometry edge cases, inverse-transpose normals, transform order,
camera orientation and focus, sRGB round trips, pattern boundaries, Phong
lighting, area-light penumbrae, shadow distance limits, multiple lights, recursive
mirrors, glass entry/exit, nested media, Snell's law, Fresnel reflection, total
internal reflection, antialiasing, and CLI validation. The original scene at
640×480 with `--samples 1` also matches the pre-extension render pixel for pixel.

Animation and gallery integration tests:

```sh
python3 scripts/test_site.py --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer"
python3 scripts/test_animation.py --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer"
python3 scripts/test_scenes.py --renderer "${RAYTRACER_TOOLING:-/workspace/.raytracer-tools}/build/target/release/raytracer"
```

Additional tests check Perlin continuity and seeding, procedural materials, bump
normals, rough-ray hemisphere constraints, OBJ parsing/smooth normals, BVH results
against brute force and triangle-test counts, volume depth/extinction/self-shadowing,
and APNG timing, checksums, frame differences, and gallery assets.
