# Windows executable build

Build from the project root with an environment containing the runtime dependencies and PyInstaller. The spec explicitly collects dynamically loaded core, action, plugin and memory modules.

```powershell
python -m PyInstaller --noconfirm ultron.spec
python prepare_app_bundle.py
dist\ULTRON\ULTRON.exe --check-bundle scratch\bundle-check.json
& "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe" installer.iss
```

The application is `dist/ULTRON/ULTRON.exe`; its adjacent `_internal`, actions, plugins, core and other directories are required. Do not move just this EXE. The installer is `dist/ultron_nibir.exe`.

For a fresh staging build, supply `--distpath scratch/exe-dist --workpath scratch/exe-build` to PyInstaller and `--destination scratch/exe-dist/ULTRON` to the bundle preparation script. This avoids replacing an existing bundle during compilation.

`--check-bundle REPORT_PATH` exits without opening a window, microphone or live session. It checks the relevant imports, assets and current phone/browser tool declarations, writes a JSON report and returns a nonzero exit code on failure. This checks packaging; it does not prove live Gemini, phone connectivity or every desktop workflow works.

The prepared distribution contains empty credential and user-data defaults. The installer preserves existing credentials, contacts, local calendar and memory JSON files when updating. Generated executables, runtime reports and personal settings stay outside Git.
