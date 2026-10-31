#!/usr/bin/env python3
import argparse
from pathlib import Path
import struct
import tempfile
import unittest
import zlib
from render_animation import assemble,chunk,chunks,render,SIGNATURE


def pixel(color):
    header=struct.pack('>IIBBBBB',1,1,8,2,0,0,0)
    return SIGNATURE+chunk(b'IHDR',header)+chunk(b'IDAT',zlib.compress(b'\0'+bytes(color)))+chunk(b'IEND',b'')


class AnimationTests(unittest.TestCase):
    renderer=None
    def test_apng_retains_distinct_frames_and_timing(self):
        red=pixel((255,0,0));blue=pixel((0,0,255))
        result=list(chunks(assemble([red,blue],6)))
        self.assertIn((b'acTL',struct.pack('>II',2,0)),result)
        controls=[p for k,p in result if k==b'fcTL']
        self.assertEqual([struct.unpack('>IIIIIHHBB',p)[6] for p in controls],[6,6])
        sequence=[struct.unpack('>I',p[:4])[0] for k,p in result if k in (b'fcTL',b'fdAT')]
        self.assertEqual(sequence,list(range(len(sequence))))
        data=next(p[4:] for k,p in result if k==b'fdAT')
        self.assertEqual(zlib.decompress(data),b'\0\x00\x00\xff')
    def test_bad_checksums_and_mixed_dimensions_fail(self):
        data=bytearray(pixel((1,2,3)));data[40]^=1
        with self.assertRaises(ValueError): list(chunks(bytes(data)))
        other=SIGNATURE+chunk(b'IHDR',struct.pack('>IIBBBBB',2,1,8,2,0,0,0))+chunk(b'IDAT',b'bad')+chunk(b'IEND',b'')
        with self.assertRaises(ValueError): assemble([pixel((1,2,3)),other],6)
    def test_actual_renderer_produces_changing_cloud_frames(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'clouds.png'
            metadata=render(self.renderer,output,2,6,32,24,1,12.0,depth=1,light_samples=1)
            parsed=list(chunks(output.read_bytes()))
            self.assertEqual(metadata['frames'],2)
            first=b''.join(p for k,p in parsed if k==b'IDAT')
            second=b''.join(p[4:] for k,p in parsed if k==b'fdAT')
            self.assertNotEqual(first,second)
            self.assertTrue(output.with_name('clouds-still.png').exists())


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--renderer',required=True,type=Path)
    args,remaining=p.parse_known_args();AnimationTests.renderer=args.renderer.resolve(strict=True)
    unittest.main(argv=[__file__]+remaining)
