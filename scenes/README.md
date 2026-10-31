# Editable scenes

The JSON scene format constructs the same `Camera`, `Scene`, `Object`, `Material`,
`Light`, and `Shape` types used by the built-in demos. You can change a scene
without editing or recompiling Rust.

```sh
./scripts/cargo.sh run --release --locked -- \
  --scene-file scenes/cornell-box.json --output renders/cornell.png

# Check the complete scene and load its assets without rendering pixels.
./scripts/cargo.sh run --release --locked -- \
  --scene-file scenes/marble-temple.json --validate

# Override quality and resolution for a quick composition check.
./scripts/cargo.sh run --release --locked -- \
  --scene-file scenes/glass-garden.json --width 320 --height 240 \
  --samples 2 --light-samples 2 --depth 5 --output renders/garden.png
```

| Scene | What to explore |
| --- | --- |
| [Cornell box](cornell-box.json) | Red/green walls, soft shadows, rotated blocks, glass and copper |
| [Glass garden](glass-garden.json) | Nested optical media, tinted glass, frosted transmission, depth of field |
| [Marble temple](marble-temple.json) | Perlin marble, reusable column materials, transformed OBJ teapot |
| [Mirror maze](mirror-maze.json) | Repeated mirror reflections, colored lights, glass and metal |

All four scenes appear in the generated gallery with a **Scene JSON** link. The
gallery artifact includes their JSON, schema, OBJ, and mesh license, so their
relative asset paths also work after downloading the gallery.

## Small complete example

```json
{
  "$schema": "./scene.schema.json",
  "version": 1,
  "title": "A glass sphere",
  "description": "Glass above a checkerboard under a cloudy sky.",
  "render": { "width": 640, "height": 480, "samples": 4, "depth": 7 },
  "camera": { "from": [0, 2, 6], "at": [0, 1, 0], "fov": 50 },
  "environment": { "type": "clouds", "seed": 17 },
  "materials": {
    "glass": { "preset": "glass" },
    "floor": {
      "reflective": 0.15,
      "pattern": {
        "type": "checkers",
        "color_a": [230, 230, 220], "color_b": [30, 40, 55], "size": 1
      }
    }
  },
  "lights": [
    { "type": "area", "corner": [-3, 6, 2],
      "u": [2, 0, 0], "v": [0, 0, 2], "intensity": 100 }
  ],
  "objects": [
    { "type": "plane", "material": "floor" },
    { "type": "sphere", "center": [0, 1, 0], "radius": 1, "material": "glass" }
  ]
}
```

The optional `$schema` field enables autocomplete and validation in editors
that support JSON Schema. Keep [scene.schema.json](scene.schema.json) beside the
file, or update that link when you move it. Runtime validation also catches
unknown keys, missing material names, degenerate geometry, and missing assets.
Errors include the scene filename and field path, such as
`scene.json.objects[2].transform[0].scale`.

`version`, `camera`, and `objects` are required. `version` is currently `1`.
An empty object array is useful for rendering a sky. Everything else is optional.
Colors are three **sRGB integers from 0 to 255**. Positions and directions are
three finite numbers. Angles are in degrees; distances use your scene's units.

## Render and camera settings

| Field | Default / range |
| --- | --- |
| `render.width`, `render.height` | 640×480; each 1–8192 |
| `render.samples` | 2 per axis; 1–16; total camera samples are N² |
| `render.light_samples` | 3 per axis; 1–64; applies to every area light |
| `render.depth` | 5 reflection/refraction bounces; 0–16 |
| `camera.from`, `camera.at` | Required eye position and target; must differ |
| `camera.up` | `[0, 1, 0]`; must be nonzero and not parallel to the view |
| `camera.fov` | Horizontal field of view, 50 degrees; 1–179 |
| `camera.aperture` | 0 (pinhole); positive values enable depth of field |
| `camera.focus` | Distance from `from` to `at`; must be positive |

Explicit CLI flags override file values, including an explicit value equal to a
built-in default. `--aperture 0` disables a file's depth of field; otherwise a
nonzero aperture requires at least two camera samples per axis. The output path
is selected with `--output`. `--scene-file` cannot be combined with `--scene` or
the built-in teapot scene's `--obj` flag.

## Materials

`materials` maps reusable names to material objects. An object's `material` can
be a name or an inline object. Its optional `color` overrides the material's base
color; procedural patterns supply their own colors.

Presets are `default`, `glass` (index 1.5), `mirror` (reflectivity 0.95), and
`unlit`. Individual properties override the preset:

| Property | Meaning |
| --- | --- |
| `color` | Base sRGB color, default white |
| `ambient`, `diffuse`, `specular` | Phong coefficients, each 0–1 |
| `shininess` | Highlight exponent, at least 1 |
| `reflective`, `transparency` | Mirror and transmission fractions, each 0–1 |
| `refractive_index` | Positive optical index; 1 is air |
| `roughness`, `transmission_roughness` | Rough reflection/transmission, each 0–1 |
| `pattern` | `type`, `color_a`, `color_b`, positive `size` |
| `bump` | Positive `scale`, nonnegative `strength`, optional unsigned `seed` |

Pattern types are `checkers` (x/z floor tiles), `checkers3d`, `stripes`, `marble`,
`wood`, and `stone`. Patterns and bumps follow the object's local coordinates,
including transforms. Use nested glass spheres with an inner index of 1 to
describe a hollow glass shell; the renderer tracks the medium on entry and exit.

## Objects and transforms

Every object has a `type`, optional `name`, `material`, `color`, and `transform`.

| Type | Geometry |
| --- | --- |
| `sphere` | `center` (default origin), positive `radius` (default 1) |
| `plane` | `point` (default origin), nonzero `normal` (default up) |
| `cube` | `min`/`max` corners, default `[-1,-1,-1]` / `[1,1,1]` |
| `triangle` | Exactly three non-collinear `vertices` |
| `mesh` | Required OBJ `path`, optional positive `fit` size |

OBJ and panorama paths resolve relative to the **scene file's directory**, so
changing your working directory does not break a scene. Absolute asset paths
also work. An OBJ `fit` centers the x/z bounds, puts its bottom at y=0, and scales
its longest dimension to the requested size before applying object transforms.

Transforms are an array, applied in reading order. Each entry contains one
operation; rotations use the right-hand rule. Negative scale is supported;
zero scale is rejected. Normals use the existing inverse-transpose transform.

```json
"transform": [
  { "scale": [0.5, 2, 0.5] },
  { "rotate_y": 30 },
  { "translate": [3, 2, -1] }
]
```

Available operations are `scale`, `translate`, `rotate_x`, `rotate_y`, and
`rotate_z`. A sphere can become an ellipsoid; a cube can become a column or wall.

## Lights, sky, and haze

Lights have optional sRGB `color` (white) and nonnegative `intensity` (1).
`point` lights require `position`; `directional` lights require a nonzero
`direction` describing where light travels; rectangular `area` lights require
`corner` and nonparallel edge vectors `u` and `v`. Point/area lights have
inverse-square falloff, so their intensity usually needs to be tens or hundreds.

The root `background` color defaults to black. `ambient` controls the scene's
ambient color (default white). An optional `environment` selects a sky:

```json
"environment": { "type": "clouds", "seed": 17, "time": 0, "loop_duration": 2 }
```

Other sky types are `volume` (optional integration `steps`, 8–128) and `panorama`
(required PNG/JPEG `path`). Sky `rotation` is in degrees. The CLI's `--sky`,
`--clouds`, `--cloud-steps`, `--time`, `--cloud-loop`, and `--sky-rotation` override
the corresponding file choices. Without an environment, background misses use
the solid background color.

`"haze": { "start": 25, "end": 110 }` fades distant surfaces into the sky; both
distances must be finite, `start` nonnegative, and `end` greater than `start`.
The Cornell box demonstrates direct illumination and soft shadows. The current
Whitted renderer does not add diffuse color bleeding or caustics.

## Animation and gallery

```sh
python3 scripts/render_animation.py \
  --renderer /workspace/.raytracer-tools/build/target/release/raytracer \
  --scene-file scenes/glass-garden.json --preset quality \
  --output renders/garden-animation.png

python3 scripts/build_site.py \
  --renderer /workspace/.raytracer-tools/build/target/release/raytracer \
  --scene-file scenes/marble-temple.json --extras --output _site
```

Animation presets and gallery quality flags override file render defaults. The
animation keeps the file's camera, geometry, materials, and environment; an
explicit `--clouds` changes its sky mode. Animation time and loop length come
from the exporter. Its resumable cache includes hashes of the scene and every
referenced OBJ/panorama, in addition to renderer bytes and render settings.
