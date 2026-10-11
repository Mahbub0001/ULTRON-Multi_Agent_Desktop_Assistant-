"""Bulk Excel data operations with actual ListObjects and read-back verification."""
import math
import random
import re

from core.tool_result import VerifiedResult


def matrix(params):
    headers = params.get("headers", [])
    data = params.get("data", [])
    if not isinstance(headers, list) or not all(isinstance(x, str) and x.strip() for x in headers):
        raise ValueError("headers must contain nonempty column names")
    if any(len(h) > 255 for h in headers):
        raise ValueError("Excel table headers must be at most 255 characters")
    if headers and len({h.casefold() for h in headers}) != len(headers):
        raise ValueError("Column names must be unique")
    if params.get("random_values"):
        count = params.get("row_count", 5)
        if not headers or not isinstance(count, int) or isinstance(count, bool) or not 1 <= count <= 1000:
            raise ValueError("Random data needs headers and row_count between 1 and 1000")
        if data:
            raise ValueError("Supply data or random_values, not both")
        data = [[random.randint(1, 100) for _ in headers] for _ in range(count)]
    elif not data and headers and "row_count" in params:
        count = params["row_count"]
        if not isinstance(count, int) or isinstance(count, bool) or not 1 <= count <= 1000:
            raise ValueError("row_count must be between 1 and 1000")
        data = [[None for _ in headers] for _ in range(count)]
    if not isinstance(data, list) or not data or not all(isinstance(row, (list, tuple)) for row in data):
        raise ValueError("data must be a nonempty 2D array: each inner array is ONE ROW")
    if "row_count" in params and (not isinstance(params["row_count"], int)
            or isinstance(params["row_count"], bool) or params["row_count"] != len(data)):
        raise ValueError("row_count must equal the number of supplied data rows")
    width = len(headers) if headers else len(data[0])
    if not 1 <= width <= 256 or len(data) > 10000 or any(len(row) != width for row in data):
        raise ValueError("All rows must have exactly the same number of values as columns (max 10000 x 256)")
    def cell(value):
        if value is None or isinstance(value, bool):
            return value
        if isinstance(value, (int, float)):
            if not math.isfinite(value):
                raise ValueError("Cell numbers must be finite")
            return value
        if not isinstance(value, str) or len(value) > 32767:
            raise ValueError("Cells must be text, numbers or null")
        if re.fullmatch(r"-?(?:0|[1-9]\d*)(?:\.\d+)?", value):
            return float(value) if "." in value else int(value)
        # This is a values API. Never execute a formula hidden inside text.
        return "'" + value if value.startswith(("=", "+", "-", "@")) else value
    rows = [[cell(v) for v in row] for row in data]
    literal_headers = ["'" + h if h.startswith(("=", "+", "-", "@")) else h for h in headers]
    return ([literal_headers] if headers else []) + rows


def _range(sheet, start, rows, columns):
    if not re.fullmatch(r"[A-Za-z]{1,3}[1-9]\d{0,6}", start):
        raise ValueError("start_cell must be one A1 address")
    origin = sheet.Range(start)
    if origin.Row + rows - 1 > 1048576 or origin.Column + columns - 1 > 16384:
        raise ValueError("Range exceeds Excel worksheet bounds")
    # Range.Resize is an indexed COM property; late-bound dispatch can invoke
    # its default Item member instead. Explicit corners work with both bindings.
    return sheet.Range(sheet.Cells(origin.Row, origin.Column),
                       sheet.Cells(origin.Row + rows - 1, origin.Column + columns - 1))


def _values(target):
    raw = target.Value2
    if target.Rows.Count == 1 and target.Columns.Count == 1:
        return [[raw]]
    return [list(row) for row in raw]


def _tables(sheet):
    return [sheet.ListObjects.Item(i) for i in range(1, sheet.ListObjects.Count + 1)]


def write_data(excel, sheet, params, as_table=False):
    data = matrix(params)
    if as_table and not params.get("headers"):
        raise ValueError("create_table needs headers plus data rows")
    target = _range(sheet, params.get("start_cell", "A1"), len(data), len(data[0]))
    for table in _tables(sheet):
        if excel.Intersect(target, table.Range) is not None:
            raise ValueError("Target overlaps an existing table. Inspect it, then explicitly clear_table before replacing it")
    if not params.get("overwrite", False) and excel.WorksheetFunction.CountA(target):
        raise ValueError("Target contains data. Use overwrite=true only when the user requested replacing it")
    target.Value2 = tuple(tuple(row) for row in data)
    expected = [[v[1:] if isinstance(v, str) and v.startswith("'") else v for v in row] for row in data]
    normalize = lambda rows: [[None if value == "" else value for value in row] for row in rows]
    if normalize(_values(target)) != normalize(expected):
        raise RuntimeError("Written cell values did not match read-back; inspect the range before retrying")
    table = None
    if as_table:
        table = sheet.ListObjects.Add(1, target, None, 1)
        table.TableStyle = "TableStyleMedium2"
        if table.ListRows.Count != len(data) - 1 or table.ListColumns.Count != len(data[0]):
            raise RuntimeError("Excel table dimensions did not match requested data")
    target.Columns.AutoFit()
    evidence = {"workbook": excel.ActiveWorkbook.Name, "sheet": sheet.Name,
                "range": target.Address, "data_rows": len(data) - bool(params.get("headers")),
                "columns": len(data[0]), "table": table.Name if table is not None else None,
                "values_read_back": True}
    return VerifiedResult(f"Verified {'table' if as_table else 'data'} at {sheet.Name}!{target.Address}: "
                          f"{evidence['data_rows']} data rows, {evidence['columns']} columns "
                          f"{'+ 1 header row' if params.get('headers') else ''}.", evidence)


def clear_table(excel, sheet, params):
    tables = _tables(sheet)
    name = params.get("table_name", "")
    selected = [t for t in tables if t.Name.casefold() == name.casefold()] if name else tables
    if len(selected) != 1:
        raise ValueError("Specify table_name from read_sheet; there must be exactly one target table")
    table = selected[0]
    name, target = table.Name, table.Range
    address = target.Address
    table.Unlist()
    target.Clear()
    if excel.WorksheetFunction.CountA(target) or any(t.Name == name for t in _tables(sheet)):
        raise RuntimeError("Table deletion could not be verified")
    return VerifiedResult(f"Verified removal of table {name} and its cells at {address}.",
                          {"table_removed": name, "range": address, "remaining_values": 0})
