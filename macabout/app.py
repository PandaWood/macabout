from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor

from . import formatters, hwinfo, ui


def main() -> None:
    parser = argparse.ArgumentParser(description="About This Computer")
    parser.add_argument(
        "--mock",
        action="store_true",
        help="Force mock data (for development on non-Linux platforms)",
    )
    args = parser.parse_args()

    # Both run commands; side by side, the window opens sooner.
    with ThreadPoolExecutor(max_workers=1) as pool:
        dark = pool.submit(ui.detect_dark_mode)
        info = hwinfo.collect_system_info(force_mock=args.mock)
    display = formatters.format_all(info)
    ui.show_dialog(display, dark=dark.result())


if __name__ == "__main__":
    main()
