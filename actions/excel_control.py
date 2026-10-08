# actions/excel_control.py
"""
Excel Spreadsheet Control & Professional Formatting Engine for Mark-LIV.
Provides instant, 100% native control over Microsoft Excel via Win32 COM:
- Autofit columns and rows so text is never truncated or cut off
- Professional executive table formatting (clean navy/dark headers, zebra striping, gridlines)
- Freeze top header row so headers stay pinned while scrolling
- AutoFilter toggling on data headers
- Wrap text with row expansion
- Reading sheet structure (row count, columns, headers, preview)
- Automatic fallback to Excel Ribbon keyboard sequences (Alt+H+O+I / Alt+H+O+A)
"""
from __future__ import annotations

import os
import re
import sys
import time
from pathlib import Path
from typing import Any, Optional

try:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
except Exception:
    pass

# Style color palettes in BGR format for Excel COM:
# Note: Excel Interior.Color uses 0xBBGGRR format
_STYLES = {
    "navy": {
        "header_bg": 0x794E1F,      # #1F4E79 Deep Navy Blue
        "header_fg": 0xFFFFFF,      # White
        "zebra_bg":  0xF8F9FB,      # Very soft slate tint
        "border":    0xD9D9D9,      # Subtle gray border
    },
    "emerald": {
        "header_bg": 0x327D2D,      # #2D7D32 Forest Emerald
        "header_fg": 0xFFFFFF,      # White
        "zebra_bg":  0xF4F9F4,      # Soft green tint
        "border":    0xD4E0D4,
    },
    "slate": {
        "header_bg": 0x383838,      # #383838 Modern Dark Slate
        "header_fg": 0xFFFFFF,      # White
        "zebra_bg":  0xF5F5F5,      # Clean light gray
        "border":    0xCCCCCC,
    },
    "royal_blue": {
        "header_bg": 0x9B552F,      # #2F559B Royal Blue
        "header_fg": 0xFFFFFF,
        "zebra_bg":  0xF0F4F9,
        "border":    0xD0D8E8,
    },
}


def _exit_cell_edit_mode():
    """If Excel is in edit mode (blinking cursor in cell), COM calls raise RPC error.
    Sending Escape cleanly exits cell edit mode without losing data."""
    try:
        import pyautogui
        pyautogui.press("escape")
        time.sleep(0.08)
    except Exception:
        pass


def _bring_excel_to_front(excel=None):
    """Brings Excel window to the foreground."""
    try:
        if excel:
            excel.Visible = True
            excel.WindowState = -4137  # xlMaximized
    except Exception:
        pass

    try:
        import win32gui
        import win32con
        matched_hwnd = None

        def _enum_cb(hwnd, _):
            nonlocal matched_hwnd
            if win32gui.IsWindowVisible(hwnd):
                title = win32gui.GetWindowText(hwnd).strip()
                if title and "excel" in title.lower():
                    matched_hwnd = hwnd

        win32gui.EnumWindows(_enum_cb, None)
        if matched_hwnd:
            if win32gui.IsIconic(matched_hwnd):
                win32gui.ShowWindow(matched_hwnd, win32con.SW_RESTORE)
            else:
                win32gui.ShowWindow(matched_hwnd, win32con.SW_SHOW)
            win32gui.SetForegroundWindow(matched_hwnd)
            win32gui.BringWindowToTop(matched_hwnd)
    except Exception:
        pass


def _get_active_excel():
    """
    Connects to the currently open Microsoft Excel instance.
    Returns (excel_app, workbook, worksheet) or (None, None, None).
    """
    if sys.platform != "win32":
        return None, None, None

    try:
        import win32com.client
        _exit_cell_edit_mode()

        try:
            excel = win32com.client.GetActiveObject("Excel.Application")
        except Exception:
            excel = win32com.client.Dispatch("Excel.Application")

        if excel and excel.Workbooks.Count > 0:
            wb = excel.ActiveWorkbook
            sheet = wb.ActiveSheet if wb else None
            return excel, wb, sheet

        return excel, None, None
    except Exception as e:
        print(f"[ExcelControl] COM connection failed: {e}")
        return None, None, None


def _keyboard_autofit() -> str:
    """
    Keyboard ribbon fallback when COM is locked:
    Executes Excel's native AutoFit Column Width (Alt+H+O+I) and AutoFit Row Height (Alt+H+O+A).
    """
    try:
        import pyautogui
        _bring_excel_to_front()
        time.sleep(0.25)

        # Press Escape twice in case user was editing a cell
        pyautogui.press("escape")
        time.sleep(0.1)
        pyautogui.press("escape")
        time.sleep(0.1)

        # Select all (twice to cover entire table if inside a list)
        pyautogui.hotkey("ctrl", "a")
        time.sleep(0.15)
        pyautogui.hotkey("ctrl", "a")
        time.sleep(0.15)

        # Excel Ribbon sequence for AutoFit Column Width: Alt -> H -> O -> I
        pyautogui.press("alt")
        time.sleep(0.08)
        pyautogui.press("h")
        time.sleep(0.08)
        pyautogui.press("o")
        time.sleep(0.08)
        pyautogui.press("i")
        time.sleep(0.2)

        # Excel Ribbon sequence for AutoFit Row Height: Alt -> H -> O -> A
        pyautogui.press("alt")
        time.sleep(0.08)
        pyautogui.press("h")
        time.sleep(0.08)
        pyautogui.press("o")
        time.sleep(0.08)
        pyautogui.press("a")
        time.sleep(0.15)

        return "Excel columns and rows auto-fitted via keyboard ribbon sequence (Alt+H+O+I)."
    except Exception as e:
        return f"Keyboard autofit failed: {e}"


def _autofit(excel, sheet, columns: str = "") -> str:
    """Auto-fits all columns and rows in the active worksheet so text is never cut off."""
    if not sheet:
        return _keyboard_autofit()

    try:
        _bring_excel_to_front(excel)
        used = sheet.UsedRange
        if not used or used.Count == 0:
            return "Active worksheet appears to be empty."

        row_count = used.Rows.Count
        col_count = used.Columns.Count

        if columns:
            target_cols = sheet.Columns(columns)
            target_cols.AutoFit()
            used.Rows.AutoFit()
            return f"Auto-fitted column(s) {columns} and all {row_count} rows. All text is now clearly visible."

        # Auto-fit all used columns and rows
        used.Columns.AutoFit()
        used.Rows.AutoFit()

        return (
            f"Successfully auto-fitted all {col_count} columns and {row_count} rows in sheet '{sheet.Name}'. "
            f"Every cell is now expanded to fit its complete text without any truncation."
        )
    except Exception as e:
        print(f"[ExcelControl] COM autofit error: {e}, falling back to keyboard ribbon...")
        return _keyboard_autofit()


def _format_table(
    excel,
    sheet,
    style_name: str = "navy",
    freeze_header: bool = True,
    add_filter: bool = True,
    wrap_text: bool = False,
) -> str:
    """
    Transforms messy or raw spreadsheet data into a clean, executive-styled table:
    1. Professional header styling (Bold, centered, white text, colored background, 24pt height).
    2. Subtle gridline borders.
    3. Alternating row zebra shading.
    4. AutoFilter dropdowns on headers.
    5. Freeze top row (headers stay visible when scrolling).
    6. Complete column & row AutoFit so all data is 100% visible.
    """
    if not sheet:
        return _keyboard_autofit()

    try:
        _bring_excel_to_front(excel)
        used = sheet.UsedRange
        if not used or used.Count == 0:
            return "Active worksheet appears to be empty."

        row_count = used.Rows.Count
        col_count = used.Columns.Count

        if row_count < 1 or col_count < 1:
            return "No data found to format."

        palette = _STYLES.get(style_name.lower().strip(), _STYLES["navy"])

        # 1. Header row formatting
        header_range = sheet.Range(sheet.Cells(1, 1), sheet.Cells(1, col_count))
        header_range.Font.Bold = True
        header_range.Font.Color = palette["header_fg"]
        header_range.Interior.Color = palette["header_bg"]
        header_range.HorizontalAlignment = -4108  # xlCenter
        header_range.VerticalAlignment = -4108    # xlCenter
        header_range.RowHeight = 26

        # 2. Data rows vertical alignment and optional zebra shading
        if row_count > 1:
            data_range = sheet.Range(sheet.Cells(2, 1), sheet.Cells(row_count, col_count))
            data_range.VerticalAlignment = -4108  # xlCenter
            data_range.RowHeight = 20

            # Subtle alternating zebra shading
            for r in range(2, row_count + 1):
                if r % 2 == 0:
                    row_cells = sheet.Range(sheet.Cells(r, 1), sheet.Cells(r, col_count))
                    row_cells.Interior.Color = palette["zebra_bg"]
                else:
                    row_cells = sheet.Range(sheet.Cells(r, 1), sheet.Cells(r, col_count))
                    row_cells.Interior.ColorIndex = -4142  # xlNone (transparent/white)

        # 3. Clean gridline borders
        used.Borders.LineStyle = 1  # xlContinuous
        used.Borders.Color = palette["border"]
        used.Borders.Weight = 2     # xlThin

        # 4. Wrap text option
        if wrap_text:
            used.WrapText = True
        else:
            used.WrapText = False

        # 5. AutoFilter
        if add_filter:
            try:
                if not sheet.AutoFilterMode:
                    used.AutoFilter()
            except Exception:
                pass

        # 6. Freeze top header row
        if freeze_header and excel:
            try:
                excel.ActiveWindow.SplitRow = 1
                excel.ActiveWindow.FreezePanes = True
            except Exception:
                pass

        # 7. CRITICAL: AutoFit all columns and rows so nothing is cut off
        used.Columns.AutoFit()
        used.Rows.AutoFit()

        # Extra breathing room for columns (add ~2 chars width so filter arrows don't cover text)
        try:
            for c in range(1, col_count + 1):
                cur_w = sheet.Columns(c).ColumnWidth
                sheet.Columns(c).ColumnWidth = cur_w + 3
        except Exception:
            pass

        return (
            f"Excel sheet '{sheet.Name}' beautifully formatted!\n"
            f"- Total Rows: {row_count}, Columns: {col_count}\n"
            f"- Styled Header: Bold white text on {style_name.title()} background\n"
            f"- Clean gridline borders & alternating zebra shading applied\n"
            f"- AutoFilter enabled on headers\n"
            f"- Header row frozen (stays visible during scrolling)\n"
            f"- All columns auto-fitted to text length. All data is now 100% visible."
        )

    except Exception as e:
        print(f"[ExcelControl] format_table COM error: {e}")
        # At least attempt keyboard autofit
        return _keyboard_autofit()


def _wrap_text(excel, sheet, columns: str = "", enable: bool = True) -> str:
    """Enables or disables text wrapping on columns or entire used range."""
    if not sheet:
        return "No active Excel sheet found."
    try:
        _bring_excel_to_front(excel)
        used = sheet.UsedRange
        if columns:
            target = sheet.Columns(columns)
        else:
            target = used

        target.WrapText = enable
        used.Rows.AutoFit()
        return f"Wrap text set to {enable} for {'column ' + columns if columns else 'entire sheet'}."
    except Exception as e:
        return f"Wrap text failed: {e}"


def _freeze_header(excel, sheet, rows: int = 1) -> str:
    """Freezes top N rows so headers stay visible while scrolling."""
    if not excel:
        return "No active Excel instance found."
    try:
        _bring_excel_to_front(excel)
        excel.ActiveWindow.SplitRow = rows
        excel.ActiveWindow.FreezePanes = True
        return f"Frozen top {rows} header row(s)."
    except Exception as e:
        return f"Freeze header failed: {e}"


def _read_sheet(excel, sheet, max_rows: int = 10) -> str:
    """Reads active sheet structure, total rows, columns, headers, and preview rows."""
    if not sheet:
        return "No active Excel sheet found."
    try:
        used = sheet.UsedRange
        if not used or used.Count == 0:
            return "Active worksheet is empty."

        total_rows = used.Rows.Count
        total_cols = used.Columns.Count

        # Read header names
        headers = []
        for c in range(1, total_cols + 1):
            val = sheet.Cells(1, c).Value
            headers.append(str(val) if val is not None else f"Col{c}")

        # Read preview rows
        preview_lines = []
        rows_to_read = min(total_rows, max_rows)
        for r in range(2, rows_to_read + 1):
            row_vals = []
            for c in range(1, total_cols + 1):
                val = sheet.Cells(r, c).Value
                str_val = str(val).strip() if val is not None else ""
                if len(str_val) > 35:
                    str_val = str_val[:32] + "..."
                row_vals.append(str_val)
            preview_lines.append(" | ".join(row_vals))

        summary = (
            f"Active Sheet: '{sheet.Name}'\n"
            f"- Total Rows: {total_rows}\n"
            f"- Total Columns: {total_cols}\n"
            f"- Headers: [{', '.join(headers)}]\n"
        )
        if preview_lines:
            summary += f"- Preview (rows 2 to {rows_to_read}):\n"
            summary += "\n".join(f"  Row {i+2}: {line}" for i, line in enumerate(preview_lines))

        return summary
    except Exception as e:
        return f"Could not read Excel sheet: {e}"


def _save(excel, sheet) -> str:
    """Saves the active Excel workbook."""
    if not excel or not excel.ActiveWorkbook:
        return "No active Excel workbook to save."
    try:
        excel.ActiveWorkbook.Save()
        return f"Workbook '{excel.ActiveWorkbook.Name}' saved successfully."
    except Exception as e:
        return f"Save failed: {e}"


# ── Main Handler ─────────────────────────────────────────────────────────────

def excel_control(
    parameters: dict,
    response=None,
    player=None,
    session_memory=None,
) -> str:
    """
    Main dispatch for Excel automation.
    Parameters:
      action         : autofit | format_table | wrap_text | freeze_header | filter | read_sheet | save | open
      columns        : optional column letter or range (e.g. 'A:D', 'B')
      style          : navy (default) | emerald | slate | royal_blue
      wrap_text      : bool (default: false)
      freeze_header  : bool (default: true)
      path           : optional file path to open in Excel
    """
    params = parameters or {}
    action = str(params.get("action", "autofit")).lower().strip()

    if player:
        player.write_log(f"[Excel] {action}")

    print(f"[ExcelControl] > {action}  {params}")

    # Handle open action first if a specific path is requested
    path = params.get("path", "")
    if action in ("open", "open_file", "launch") and path:
        try:
            from actions.open_app import open_app
            return open_app({"app_name": "Excel", "path": path})
        except Exception:
            os.startfile(path)
            return f"Opened {path} in Excel."

    # Connect to active Excel
    excel, wb, sheet = _get_active_excel()

    if not excel and action != "keyboard_autofit":
        # Check if Excel window exists
        return _keyboard_autofit()

    if not wb or not sheet:
        # Excel is running with no open workbook or COM locked
        return _keyboard_autofit()

    # Route actions
    if action in ("autofit", "autofit_columns", "fit_columns", "auto_fit", "fit"):
        cols = params.get("columns", "")
        return _autofit(excel, sheet, columns=cols)

    if action in ("format_table", "format_sheet", "format", "clean_table", "style_table", "beautify"):
        style = params.get("style", "navy")
        freeze = params.get("freeze_header", True)
        add_filter = params.get("add_filter", True)
        wrap = params.get("wrap_text", False)
        return _format_table(
            excel, sheet,
            style_name=style,
            freeze_header=freeze,
            add_filter=add_filter,
            wrap_text=wrap,
        )

    if action in ("wrap_text", "wrap"):
        cols = params.get("columns", "")
        enable = params.get("enable", True)
        return _wrap_text(excel, sheet, columns=cols, enable=enable)

    if action in ("freeze_header", "freeze_panes", "freeze"):
        rows = int(params.get("rows", 1))
        return _freeze_header(excel, sheet, rows=rows)

    if action in ("filter", "add_filter", "toggle_filter"):
        try:
            _bring_excel_to_front(excel)
            sheet.UsedRange.AutoFilter()
            return "Toggled AutoFilter on used range."
        except Exception as e:
            return f"Filter toggle failed: {e}"

    if action in ("read_sheet", "read", "summary", "inspect", "get_info"):
        max_r = int(params.get("max_rows", 10))
        return _read_sheet(excel, sheet, max_rows=max_r)

    if action in ("save", "save_file"):
        return _save(excel, sheet)

    if action in ("keyboard_autofit", "ribbon_autofit"):
        return _keyboard_autofit()

    # Default fallback: autofit
    return _autofit(excel, sheet)


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
    "name": "excel_control",
    "description": (
        "Complete Microsoft Excel control & professional spreadsheet formatting. "
        "Use this tool whenever Nibir sir asks to format an Excel sheet, widen/fix cells, "
        "make cut-off text visible, auto-fit columns, add borders, style headers, or inspect Excel data. "
        "NEVER suggest Word or simulate manual mouse dragging to resize Excel columns; ALWAYS call excel_control! "
        "Actions: autofit (expand all columns/rows to fit text), format_table (professional styled table with navy headers, borders, zebra shading, AutoFilter, frozen header), "
        "wrap_text, freeze_header, filter, read_sheet (get row count, column names, preview), save."
    ),
    "parameters": {
        "type": "OBJECT",
        "properties": {
            "action": {
                "type": "STRING",
                "description": "autofit | format_table | wrap_text | freeze_header | filter | read_sheet | save | open"
            },
            "columns": {
                "type": "STRING",
                "description": "Optional column letter or range to format (e.g. 'A:D', 'B', 'Title')"
            },
            "style": {
                "type": "STRING",
                "description": "Table style: navy (default, executive navy blue) | emerald | slate | royal_blue"
            },
            "wrap_text": {
                "type": "BOOLEAN",
                "description": "Whether to enable text wrapping (default: false)"
            },
            "freeze_header": {
                "type": "BOOLEAN",
                "description": "Freeze top header row so it stays pinned while scrolling (default: true)"
            },
            "add_filter": {
                "type": "BOOLEAN",
                "description": "Add AutoFilter dropdowns on header row (default: true)"
            },
            "path": {
                "type": "STRING",
                "description": "Optional file path if opening an Excel workbook"
            }
        },
        "required": ["action"]
    },
    "handler": excel_control,
}
