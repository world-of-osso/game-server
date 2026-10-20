"""Root ADT sampling using authored chunk axes and four-triangle fans."""

import importlib.util
import struct
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
SCRIPT = Path(__file__).resolve().parents[1] / "terrain_height.py"
SIZE = 100.0 / 3.0
UNIT = SIZE / 8.0
TILE = SIZE * 16
ORIGIN_X = (32 - 15) * TILE
ORIGIN_Y = (32 - 40) * TILE


def chunk(tag, payload):
    return tag[::-1] + struct.pack("<I", len(payload)) + payload


def terrain(ix=8, iy=10, heights=None, base=100.0, origin_delta=0.0):
    header = bytearray(128)
    struct.pack_into("<II", header, 4, ix, iy)
    struct.pack_into(
        "<3f",
        header,
        104,
        ORIGIN_X - iy * SIZE + origin_delta,
        ORIGIN_Y - ix * SIZE,
        base,
    )
    sub = (
        b""
        if heights is None
        else chunk(b"MCVT", struct.pack(f"<{len(heights)}f", *heights))
    )
    return chunk(b"MCNK", header + sub)


def fan_heights():
    heights = [0.0] * 145
    # Quad row 2, col 3: asymmetric corners and a raised center.
    for index, height in [(37, 2), (38, 10), (54, 18), (55, 30), (46, 50)]:
        heights[index] = height
    return heights


class TerrainHeightTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location("terrain_height", SCRIPT)
        cls.sampler = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.sampler)

    def sample(self, data, row=2.5, col=3.5, tile_row=40, tile_col=15):
        x = ORIGIN_X - 10 * SIZE - row * UNIT
        y = ORIGIN_Y - 8 * SIZE - col * UNIT
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "root.adt"
            path.write_bytes(data)
            adt = self.sampler.read_adt_height(path)
        return self.sampler.sample_adt_height(adt, x, y, tile_row, tile_col)

    def test_all_four_triangles_use_center_and_correct_axes(self):
        data = chunk(b"MVER", struct.pack("<I", 18)) + terrain(heights=fan_heights())
        # Half center plus quarter of each edge corner; not bilinear or nearest vertex.
        for row, col, expected in [
            (2.25, 3.5, 128),
            (2.5, 3.75, 135),
            (2.75, 3.5, 137),
            (2.5, 3.25, 130),
            (2.5, 3.5, 150),
            (2.0, 3.0, 102),
        ]:
            with self.subTest(row=row, col=col):
                self.assertAlmostEqual(self.sample(data, row, col), expected, places=7)

    def test_nonzero_indices_select_correct_chunk_not_stream_order(self):
        data = terrain(ix=9, heights=[99.0] * 145) + terrain(heights=fan_heights())
        self.assertAlmostEqual(self.sample(data), 150.0, places=7)

    def test_chunk_edge_is_half_open_and_selects_neighbor(self):
        data = terrain(heights=[1.0] * 145) + terrain(iy=11, heights=[7.0] * 145)
        self.assertAlmostEqual(self.sample(data, row=8), 107.0)
        with self.assertRaisesRegex(ValueError, "missing.*chunk"):
            self.sample(terrain(heights=[1.0] * 145), row=8)

    def test_tile_mismatch_rejected(self):
        with self.assertRaisesRegex(ValueError, "tile"):
            self.sample(terrain(heights=fan_heights()), tile_row=39)

    def test_wrong_tile_artifact_rejected_by_header_origin(self):
        with self.assertRaisesRegex(ValueError, "origin"):
            self.sample(terrain(heights=fan_heights(), origin_delta=TILE))

    def test_chunk_mismatch_rejected_by_header_origin(self):
        with self.assertRaisesRegex(ValueError, "origin"):
            self.sample(terrain(heights=fan_heights(), origin_delta=SIZE))

    def test_missing_and_invalid_heights_rejected(self):
        cases = [None, [0.0] * 144, [float("nan")] * 145, [float("inf")] * 145]
        for heights in cases:
            with (
                self.subTest(heights=heights),
                self.assertRaisesRegex(ValueError, "MCVT|height"),
            ):
                self.sample(terrain(heights=heights))
        with self.assertRaisesRegex(ValueError, "height"):
            self.sample(terrain(heights=fan_heights(), base=float("nan")))

    def test_missing_chunk_rejected(self):
        with self.assertRaisesRegex(ValueError, "missing.*chunk"):
            self.sample(terrain(ix=9, heights=fan_heights()))

    def test_truncated_framing_rejected(self):
        whole = terrain(heights=fan_heights())
        for data, message in [
            (b"KN", "truncated chunk header at byte 0"),
            (whole[:-1], "truncated b'MCNK' payload at byte 0"),
            (chunk(b"MCNK", bytes(127)), r"truncated MCNK header \(need 128 bytes\)"),
            (whole + b"M", f"truncated chunk header at byte {len(whole)}"),
        ]:
            with (
                self.subTest(length=len(data)),
                self.assertRaisesRegex(ValueError, message),
            ):
                self.sample(data)

    def test_missing_cli_names_the_pinned_install_command(self):
        dump = self.sampler.wow_formats_dump
        installed = dump.BINARY
        dump.BINARY = Path("/nonexistent/wow-formats-dump")
        try:
            with self.assertRaisesRegex(
                ValueError, f"cargo install --locked .* --rev {dump.REV}"
            ):
                self.sampler.read_adt_height(Path("/nonexistent/root.adt"))
        finally:
            dump.BINARY = installed

    def test_nonfinite_world_coordinates_and_invalid_tile_rejected(self):
        for x, y, row, col in [
            (float("nan"), 0, 40, 15),
            (0, float("inf"), 40, 15),
            (0, 0, -1, 15),
            (0, 0, 40, 64),
        ]:
            with (
                self.subTest(x=x, y=y, row=row, col=col),
                self.assertRaises(ValueError),
            ):
                self.sampler.sample_adt_height(b"", x, y, row, col)

    def test_cli_prints_height_or_explicit_error(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "6173902.adt"
            path.write_bytes(terrain(heights=fan_heights()))
            args = [
                sys.executable,
                str(SCRIPT),
                str(path),
                "--tile-row",
                "40",
                "--tile-col",
                "15",
                "--world-x",
                str(ORIGIN_X - 10 * SIZE - 2.5 * UNIT),
                "--world-y",
                str(ORIGIN_Y - 8 * SIZE - 3.5 * UNIT),
            ]
            result = subprocess.run(args, capture_output=True, text=True, check=False)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(float(result.stdout), 150.0)
            self.assertEqual(result.stderr, "")
            path.write_bytes(terrain())
            result = subprocess.run(args, capture_output=True, text=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")
            self.assertIn("MCVT", result.stderr)


if __name__ == "__main__":
    unittest.main()
