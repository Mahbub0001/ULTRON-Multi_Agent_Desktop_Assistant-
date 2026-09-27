# Agent Town Visual Cyber-Office Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an animated, interactive 2D Cyber-Office Canvas ("Living Office") for Mark-LIV's Agent Town where characters sit at desks, chat in real time via speech bubbles, exchange laser collaboration packets, and respond to clicks.

**Architecture:** A custom PyQt6 widget `TownOfficeCanvas` drawn with `QPainter` on a 30 FPS timer; integrated into `AgentTownDrawer` with tabbed or split views for the visual floor and detailed desk cards.

**Tech Stack:** PyQt6 (`QPainter`, `QTimer`, `QPen`, `QBrush`, `QColor`), Python 3.10+, unittest.

---

### Task 1: Write Unit Test for `TownOfficeCanvas`

**Files:**
- Create: `tests/test_town_canvas.py`

- [ ] **Step 1: Write tests for canvas instantiation, desk detection, and bubble updates**
- [ ] **Step 2: Run test to verify it fails initially**

---

### Task 2: Implement `TownOfficeCanvas` in `ui.py`

**Files:**
- Modify: `ui.py`

- [ ] **Step 1: Implement `TownOfficeCanvas` with isometric floor, desks, characters, and speech bubbles**
- [ ] **Step 2: Implement mouse move/hover and click detection**
- [ ] **Step 3: Implement inter-agent collaboration packet animations**

---

### Task 3: Integrate Canvas into `AgentTownDrawer`

**Files:**
- Modify: `ui.py`

- [ ] **Step 1: Add view toggle ("◈ OFFICE FLOOR" / "≡ DESK CARDS")**
- [ ] **Step 2: Wire desk clicks to agent task dispatch and report dialogs**

---

### Task 4: Full Test Verification & Evidence

- [ ] **Step 1: Run complete test suite**
- [ ] **Step 2: Verify git status and commit**
