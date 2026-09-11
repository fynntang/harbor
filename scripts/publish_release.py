#!/usr/bin/env python3
"""Publish a complete new stable release. Existing releases are never overwritten."""
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path
from version import validate_display
from installation import FIRST_LAUNCH


def assets_for(tag, directory):
    if not tag.startswith('v'):
        raise ValueError('Expected a v-prefixed release tag')
    version = validate_display(tag[1:])
    names = [f'Harbor-{version}-{target}.dmg' for target in ('aarch64-apple-darwin', 'x86_64-apple-darwin')]
    names += [f'Harbor-{version}-macos-universal.zip']
    names += ['appcast.xml', 'SHA256SUMS.txt']
    assets = [directory / name for name in names]
    if any(not asset.is_file() for asset in assets):
        raise ValueError('Release requires both architecture DMGs, Universal ZIP, signed appcast and checksums')
    hashes = dict(line.split('  ', 1)[::-1] for line in assets[-1].read_text().splitlines())
    for asset in assets[:-1]:
        if hashes.get(asset.name) != hashlib.sha256(asset.read_bytes()).hexdigest():
            raise ValueError('Release asset checksum mismatch')
    return assets


def main():
    tag, directory = sys.argv[1], Path(sys.argv[2])
    assets = assets_for(tag, directory)
    repository = os.environ.get('GH_REPO', 'fynntang/harbor')
    # An API/list error aborts; it is never interpreted as a missing release.
    releases = json.loads(subprocess.check_output(['gh', 'api', '--paginate', '--slurp', f'repos/{repository}/releases?per_page=100']))
    if any(release['tag_name'] == tag for page in releases for release in page):
        raise ValueError('Release already exists; inspect it manually instead of overwriting')
    latest = next((release for page in releases for release in page if not release['draft'] and not release['prerelease']), None)
    if latest:
        try:
            previous = tuple(map(int, latest['tag_name'].removeprefix('v').split('.')))
            current = tuple(map(int, tag[1:].split('.')))
        except ValueError:
            raise ValueError('Cannot compare the existing stable release version') from None
        if current <= previous:
            raise ValueError('New stable release must be newer than the existing stable release')
    notes = directory / 'release-notes.md'
    notes.write_text(f'''Harbor {tag}\n\nmacOS 14+ · Apple Silicon + Intel (arm64 / x86_64)\n\nApple Silicon 选择 aarch64-apple-darwin.dmg，Intel 选择 x86_64-apple-darwin.dmg。打开 DMG，将 Harbor.app 拖入应用程序。现有 0.0.1 用户需手动安装一次以启用后续自动更新。\nChoose aarch64-apple-darwin.dmg for Apple Silicon or x86_64-apple-darwin.dmg for Intel. Open the DMG and drag Harbor.app into Applications. Existing 0.0.1 users need one manual upgrade to enable future automatic updates.\n\n本版本为 ad-hoc 签名，未经 Apple 公证；Sparkle 更新包和清单均有 Ed25519 签名。\nThis build is ad-hoc signed and not notarized by Apple. The Sparkle update archive and feed are Ed25519-signed.\n\n{FIRST_LAUNCH}''')
    subprocess.run(['gh', 'release', 'create', tag, *map(str, assets), '--repo', repository, '--verify-tag', '--draft', '--title', f'Harbor {tag}', '--notes-file', str(notes), '--generate-notes'], check=True)
    # Inspect uploaded byte lengths before making the feed reachable as latest.
    release = json.loads(subprocess.check_output(['gh', 'release', 'view', tag, '--repo', repository, '--json', 'assets,isDraft']))
    actual = {asset['name']: asset['size'] for asset in release['assets']}
    expected = {asset.name: asset.stat().st_size for asset in assets}
    if not release['isDraft'] or actual != expected:
        raise ValueError('Uploaded assets differ; release kept as a draft')
    subprocess.run(['gh', 'release', 'edit', tag, '--repo', repository, '--draft=false', '--latest'], check=True)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError) as error:
        raise SystemExit(str(error))
