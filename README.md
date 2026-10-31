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
spheres, smaller glossy red, blue, and gold spheres, and a reflective checkerboard
floor beneath a procedural cloudy sky. A low, centered camera emphasizes the reflections. It
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

The default sky uses seeded fractal noise, a daylight gradient, and shaded cloud
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
reflection/refraction bounces. Samples are deterministic, so repeated renders
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
  --renderer "$RAYTRACER_TOOLING/build/target/release/raytracer" --output _site
python3 scripts/test_site.py \
  --renderer "$RAYTRACER_TOOLING/build/target/release/raytracer"
```

Open `_site/index.html` locally, or use `python3 -m http.server --directory _site`
for a local HTTP check. The generator uses Python 3.9+ with no third-party
packages. Its `--width`, `--height`, `--samples`, `--light-samples`, and `--depth`
options control the rendered image. `GITHUB_SHA` and `GITHUB_REPOSITORY` supply
the source link automatically on Actions; local runs show “Local render”.
The generated site is ignored by Git.

## Features

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
path-traced indirect illumination, caustics, volumetrics, image textures on object surfaces,
mesh loading, or spatial acceleration. Intersection search is linear in object
count, appropriate for the small procedural scenes here. Glass shadows use
straight filtered rays rather than solving refracted light paths; tint is
applied at each interface rather than using volume absorption.

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
