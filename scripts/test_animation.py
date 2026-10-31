#!/usr/bin/env python3
import argparse
from pathlib import Path
import struct
import tempfile
import unittest
import zlib
from render_animation import assemble,chunk,chunks,render,SIGNATURE
from render_animation import PRESETS, is_quality, render_profile
from unittest.mock import patch
import subprocess
import json


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

    def test_quality_preset_and_loop_timing(self):
        preset=PRESETS['quality']
        self.assertEqual((preset['width'],preset['height'],preset['fps'],preset['frames']),(640,480,24,48))
        self.assertEqual((preset['samples'],preset['light_samples'],preset['depth']),(4,2,6))
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'loop.png'
            metadata=render_profile(self.renderer,output,'preview',width=16,height=12,frames=4,fps=4,
                                    samples=1,depth=1,light_samples=1,jobs=2)
            self.assertEqual(metadata['times'],[0.0,0.25,0.5,0.75])
            self.assertEqual(metadata['loop_duration'],1.0)
            self.assertTrue(metadata['seamless'])
            self.assertFalse(is_quality(metadata))

    def test_real_loop_endpoint_matches_for_procedural_and_volume(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            for mode in ('procedural','volume'):
                for i,t in enumerate((0.0,2.0)):
                    subprocess.run([str(self.renderer),'--width','24','--height','18','--samples','1',
                        '--light-samples','1','--depth','2','--clouds',mode,'--cloud-steps','8',
                        '--cloud-loop','2','--time',str(t),'--output',str(root/f'{i}.png')],check=True,capture_output=True)
                self.assertEqual((root/'0.png').read_bytes(),(root/'1.png').read_bytes())

    def test_cache_resume_and_invalidation(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);binary=root/'renderer';binary.write_bytes(b'one')
            output=root/'out.png';cache=root/'cache'
            def frame(command,**kwargs):
                Path(command[command.index('--output')+1]).write_bytes(pixel((255,0,0)))
                return subprocess.CompletedProcess(command,0,'','')
            options=dict(frames=2,fps=24,width=1,height=1,samples=4,cache_dir=cache)
            with patch('render_animation.subprocess.run',side_effect=frame) as run:
                render(binary,output,**options);self.assertEqual(run.call_count,2)
            with patch('render_animation.subprocess.run') as run:
                render(binary,output,**options);run.assert_not_called()
            cached=next(cache.glob('*/frame-0000.png'));cached.write_bytes(b'corrupt')
            with patch('render_animation.subprocess.run',side_effect=frame) as run:
                render(binary,output,**options);self.assertEqual(run.call_count,1)
            binary.write_bytes(b'two')
            with patch('render_animation.subprocess.run',side_effect=frame) as run:
                render(binary,output,**options);self.assertEqual(run.call_count,2)
            with patch('render_animation.subprocess.run',side_effect=frame) as run:
                render(binary,output,**dict(options,samples=3));self.assertEqual(run.call_count,2)

    def test_failed_render_does_not_replace_a_previous_animation(self):
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/'old.png';output.write_bytes(b'keep')
            with patch('render_animation.subprocess.run',return_value=subprocess.CompletedProcess([],1,'','failure')):
                with self.assertRaises(RuntimeError):
                    render(self.renderer,output,frames=2,width=1,height=1,jobs=1)
            self.assertEqual(output.read_bytes(),b'keep')

    def test_scene_animation_keeps_the_environment_and_supports_overrides(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);scene=root/'scene.json'
            scene.write_text(json.dumps(dict(version=1,camera={'from':[0,0,3],'at':[0,1,0]},
                                            environment={'type':'volume','seed':31},objects=[])))
            options=dict(frames=2,fps=2,width=24,height=18,samples=1,depth=1,light_samples=1,
                         cloud_steps=8,scene_file=scene)
            result=render(self.renderer,root/'volume.png',**options)
            self.assertIn('scene',result)
            render(self.renderer,root/'sky.png',clouds='procedural',**options)
            self.assertNotEqual((root/'volume-still.png').read_bytes(),(root/'sky-still.png').read_bytes())

    def test_scene_cache_tracks_scene_mesh_and_panorama_contents(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);binary=root/'renderer';binary.write_bytes(b'renderer')
            scene=root/'scene.json';mesh=root/'model.obj';sky=root/'sky.png'
            mesh.write_text('v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n');sky.write_bytes(pixel((20,30,40)))
            document=dict(version=1,camera={'from':[0,0,3],'at':[0,0,0]},
                          environment={'type':'panorama','path':'sky.png'},objects=[{'type':'mesh','path':'model.obj'}])
            scene.write_text(json.dumps(document))
            def frame(command,**kwargs):
                self.assertIn(str(scene),command)
                self.assertNotIn('--clouds',command)
                Path(command[command.index('--output')+1]).write_bytes(pixel((255,0,0)))
                return subprocess.CompletedProcess(command,0,'','')
            options=dict(frames=2,width=1,height=1,scene_file=scene,cache_dir=root/'cache')
            def run(expected):
                with patch('render_animation.subprocess.run',side_effect=frame) as call:
                    render(binary,root/'out.png',**options)
                    self.assertEqual(call.call_count,expected)
            run(2);run(0)
            document['title']='Edited scene';scene.write_text(json.dumps(document));run(2)
            mesh.write_text(mesh.read_text().replace('v 1 0 0','v 2 0 0'));run(2)
            sky.write_bytes(pixel((70,80,90)));run(2)

    def test_publication_checks_source_and_actual_timing(self):
        from attach_animation import attach,validate
        from build_site import generate
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);site=root/'site';movie=root/'quality'/'clouds.png'
            sha='1'*40
            generate(self.renderer,site,16,12,1,1,1,sha,'drewchandler/raytracer')
            render_profile(self.renderer,movie,'quality',frames=4,fps=24,width=16,height=12,
                           samples=1,depth=1,light_samples=1,source_commit=sha)
            self.assertEqual(validate(movie,sha)['fps'],24)
            with self.assertRaises(ValueError): validate(movie,'2'*40)
            with self.assertRaises(ValueError): validate(movie,sha,require_quality=True)
            attach(site,movie,sha)
            self.assertIn('Quick preview',(site/'index.html').read_text())
            self.assertIn('Play animation',(site/'index.html').read_text())
            self.assertIn('max-width: 100%',(site/'index.html').read_text())
            metadata=json.loads(movie.with_suffix('.json').read_text())
            metadata['fps']=12
            movie.with_suffix('.json').write_text(json.dumps(metadata))
            with self.assertRaises(ValueError): validate(movie,sha)
            metadata['fps']=24
            metadata['source_commit']='2'*40
            movie.with_suffix('.json').write_text(json.dumps(metadata))
            old=(site/'clouds.png').read_bytes()
            with self.assertRaises(ValueError): attach(site,movie)
            self.assertEqual((site/'clouds.png').read_bytes(),old)


if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--renderer',required=True,type=Path)
    args,remaining=p.parse_known_args();AnimationTests.renderer=args.renderer.resolve(strict=True)
    unittest.main(argv=[__file__]+remaining)
