#!/usr/bin/env python3
"""Retain one player's WCL cast-position samples between Dawnbreaker boss pulls."""
import argparse
import json
from pathlib import Path

# WCL x3HzDRn6yVjqCbt8 fight 7: previous kill to next pull, milliseconds.
WINDOWS = ((2837, 0, 360644), (2838, 600978, 1009307), (2839, 1290864, 1394568))


def positions(events):
    legs = []
    for encounter, start, end in WINDOWS:
        points = []
        for event in events:
            if event.get('sourceID') != 5 or 'x' not in event or 'y' not in event:
                continue
            timestamp = event['fight_ms']
            if start <= timestamp <= end:
                xy = [event['y'] / 100, event['x'] / 100]
                if not points or xy != points[-1][1:]:
                    points.append([int(timestamp), *xy])
        if not points:
            raise ValueError(f'no WCL positions in encounter {encounter} window')
        legs.append({'encounter': encounter, 'window_ms': [start, end], 'points': points})
    return {
        'report': 'x3HzDRn6yVjqCbt8',
        'fight': 7,
        'actor': 5,
        'coordinate_system': 'Bevy x,z from WCL y,x divided by 100',
        'selection': 'Cast-resource source positions between prior boss kill and next boss pull; consecutive identical positions omitted. These samples alone do not prove airborne state.',
        'legs': legs,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    with args.input.open() as source:
        result = positions([json.loads(line) for line in source])
    args.output.write_text(json.dumps(result, separators=(',', ':')) + '\n')


if __name__ == '__main__':
    main()
