#!/usr/bin/env python3
"""Validate a quality animation and attach it to the gallery from the same commit."""
import argparse
import json
from pathlib import Path
import shutil
import struct

from build_site import write_page
from render_animation import chunks, is_quality, read_frame


def validate(animation, expected_commit=None, require_quality=False):
    animation = Path(animation)
    metadata = json.loads(animation.with_suffix('.json').read_text())
    if expected_commit is not None and metadata.get('source_commit') != expected_commit:
        raise ValueError('animation source commit does not match the requested commit')
    if require_quality and not is_quality(metadata):
        raise ValueError('animation does not meet the quality preset requirements')
    parsed = list(chunks(animation.read_bytes()))
    if not parsed or parsed[0][0] != b'IHDR':
        raise ValueError('animation has no PNG header')
    dimensions = struct.unpack('>II', parsed[0][1][:8])
    if dimensions != (metadata['width'], metadata['height']):
        raise ValueError('animation dimensions disagree with metadata')
    controls = [p for k, p in parsed if k == b'fcTL']
    count = [p for k, p in parsed if k == b'acTL']
    if len(count) != 1 or struct.unpack('>II', count[0])[0] != metadata['frames'] or len(controls) != metadata['frames']:
        raise ValueError('animation frame count disagrees with metadata')
    sequence = [struct.unpack('>I', p[:4])[0] for k, p in parsed if k in (b'fcTL', b'fdAT')]
    if sequence != list(range(len(sequence))):
        raise ValueError('invalid animation frame sequence')
    for control in controls:
        _, w, h, x, y, numerator, denominator, _, _ = struct.unpack('>IIIIIHHBB', control)
        if (w, h) != dimensions or x != 0 or y != 0 or numerator != 1 or denominator != metadata['fps']:
            raise ValueError('animation frame timing or dimensions disagree with metadata')
    poster = animation.with_name(animation.stem + '-still.png')
    read_frame(poster.read_bytes(), dimensions)
    return metadata


def attach(site, animation, expected_commit=None, require_quality=False):
    site = Path(site)
    animation = Path(animation)
    gallery = json.loads((site / 'render.json').read_text())
    metadata = validate(animation, expected_commit, require_quality)
    if gallery.get('commit', '') != metadata.get('source_commit', ''):
        raise ValueError('gallery and animation were rendered from different commits')
    for source, name in [(animation, 'clouds.png'),
                         (animation.with_name(animation.stem + '-still.png'), 'clouds-still.png'),
                         (animation.with_suffix('.json'), 'clouds.json')]:
        target = site / name
        if source.resolve() != target.resolve():
            pending = target.with_name(target.name + '.pending')
            shutil.copyfile(source, pending)
            pending.replace(target)
    item = next((r for r in gallery.get('additional_renders', []) if r.get('image') == 'clouds.png'), None)
    if item is None:
        item = dict(image='clouds.png')
        gallery.setdefault('additional_renders', []).append(item)
    item.update(width=metadata['width'], height=metadata['height'], animation=metadata,
                title='Clouds in motion', description='A seamless sky loop, reflected and refracted in the spheres.')
    write_page(site, gallery)
    return metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--site', type=Path)
    parser.add_argument('--animation', required=True, type=Path)
    parser.add_argument('--expected-commit')
    parser.add_argument('--require-quality', action='store_true')
    parser.add_argument('--check-only', action='store_true')
    args = parser.parse_args()
    try:
        if args.check_only:
            result = validate(args.animation, args.expected_commit, args.require_quality)
        else:
            if args.site is None:
                parser.error('--site is required unless using --check-only')
            result = attach(args.site, args.animation, args.expected_commit, args.require_quality)
        print('{}×{}, {} FPS, {} frames, {} samples/pixel, seamless={}'.format(
            result['width'], result['height'], result['fps'], result['frames'],
            result['samples_per_axis'] ** 2, result['seamless']))
    except (ValueError, OSError) as error:
        parser.error(str(error))


if __name__ == '__main__':
    main()
