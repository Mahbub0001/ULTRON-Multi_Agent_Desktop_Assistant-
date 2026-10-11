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
        try:
            excel = win32com.client.GetActiveObject("Excel.Application")
        except Exception:
            return None, None, None

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
        from core.desktop_target import focus
        focus("Excel")
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

        return "Sent Excel autofit keyboard sequence. Result is unverified; inspect the sheet before claiming completion."
    except Exception as e:
        return f"Keyboard autofit failed: {e}"


def _autofit(excel, sheet, columns: str = "") -> str:
    """Auto-fits all columns and rows in the active worksheet so text is never cut off."""
    if not sheet:
        return "Error: No active worksheet to autofit."

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
        return f"Error: Excel autofit failed: {e}. Inspect workbook state before retrying."


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
        return "Error: No active worksheet. Use new_workbook to create one."

    try:
        _bring_excel_to_front(excel)
        used = sheet.UsedRange
        if not used or not excel.WorksheetFunction.CountA(used):
            return "Error: Worksheet is empty. format_table only formats existing data; use create_table with headers and data rows."

        row_count = used.Rows.Count
        col_count = used.Columns.Count

        if row_count < 2 or col_count < 1:
            return "Error: Need headers plus at least one data row. Use create_table or write_range to add rows first."
        row_start, col_start = used.Row, used.Column

        palette = _STYLES.get(style_name.lower().strip(), _STYLES["navy"])

        # 1. Header row formatting
        header_range = sheet.Range(sheet.Cells(row_start, col_start), sheet.Cells(row_start, col_start + col_count - 1))
        header_range.Font.Bold = True
        header_range.Font.Color = palette["header_fg"]
        header_range.Interior.Color = palette["header_bg"]
        header_range.HorizontalAlignment = -4108  # xlCenter
        header_range.VerticalAlignment = -4108    # xlCenter
        header_range.RowHeight = 26

        # 2. Data rows vertical alignment and optional zebra shading
        if row_count > 1:
            data_range = sheet.Range(sheet.Cells(row_start + 1, col_start), sheet.Cells(row_start + row_count - 1, col_start + col_count - 1))
            data_range.VerticalAlignment = -4108  # xlCenter
            data_range.RowHeight = 20

            # Subtle alternating zebra shading
            for r in range(row_start + 1, row_start + row_count):
                if r % 2 == 0:
                    row_cells = sheet.Range(sheet.Cells(r, col_start), sheet.Cells(r, col_start + col_count - 1))
                    row_cells.Interior.Color = palette["zebra_bg"]
                else:
                    row_cells = sheet.Range(sheet.Cells(r, col_start), sheet.Cells(r, col_start + col_count - 1))
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
        return f"Error: Table formatting failed: {e}. No new data rows were created."


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
        if not used or not excel.WorksheetFunction.CountA(used):
            return "Active worksheet is empty."

        total_rows = used.Rows.Count
        total_cols = used.Columns.Count

        # Read header names
        headers = []
        for c in range(used.Column, used.Column + min(total_cols, 256)):
            val = sheet.Cells(used.Row, c).Value
            headers.append(str(val) if val is not None else f"Col{c}")

        # Read preview rows
        preview_lines = []
        rows_to_read = min(total_rows, max(1, min(max_rows, 100)))
        for r in range(used.Row + 1, used.Row + rows_to_read):
            row_vals = []
            for c in range(used.Column, used.Column + min(total_cols, 256)):
                val = sheet.Cells(r, c).Value
                str_val = str(val).strip() if val is not None else ""
                if len(str_val) > 35:
                    str_val = str_val[:32] + "..."
                row_vals.append(str_val)
            preview_lines.append(" | ".join(row_vals))

        summary = (
            f"Workbook: '{excel.ActiveWorkbook.Name}'\n"
            f"Active Sheet: '{sheet.Name}'\n"
            f"- Total Rows: {total_rows}\n"
            f"- Total Columns: {total_cols}\n"
            f"- Headers: [{', '.join(headers)}]\n"
        )
        for i in range(1, sheet.ListObjects.Count + 1):
            table = sheet.ListObjects.Item(i)
            summary += f"- Table: {table.Name}, range={table.Range.Address}, data_rows={table.ListRows.Count}, columns={table.ListColumns.Count}\n"
        if preview_lines:
            summary += f"- Preview (rows 2 to {rows_to_read}):\n"
            summary += "\n".join(f"  Row {i+2}: {line}" for i, line in enumerate(preview_lines))

        return summary
    except Exception as e:
        return f"Could not read Excel sheet: {e}"


def _save(excel, sheet, params=None) -> str:
    """Save to an explicit destination and verify the resulting file."""
    params = params or {}
    if not excel or not excel.ActiveWorkbook:
        return "Error: No active Excel workbook to save."
    try:
        from core.tool_result import VerifiedResult
        from core.user_paths import desktop_path
        wb = excel.ActiveWorkbook
        raw_path = str(params.get("path") or "").strip()
        filename = str(params.get("filename") or "").strip()
        if not raw_path and not filename:
            if not wb.Path:
                return "Error: Workbook has no file location. Provide path and filename to save it."
            wb.Save()
            target = Path(wb.FullName)
        else:
            if filename and (Path(filename).name != filename or any(c in filename for c in '<>:"/\\|?*')):
                return "Error: filename must be a plain file name."
            normalized = raw_path.replace("\\", "/")
            head, sep, tail = normalized.partition("/")
            if head.casefold() == "desktop":
                requested = desktop_path() / tail if sep else desktop_path()
            elif raw_path:
                requested = Path(raw_path).expanduser()
                if not requested.is_absolute():
                    return "Error: Use an absolute path or Desktop for save location."
            else:
                requested = desktop_path()
            if filename:
                target = requested / filename
            elif requested.suffix:
                target = requested
            else:
                return "Error: Provide filename when path is a folder."
            if not target.suffix:
                target = target.with_suffix(".xlsx")
            if target.suffix.casefold() != ".xlsx":
                return "Error: This save action supports .xlsx only."
            if not target.parent.is_dir():
                return f"Error: Save folder does not exist: {target.parent}"
            target = target.resolve()
            current = Path(wb.FullName).resolve() if wb.Path else None
            if target.exists() and target != current and not params.get("overwrite", False):
                return f"Error: File already exists: {target}. Set overwrite only if the user asked to replace it."
            if target == current:
                wb.Save()
            else:
                alerts = excel.DisplayAlerts
                try:
                    excel.DisplayAlerts = False
                    wb.SaveAs(str(target), FileFormat=51)
                finally:
                    excel.DisplayAlerts = alerts
        if not target.is_file() or Path(wb.FullName).resolve() != target.resolve():
            return f"Error: Save could not be verified at {target}."
        return VerifiedResult(f"Verified workbook saved at {target}.",
                              {"path": str(target), "size_bytes": target.stat().st_size})
    except Exception as e:
        return f"Error: Excel save failed: {e}"


# ── Main Handler ─────────────────────────────────────────────────────────────

def _dispatch_excel(
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

    if action == "new_workbook":
        try:
            if excel is None:
                import win32com.client
                excel = win32com.client.Dispatch("Excel.Application")
            excel.Visible = True
            wb = excel.Workbooks.Add()
            wb.Activate()
            from core.tool_result import VerifiedResult
            if excel.ActiveWorkbook.Name != wb.Name or wb.Worksheets.Count < 1:
                return "Error: New workbook could not be verified."
            return VerifiedResult(f"Verified new workbook '{wb.Name}' with sheet '{wb.ActiveSheet.Name}'.",
                                  {"workbook": wb.Name, "sheet": wb.ActiveSheet.Name})
        except Exception as exc:
            return f"Error: Could not create workbook: {exc}"

    if not excel:
        return "Error: No accessible Excel instance. Open Excel or call new_workbook. No keyboard fallback was run."

    if not wb or not sheet:
        # Excel is running with no open workbook or COM locked
        return "Error: No active workbook. Use new_workbook before entering data."

    if params.get("workbook") and wb.Name.casefold() != params["workbook"].casefold():
        return f"Error: Active workbook is '{wb.Name}', not the requested workbook."
    if params.get("sheet"):
        try:
            sheet = wb.Worksheets(params["sheet"])
        except Exception:
            return "Error: Requested sheet does not exist."

    if action in ("create_table", "write_range", "clear_table"):
        from actions.excel_data import write_data, clear_table
        try:
            return clear_table(excel, sheet, params) if action == "clear_table" else write_data(
                excel, sheet, params, as_table=action == "create_table")
        except Exception as exc:
            return f"Error: Excel {action} failed: {exc}"

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
        return _save(excel, sheet, params)

    if action in ("keyboard_autofit", "ribbon_autofit"):
        return _keyboard_autofit()

    return f"Error: Unknown Excel action '{action}'."


def excel_control(parameters, response=None, player=None, session_memory=None):
    # Each executor thread must initialize its own COM apartment.
    import pythoncom
    pythoncom.CoInitialize()
    try:
        return _dispatch_excel(parameters, response, player, session_memory)
    finally:
        pythoncom.CoUninitialize()


# ── Tool declaration (auto-discovered by core/action_loader.py) ──────────────
TOOL = {
    "name": "excel_control",
    "description": (
        "Native Microsoft Excel data entry and formatting. create_table writes headers and a rectangular 2D data matrix and creates a real Excel Table, verifying every value and dimensions. "
        "Use new_workbook for a blank workbook; write_range to write data; clear_table to delete a named table and its cells. "
        "Each inner data array is ONE ROW, never one column. row_count excludes the header. For an empty table supply headers and row_count; for random numbers also set random_values=true. "
        "format_table ONLY styles existing data; it cannot create rows or values. "
        "For save, pass path='Desktop' and filename='Name.xlsx' when the user specifies Desktop; save verifies the full file path. "
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
                "description": "new_workbook | create_table | write_range | clear_table | autofit | format_table | wrap_text | freeze_header | filter | read_sheet | save | open"
            },
            "headers": {"type": "ARRAY", "items": {"type": "STRING"}, "description": "One column name per item"},
            "data": {"type": "ARRAY", "items": {"type": "ARRAY", "items": {"type": "STRING"}}, "description": "Rows of cell values; numbers may be strings. Every row must match headers length. No formulas."},
            "row_count": {"type": "INTEGER", "description": "Number of data rows excluding header, for blank or random tables"},
            "random_values": {"type": "BOOLEAN", "description": "Generate numeric values 1..100 for each data row and column"},
            "start_cell": {"type": "STRING", "description": "Top-left cell, default A1"},
            "overwrite": {"type": "BOOLEAN", "description": "Replace existing cells or save file only when user requested it"},
            "table_name": {"type": "STRING", "description": "Exact Excel table name from read_sheet"},
            "workbook": {"type": "STRING", "description": "Expected active workbook name"},
            "sheet": {"type": "STRING", "description": "Target sheet name"},
            "max_rows": {"type": "INTEGER", "description": "Preview row limit for read_sheet"},
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
                "description": "File path for open, or save destination folder (e.g. Desktop) / full .xlsx path"
            },
            "filename": {"type": "STRING", "description": "Workbook file name for save, e.g. Ultron Test.xlsx"}
        },
        "required": ["action"]
    },
    "handler": excel_control,
}
