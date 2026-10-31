#!/usr/bin/env python3
"""Render cloud motion to an animated PNG, without third-party Python packages."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import zlib

SIGNATURE=b'\x89PNG\r\n\x1a\n'


def chunks(data):
    if not data.startswith(SIGNATURE): raise ValueError('not a PNG')
    position=8
    while position<len(data):
        if position+12>len(data): raise ValueError('truncated PNG chunk')
        length=struct.unpack('>I',data[position:position+4])[0]
        kind=data[position+4:position+8]
        end=position+8+length
        payload=data[position+8:end]
        if end+4>len(data): raise ValueError('truncated PNG payload')
        crc=struct.unpack('>I',data[end:end+4])[0]
        if zlib.crc32(kind+payload)&0xffffffff!=crc: raise ValueError('PNG checksum mismatch')
        yield kind,payload
        position=end+4
        if kind==b'IEND':
            if position!=len(data): raise ValueError('trailing PNG data')
            return
    raise ValueError('missing PNG end chunk')


def chunk(kind,payload):
    return struct.pack('>I',len(payload))+kind+payload+struct.pack('>I',zlib.crc32(kind+payload)&0xffffffff)


def assemble(frames,fps):
    if not frames or not 1<=fps<=60: raise ValueError('need frames and fps in 1..60')
    parsed=[list(chunks(data)) for data in frames]
    header=parsed[0][0][1]
    width,height=struct.unpack('>II',header[:8])
    # The renderer emits RGB8 noninterlaced PNGs. Do not silently combine formats.
    if len(header)!=13 or header[8:]!=b'\x08\x02\x00\x00\x00': raise ValueError('expected RGB8 noninterlaced PNG')
    for frame in parsed:
        if frame[0]!=(b'IHDR',header) or not any(k==b'IDAT' for k,_ in frame):
            raise ValueError('animation frames must have matching dimensions and format')
    out=bytearray(SIGNATURE+chunk(b'IHDR',header)+chunk(b'acTL',struct.pack('>II',len(frames),0)))
    sequence=0
    for index,frame in enumerate(parsed):
        control=struct.pack('>IIIIIHHBB',sequence,width,height,0,0,1,fps,0,0)
        out.extend(chunk(b'fcTL',control)); sequence+=1
        for kind,payload in frame:
            if kind!=b'IDAT': continue
            if index==0: out.extend(chunk(b'IDAT',payload))
            else:
                out.extend(chunk(b'fdAT',struct.pack('>I',sequence)+payload)); sequence+=1
    out.extend(chunk(b'IEND',b''))
    return bytes(out)


def render(renderer,output,frames=12,fps=6,width=320,height=240,samples=2,
           step=2.0,clouds='procedural',depth=4,light_samples=2,start=0.0):
    renderer=Path(renderer).resolve(strict=True); output=Path(output).resolve()
    if output.suffix.lower()!='.png': raise ValueError('animated PNG output must use a .png extension')
    if not 1<=frames<=240 or not 1<=fps<=60: raise ValueError('frames 1..240; fps 1..60')
    if not 1<=width<=8192 or not 1<=height<=8192 or not 1<=samples<=16: raise ValueError('invalid resolution or samples')
    if clouds not in ('procedural','volume'): raise ValueError('invalid cloud mode')
    times=[start+i*step for i in range(frames)]
    if not all(-1000000<=t<=1000000 for t in times): raise ValueError('animation times out of range')
    output.parent.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory() as directory:
        images=[]
        for index,t in enumerate(times):
            path=Path(directory)/f'frame-{index:04d}.png'
            subprocess.run([str(renderer),'--width',str(width),'--height',str(height),
                '--samples',str(samples),'--depth',str(depth),'--light-samples',str(light_samples),
                '--clouds',clouds,'--cloud-steps','16','--time',str(t),'--output',str(path)],check=True)
            images.append(path.read_bytes())
        animation=assemble(images,fps)
        pending=output.with_name(output.name+'.pending')
        pending.write_bytes(animation); pending.replace(output)
        output.with_name(output.stem+'-still.png').write_bytes(images[0])
    metadata=dict(frames=frames,fps=fps,width=width,height=height,times=times,clouds=clouds)
    output.with_suffix('.json').write_text(json.dumps(metadata,indent=2)+'\n')
    return metadata


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--renderer',required=True,type=Path)
    parser.add_argument('--output',type=Path,default=Path('renders/clouds.png'))
    parser.add_argument('--frames',type=int,default=12)
    parser.add_argument('--fps',type=int,default=6)
    parser.add_argument('--width',type=int,default=320)
    parser.add_argument('--height',type=int,default=240)
    parser.add_argument('--samples',type=int,default=2)
    parser.add_argument('--step',type=float,default=2.0,help='seconds of cloud motion between frames')
    parser.add_argument('--start',type=float,default=0.0)
    parser.add_argument('--depth',type=int,default=4)
    parser.add_argument('--clouds',choices=('procedural','volume'),default='procedural')
    args=parser.parse_args()
    try:
        render(args.renderer,args.output,args.frames,args.fps,args.width,args.height,args.samples,args.step,args.clouds,args.depth,start=args.start)
    except ValueError as error: parser.error(str(error))
