"""Deterministic pure-wheel generator for pip --user probe (ASCII).

Writes orz_s4_probe-0.1.0-py3-none-any.whl into the current directory.
"""

import base64
import hashlib
import os
import zipfile


NAME = "orz_s4_probe"
VERSION = "0.1.0"
DIST = "%s-%s" % (NAME, VERSION)
FILENAME = "%s-py3-none-any.whl" % DIST


def _sha256_b64(data):
    digest = hashlib.sha256(data).digest()
    return base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")


def main():
    files = {
        "%s/__init__.py" % NAME: b'VERSION = "0.1.0"\n',
        "%s.dist-info/METADATA" % DIST: (
            b"Metadata-Version: 2.1\n"
            b"Name: orz-s4-probe\n"
            b"Version: 0.1.0\n"
        ),
        "%s.dist-info/WHEEL" % DIST: (
            b"Wheel-Version: 1.0\n"
            b"Generator: orz-s4-probe-seed-0.1\n"
            b"Root-Is-Purelib: true\n"
            b"Tag: py3-none-any\n"
        ),
    }
    record_lines = []
    for rel in sorted(files):
        data = files[rel]
        record_lines.append(
            "%s,sha256=%s,%d" % (rel, _sha256_b64(data), len(data))
        )
    record_lines.append("%s.dist-info/RECORD,," % DIST)
    files["%s.dist-info/RECORD" % DIST] = (
        ("\n".join(record_lines) + "\n").encode("ascii")
    )

    out = os.path.join(os.getcwd(), FILENAME)
    if os.path.exists(out):
        os.remove(out)
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as zf:
        for rel in sorted(files):
            zf.writestr(rel, files[rel])
    print("WHEEL_WRITTEN=%s" % FILENAME)


if __name__ == "__main__":
    main()
