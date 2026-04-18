from __future__ import annotations

import argparse

from . import formatters, hwinfo, ui


def main() -> None:
    parser = argparse.ArgumentParser(description="About This Computer")
    parser.add_argument(
        "--mock",
        action="store_true",
        help="Force mock data (for development on non-Linux platforms)",
    )
    args = parser.parse_args()

    info = hwinfo.collect_system_info(force_mock=args.mock)
    display = formatters.format_all(info)
    ui.show_dialog(display)


if __name__ == "__main__":
    main()
