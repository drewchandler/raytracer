#!/usr/bin/env python3
"""Exercise scene files through the actual renderer, including invalid input."""
import argparse
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
import zlib

from render_animation import SIGNATURE, chunk

ROOT = Path(__file__).resolve().parent.parent
EXAMPLES = ('cornell-box', 'glass-garden', 'marble-temple', 'mirror-maze')


def simple_scene():
    return dict(version=1, camera={'from': [0, 0, 3], 'at': [0, 0, 0]},
                render={'width': 24, 'height': 18, 'samples': 1, 'depth': 2},
                objects=[dict(type='sphere', material=dict(preset='unlit', color=[245, 35, 50]))])


class SceneTests(unittest.TestCase):
    renderer = None

    def invoke(self, root, scene, *flags):
        path = root / 'scene.json'
        path.write_text(json.dumps(scene, allow_nan=False))
        output = root / 'out.png'
        output.unlink(missing_ok=True)
        result = subprocess.run([str(self.renderer), '--scene-file', str(path), '--output', str(output), *flags],
                                cwd=root, capture_output=True, text=True)
        return result, output

    def test_all_examples_validate_and_render_from_another_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in EXAMPLES:
                with self.subTest(scene=name):
                    command = [str(self.renderer), '--scene-file', str(ROOT / 'scenes' / (name+'.json')),
                               '--width', '32', '--height', '24', '--samples', '2', '--light-samples', '1',
                               '--depth', '3', '--output', str(root / 'out.png')]
                    result = subprocess.run(command+['--validate'], cwd=root, capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn('Valid scene:', result.stdout)
                    self.assertFalse((root / 'out.png').exists())
                    result = subprocess.run(command, cwd=root, capture_output=True, text=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(struct.unpack('>II', (root / 'out.png').read_bytes()[16:24]), (32,24))
                    (root / 'out.png').unlink()

    def test_defaults_and_explicit_cli_values_take_precedence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scene = simple_scene()
            scene['render'].update(width=40, height=30, samples=3, depth=7)
            result, output = self.invoke(root, scene)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(struct.unpack('>II', output.read_bytes()[16:24]), (40,30))
            self.assertIn('9 samples/pixel, 7 bounces', result.stdout)
            result, output = self.invoke(root, scene, '--width', '16', '--height', '12', '--samples', '2', '--depth', '5')
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(struct.unpack('>II', output.read_bytes()[16:24]), (16,12))
            self.assertIn('4 samples/pixel, 5 bounces', result.stdout)

    def test_reusable_materials_and_ordered_transforms_match_explicit_geometry(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            scene = simple_scene()
            scene['objects'][0].update(center=[.3,0,-1], radius=.8)
            result, output = self.invoke(root, scene)
            self.assertEqual(result.returncode, 0, result.stderr)
            expected = output.read_bytes()
            scene['materials'] = {'red': scene['objects'][0]['material']}
            scene['objects'] = [dict(type='sphere', material='red',
                                    transform=[{'scale': [.8,.8,.8]}, {'translate': [.3,0,-1]}])]
            result, output = self.invoke(root, scene)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(output.read_bytes(), expected)
            scene['objects'][0]['color'] = [10,240,30]
            result, output = self.invoke(root, scene)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertNotEqual(output.read_bytes(), expected)

    def test_relative_mesh_and_sky_assets_are_resolved_beside_the_scene(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'triangle.obj').write_text('v -1 -1 0\nv 1 -1 0\nv 0 1 0\nf 1 2 3\n')
            header = struct.pack('>IIBBBBB',2,1,8,2,0,0,0)
            image = SIGNATURE+chunk(b'IHDR',header)+chunk(b'IDAT',zlib.compress(b'\0\xff\x00\x00\x00\x00\xff'))+chunk(b'IEND',b'')
            (root / 'sky.png').write_bytes(image)
            scene = simple_scene()
            scene['environment'] = dict(type='panorama', path='sky.png', rotation=90)
            scene['objects'] = [dict(type='mesh', path='triangle.obj', fit=1,
                                     material=dict(preset='unlit', color=[0,255,0]))]
            path = root / 'scene.json'
            path.write_text(json.dumps(scene))
            result = subprocess.run([str(self.renderer),'--scene-file',str(path),'--output',str(root/'out.png')],
                                    cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertTrue((root/'out.png').exists())

    def test_invalid_scenes_report_the_field_and_never_panic_or_render(self):
        mutations = [
            (lambda s:s.update(version=2), 'version'),
            (lambda s:s.update(typo=1), 'typo'),
            (lambda s:s['camera'].update(from_=[0,0,3]), 'from_'),
            (lambda s:s['camera'].update(at=[0,0,3]), 'camera'),
            (lambda s:s['camera'].update(up=[0,0,1]), 'camera'),
            (lambda s:s['camera'].update(up=[0,0,0]), 'up'),
            (lambda s:s['camera'].update(fov=180), 'fov'),
            (lambda s:s['render'].update(width=0), 'width'),
            (lambda s:s['render'].update(samples=1.5), 'samples'),
            (lambda s:s['objects'][0].update(radius=-1), 'radius'),
            (lambda s:s['objects'][0].update(material='missing'), 'material'),
            (lambda s:s['objects'][0]['material'].update(reflective=2), 'reflective'),
            (lambda s:s['objects'][0]['material'].update(color=[256,0,0]), 'color'),
            (lambda s:s['objects'][0]['material'].update(pattern=dict(type='marble',color_a=[0,0,0],color_b=[255,255,255],size=0)), 'size'),
            (lambda s:s['objects'][0].update(transform=[{'scale':[1,0,1]}]), 'scale'),
            (lambda s:s['objects'][0].update(transform=[{'scale':[1e200,1,1]},{'scale':[1e200,1,1]}]), 'transform'),
            (lambda s:s['objects'][0].update(transform=[{'translate':[0,1,0],'rotate_y':30}]), 'transform'),
            (lambda s:s.update(objects=[dict(type='cube',min=[1,0,0],max=[0,1,1])]), 'objects[0]'),
            (lambda s:s.update(objects=[dict(type='triangle',vertices=[[0,0,0],[1,0,0],[2,0,0]])]), 'objects[0]'),
            (lambda s:s.update(objects=[dict(type='mesh',path='missing.obj')]), 'objects[0].path'),
            (lambda s:s.update(lights=[dict(type='directional',direction=[0,0,0])]), 'direction'),
            (lambda s:s.update(lights=[dict(type='area',corner=[0,1,0],u=[1,0,0],v=[2,0,0])]), 'lights[0]'),
            (lambda s:s.update(haze=dict(start=10,end=2)), 'haze'),
            (lambda s:s.update(environment=dict(type='volume',steps=0)), 'steps'),
            (lambda s:s.update(environment=dict(type='clouds',seed=-1)), 'seed'),
        ]
        with tempfile.TemporaryDirectory() as directory:
            for mutate,field in mutations:
                with self.subTest(field=field):
                    scene = simple_scene();mutate(scene)
                    result,output = self.invoke(Path(directory),scene,'--validate')
                    self.assertNotEqual(result.returncode,0)
                    self.assertIn(field,result.stderr)
                    self.assertIn('scene.json',result.stderr)
                    self.assertNotIn('panicked',result.stderr)
                    self.assertFalse(output.exists())

    def test_lens_validation_uses_the_combined_file_and_cli_settings(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);scene=simple_scene()
            scene['camera']['aperture']=.02
            result,output=self.invoke(root,scene,'--validate')
            self.assertNotEqual(result.returncode,0)
            self.assertIn('Depth of field',result.stderr)
            result,output=self.invoke(root,scene,'--samples','2','--validate')
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertFalse(output.exists())
            result,output=self.invoke(root,scene,'--aperture','0','--validate')
            self.assertEqual(result.returncode,0,result.stderr)

    def test_conflicting_or_missing_cli_arguments_fail(self):
        for flags in [ ['--validate'], ['--scene-file'], ['--scene-file','test.json','--scene','demo'],
                       ['--scene-file','test.json','--obj','test.obj'] ]:
            result=subprocess.run([str(self.renderer),*flags],capture_output=True,text=True)
            self.assertNotEqual(result.returncode,0)
            self.assertNotIn('panicked',result.stderr)

    def test_invalid_json_reports_the_file_and_does_not_create_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/'broken.json';output=root/'out.png'
            for text in ('{"version":1,', '{"version":NaN}', '[]'):
                path.write_text(text)
                result=subprocess.run([str(self.renderer),'--scene-file',str(path),'--output',str(output)],
                                      capture_output=True,text=True)
                self.assertNotEqual(result.returncode,0)
                self.assertIn('broken.json',result.stderr)
                self.assertNotIn('panicked',result.stderr)
                self.assertFalse(output.exists())

    def test_validation_does_not_create_output_directories_but_rendering_does(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);path=root/'scene.json';path.write_text(json.dumps(simple_scene()))
            output=root/'new'/'renders'/'image.png'
            command=[str(self.renderer),'--scene-file',str(path),'--output',str(output)]
            result=subprocess.run(command+['--validate'],capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertFalse(output.parent.exists())
            result=subprocess.run(command,capture_output=True,text=True)
            self.assertEqual(result.returncode,0,result.stderr)
            self.assertTrue(output.is_file())


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--renderer',required=True,type=Path)
    args,remaining=parser.parse_known_args()
    SceneTests.renderer=args.renderer.resolve(strict=True)
    unittest.main(argv=[__file__]+remaining)
