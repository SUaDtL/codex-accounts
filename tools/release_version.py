"""Strict Semantic Versioning shared by build and release tooling."""
from __future__ import annotations

import re

IDENTIFIER = r'(?:0|[1-9]\d*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)'
SEMVER = re.compile(
    rf'(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)'
    rf'(?:-(?P<prerelease>{IDENTIFIER}(?:\.{IDENTIFIER})*))?'
    r'(?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?'
)


def parse_semver(value: str) -> re.Match[str]:
    match = SEMVER.fullmatch(value)
    if match is None:
        raise ValueError('Version must be valid Semantic Versioning')
    return match
