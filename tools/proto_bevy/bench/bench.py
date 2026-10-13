"""Alternating Bevy/Godot bench for the engine prototype (skeleton)."""

import argparse


def main() -> None:
    """Parse arguments; the bench itself lands in lot A."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--passes", type=int, default=3, help="Passes per engine.")
    parser.parse_args()


if __name__ == "__main__":
    main()
