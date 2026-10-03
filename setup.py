"""
ULTRON (MARK LIV) — one-time setup.

Installs the Python dependencies for THIS operating system only: the OS-specific
packages in requirements.txt carry `sys_platform` markers, so a macOS or Linux
user never pulls Windows-only libraries (and vice-versa). Then it fetches the
Playwright browsers needed for web automation (current-OS builds only).

Two things it deliberately does NOT install:
  * the optional local wake word ("Hey ULTRON") — one-click, opt-in, from
    ⚙ → WAKE WORD inside the app;
  * anything for the avatar — the holographic head renders in software on the
    PyQt6 and numpy already listed here. No GPU, no OpenGL, no extra packages.
"""
import platform
import subprocess
import sys
from pathlib import Path

OS = platform.system()  # "Windows" | "Darwin" | "Linux"
HERE = Path(__file__).resolve().parent

MIN_PY = (3, 11)        # hard floor: below this the syntax used here won't parse
MAX_PY = (3, 14)        # highest version this is tested and verified on


def _run(label: str, args: list[str]) -> None:
    print(f"\n▶ {label}")
    subprocess.run(args, check=True)


def _check_python() -> None:
    """Fail immediately and clearly rather than deep inside a pip resolver.

    A wrong interpreter is the single most common way this install goes sideways,
    and the error it produces on its own names a wheel, not the real problem.
    """
    v = sys.version_info[:2]
    if v > MAX_PY:
        # Newer is a warning, not a wall.
        print(f"\n⚠️  Python {v[0]}.{v[1]} is newer than the "
              f"{MAX_PY[0]}.{MAX_PY[1]} this is tested on. Continuing — if a "
              f"package has no wheel yet, install Python "
              f"{MAX_PY[0]}.{MAX_PY[1]} and run setup with that.")
        return
    if v < MIN_PY:
        print(f"\n❌ Python {v[0]}.{v[1]} detected — ULTRON needs at "
              f"least Python {MIN_PY[0]}.{MIN_PY[1]}.")
        print("   Install a supported version and run setup with it, e.g.:")
        print(f"     py -{MIN_PY[0]}.{MIN_PY[1]} setup.py        (Windows)")
        print(f"     python{MIN_PY[0]}.{MIN_PY[1]} setup.py      (macOS / Linux)")
        sys.exit(1)


def _check_assets() -> None:
    """The avatar's face is a shipped file; a truncated clone should say so."""
    face = HERE / "core" / "face_model.obj"
    if not face.exists() or face.stat().st_size < 4096:
        print(
            "\n⚠️  core/face_model.obj is missing or truncated — the avatar will "
            "fall back to the plain glowing core.\n"
            "    Re-clone the repository, or fetch that one file again."
        )


def _run_setuptools() -> None:
    """Standard setuptools entrypoint when invoked via `pip install .` or `pip install -e .`"""
    try:
        from setuptools import setup, find_packages
    except ImportError:
        subprocess.run([sys.executable, "-m", "pip", "install", "setuptools"], check=True)
        from setuptools import setup, find_packages

    req_file = HERE / "requirements.txt"
    install_requires = []
    if req_file.exists():
        for line in req_file.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line and not line.startswith("#"):
                install_requires.append(line)

    setup(
        name="ultron-desktop-assistant",
        version="1.54.0",
        description="ULTRON — Advanced Multi-Agent Desktop AI Assistant with Living Office & Autonomous Capabilities",
        long_description=(HERE / "readme.md").read_text(encoding="utf-8") if (HERE / "readme.md").exists() else "",
        long_description_content_type="text/markdown",
        author="Nibir",
        python_requires=">=3.11",
        packages=find_packages(),
        install_requires=install_requires,
        classifiers=[
            "Programming Language :: Python :: 3",
            "Operating System :: Microsoft :: Windows",
            "Operating System :: MacOS",
            "Operating System :: POSIX :: Linux",
        ],
    )


def main() -> None:
    print(f"⚙  ULTRON setup — detected OS: {OS or 'unknown'}, "
          f"Python {sys.version_info[0]}.{sys.version_info[1]}")
    _check_python()

    # requirements.txt filters OS-specific extras by itself via pip markers.
    _run("Installing Python dependencies (OS-specific extras auto-filtered)…",
         [sys.executable, "-m", "pip", "install", "-r", str(HERE / "requirements.txt")])

    # Chromium covers Chrome/Edge/Opera/Brave/Vivaldi; Firefox for Firefox.
    # (Safari automation additionally needs: python -m playwright install webkit)
    try:
        _run("Installing Playwright browsers (chromium + firefox)…",
             [sys.executable, "-m", "playwright", "install", "chromium", "firefox"])
    except (subprocess.CalledProcessError, FileNotFoundError) as e:
        print(f"\n⚠️  Playwright browsers were not installed ({e}).")
        print("    Everything except browser automation works. Retry later with:")
        print(f'    {sys.executable} -m playwright install chromium firefox')

    _check_assets()

    # ── OS-specific post-install notes ────────────────────────────────────────
    if OS == "Windows":
        try:
            import win32com.client  # noqa: F401
        except ImportError:
            postinstall = Path(sys.executable).parent / "Scripts" / "pywin32_postinstall.py"
            print(
                "\n⚠️  pywin32 did not register correctly — desktop-shortcut "
                "creation will use a slower fallback. To fix it, run:\n"
                f'    "{sys.executable}" -m pip install --force-reinstall pywin32\n'
                f'    "{sys.executable}" "{postinstall}" -install'
            )
    elif OS == "Linux":
        print(
            "\nℹ️  Linux note — a few voice-controlled OS actions shell out to "
            "native tools. Install the ones you'll use via your package manager:\n"
            "    • volume      → pulseaudio-utils   (pactl)\n"
            "    • brightness  → brightnessctl\n"
            "    • reminders   → systemd (systemd-run) or 'at'\n"
            "    • open URLs   → xdg-utils          (xdg-open)"
        )
    elif OS == "Darwin":
        print(
            "\nℹ️  macOS note — volume, brightness and reminders use the built-in "
            "'osascript' / LaunchAgents, so no extra tools are required.\n"
            "    For Safari automation only: python -m playwright install webkit"
        )

    print("\n✅ Setup complete!")
    print("   1) Launch it:  python main.py")
    print("   2) Paste your free Gemini API key when the setup screen appears.")
    print("   3) (Optional) Enable 'Hey ULTRON' from ⚙ → WAKE WORD.")


if __name__ == "__main__":
    _SETUP_ARGS = {"install", "develop", "egg_info", "dist_info", "bdist_wheel", "sdist", "build"}
    if len(sys.argv) > 1 and any(arg in _SETUP_ARGS or arg.startswith("-") for arg in sys.argv[1:]):
        _run_setuptools()
    else:
        main()
