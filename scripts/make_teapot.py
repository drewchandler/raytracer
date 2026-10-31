#!/usr/bin/env python3
"""Tessellate the bundled MIT-licensed Utah teapot patches into a smooth OBJ."""
import argparse
import json
import math
from pathlib import Path


def generate(output, segments=8):
    data=json.loads((Path(__file__).resolve().parents[1]/'assets/teapot-patches.json').read_text())
    controls=[data['vertices'][i:i+3] for i in range(0,len(data['vertices']),3)]
    vertices=[]; lookup={}; faces=[]
    def vertex(point):
        key=tuple(round(v,7) for v in point)
        if key not in lookup:
            lookup[key]=len(vertices); vertices.append(key)
        return lookup[key]
    def basis(t):
        return ((1-t)**3,3*t*(1-t)**2,3*t*t*(1-t),t**3)
    for offset in range(0,len(data['patches']),16):
        indices=data['patches'][offset:offset+16]
        grid=[]
        for i in range(segments+1):
            row=[]; u=basis(i/segments)
            for j in range(segments+1):
                v=basis(j/segments)
                p=[sum(u[r]*v[c]*controls[indices[r*4+c]][axis] for r in range(4) for c in range(4)) for axis in range(3)]
                row.append(vertex((p[0],p[2],-p[1])))
            grid.append(row)
        for i in range(segments):
            for j in range(segments):
                a,b,c,d=grid[i][j],grid[i][j+1],grid[i+1][j+1],grid[i+1][j]
                for face in ((a,b,c),(a,c,d)):
                    if len(set(face))==3: faces.append(face)
    normals=[[0.,0.,0.] for _ in vertices]; valid=[]
    for face in faces:
        a,b,c=(vertices[i] for i in face)
        u=[b[i]-a[i] for i in range(3)]; v=[c[i]-a[i] for i in range(3)]
        n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
        if sum(x*x for x in n)<1.e-20: continue
        valid.append(face)
        for index in face:
            for axis in range(3): normals[index][axis]+=n[axis]
    lines=['# Utah teapot; generated from three.js r180, MIT; see LICENSE-threejs.txt']
    lines.extend('v '+' '.join(format(x,'.8f') for x in p) for p in vertices)
    for n in normals:
        length=math.sqrt(sum(x*x for x in n))
        n=[x/length for x in n] if length else [0.,1.,0.]
        lines.append('vn '+' '.join(format(x,'.8f') for x in n))
    lines.extend('f '+' '.join(f'{i+1}//{i+1}' for i in face) for face in valid)
    Path(output).write_text('\n'.join(lines)+'\n')
    print(f'Teapot: {len(vertices)} vertices, {len(valid)} triangles')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',default=str(Path(__file__).resolve().parents[1]/'assets/teapot.obj'))
    parser.add_argument('--segments',type=int,default=8)
    args=parser.parse_args()
    if not 2<=args.segments<=64: parser.error('--segments must be 2..64')
    generate(args.output,args.segments)
