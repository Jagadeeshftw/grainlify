import unittest
import tempfile
from pathlib import Path

from scripts import verify_manifest


def sha256_of_bytes(b: bytes) -> str:
    import hashlib
    return hashlib.sha256(b).hexdigest()


class VerifyManifestTests(unittest.TestCase):
    def test_verify_and_detect_tamper(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            out = tmp / "artifacts"
            out.mkdir()
            file1 = out / "crateA-abcdef.wasm"
            file1.write_bytes(b"original content")
            sha = sha256_of_bytes(b"original content")
            manifest = out / "sha256-manifest.txt"
            manifest.write_text(f"{sha}  {file1.name}\n")

            # should succeed
            rc = verify_manifest.verify_manifest(manifest)
            self.assertEqual(rc, 0)

            # tamper
            file1.write_bytes(b"tampered")
            rc2 = verify_manifest.verify_manifest(manifest)
            self.assertEqual(rc2, 1)


if __name__ == '__main__':
    unittest.main()
