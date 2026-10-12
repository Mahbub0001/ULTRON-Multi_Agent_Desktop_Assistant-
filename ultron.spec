# -*- mode: python ; coding: utf-8 -*-
from PyInstaller.utils.hooks import collect_all, collect_submodules

hiddenimports = [
    'sounddevice',
    'numpy',
    'google.genai',
    'google.genai.types',
    'PyQt6',
    'PyQt6.QtCore',
    'PyQt6.QtWidgets',
    'PyQt6.QtGui',
    'miniaudio',
    'edge_tts',
    'yt_dlp',
    'pyautogui',
    'pyperclip',
    'pygetwindow',
    'PIL',
    'cv2',
    'mss',
    'psutil',
    'send2trash',
    'docx',
    'pptx',
    'pdfplumber',
    'PyPDF2',
    'requests',
    'bs4',
    'ddgs',
    'duckduckgo_search',
    'fastapi',
    'uvicorn',
    'cryptography',
    'comtypes',
    'pycaw',
    'win10toast',
    'pywinauto',
    'wmi',
    'pynvml',
    'youtube_transcript_api',
    'core',
    'core.gemini',
    'core.action_loader',
    'core.plugin_loader',
    'core.avatar',
    'memory',
    'memory.memory_manager',
    'memory.config_manager',
]

datas = []
binaries = []

# Tools are loaded from source files at runtime, so static analysis cannot see
# their imports. Freeze the packages as well as copying their editable sources.
for package in ('core', 'actions', 'plugins', 'memory'):
    hiddenimports += collect_submodules(package, on_error='warn')

packages_to_collect = [
    'google.genai',
    'sounddevice',
    'miniaudio',
    'edge_tts',
    'yt_dlp',
    'fastapi',
    'uvicorn',
    'pycaw',
    'win10toast',
    'pywinauto',
    'wmi',
]

for pkg in packages_to_collect:
    try:
        d, b, h = collect_all(pkg)
        datas += d
        binaries += b
        hiddenimports += h
    except Exception as e:
        print(f"collect_all({pkg}) notice: {e}")

a = Analysis(
    ['main.py'],
    pathex=['.'],
    binaries=binaries,
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        'torch',
        'torchvision',
        'torchaudio',
        'ultralytics',
        'ultralytics_thop',
        'scipy',
        'pandas',
        'matplotlib',
        'IPython',
        'jupyter',
        'notebook',
        'tkinter',
        'pytest',
    ],
    noarchive=False,
    optimize=0,
)
# Modern Windows provides these APIs. DLLs accidentally found through a
# third-party PATH can shadow the OS runtime and break Qt's procedure imports.
a.binaries = [entry for entry in a.binaries
              if not entry[0].lower().startswith('api-ms-win-')
              and entry[0].lower() != 'ucrtbase.dll'
              and 'codex-runtimes' not in entry[1].lower()]
pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name='ULTRON',
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=True,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
    icon=['config/jarvis.ico'],
)
coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=True,
    upx_exclude=[],
    name='ULTRON',
)
