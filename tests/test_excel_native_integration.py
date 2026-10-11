"""Opt-in test: owns a separate Excel process and never touches a user's workbook."""
import os
import unittest
from actions.excel_data import write_data, clear_table


@unittest.skipUnless(os.environ.get("ULTRON_TEST_EXCEL_COM") == "1", "Opt-in isolated Excel COM test")
class TestNativeExcel(unittest.TestCase):
    def test_month_table_and_delete(self):
        import pythoncom
        import win32com.client
        pythoncom.CoInitialize()
        excel = None
        try:
            excel = win32com.client.DispatchEx("Excel.Application")
            excel.DisplayAlerts = False
            wb = excel.Workbooks.Add()
            sheet = wb.Worksheets(1)
            headers = ["January", "February", "March", "April", "May", "June",
                       "July", "August", "September", "October", "November", "December"]
            result = write_data(excel, sheet, {"headers": headers, "row_count": 5, "random_values": True}, True)
            self.assertEqual(result.evidence["data_rows"], 5)
            self.assertEqual(sheet.ListObjects.Count, 1)
            table = sheet.ListObjects.Item(1)
            self.assertEqual(table.ListRows.Count, 5)
            self.assertEqual(table.ListColumns.Count, 12)
            self.assertEqual(list(sheet.Range("A1:L1").Value2[0]), headers)
            clear_table(excel, sheet, {"table_name": table.Name})
            self.assertEqual(sheet.ListObjects.Count, 0)
            self.assertEqual(excel.WorksheetFunction.CountA(sheet.Range("A1:L6")), 0)
            blank = write_data(excel, sheet, {"headers": ["A", "B", "C", "D", "E"], "row_count": 3}, True)
            self.assertEqual(blank.evidence["data_rows"], 3)
            self.assertEqual(sheet.ListObjects.Item(1).ListRows.Count, 3)
            self.assertEqual(sheet.ListObjects.Item(1).ListColumns.Count, 5)
            clear_table(excel, sheet, {})
            write_data(excel, sheet, {"headers": ["=1+1"], "data": [["sample"]]}, True)
            self.assertEqual(sheet.Range("A1").Value2, "=1+1")
            self.assertFalse(sheet.Range("A1").HasFormula)
            wb.Close(False)
        finally:
            sheet = table = wb = None
            import gc
            gc.collect()
            if excel is not None:
                excel.Quit()
            excel = None
            gc.collect()
            pythoncom.CoUninitialize()
