#!/usr/bin/env python3
"""Render a real scene and produce a static gallery using only Python's stdlib."""
import argparse
from datetime import datetime, timezone
import html
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time
from render_animation import render_profile, is_quality

ROOT = Path(__file__).resolve().parent.parent
GALLERY_SCENES = ('cornell-box', 'glass-garden', 'marble-temple', 'mirror-maze')


def bounded_int(low, high):
    def parse(value):
        number = int(value)
        if not low <= number <= high:
            raise argparse.ArgumentTypeError("must be between {} and {}".format(low, high))
        return number
    return parse


def generate(renderer, output, width=800, height=600, samples=4, light_samples=4,
             depth=6, commit="", repository="", extras=False, scene_file=None):
    renderer = Path(renderer).resolve(strict=True)
    output = Path(output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    # Use a temporary output name, so a failed render cannot pass with an old image.
    pending = output / "scene.pending.png"
    pending.unlink(missing_ok=True)
    source = ['--scene-file', str(Path(scene_file).resolve(strict=True))] if scene_file else ['--scene', 'demo']
    command = [str(renderer)] + source + ["--width", str(width),
               "--height", str(height), "--samples", str(samples),
               "--light-samples", str(light_samples), "--depth", str(depth),
               "--output", str(pending)]
    started = time.monotonic()
    subprocess.run(command, check=True)
    elapsed = time.monotonic() - started
    # Verify this invocation actually produced a PNG with the requested dimensions.
    with pending.open("rb") as image:
        header = image.read(24)
    if (len(header) != 24 or header[:8] != b"\x89PNG\r\n\x1a\n" or
            header[12:16] != b"IHDR" or struct.unpack(">II", header[16:24]) != (width, height)):
        raise ValueError("renderer did not produce a PNG with the requested dimensions")
    pending.replace(output / "scene.png")

    additional = []
    if extras:
        def dimensions(max_width, max_height):
            scale = min(1.0, max_width / width, max_height / height)
            return max(1, round(width * scale)), max(1, round(height * scale))
        for filename, scene, clouds, limits, title, description in [
            ("teapot.png", "teapot", "procedural", (480, 360), "Utah teapot", "A smooth copper teapot, imported as a triangle mesh."),
            ("volumetric.png", "classic", "volume", (320, 240), "Volumetric clouds", "Sunlight scatters through a cloud layer with depth and self-shadowing.")]:
            w, h = dimensions(*limits)
            pending_extra = output / (filename + ".pending.png")
            pending_extra.unlink(missing_ok=True)
            subprocess.run([str(renderer), "--scene", scene, "--clouds", clouds,
                "--cloud-steps", "32", "--width", str(w), "--height", str(h),
                "--samples", "2", "--light-samples", "2",
                "--depth", "4", "--output", str(pending_extra)], check=True)
            with pending_extra.open("rb") as image:
                extra_header = image.read(24)
            if extra_header[:8] != b"\x89PNG\r\n\x1a\n" or struct.unpack(">II", extra_header[16:24]) != (w, h):
                raise ValueError("invalid gallery render: " + filename)
            pending_extra.replace(output / filename)
            additional.append(dict(image=filename, width=w, height=h, title=title, description=description))
        w, h = dimensions(320, 240)
        animation = render_profile(renderer, output / "clouds.png", 'preview', width=w, height=h,
                                   jobs=2, source_commit=commit)
        additional.append(dict(image="clouds.png", width=w, height=h, title="Clouds in motion",
            description="A seamless sky loop, reflected and refracted in the spheres.", animation=animation))
        # Ship the editable examples and their bundled assets with the gallery.
        scene_output = output / 'scenes'
        scene_output.mkdir(exist_ok=True)
        (output / 'assets').mkdir(exist_ok=True)
        for filename in ('teapot.obj', 'LICENSE-threejs.txt'):
            asset = ROOT / 'assets' / filename
            shutil.copyfile(asset, output / 'assets' / filename)
        shutil.copyfile(ROOT / 'scenes' / 'scene.schema.json', scene_output / 'scene.schema.json')
        for name in GALLERY_SCENES:
            path = ROOT / 'scenes' / (name + '.json')
            info = json.loads(path.read_text())
            shutil.copyfile(path, scene_output / path.name)
            w, h = dimensions(400, 300)
            filename = name + '.png'
            pending_extra = output / (filename + '.pending.png')
            pending_extra.unlink(missing_ok=True)
            subprocess.run([str(renderer), '--scene-file', str(path), '--width', str(w), '--height', str(h),
                            '--samples', '2', '--light-samples', '2', '--depth', '6', '--aperture', '0',
                            '--output', str(pending_extra)], check=True)
            with pending_extra.open('rb') as image:
                header = image.read(24)
            if header[:8] != b'\x89PNG\r\n\x1a\n' or struct.unpack('>II', header[16:24]) != (w,h):
                raise ValueError('invalid scene-file gallery render: ' + filename)
            pending_extra.replace(output / filename)
            additional.append(dict(image=filename, width=w, height=h, title=info['title'],
                                   description=info['description'], scene_file='scenes/' + path.name))

    metadata = dict(commit=commit, repository=repository, generated_at=datetime.now(timezone.utc).isoformat(timespec="seconds"),
                    width=width, height=height, samples_per_axis=samples,
                    light_samples_per_axis=light_samples, max_depth=depth,
                    render_seconds=round(elapsed, 3), additional_renders=additional)
    if scene_file:
        info = json.loads(Path(scene_file).read_text())
        metadata.update(scene_title=info.get('title', Path(scene_file).stem),
                        scene_description=info.get('description', 'A scene rendered from an editable JSON file.'))
    write_page(output, metadata)
    return metadata


def write_page(output, metadata):
    """Write the gallery from verified renders; used again when the quality job finishes."""
    output = Path(output)
    commit, repository = metadata.get('commit', ''), metadata.get('repository', '')
    width, height = metadata['width'], metadata['height']
    samples, light_samples, depth = (metadata[k] for k in ('samples_per_axis', 'light_samples_per_axis', 'max_depth'))
    additional = metadata.get('additional_renders', [])
    # Only generate source links for valid GitHub identities and full commit hashes.
    linked = (re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository) and
              re.fullmatch(r"[0-9a-fA-F]{40}", commit))
    if linked:
        commit_url = "https://github.com/{}/commit/{}".format(repository, commit)
        source = '<a href="{}">Source commit <code>{}</code></a>'.format(
            html.escape(commit_url, quote=True), html.escape(commit[:7]))
    else:
        source = "Local render"
    timestamp = metadata["generated_at"]
    (output / "render.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    extra_html = ""
    if additional:
        cards = []
        for item in additional:
            if not re.fullmatch(r'[A-Za-z0-9_.-]+\.png', item['image']):
                raise ValueError('invalid gallery image filename')
            values = dict(item)
            for key in ('image', 'title', 'description'):
                values[key] = html.escape(str(values[key]), quote=True)
            values['width'], values['height'] = int(item['width']), int(item['height'])
            image = '<img src="{image}" width="{width}" height="{height}" loading="lazy" alt="{description}">'.format(**values)
            control, specs, css = '', '', ''
            if 'scene_file' in item:
                if not re.fullmatch(r'scenes/[A-Za-z0-9_.-]+\.json', item['scene_file']):
                    raise ValueError('invalid scene source filename')
                control = '<a class="scene-source" href="{}">Scene JSON</a>'.format(item['scene_file'])
            if 'animation' in item:
                poster = item['image'][:-4] + '-still.png'
                image = image.replace('src="' + values['image'] + '"', 'src="' + poster + '"')
                image = '<picture class="animation-picture"><source media="(prefers-reduced-motion: reduce)" srcset="' + poster + '">' + image + '</picture>'
                animation = item['animation']
                label = 'High quality' if is_quality(animation) else 'Quick preview'
                specs = '<small class="animation-specs">{} · {} × {} · {} FPS · {} samples/pixel</small>'.format(
                    label, int(animation['width']), int(animation['height']), int(animation['fps']),
                    int(animation.get('samples_per_axis', 1)) ** 2)
                control = '<button type="button" class="animation-control" data-animation="{}" data-poster="{}" aria-pressed="false">Play animation</button><span class="animation-status" role="status"></span>'.format(values['image'], poster)
                if not is_quality(animation) and linked:
                    control += '<a class="quality-link" href="https://github.com/{}/actions/workflows/animation-quality.yml">Full-quality render workflow</a>'.format(repository)
                css = ' class="animation-card"'
            cards.append('<figure{css}><a href="{image}">{picture}</a><figcaption><strong>{title}</strong><br>{description}{specs}{control}</figcaption></figure>'.format(
                css=css, picture=image, specs=specs, control=control, **values))
        extra_html = '<section aria-label="More rendered scenes" class="additional">' + ''.join(cards) + '</section>'
        extra_html += """<script>
document.querySelectorAll('.animation-control').forEach(button => {
  const card = button.closest('figure');
  const image = card.querySelector('img');
  const source = card.querySelector('source');
  const status = card.querySelector('.animation-status');
  const show = (file, playing) => {
    source.srcset = file;
    image.src = file;
    button.setAttribute('aria-pressed', String(playing));
    button.textContent = playing ? 'Pause animation' : 'Play animation';
    button.disabled = false;
  };
  button.addEventListener('click', () => {
    status.textContent = '';
    if (button.getAttribute('aria-pressed') === 'true') {
      show(button.dataset.poster, false);
      return;
    }
    button.disabled = true;
    button.textContent = 'Loading animation…';
    const preload = new Image();
    preload.onload = () => show(button.dataset.animation, true);
    preload.onerror = () => {
      show(button.dataset.poster, false);
      status.textContent = 'Could not load the animation. Please try again.';
    };
    preload.src = button.dataset.animation;
  });
});
</script>"""
    page = """<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="description" content="A classic ray-traced scene with a cloudy sky, glass and chrome spheres, soft shadows, and a checkerboard floor.">
  <title>Raytracer — Render gallery</title>
  <style>
    :root { color-scheme: dark; font-family: system-ui, sans-serif; background: #111820; color: #eef2f6; }
    * { box-sizing: border-box; }
    body { margin: 0; padding: 48px 24px; }
    main { max-width: 1080px; margin: 0 auto; }
    .eyebrow { color: #f2bb78; text-transform: uppercase; letter-spacing: .14em; font-size: .75rem; font-weight: 700; }
    h1 { margin: 12px 0; font-size: clamp(2.5rem, 7vw, 4.6rem); letter-spacing: -.05em; line-height: 1.05; }
    .intro { max-width: 650px; line-height: 1.65; color: #b8c6d5; }
    figure { margin: 32px 0 0; border: 1px solid #364553; border-radius: 16px; overflow: hidden; background: #1c2732; }
    img { display: block; width: 100%; height: auto; }
    figcaption { padding: 16px 20px; line-height: 1.6; color: #c3ceda; }
    .additional { display: grid; grid-template-columns: repeat(2, 1fr); gap: 20px; margin-bottom: 28px; }
    .additional figure { margin: 0; }
    .animation-picture img { width: auto; max-width: 100%; margin: 0 auto; }
    .animation-specs { display: block; color: #d1bd9e; margin: 10px 0; }
    .animation-control { padding: 9px 14px; border: 1px solid #f2bb78; border-radius: 6px; color: #f2bb78; background: transparent; font: inherit; cursor: pointer; }
    .animation-control:disabled { opacity: .65; cursor: wait; }
    .animation-control:focus-visible { outline: 2px solid #f2bb78; outline-offset: 4px; }
    .animation-status, .quality-link { display: block; margin-top: 8px; }
    @media (max-width: 600px) { .additional { grid-template-columns: 1fr; } }
    dl { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin: 24px 0; }
    dl div { border: 1px solid #364553; border-radius: 10px; padding: 18px; }
    dt { font-size: .8rem; color: #b8c6d5; }
    dd { margin: 8px 0 0; font-size: 1.2rem; font-weight: 650; }
    footer { display: flex; gap: 16px; flex-wrap: wrap; align-items: center; line-height: 1.8; color: #b8c6d5; font-size: .85rem; }
    a { color: #f2bb78; text-underline-offset: .2em; }
    a:hover { color: #ffe0b8; }
    a:focus-visible { outline: 2px solid #f2bb78; outline-offset: 5px; }
    @media (max-width: 600px) { body { padding: 28px 16px; } dl { grid-template-columns: repeat(2, 1fr); } }
  </style>
</head>
<body>
  <main>
    <header>
      <div class="eyebrow">Built from source · rendered on the CPU</div>
      <h1>Raytracer</h1>
      <p class="intro">Procedural materials, glass, clouds, and scenes you can edit.
      These images are generated by the Rust renderer, then published with the code that produced them.</p>
    </header>
    <figure>
      <a href="scene.png" aria-label="Open the rendered scene at full resolution">
        <img src="scene.png" width="WIDTH" height="HEIGHT" alt="SCENE_DESCRIPTION">
      </a>
      <figcaption><strong>SCENE_TITLE</strong><br>SCENE_DESCRIPTION</figcaption>
    </figure>
    <dl>
      <div><dt>Resolution</dt><dd>WIDTH × HEIGHT</dd></div>
      <div><dt>Samples per pixel</dt><dd>PIXEL_SAMPLES</dd></div>
      <div><dt>Area-light samples</dt><dd>LIGHT_SAMPLES</dd></div>
      <div><dt>Ray bounces</dt><dd>DEPTH</dd></div>
    </dl>
    EXTRAS
    <footer>
      SOURCE
      <a href="scene.png" download>Download PNG</a>
      <a href="render.json">Render details</a>
      <span>Generated <time datetime="TIMESTAMP">TIMESTAMP</time></span>
    </footer>
  </main>
</body>
</html>
"""
    scene_title = html.escape(str(metadata.get('scene_title', 'Procedural materials')), quote=True)
    scene_description = html.escape(str(metadata.get('scene_description',
        'Marble, wood, and stone alongside chrome and frosted glass, under a cloudy sky with soft shadows and a gently textured checkerboard.')), quote=True)
    for key, value in dict(WIDTH=width, HEIGHT=height, PIXEL_SAMPLES=samples * samples,
                           LIGHT_SAMPLES=light_samples * light_samples, DEPTH=depth,
                           SCENE_TITLE=scene_title, SCENE_DESCRIPTION=scene_description,
                           TIMESTAMP=timestamp, EXTRAS=extra_html, SOURCE=source).items():
        page = page.replace(key, str(value))
    (output / "index.html").write_text(page, encoding="utf-8")
    (output / ".nojekyll").write_text("", encoding="utf-8")



def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--renderer", required=True, type=Path)
    parser.add_argument("--output", default="_site", type=Path)
    parser.add_argument("--width", type=bounded_int(1, 8192), default=800)
    parser.add_argument("--height", type=bounded_int(1, 8192), default=600)
    parser.add_argument("--samples", type=bounded_int(1, 16), default=4)
    parser.add_argument("--light-samples", type=bounded_int(1, 64), default=4)
    parser.add_argument("--depth", type=bounded_int(0, 16), default=6)
    parser.add_argument("--extras", action="store_true", help="also render the teapot, volumetric clouds, and an animated PNG")
    parser.add_argument("--scene-file", type=Path, help="render a JSON scene as the main gallery image")
    args = parser.parse_args()
    generate(args.renderer, args.output, args.width, args.height, args.samples,
             args.light_samples, args.depth, os.environ.get("GITHUB_SHA", ""),
             os.environ.get("GITHUB_REPOSITORY", ""), args.extras, args.scene_file)
    print("Gallery written to {}".format(args.output))


if __name__ == "__main__":
    main()
