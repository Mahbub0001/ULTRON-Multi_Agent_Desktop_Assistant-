# Agent Town Visual Cyber-Office Specification for Mark-LIV

## Overview
This specification details the interactive visual "Living Office" for Mark-LIV's Agent Town, inspired by Stonic AI. It provides an animated, software-rendered 2D/isometric cyber-office floor where Alice, Bob, Carol, and Dave sit at specialized workstations, communicate in real time through floating speech bubbles, exchange collaboration laser packets, and respond to direct clicks.

## Architecture & Components

### 1. Canvas Engine (`TownOfficeCanvas` in `ui.py`)
- **Technology:** Pure PyQt6 `QPainter` with 30 FPS animation timer (`QTimer`, 33ms tick).
- **Zero Heavy Dependencies:** No OpenGL or external graphics assets required; drawn dynamically using vector geometry, gradients, and typography.
- **Cyber-Office Floor Plan:**
  - Perspective isometric grid with ambient neon scanlines.
  - Central data core connecting the four perimeter desks.
  - Four dedicated workstations:
    - **Alice (Top-Left):** Cyan `#00d4ff` — Holographic data rings and analytics screens.
    - **Bob (Top-Right):** Emerald `#00ff88` — Holographic terminal screens and code streams.
    - **Carol (Bottom-Left):** Amber `#ffcc00` — Server telemetry racks and system dials.
    - **Dave (Bottom-Right):** Orange `#ff6b00` — Floating document sheets and drafting tablet.

### 2. Real-Time Character Interactions & Living Behaviors
- **Animated Avatars:** Characters feature subtle idle breathing motion (`math.sin(t)`), glowing status halos, and avatar badges.
- **Floating Dialogue & Speech Bubbles:**
  - Speech bubbles appear above character heads with smooth opacity fade and rise.
  - When `WORKING`: Bubbles display real-time active tasks (e.g. *"⚡ Writing cleanup.py"*).
  - When `IDLE`: Characters exchange witty ambient dialogue and office commentary (*"Paper jam again? 😄"*, *"Alice: Cross-referencing arXiv benchmarks"*, *"Bob: Coffee break ☕"*, etc.).
- **Inter-Agent Collaboration Laser Packets:**
  - When agents delegate tasks to one another, neon energy packets traverse the grid lines between desks.

### 3. User Interaction
- **Hover Feedback:** Workstations highlight with glowing neon borders when hovered over.
- **Click to Focus / Dispatch:** Clicking any desk immediately opens that agent's direct interaction console.
- **View Switching:** Seamless toggle between "◈ OFFICE FLOOR" (visual canvas) and "≡ DESK CARDS" (tabular list).
