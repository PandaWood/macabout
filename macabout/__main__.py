import platform
import sys


def _check_tkinter() -> None:
    try:
        import tkinter  # noqa: F401
    except ImportError:
        v = sys.version_info
        if platform.system() == "Darwin":
            msg = (
                f"tkinter is not available.\n\n"
                f"Install it with Homebrew (version must match your Python):\n\n"
                f"  brew install python-tk@{v.major}.{v.minor}\n\n"
                f"Your Python: {v.major}.{v.minor}.{v.micro}"
            )
        elif platform.system() == "Linux":
            msg = (
                "tkinter is not available.\n\n"
                "Install it with your package manager:\n\n"
                "  sudo apt install python3-tk       # Debian / Ubuntu / Zorin / Mint\n"
                "  sudo dnf install python3-tkinter  # Fedora\n"
                "  sudo pacman -S tk                 # Arch"
            )
        else:
            msg = "tkinter is not available. Install the tk bindings for your Python."
        print(f"macabout: {msg}", file=sys.stderr)
        sys.exit(1)


_check_tkinter()

from .app import main  # noqa: E402

main()
