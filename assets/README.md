# Utah teapot

`teapot.obj` is a smooth, Y-up tessellation of Martin Newell's Utah teapot.
The 32 Bezier patches in `teapot-patches.json` come from three.js r180:
https://github.com/mrdoob/three.js/blob/r180/examples/jsm/geometries/TeapotGeometry.js

The patch data is distributed under the three.js MIT license, reproduced in
`LICENSE-threejs.txt`. This asset is bundled so rendering needs no network access.
Reproduce it with `python3 scripts/make_teapot.py`. The generator welds patch
seams, skips degenerate cusp triangles, and exports averaged vertex normals.
