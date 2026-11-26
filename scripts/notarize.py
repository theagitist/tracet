#!/usr/bin/env python3
"""Notarize a file through Apple's Notary REST API, uploading in parallel.

Usage: scripts/notarize.py <file.dmg>
Env:   APPLE_API_KEY (key id), APPLE_API_ISSUER, APPLE_API_KEY_PATH (.p8)

Why not just `xcrun notarytool submit`: notarytool uploads over a single
connection, and on some networks one connection to Apple's S3 bucket gets a
tiny fraction of the link (seen 2026-11-25: 80 KB/s on a 17.8 Mbps uplink,
so the 650 MB dmg never finished). The same API with a multipart S3 upload
uses the whole link. Needs only the standard library, `openssl` and the
`aws` CLI. Exit status 0 means Apple accepted the submission.
"""

import base64
import hashlib
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request

API = "https://appstoreconnect.apple.com/notary/v2/submissions"


def b64url(data: bytes) -> str:
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode()


def der_to_raw(sig: bytes) -> bytes:
    """openssl emits an ASN.1 DER ECDSA signature; JWT ES256 wants r || s."""
    assert sig[0] == 0x30
    i = 2 if sig[1] < 0x80 else 2 + (sig[1] & 0x7F)
    out = b""
    for _ in range(2):
        assert sig[i] == 0x02
        length = sig[i + 1]
        value = sig[i + 2 : i + 2 + length].lstrip(b"\x00")
        out += value.rjust(32, b"\x00")
        i += 2 + length
    return out


def token() -> str:
    now = int(time.time())
    header = {"alg": "ES256", "kid": os.environ["APPLE_API_KEY"], "typ": "JWT"}
    payload = {
        "iss": os.environ["APPLE_API_ISSUER"],
        "iat": now,
        "exp": now + 15 * 60,
        "aud": "appstoreconnect-v1",
    }
    signing_input = f"{b64url(json.dumps(header).encode())}.{b64url(json.dumps(payload).encode())}"
    key = os.path.expanduser(os.environ["APPLE_API_KEY_PATH"])
    der = subprocess.run(
        ["openssl", "dgst", "-sha256", "-sign", key],
        input=signing_input.encode(),
        capture_output=True,
        check=True,
    ).stdout
    return f"{signing_input}.{b64url(der_to_raw(der))}"


def api(method: str, url: str, body=None) -> dict:
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token()}")
    if data is not None:
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            return json.load(resp)
    except urllib.error.HTTPError as e:
        sys.exit(f"Notary API {method} {url} failed: {e.code} {e.read().decode(errors='replace')}")


def sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def upload(path: str, attrs: dict) -> None:
    with tempfile.TemporaryDirectory() as tmp:
        config = os.path.join(tmp, "config")
        with open(config, "w") as f:
            f.write(
                "[default]\nregion = us-west-2\ns3 =\n"
                "  max_concurrent_requests = 16\n"
                "  multipart_threshold = 16MB\n"
                "  multipart_chunksize = 16MB\n"
            )
        env = {
            **os.environ,
            "AWS_CONFIG_FILE": config,
            "AWS_SHARED_CREDENTIALS_FILE": os.path.join(tmp, "none"),
            "AWS_ACCESS_KEY_ID": attrs["awsAccessKeyId"],
            "AWS_SECRET_ACCESS_KEY": attrs["awsSecretAccessKey"],
            "AWS_SESSION_TOKEN": attrs["awsSessionToken"],
        }
        env.pop("AWS_PROFILE", None)
        target = f"s3://{attrs['bucket']}/{attrs['object']}"
        for attempt in range(1, 4):
            started = time.time()
            if subprocess.run(["aws", "s3", "cp", "--only-show-errors", path, target], env=env).returncode == 0:
                mb = os.path.getsize(path) / 1e6
                secs = max(1.0, time.time() - started)
                print(f"Uploaded {mb:.0f} MB in {secs:.0f} s ({mb / secs:.1f} MB/s)")
                return
            print(f"Upload attempt {attempt} failed", file=sys.stderr)
        sys.exit("Upload failed")


def main() -> None:
    path = sys.argv[1]
    print(f"Submitting {os.path.basename(path)}")
    sub = api("POST", API, {"submissionName": os.path.basename(path), "sha256": sha256(path)})
    sub_id = sub["data"]["id"]
    print(f"Submission id: {sub_id}")
    upload(path, sub["data"]["attributes"])

    status = "In Progress"
    while status == "In Progress":
        time.sleep(20)
        status = api("GET", f"{API}/{sub_id}")["data"]["attributes"]["status"]
    print(f"status: {status}")
    if status != "Accepted":
        log_url = api("GET", f"{API}/{sub_id}/logs")["data"]["attributes"]["developerLogUrl"]
        with urllib.request.urlopen(log_url, timeout=60) as resp:
            print(json.dumps(json.load(resp), indent=2))
        sys.exit(1)


if __name__ == "__main__":
    main()
