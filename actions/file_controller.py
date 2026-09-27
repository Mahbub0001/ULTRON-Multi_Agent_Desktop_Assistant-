import os
import re
import shutil
import platform
import subprocess
from pathlib import Path
from datetime import datetime

try:
    import send2trash
    _SEND2TRASH = True
except ImportError:
    _SEND2TRASH = False

from core.undo import push_undo

_OS = platform.system()  # "Windows" | "Darwin" | "Linux"

# Undo keeps a file's previous contents in memory so `write` can be reversed.
# Above this size it does not — a 200 MB log would sit in RAM for the rest of
# the session to protect an edit nobody is going to take back.
_UNDO_CONTENT_LIMIT = 1_000_000


def _undo_move(src: Path, dst: Path):
    """Reverse of a move: put it back where it came from."""
    def _fn():
        if not dst.exists():
            return f"'{dst.name}' is no longer there — nothing moved back."
        src.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(str(dst), str(src))
        return f"'{src.name}' is back in {src.parent.name}/."
    return _fn


def _undo_create(target: Path):
    """Reverse of a create: remove what we made — and only if we still made it.

    Deliberately refuses to touch a directory that has since been filled: the
    undo for 'create a folder' is not 'delete whatever ended up in it'."""
    def _fn():
        if not target.exists():
            return f"'{target.name}' is already gone."
        if target.is_dir():
            if any(target.iterdir()):
                return (f"'{target.name}' is not empty any more — "
                        f"leaving it alone rather than deleting your files.")
            target.rmdir()
        else:
            target.unlink()
        return f"Removed '{target.name}'."
    return _fn


def _undo_write(target: Path, previous: str | None):
    """Reverse of a write: restore the old contents, or remove a file that did
    not exist before the write created it."""
    def _fn():
        if previous is None:
            if target.exists():
                target.unlink()
                return f"Removed '{target.name}' — it did not exist before."
            return f"'{target.name}' is already gone."
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(previous, encoding="utf-8")
        return f"Restored the previous contents of '{target.name}'."
    return _fn


def _restore_from_trash(original: Path) -> str:
    """Best-effort undelete.

    delete_file uses send2trash, which is the right call: the file lands in the
    Recycle Bin / Trash where the person can also find it themselves. Getting it
    back out again is shell work and only reliable on Windows, where pywin32 is
    already a dependency. Everywhere else this says where the file is instead of
    pretending it failed — the file is not lost either way."""
    if _OS == "Windows":
        try:
            import win32com.client
            shell = win32com.client.Dispatch("Shell.Application")
            bin_folder = shell.NameSpace(10)      # ssfBITBUCKET
            for item in bin_folder.Items():
                if str(bin_folder.GetDetailsOf(item, 1)).strip().lower() == \
                        str(original.parent).strip().lower():
                    if str(item.Name).strip().lower() == original.name.strip().lower():
                        item.InvokeVerb("UNDELETE")
                        return f"'{original.name}' restored from the Recycle Bin."
        except Exception as e:
            print(f"[file] Recycle Bin restore failed: {e}")
    return (f"'{original.name}' is in the Recycle Bin — I could not pull it back "
            f"automatically, but it is there and can be restored by hand.")


def _get_blocked_system_paths() -> list[Path]:
    blocked: list[Path] = []
    if _OS == "Windows":
        for env_var in ["SystemRoot", "windir"]:
            val = os.environ.get(env_var)
            if val:
                blocked.append(Path(val).resolve())
        for env_var in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"]:
            val = os.environ.get(env_var)
            if val:
                blocked.append(Path(val).resolve())
        for p_str in [r"C:\Windows", r"C:\Program Files", r"C:\Program Files (x86)", r"C:\Recovery", r"C:\Boot"]:
            try:
                p = Path(p_str)
                if p.exists():
                    blocked.append(p.resolve())
            except Exception:
                pass
    elif _OS == "Darwin":
        for p_str in ["/System", "/usr", "/bin", "/sbin", "/etc", "/var", "/private"]:
            try:
                blocked.append(Path(p_str).resolve())
            except Exception:
                pass
    else:  # Linux
        for p_str in ["/bin", "/boot", "/dev", "/etc", "/lib", "/lib64", "/proc", "/root", "/run", "/sbin", "/sys", "/usr"]:
            try:
                blocked.append(Path(p_str).resolve())
            except Exception:
                pass
    return blocked


def _is_safe_path(target: Path) -> bool:
    """True if target is safe for user operations and not a protected OS directory."""
    try:
        resolved = target.resolve()

        # Reject Windows System Volume Information and Recycle Bin on any drive
        parts_lower = [p.lower() for p in resolved.parts]
        for part in parts_lower:
            if "system volume information" in part or "$recycle.bin" in part:
                return False

        # Reject if target IS or IS INSIDE any protected OS system directory
        for b in _get_blocked_system_paths():
            try:
                if resolved == b or resolved.is_relative_to(b):
                    return False
            except Exception:
                continue

        if _OS == "Windows":
            # On Windows, all valid system drives (C:, D:, E:, etc.) are available for user files.
            drive = resolved.drive
            if not drive:
                return False
            drive_root = Path(f"{drive}\\")
            if not drive_root.exists():
                return False
            return True

        # Non-Windows (macOS, Linux):
        allowed_roots = [
            Path.home().resolve(),
            Path("/Volumes").resolve(),
            Path("/media").resolve(),
            Path("/mnt").resolve(),
        ]
        return any(
            (resolved == r or resolved.is_relative_to(r))
            for r in allowed_roots if r.exists()
        )
    except Exception:
        return False

def _get_desktop() -> Path:
    if _OS == "Linux":
        xdg = os.environ.get("XDG_DESKTOP_DIR", "")
        if xdg and Path(xdg).exists():
            return Path(xdg)
    return Path.home() / "Desktop"

def _get_downloads() -> Path:
    if _OS == "Linux":
        xdg = os.environ.get("XDG_DOWNLOAD_DIR", "")
        if xdg and Path(xdg).exists():
            return Path(xdg)
    return Path.home() / "Downloads"

def _get_documents() -> Path:
    if _OS == "Linux":
        xdg = os.environ.get("XDG_DOCUMENTS_DIR", "")
        if xdg and Path(xdg).exists():
            return Path(xdg)
    return Path.home() / "Documents"

def _get_pictures() -> Path:
    if _OS == "Linux":
        xdg = os.environ.get("XDG_PICTURES_DIR", "")
        if xdg and Path(xdg).exists():
            return Path(xdg)
    return Path.home() / "Pictures"

def _get_music() -> Path:
    if _OS == "Linux":
        xdg = os.environ.get("XDG_MUSIC_DIR", "")
        if xdg and Path(xdg).exists():
            return Path(xdg)
    return Path.home() / "Music"

def _get_videos() -> Path:
    if _OS == "Linux":
        xdg = os.environ.get("XDG_VIDEOS_DIR", "")
        if xdg and Path(xdg).exists():
            return Path(xdg)
    return Path.home() / "Videos"


def _resolve_path(raw: str) -> Path:
    shortcuts: dict[str, Path] = {
        "desktop":   _get_desktop(),
        "downloads": _get_downloads(),
        "documents": _get_documents(),
        "pictures":  _get_pictures(),
        "music":     _get_music(),
        "videos":    _get_videos(),
        "home":      Path.home(),
    }
    raw = (raw or "").strip().strip('"').strip("'")
    if not raw:
        return Path.home()

    lower = raw.lower()
    if lower in shortcuts:
        return shortcuts[lower]

    # "desktop/notes/a.md" and "desktop\notes\a.md" — a shortcut followed by a
    # sub-path.
    head, sep, rest = raw.replace("\\", "/").partition("/")
    if sep and head.lower() in shortcuts:
        rest = rest.strip("/")
        return shortcuts[head.lower()] / rest if rest else shortcuts[head.lower()]

    if _OS == "Windows":
        # Handle "E drive", "e drive", "e-drive", "E disk", etc.
        m = re.match(r"^([a-zA-Z])(?:\s*|-)(?:drive|disk)[\/\\]?(.*)$", raw, re.IGNORECASE)
        if m:
            dl = m.group(1).upper()
            sub = m.group(2).lstrip(r"\/")
            return Path(f"{dl}:\\{sub}") if sub else Path(f"{dl}:\\")

        # Handle "E:" or "e:" -> root of drive "E:\"
        if re.match(r"^[a-zA-Z]:$", raw):
            return Path(f"{raw[0].upper()}:\\")

        # Handle "E:something" without slash -> "E:\something"
        if re.match(r"^[a-zA-Z]:[^\/\\]", raw):
            return Path(f"{raw[:2]}\\{raw[2:]}")

        # Handle single letter "E" or "e" if that drive exists
        if re.match(r"^[a-zA-Z]$", raw) and Path(f"{raw.upper()}:\\").exists():
            return Path(f"{raw.upper()}:\\")

    return Path(raw).expanduser()


def _resolve_fuzzy_existing_path(p: Path) -> Path:
    """If p exists, return it. Otherwise attempt case-insensitive and phonetic
    match (e.g. Altron vs ultron) on each path component so existing files are found."""
    if p.exists():
        return p

    parts = p.parts
    if not parts:
        return p

    def _traverse(current: Path, remaining: list[str]) -> Path | None:
        if not remaining:
            return current if current.exists() else None
        part = remaining[0]
        rest = remaining[1:]
        exact = current / part
        if exact.exists():
            if rest:
                res = _traverse(exact, rest)
                if res:
                    return res
            else:
                return exact
        if current.is_dir():
            part_low = part.lower()
            p_target = part_low.replace("altron", "ultron")
            candidates = []
            try:
                for child in current.iterdir():
                    c_low = child.name.lower()
                    if c_low == part_low:
                        candidates.insert(0, child)
                    elif c_low.replace("altron", "ultron") == p_target:
                        candidates.append(child)
                for cand in candidates:
                    if rest:
                        res = _traverse(cand, rest)
                        if res:
                            return res
                    else:
                        return cand
            except Exception:
                pass
        return None

    res = _traverse(Path(parts[0]), list(parts[1:]))
    return res if res is not None else p


def _find_vscode_executable() -> str | None:
    which_code = shutil.which("code")
    if which_code:
        p = Path(which_code)
        cand = p.parent.parent / "Code.exe"
        if cand.exists():
            return str(cand)
        cand_same = p.parent / "Code.exe"
        if cand_same.exists():
            return str(cand_same)
    local_app_data = os.environ.get("LOCALAPPDATA", "")
    if local_app_data:
        cand = Path(local_app_data) / "Programs" / "Microsoft VS Code" / "Code.exe"
        if cand.exists():
            return str(cand)
    for pf_env in ("ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"):
        pf = os.environ.get(pf_env, "")
        if pf:
            cand = Path(pf) / "Microsoft VS Code" / "Code.exe"
            if cand.exists():
                return str(cand)
    return which_code


def _format_size(b: int) -> str:
    for unit in ["B", "KB", "MB", "GB", "TB"]:
        if b < 1024:
            return f"{b:.1f} {unit}"
        b /= 1024
    return f"{b:.1f} TB"

def _safe_trash(target: Path) -> str:

    if not _SEND2TRASH:
        return (
            "send2trash is not installed. "
            "Run: pip install send2trash — "
            "Permanent deletion is disabled for safety."
        )
    send2trash.send2trash(str(target))
    return f"Moved to Trash: {target.name}"


def list_files(path: str = "desktop", show_hidden: bool = False) -> str:
    try:
        target = _resolve_path(path)
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        if not target.exists():
            return f"Path not found: {target}"
        if not target.is_dir():
            return f"Not a directory: {target}"

        items = []
        for item in sorted(target.iterdir()):
            if not show_hidden and item.name.startswith("."):
                continue
            if item.is_dir():
                items.append(f"📁 {item.name}/")
            else:
                size = _format_size(item.stat().st_size)
                items.append(f"📄 {item.name} ({size})")

        if not items:
            return f"Directory is empty: {target.name}/"

        return f"Contents of {target.name}/ ({len(items)} items):\n" + "\n".join(items)

    except PermissionError:
        return f"Permission denied: {path}"
    except Exception as e:
        return f"Error listing files: {e}"


def open_folder(path: str = "desktop", name: str = "") -> str:
    try:
        base = _resolve_path(path)
        target = (base / name) if name else base
        target = _resolve_fuzzy_existing_path(target)

        if not target.exists():
            search_term = name or path
            # Search in common user locations
            for root in (_get_desktop(), _get_documents(), _get_downloads(), Path("e:/github-projects"), Path("e:/"), Path("d:/"), Path.home()):
                candidate = root / search_term
                candidate = _resolve_fuzzy_existing_path(candidate)
                if candidate.exists() and candidate.is_dir():
                    target = candidate
                    break

        if not target.exists():
            return f"Folder not found: {path} {name}".strip()

        if not target.is_dir():
            target = target.parent

        if _OS == "Windows":
            os.startfile(str(target.resolve()))
            return f"Opened folder in File Explorer: {target.name} ({target.resolve()})"
        elif _OS == "Darwin":
            subprocess.run(["open", str(target.resolve())])
            return f"Opened folder in Finder: {target.name}"
        else:
            subprocess.run(["xdg-open", str(target.resolve())])
            return f"Opened folder: {target.name}"
    except Exception as e:
        return f"Could not open folder: {e}"


def open_file(path: str, name: str = "", with_app: str = "") -> str:
    try:
        base = _resolve_path(path)
        target = (base / name) if name else base
        target = _resolve_fuzzy_existing_path(target)

        if not target.exists():
            search_term = name or path
            for root in (_get_desktop(), _get_documents(), _get_downloads(), Path("e:/github-projects"), Path("e:/"), Path("d:/"), Path.home()):
                candidate = root / search_term
                candidate = _resolve_fuzzy_existing_path(candidate)
                if candidate.exists() and candidate.is_file():
                    target = candidate
                    break

        if not target.exists():
            return f"File not found: {target.name}"

        app_clean = (with_app or "").strip().lower()

        flags = 0
        if _OS == "Windows" and hasattr(subprocess, "DETACHED_PROCESS") and hasattr(subprocess, "CREATE_NEW_PROCESS_GROUP"):
            flags = subprocess.DETACHED_PROCESS | subprocess.CREATE_NEW_PROCESS_GROUP

        # 1. VS Code
        if any(k in app_clean for k in ("vscode", "visual studio code", "vs code")) or app_clean == "code":
            if _OS == "Windows":
                code_exe = _find_vscode_executable()
                if code_exe:
                    subprocess.Popen(
                        [code_exe, str(target.resolve())],
                        creationflags=flags,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                    return f"Opened {target.name} in Visual Studio Code."
                else:
                    subprocess.Popen(
                        f'code "{target.resolve()}"',
                        shell=True,
                        creationflags=flags,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                    return f"Opened {target.name} in Visual Studio Code."
            elif _OS == "Darwin":
                subprocess.Popen(["open", "-a", "Visual Studio Code", str(target.resolve())])
                return f"Opened {target.name} in Visual Studio Code."
            else:
                subprocess.Popen(["code", str(target.resolve())])
                return f"Opened {target.name} in Visual Studio Code."

        # 2. Notepad / text editor
        if any(k in app_clean for k in ("notepad", "notebook", "text editor", "note pad")):
            if _OS == "Windows":
                notepad_exe = shutil.which("notepad") or str(Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32" / "notepad.exe")
                subprocess.Popen(
                    [notepad_exe, str(target.resolve())],
                    creationflags=flags,
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                )
                return f"Opened {target.name} in Notepad."
            elif _OS == "Darwin":
                subprocess.Popen(["open", "-a", "TextEdit", str(target.resolve())])
                return f"Opened {target.name} in TextEdit."
            else:
                subprocess.Popen(["gedit", str(target.resolve())])
                return f"Opened {target.name} in Text Editor."

        # 3. Web Browsers (Chrome, Edge, Firefox, Brave)
        if any(k in app_clean for k in ("chrome", "browser", "edge", "firefox", "brave")):
            if _OS == "Windows":
                browser_exe = None
                if "chrome" in app_clean:
                    for p in [
                        Path(os.environ.get("ProgramFiles", "")) / "Google" / "Chrome" / "Application" / "chrome.exe",
                        Path(os.environ.get("ProgramFiles(x86)", "")) / "Google" / "Chrome" / "Application" / "chrome.exe",
                        Path(os.environ.get("LOCALAPPDATA", "")) / "Google" / "Chrome" / "Application" / "chrome.exe",
                    ]:
                        if p.exists():
                            browser_exe = str(p)
                            break
                    if not browser_exe:
                        browser_exe = shutil.which("chrome")
                elif "edge" in app_clean:
                    for p in [
                        Path(os.environ.get("ProgramFiles(x86)", "")) / "Microsoft" / "Edge" / "Application" / "msedge.exe",
                        Path(os.environ.get("ProgramFiles", "")) / "Microsoft" / "Edge" / "Application" / "msedge.exe",
                    ]:
                        if p.exists():
                            browser_exe = str(p)
                            break
                    if not browser_exe:
                        browser_exe = shutil.which("msedge")

                if browser_exe:
                    subprocess.Popen(
                        [browser_exe, str(target.resolve())],
                        creationflags=flags,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                    return f"Opened {target.name} in {with_app}."
                else:
                    os.startfile(str(target.resolve()))
                    return f"Opened {target.name} in default browser."
            else:
                import webbrowser
                webbrowser.open(target.as_uri())
                return f"Opened {target.name} in browser."

        # 4. Other explicitly specified apps
        if app_clean:
            bin_path = shutil.which(app_clean) or shutil.which(f"{app_clean}.exe")
            if bin_path:
                if _OS == "Windows":
                    subprocess.Popen(
                        [bin_path, str(target.resolve())],
                        creationflags=flags,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                else:
                    subprocess.Popen([bin_path, str(target.resolve())])
                return f"Opened {target.name} in {with_app}."
            elif _OS == "Windows":
                try:
                    subprocess.Popen(
                        f'start "" "{app_clean}" "{target.resolve()}"',
                        shell=True,
                        creationflags=flags,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                    return f"Opened {target.name} in {with_app}."
                except Exception:
                    pass

        # 5. Default opening (associated application)
        if _OS == "Windows":
            os.startfile(str(target.resolve()))
            return f"Opened file: {target.name}"
        elif _OS == "Darwin":
            subprocess.run(["open", str(target.resolve())])
            return f"Opened file: {target.name}"
        else:
            subprocess.run(["xdg-open", str(target.resolve())])
            return f"Opened file: {target.name}"
    except Exception as e:
        return f"Could not open file: {e}"


def create_file(path: str, name: str = "", content: str = "") -> str:
    try:
        if name:
            name = re.sub(r"[\s_]+(?:নামে|নামায়|নামক|ফাইল)$", "", name.strip(), flags=re.IGNORECASE)
        base   = _resolve_path(path)
        target = (base / name) if name else base
        if name and base.name.lower() == name.lower():
            target = base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        target.parent.mkdir(parents=True, exist_ok=True)
        existed = target.exists()
        previous = None
        if existed:
            try:
                previous = target.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                previous = None
        target.write_text(content, encoding="utf-8")
        push_undo(f"created {target.name}",
                  _undo_write(target, previous) if existed else _undo_create(target))
        return f"File created: {target}"
    except Exception as e:
        return f"Could not create file: {e}"


def create_folder(path: str, name: str = "") -> str:
    try:
        if name:
            name = re.sub(r"[\s_]+(?:নামে|নামায়|নামক|ফোল্ডার)$", "", name.strip(), flags=re.IGNORECASE)
        base   = _resolve_path(path)
        target = (base / name) if name else base
        if name and base.name.lower() == name.lower():
            target = base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        already = target.exists()
        target.mkdir(parents=True, exist_ok=True)
        # Only offer to undo a folder we actually made. "mkdir -p" on something
        # that was already there is not a change, and undoing it would delete a
        # directory the user has had for years.
        if not already:
            push_undo(f"created folder {target.name}", _undo_create(target))
        return f"Folder created: {target}"
    except Exception as e:
        return f"Could not create folder: {e}"


def delete_file(path: str, name: str = "") -> str:
    try:
        base   = _resolve_path(path)
        target = (base / name) if name else base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        if not target.exists():
            return f"Not found: {target.name}"

        # Safe-directory check — protect critical user folders and drive roots
        if target.resolve() == Path(target.anchor).resolve():
            return f"Protected drive root, cannot delete: {target}"
        protected = {
            _get_desktop(), _get_downloads(), _get_documents(),
            _get_pictures(), _get_music(), _get_videos(), Path.home()
        }
        if target.resolve() in {p.resolve() for p in protected}:
            return f"Protected directory, cannot delete: {target.name}"

        original = target.resolve()
        result   = _safe_trash(target)
        if result.startswith("Moved to Trash"):
            push_undo(f"deleted {original.name}",
                      lambda p=original: _restore_from_trash(p))
        return result

    except PermissionError:
        return f"Permission denied: {path}"
    except Exception as e:
        return f"Could not delete: {e}"


def move_file(path: str, name: str = "", destination: str = "") -> str:
    try:
        base   = _resolve_path(path)
        src    = (base / name) if name else base
        dst    = _resolve_path(destination) if destination else None

        if not src.exists():
            return f"Source not found: {src.name}"
        if dst is None:
            return "No destination specified."
        if not _is_safe_path(src):
            return f"Access denied (source): {src}"
        if not _is_safe_path(dst):
            return f"Access denied (destination): {dst}"

        if dst.is_dir():
            dst = dst / src.name

        dst.parent.mkdir(parents=True, exist_ok=True)
        origin = src.resolve()
        shutil.move(str(src), str(dst))
        push_undo(f"moved {origin.name} to {dst.parent.name}/",
                  _undo_move(origin, dst.resolve()))
        return f"Moved: {src.name} → {dst.parent.name}/"

    except Exception as e:
        return f"Could not move: {e}"


def copy_file(path: str, name: str = "", destination: str = "") -> str:
    try:
        base = _resolve_path(path)
        src  = (base / name) if name else base
        dst  = _resolve_path(destination) if destination else None

        if not src.exists():
            return f"Source not found: {src.name}"
        if dst is None:
            return "No destination specified."
        if not _is_safe_path(src):
            return f"Access denied (source): {src}"
        if not _is_safe_path(dst):
            return f"Access denied (destination): {dst}"

        if dst.is_dir():
            dst = dst / src.name

        dst.parent.mkdir(parents=True, exist_ok=True)

        if src.is_dir():
            shutil.copytree(str(src), str(dst))
        else:
            shutil.copy2(str(src), str(dst))

        # The undo for a copy is deleting the copy — never the original.
        _copy = dst.resolve()
        def _undo_copy():
            if not _copy.exists():
                return f"The copy '{_copy.name}' is already gone."
            if _copy.is_dir():
                shutil.rmtree(_copy)
            else:
                _copy.unlink()
            return f"Removed the copy in {_copy.parent.name}/."
        push_undo(f"copied {src.name} to {dst.parent.name}/", _undo_copy)

        return f"Copied: {src.name} → {dst.parent.name}/"

    except Exception as e:
        return f"Could not copy: {e}"


def rename_file(path: str, name: str = "", new_name: str = "") -> str:
    try:
        base     = _resolve_path(path)
        target   = (base / name) if name else base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        if not target.exists():
            return f"Not found: {target.name}"
        if not new_name:
            return "No new name provided."

        new_path = target.parent / new_name
        if new_path.exists():
            return f"A file named '{new_name}' already exists here."

        old_path = target.resolve()
        target.rename(new_path)
        push_undo(f"renamed {old_path.name} to {new_name}",
                  _undo_move(old_path, new_path.resolve()))
        return f"Renamed: {target.name} → {new_name}"

    except Exception as e:
        return f"Could not rename: {e}"


def read_file(path: str, name: str = "", max_chars: int = 4000) -> str:
    try:
        base   = _resolve_path(path)
        target = (base / name) if name else base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        if not target.exists():
            return f"File not found: {target.name}"
        if not target.is_file():
            return f"Not a file: {target.name}"

        content = target.read_text(encoding="utf-8", errors="ignore")
        if len(content) > max_chars:
            content = content[:max_chars] + f"\n\n[Truncated — {len(content)} total chars]"
        return content

    except Exception as e:
        return f"Could not read file: {e}"


def write_file(path: str, name: str = "", content: str = "",
               append: bool = False) -> str:
    try:
        base   = _resolve_path(path)
        target = (base / name) if name else base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        target.parent.mkdir(parents=True, exist_ok=True)

        # Snapshot before writing. None means "did not exist", which is a
        # different undo (delete it) from "existed and had this in it".
        previous: str | None = None
        undoable = True
        if target.exists():
            try:
                if target.stat().st_size > _UNDO_CONTENT_LIMIT:
                    undoable = False       # too large to hold in memory
                else:
                    previous = target.read_text(encoding="utf-8", errors="ignore")
            except Exception:
                undoable = False           # binary, locked, unreadable

        mode = "a" if append else "w"
        with open(target, mode, encoding="utf-8") as f:
            f.write(content)

        action = "Appended to" if append else "Written to"
        if undoable:
            push_undo(f"wrote to {target.name}", _undo_write(target, previous))
            return f"{action}: {target.name}"
        return (f"{action}: {target.name}. "
                f"(Too large to keep a copy of the old contents, so this one "
                f"cannot be undone.)")
    except Exception as e:
        return f"Could not write file: {e}"


def find_files(name: str = "", extension: str = "",
               path: str = "home", max_results: int = 20) -> str:
    try:
        search_path = _resolve_path(path)
        if not _is_safe_path(search_path):
            return f"Access denied: {search_path}"
        if not search_path.exists():
            return f"Search path not found: {path}"

        results    = []
        dir_count  = 0
        max_dirs   = 500  # performance + safety limit

        for item in search_path.rglob("*"):
            if item.is_dir():
                dir_count += 1
                if dir_count > max_dirs:
                    break
                continue
            if not item.is_file():
                continue
            if extension and item.suffix.lower() != extension.lower():
                continue
            if name and name.lower() not in item.name.lower():
                continue
            size = _format_size(item.stat().st_size)
            results.append(f"📄 {item.name} ({size}) — {item.parent}")
            if len(results) >= max_results:
                break

        if not results:
            query = name or extension or "files"
            return f"No {query} found in {search_path.name}/"

        return f"Found {len(results)} file(s):\n" + "\n".join(results)

    except Exception as e:
        return f"Search error: {e}"


def get_largest_files(path: str = "downloads", count: int = 10) -> str:
    count = min(count, 50)  # maksimum 50
    try:
        search_path = _resolve_path(path)
        if not _is_safe_path(search_path):
            return f"Access denied: {search_path}"
        if not search_path.exists():
            return f"Path not found: {path}"

        files = []
        for item in search_path.rglob("*"):
            if item.is_file():
                try:
                    files.append((item.stat().st_size, item))
                except Exception:
                    continue

        files.sort(reverse=True)
        top = files[:count]

        if not top:
            return "No files found."

        lines = [f"Top {len(top)} largest files in {search_path.name}/:"]
        for size, f in top:
            lines.append(f"  {_format_size(size):>10}  {f.name}  ({f.parent})")

        return "\n".join(lines)

    except Exception as e:
        return f"Error: {e}"


def get_disk_usage(path: str = "home") -> str:
    try:
        target = _resolve_path(path)
        usage  = shutil.disk_usage(target)
        pct    = usage.used / usage.total * 100
        return (
            f"Disk usage ({target}):\n"
            f"  Total : {_format_size(usage.total)}\n"
            f"  Used  : {_format_size(usage.used)} ({pct:.1f}%)\n"
            f"  Free  : {_format_size(usage.free)}"
        )
    except Exception as e:
        return f"Could not get disk usage: {e}"


def organize_desktop() -> str:
    type_map = {
        "Images":    {".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".svg", ".ico", ".heic"},
        "Documents": {".pdf", ".doc", ".docx", ".txt", ".xls", ".xlsx",
                      ".ppt", ".pptx", ".csv", ".odt", ".ods", ".odp"},
        "Videos":    {".mp4", ".avi", ".mkv", ".mov", ".wmv", ".flv", ".webm", ".m4v"},
        "Music":     {".mp3", ".wav", ".flac", ".aac", ".ogg", ".wma", ".m4a"},
        "Archives":  {".zip", ".rar", ".7z", ".tar", ".gz", ".bz2", ".xz"},
        "Code":      {".py", ".js", ".ts", ".html", ".css", ".json", ".xml",
                      ".cpp", ".java", ".cs", ".go", ".rs", ".sh"},
    }

    desktop = _get_desktop()
    moved, skipped = [], []
    journal: list[tuple[Path, Path]] = []   # (where it was, where it went)

    try:
        for item in desktop.iterdir():
            # Leave folders, hidden files and organize-folders untouched
            if item.is_dir() or item.name.startswith("."):
                continue
            if item.name in {k for k in type_map}:
                continue

            ext        = item.suffix.lower()
            target_dir = desktop / "Others"
            for folder, exts in type_map.items():
                if ext in exts:
                    target_dir = desktop / folder
                    break

            target_dir.mkdir(exist_ok=True)
            new_path = target_dir / item.name

            if new_path.exists():
                skipped.append(item.name)
                continue

            origin = item.resolve()
            shutil.move(str(item), str(new_path))
            journal.append((origin, new_path.resolve()))
            moved.append(f"{item.name} → {target_dir.name}/")

        # One command, dozens of moves — so one undo that reverses all of them.
        # Without this, "organize my desktop" is the single least reversible
        # thing the assistant can do to a person's files, and it was completely
        # ungated.
        if journal:
            def _undo_organize(entries=tuple(journal)):
                restored = 0
                for origin, moved_to in entries:
                    try:
                        if moved_to.exists():
                            origin.parent.mkdir(parents=True, exist_ok=True)
                            shutil.move(str(moved_to), str(origin))
                            restored += 1
                    except Exception as e:
                        print(f"[file] undo organize: {moved_to.name}: {e}")
                # Clear away the folders we created, but only while they are
                # empty — anything the user put in since stays.
                for folder in {m.parent for _o, m in entries}:
                    try:
                        if folder.exists() and folder.is_dir() and not any(folder.iterdir()):
                            folder.rmdir()
                    except Exception:
                        pass
                return f"{restored} file(s) put back on the desktop."
            push_undo(f"organized the desktop ({len(journal)} files)", _undo_organize)

        result = f"Desktop organized: {len(moved)} files moved."
        if moved:
            preview = moved[:8]
            result += "\n" + "\n".join(preview)
            if len(moved) > 8:
                result += f"\n... and {len(moved) - 8} more."
        if skipped:
            result += f"\n{len(skipped)} file(s) skipped (name conflict)."
        return result

    except Exception as e:
        return f"Could not organize desktop: {e}"


def get_file_info(path: str, name: str = "") -> str:
    try:
        base   = _resolve_path(path)
        target = (base / name) if name else base
        if not _is_safe_path(target):
            return f"Access denied: {target}"
        if not target.exists():
            return f"Not found: {target.name}"

        stat = target.stat()
        info = {
            "Name":      target.name,
            "Type":      "Folder" if target.is_dir() else "File",
            "Size":      _format_size(stat.st_size),
            "Location":  str(target.parent),
            "Created":   datetime.fromtimestamp(stat.st_ctime).strftime("%Y-%m-%d %H:%M"),
            "Modified":  datetime.fromtimestamp(stat.st_mtime).strftime("%Y-%m-%d %H:%M"),
            "Extension": target.suffix or "—",
        }
        return "\n".join(f"  {k}: {v}" for k, v in info.items())

    except Exception as e:
        return f"Could not get file info: {e}"

def file_controller(
    parameters: dict = None,
    response=None,
    player=None,
    session_memory=None,
) -> str:
    params = parameters or {}
    action = params.get("action", "").lower().strip()
    path   = params.get("path", "desktop")
    name   = params.get("name", "")

    if player:
        player.write_log(f"[file] {action} {name or path}")

    try:
        if action == "list":
            return list_files(path)

        elif action in ("open_folder", "open_dir", "open_directory", "show_in_folder", "open"):
            return open_folder(path, name=name)

        elif action in ("open_file", "launch_file"):
            with_app = params.get("with_app") or params.get("app") or params.get("editor", "")
            return open_file(path, name=name, with_app=with_app)

        elif action == "create_file":
            return create_file(path, name=name, content=params.get("content", ""))

        elif action == "create_folder":
            return create_folder(path, name=name)

        elif action == "delete":
            return delete_file(path, name=name)

        elif action == "move":
            return move_file(path, name=name, destination=params.get("destination", ""))

        elif action == "copy":
            return copy_file(path, name=name, destination=params.get("destination", ""))

        elif action == "rename":
            return rename_file(path, name=name, new_name=params.get("new_name", ""))

        elif action == "read":
            return read_file(path, name=name)

        elif action == "write":
            return write_file(
                path, name=name,
                content=params.get("content", ""),
                append=params.get("append", False)
            )

        elif action == "find":
            return find_files(
                name=name or params.get("name", ""),
                extension=params.get("extension", ""),
                path=path,
                max_results=min(int(params.get("max_results", 20)), 50),
            )

        elif action == "largest":
            return get_largest_files(
                path=path,
                count=int(params.get("count", 10)),
            )

        elif action == "disk_usage":
            return get_disk_usage(path)

        elif action == "organize_desktop":
            return organize_desktop()

        elif action == "info":
            return get_file_info(path, name=name)

        else:
            return f"Unknown action: '{action}'"

    except Exception as e:
        return f"File controller error ({action}): {e}"


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
    "name": "file_controller",
    "description": "Manages and opens files and folders: open_folder (opens in Windows File Explorer GUI), open_file (opens file in default app or specified app like vscode, notepad, chrome), list, create_file, create_folder, delete, move, copy, rename, read, write, find, disk usage.",
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": "open_folder | open_file | list | create_file | create_folder | delete | move | copy | rename | read | write | find | largest | disk_usage | organize_desktop | info"
            },
            "path": {
                "type": "STRING",
                "description": "File/folder path or shortcut: desktop, downloads, documents, home"
            },
            "name": {
                "type": "STRING",
                "description": "File name or search term"
            },
            "with_app": {
                "type": "STRING",
                "description": "Optional application to open the file with (e.g. 'vscode', 'notepad', 'chrome', 'edge')"
            },
            "app": {
                "type": "STRING",
                "description": "Alias for with_app"
            },
            "destination": {
                "type": "STRING",
                "description": "Destination path for move/copy"
            },
            "new_name": {
                "type": "STRING",
                "description": "New name for rename"
            },
            "content": {
                "type": "STRING",
                "description": "Content for create_file/write"
            },
            "extension": {
                "type": "STRING",
                "description": "File extension to search (e.g. .pdf)"
            },
            "count": {
                "type": "INTEGER",
                "description": "Number of results for largest"
            }
        },
        "required": [
            "action"
        ]
    },
    "handler": file_controller,
}
