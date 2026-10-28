#!/usr/bin/env python3
"""Convert a top-character native JSON profile to a SimC profile."""
import argparse
import json
from pathlib import Path


def render(profile):
    lines = [
        f'{profile["class"]}="{profile["name"]}"',
        f'spec={profile["spec"]}',
        f'level={profile["level"]}',
        f'race={profile["race"].lower().replace(" ", "_")}',
        f'role={profile["role"]}',
        'position=front' if profile['role'] == 'tank' else f'position={profile["position"]}',
        f'talents={profile["talents"]}',
    ]
    lines.extend(f'{key}={value}' for key, value in profile['consumables'].items())
    for item in profile['equipment']:
        options = [f'id={item["item_id"]}']
        if item.get('item_level_override'):
            options.append(f'ilevel={item["item_level_override"]}')
        for key, option in [('bonus_list', 'bonus_id'), ('gems', 'gem_id'), ('crafted_stats', 'crafted_stats')]:
            if item.get(key):
                options.append(f'{option}=' + '/'.join(map(str, item[key])))
        if item.get('enchant'):
            options.append(f'enchant_id={item["enchant"]}')
        lines.append(f'{item["slot"]}=,' + ','.join(options))
    return '\n'.join(lines) + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('profile', type=Path)
    args = parser.parse_args()
    print(render(json.loads(args.profile.read_text())), end='')


if __name__ == '__main__':
    main()
