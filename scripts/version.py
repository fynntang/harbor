#!/usr/bin/env python3
"""Map Cargo's canonical version to YY.M.DHHmm release labels."""
import datetime
import re
import sys


def normalized(version):
    if not isinstance(version, str) or not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", version):
        raise ValueError("Expected three numeric version components")
    return ".".join(str(int(part)) for part in version.split("."))


def display(version):
    year, month, packed = map(int, normalized(version).split("."))
    day, clock = divmod(packed, 10000)
    if not 0 <= year <= 99:
        raise ValueError("Calendar release year must be between 00 and 99")
    datetime.date(2000 + year, month, day)
    hour, minute = divmod(clock, 100)
    datetime.time(hour, minute)
    return f"{year}.{month}.{day}{clock:04d}"


def validate_display(version):
    if display(version) != version:
        raise ValueError("Use YY.M.DHHmm, for example 26.9.101156")
    return version


if __name__ == "__main__":
    print(display(sys.argv[1]))
