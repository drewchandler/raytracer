#!/usr/bin/env python3
"""Render lossless, seamless cloud animations with preview and quality presets."""
import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
from contextlib import nullcontext
from datetime import datetime, timezone
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import tempfile
from threading import Event, Lock
import time
import zlib

SIGNATURE = b'\x89PNG\r\n\x1a\n'
PRESETS = {
    'preview': dict(frames=24, fps=12, width=320, height=240, samples=2,
                    light_samples=2, depth=5, cloud_steps=24),
    'quality': dict(frames=48, fps=24, width=640, height=480, samples=4,
                    light_samples=2, depth=6, cloud_steps=48),
}


def chunks(data):
    if not data.startswith(SIGNATURE):
        raise ValueError('not a PNG')
    position = 8
    while position < len(data):
        if position + 12 > len(data):
            raise ValueError('truncated PNG chunk')
        length = struct.unpack('>I', data[position:position+4])[0]
        kind = data[position+4:position+8]
        end = position + 8 + length
        payload = data[position+8:end]
        if end + 4 > len(data):
            raise ValueError('truncated PNG payload')
        crc = struct.unpack('>I', data[end:end+4])[0]
        if zlib.crc32(kind + payload) & 0xffffffff != crc:
            raise ValueError('PNG checksum mismatch')
        yield kind, payload
        position = end + 4
        if kind == b'IEND':
            if position != len(data):
                raise ValueError('trailing PNG data')
            return
    raise ValueError('missing PNG end chunk')


def chunk(kind, payload):
    return (struct.pack('>I', len(payload)) + kind + payload +
            struct.pack('>I', zlib.crc32(kind + payload) & 0xffffffff))


def read_frame(data, dimensions=None):
    parsed = list(chunks(data))
    if not parsed or parsed[0][0] != b'IHDR' or len(parsed[0][1]) != 13:
        raise ValueError('missing or invalid PNG header')
    header = parsed[0][1]
    width, height = struct.unpack('>II', header[:8])
    if header[8:] != b'\x08\x02\x00\x00\x00' or width == 0 or height == 0:
        raise ValueError('expected RGB8 noninterlaced PNG')
    if dimensions is not None and (width, height) != dimensions:
        raise ValueError('animation frame dimensions differ')
    compressed = b''.join(payload for kind, payload in parsed if kind == b'IDAT')
    if not compressed:
        raise ValueError('missing frame image data')
    return header, compressed


def assemble(frames, fps):
    if not frames or not 1 <= fps <= 60:
        raise ValueError('need frames and fps in 1..60')
    header, _ = read_frame(frames[0])
    width, height = struct.unpack('>II', header[:8])
    out = bytearray(SIGNATURE + chunk(b'IHDR', header) +
                    chunk(b'acTL', struct.pack('>II', len(frames), 0)))
    sequence = 0
    for index, data in enumerate(frames):
        frame_header, compressed = read_frame(data, (width, height))
        if frame_header != header:
            raise ValueError('animation frames must have matching format')
        control = struct.pack('>IIIIIHHBB', sequence, width, height, 0, 0, 1, fps, 0, 0)
        out.extend(chunk(b'fcTL', control))
        sequence += 1
        if index == 0:
            out.extend(chunk(b'IDAT', compressed))
        else:
            out.extend(chunk(b'fdAT', struct.pack('>I', sequence) + compressed))
            sequence += 1
    out.extend(chunk(b'IEND', b''))
    return bytes(out)


def is_quality(metadata):
    return (metadata.get('profile') == 'quality' and metadata.get('seamless') is True
            and metadata.get('width', 0) >= 640 and metadata.get('height', 0) >= 480
            and metadata.get('fps', 0) >= 24 and metadata.get('frames', 0) >= 48
            and metadata.get('samples_per_axis', 0) >= 4
            and metadata.get('light_samples_per_axis', 0) >= 2
            and metadata.get('max_depth', 0) >= 6)


def render_profile(renderer, output, preset='preview', **overrides):
    if preset not in PRESETS:
        raise ValueError('unknown animation preset')
    options = dict(PRESETS[preset])
    options.update({k: v for k, v in overrides.items() if v is not None})
    return render(renderer, output, profile=preset, **options)


def render(renderer, output, frames=24, fps=12, width=320, height=240, samples=2,
           step=None, clouds='procedural', depth=5, light_samples=2, start=0.0,
           loop_duration=None, jobs=1, cache_dir=None, cloud_steps=24, profile='custom', source_commit=None):
    renderer = Path(renderer).resolve(strict=True)
    output = Path(output).resolve()
    if output.suffix.lower() != '.png':
        raise ValueError('animated PNG output must use a .png extension')
    if not 1 <= frames <= 240 or not 1 <= fps <= 60 or not 1 <= jobs <= 16:
        raise ValueError('frames 1..240; fps 1..60; jobs 1..16')
    if not 1 <= width <= 8192 or not 1 <= height <= 8192 or not 1 <= samples <= 16:
        raise ValueError('invalid resolution or samples')
    if not 1 <= light_samples <= 64 or not 0 <= depth <= 16 or not 8 <= cloud_steps <= 128:
        raise ValueError('invalid light, bounce, or cloud sampling settings')
    if clouds not in ('procedural', 'volume'):
        raise ValueError('invalid cloud mode')
    if step is not None and loop_duration is not None:
        raise ValueError('choose linear --step or --loop-duration')
    if step is None:
        loop_duration = frames / fps if loop_duration is None else loop_duration
        if not math.isfinite(loop_duration) or loop_duration <= 0:
            raise ValueError('loop duration must be positive and finite')
        interval = loop_duration / frames
    else:
        interval = step
    times = [start + i * interval for i in range(frames)]
    if not all(math.isfinite(t) and -1000000 <= t <= 1000000 for t in times):
        raise ValueError('animation times out of range')
    binary_hash = hashlib.sha256(renderer.read_bytes()).hexdigest()
    settings = dict(frames=frames, fps=fps, width=width, height=height,
                    samples_per_axis=samples, light_samples_per_axis=light_samples,
                    max_depth=depth, cloud_steps=cloud_steps, times=times, clouds=clouds,
                    loop_duration=loop_duration, renderer_sha256=binary_hash)
    key = hashlib.sha256(json.dumps(settings, sort_keys=True).encode()).hexdigest()
    output.parent.mkdir(parents=True, exist_ok=True)
    storage = nullcontext(str(Path(cache_dir).resolve() / key)) if cache_dir else tempfile.TemporaryDirectory()
    started = time.monotonic()
    cancelled = Event()
    lock = Lock()
    completed = 0
    with storage as directory:
        root = Path(directory)
        root.mkdir(parents=True, exist_ok=True)
        (root / 'settings.json').write_text(json.dumps(settings, indent=2) + '\n')
        paths = [root / f'frame-{i:04d}.png' for i in range(frames)]

        def run_frame(index):
            nonlocal completed
            if cancelled.is_set():
                return
            path = paths[index]
            cached = False
            if path.exists():
                try:
                    read_frame(path.read_bytes(), (width, height))
                    cached = True
                except (ValueError, OSError):
                    path.unlink(missing_ok=True)
            if not cached:
                pending = path.with_name(path.stem + '.pending.png')
                command = [str(renderer), '--width', str(width), '--height', str(height),
                           '--samples', str(samples), '--depth', str(depth),
                           '--light-samples', str(light_samples), '--clouds', clouds,
                           '--cloud-steps', str(cloud_steps), '--time', str(times[index]),
                           '--output', str(pending)]
                if loop_duration is not None:
                    command += ['--cloud-loop', str(loop_duration)]
                result = subprocess.run(command, capture_output=True, text=True)
                if result.returncode:
                    raise RuntimeError(f'Frame {index} failed: {result.stderr or result.stdout}')
                read_frame(pending.read_bytes(), (width, height))
                pending.replace(path)
            with lock:
                completed += 1
                print(f'Frame {index+1}/{frames}: {"cached" if cached else "rendered"} '
                      f'({completed}/{frames} complete)', flush=True)

        with ThreadPoolExecutor(max_workers=jobs) as executor:
            futures = [executor.submit(run_frame, i) for i in range(frames)]
            try:
                for future in as_completed(futures):
                    future.result()
            except BaseException:
                cancelled.set()
                for future in futures:
                    future.cancel()
                raise
        images = [path.read_bytes() for path in paths]
        animation = assemble(images, fps)
        pending = output.with_name(output.name + '.pending')
        pending.write_bytes(animation)
        pending.replace(output)
        output.with_name(output.stem + '-still.png').write_bytes(images[0])
    metadata = dict(settings, profile=profile, seamless=loop_duration is not None,
                    duration_seconds=frames/fps, jobs=jobs,
                    render_seconds=round(time.monotonic()-started, 3),
                    source_commit=os.environ.get('GITHUB_SHA', '') if source_commit is None else source_commit,
                    generated_at=datetime.now(timezone.utc).isoformat(timespec='seconds'),
                    file_bytes=output.stat().st_size)
    output.with_suffix('.json').write_text(json.dumps(metadata, indent=2) + '\n')
    return metadata


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--renderer', required=True, type=Path)
    parser.add_argument('--output', type=Path, default=Path('renders/clouds.png'))
    parser.add_argument('--preset', choices=tuple(PRESETS), default='preview')
    for flag in ('frames', 'fps', 'width', 'height', 'samples', 'light-samples', 'depth', 'cloud-steps'):
        parser.add_argument('--' + flag, type=int)
    parser.add_argument('--step', type=float, help='explicit linear wind instead of a seamless loop')
    parser.add_argument('--loop-duration', type=float)
    parser.add_argument('--start', type=float, default=0.0)
    parser.add_argument('--clouds', choices=('procedural', 'volume'), default='procedural')
    parser.add_argument('--jobs', type=int)
    parser.add_argument('--cache-dir', type=Path, help='resume completed frames with matching settings and renderer')
    parser.add_argument('--source-commit', help='source commit for publication provenance (default: GITHUB_SHA)')
    args = parser.parse_args()
    options = vars(args).copy()
    renderer, output, preset = (options.pop(k) for k in ('renderer', 'output', 'preset'))
    if options['jobs'] is None:
        available = len(os.sched_getaffinity(0)) if hasattr(os, 'sched_getaffinity') else (os.cpu_count() or 1)
        options['jobs'] = min(4, available)
    if preset == 'quality' and options['cache_dir'] is None:
        options['cache_dir'] = Path('renders/.animation-cache')
    try:
        render_profile(renderer, output, preset, **options)
    except ValueError as error:
        parser.error(str(error))


if __name__ == '__main__':
    main()
