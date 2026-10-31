#!/usr/bin/env python3
"""Exercise gallery generation with the actual compiled renderer."""
import argparse
from html.parser import HTMLParser
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from build_site import generate


class Page(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.images = []
        self.links = []
        self.feed(text)

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        if tag == "img":
            self.images.append(attributes)
        if tag == "a":
            self.links.append(attributes.get("href", ""))


class GalleryTests(unittest.TestCase):
    renderer = None

    def test_real_render_and_relative_links(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "gallery"
            commit = "0123456789abcdef0123456789abcdef01234567"
            generate(self.renderer, output, 64, 48, 1, 1, 2, commit, "drewchandler/raytracer")
            page = Page((output / "index.html").read_text())
            self.assertEqual(len(page.images), 1)
            self.assertEqual(page.images[0]["src"], "scene.png")
            self.assertTrue(page.images[0]["alt"])
            self.assertIn("https://github.com/drewchandler/raytracer/commit/" + commit, page.links)
            for link in page.links:
                if not link.startswith("https://"):
                    self.assertTrue((output / link).is_file(), link)
            png = (output / "scene.png").read_bytes()
            self.assertEqual(png[:8], b"\x89PNG\r\n\x1a\n")
            self.assertEqual(struct.unpack(">II", png[16:24]), (64, 48))
            metadata = json.loads((output / "render.json").read_text())
            self.assertEqual(metadata["commit"], commit)
            self.assertEqual(metadata["width"], 64)
            self.assertEqual(metadata["height"], 48)
            self.assertEqual(metadata["samples_per_axis"], 1)
            self.assertTrue((output / ".nojekyll").is_file())

    def test_renderer_failure_does_not_publish_stale_files(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "scene.pending.png").write_bytes(b"stale")
            error = subprocess.CalledProcessError(1, [str(self.renderer)])
            with patch("build_site.subprocess.run", side_effect=error):
                with self.assertRaises(subprocess.CalledProcessError):
                    generate(self.renderer, output, 64, 48)
            self.assertFalse((output / "index.html").exists())
            self.assertFalse((output / "scene.pending.png").exists())

    def test_missing_or_invalid_png_cannot_pass_as_a_success(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            with patch("build_site.subprocess.run"):
                with self.assertRaises(FileNotFoundError):
                    generate(self.renderer, output, 64, 48)
            def invalid_image(*args, **kwargs):
                (output / "scene.pending.png").write_bytes(b"not a PNG")
            with patch("build_site.subprocess.run", side_effect=invalid_image):
                with self.assertRaises(ValueError):
                    generate(self.renderer, output, 64, 48)
            self.assertFalse((output / "index.html").exists())

    def test_untrusted_metadata_does_not_become_markup_or_links(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            generate(self.renderer, output, 8, 8, 1, 1, 0, '<script>alert(1)</script>', '" onload="alert(1)')
            text = (output / "index.html").read_text()
            self.assertNotIn("<script>", text)
            self.assertNotIn("onload", text)
            self.assertIn("Local render", text)

    def test_extra_gallery_renders_mesh_volume_and_real_animation(self):
        from render_animation import chunks
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)
            generate(self.renderer,output,32,24,1,1,2,extras=True)
            page=Page((output/'index.html').read_text())
            self.assertEqual(len(page.images),4)
            for image in page.images:
                self.assertTrue((output/image['src']).is_file())
            for name in ('teapot.png','volumetric.png','clouds.png','clouds-still.png'):
                self.assertTrue((output/name).exists())
            animation=list(chunks((output/'clouds.png').read_bytes()))
            self.assertEqual(sum(k==b'fcTL' for k,_ in animation),8)
            metadata=json.loads((output/'render.json').read_text())
            self.assertEqual(len(metadata['additional_renders']),3)
            self.assertIn('prefers-reduced-motion',(output/'index.html').read_text())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--renderer", required=True, type=Path)
    arguments, remainder = parser.parse_known_args()
    GalleryTests.renderer = arguments.renderer.resolve(strict=True)
    unittest.main(argv=[__file__] + remainder)
