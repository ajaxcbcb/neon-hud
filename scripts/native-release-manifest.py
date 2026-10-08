"""Package a separate signed native channel; never modify the stable updater feed."""
import base64
import hashlib
import json
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def verify_signatures(directory):
    public = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())["plugins"]["updater"]["pubkey"]
    key = directory / "release-public.pub"
    key.write_bytes(base64.b64decode(public, validate=True))
    files = [directory / "native-update.json", *sorted(directory.glob("*.zip"))]
    for file in files:
        sig = file.with_name(file.name + ".minisig")
        sig.write_bytes(base64.b64decode(file.with_name(file.name + ".sig").read_text().strip(), validate=True))
        subprocess.run(["minisign", "-Vm", str(file), "-p", str(key), "-x", str(sig)], check=True)
        sig.unlink()
    key.unlink()
    manifest = json.loads((directory / "native-update.json").read_text())
    for asset in manifest["platforms"].values():
        file = directory / asset["url"].rsplit("/", 1)[-1]
        assert file.stat().st_size == asset["size"]
        assert hashlib.sha256(file.read_bytes()).hexdigest() == asset["sha256"]
    print("Native manifest, archive signatures and digests verified")


def package(source, output, commit):
    assert re.fullmatch(r"[a-f0-9]{40}", commit), "Invalid source commit"
    version = re.search(r'^version = "([^"]+)"', (ROOT / "src-native/Cargo.toml").read_text(), re.M)[1]
    assert re.fullmatch(r"0\.2\.0-alpha\.[0-9]+", version), "Not a native preview version"
    output.mkdir(parents=True, exist_ok=False)
    assets = {
        "windows-x86_64": "neon-hud-native-windows-x64-unsigned-preview.zip",
        "macos-aarch64": "neon-hud-native-macos-unsigned-preview.zip",
    }
    manifest = {"schema": 1, "channel": "native-preview", "version": version, "sourceCommit": commit, "platforms": {}}
    for platform, name in assets.items():
        matches = list(source.rglob(name))
        assert len(matches) == 1, f"Need one {name}"
        target = output / name
        shutil.copyfile(matches[0], target)
        size = target.stat().st_size
        assert 0 < size <= 64 * 1024 * 1024, "Unexpected archive size"
        manifest["platforms"][platform] = {
            "url": f"https://github.com/ajaxcbcb/neon-hud/releases/download/v{version}/{name}",
            "size": size,
            "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
        }
    dmgs = list(source.rglob("neon-hud-native-macos-unsigned-preview.dmg"))
    assert len(dmgs) == 1
    shutil.copyfile(dmgs[0], output / dmgs[0].name)
    (output / "native-update.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    lines = [f"{hashlib.sha256(file.read_bytes()).hexdigest()}  {file.name}" for file in sorted(output.iterdir())]
    (output / "SHA256SUMS.txt").write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(json.dumps({"version": version, "sourceCommit": commit, "platforms": list(assets)}))


if __name__ == "__main__":
    if sys.argv[1] == "--verify-signatures":
        verify_signatures(Path(sys.argv[2]))
    else:
        package(Path(sys.argv[1]), Path(sys.argv[2]), sys.argv[3])
