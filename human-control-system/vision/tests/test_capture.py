"""Tests for capture module."""

import pytest
import numpy as np
from unittest.mock import Mock, patch, MagicMock

from src.capture import CaptureFactory, MonitorInfo, Region, CaptureResult, BaseCapture
from src.capture.windows import WindowsCapture, GDICapture
from src.capture.linux import MSSCapture


class TestMonitorInfo:
    def test_monitor_info_creation(self):
        monitor = MonitorInfo(
            index=0,
            name="Test Monitor",
            x=0,
            y=0,
            width=1920,
            height=1080,
            is_primary=True,
            scale_factor=1.0
        )
        assert monitor.index == 0
        assert monitor.name == "Test Monitor"
        assert monitor.width == 1920
        assert monitor.height == 1080
        assert monitor.is_primary is True


class TestRegion:
    def test_region_empty(self):
        region = Region()
        assert region.is_empty() is True

    def test_region_not_empty(self):
        region = Region(x=0, y=0, width=100, height=100)
        assert region.is_empty() is False


class TestCaptureFactory:
    def test_create_mss_backend(self):
        with patch('sys.platform', 'linux'):
            CaptureFactory.reset()
            capture = CaptureFactory.create("mss")
            assert isinstance(capture, MSSCapture)
            CaptureFactory.reset()

    def test_singleton_pattern(self):
        with patch('sys.platform', 'linux'):
            CaptureFactory.reset()
            c1 = CaptureFactory.create("mss")
            c2 = CaptureFactory.create("mss")
            assert c1 is c2
            CaptureFactory.reset()


class TestMSSCapture:
    @patch('mss.mss')
    def test_get_monitors(self, mock_mss):
        mock_sct = Mock()
        mock_sct.monitors = [
            {"left": 0, "top": 0, "width": 3840, "height": 1080},  # All monitors
            {"left": 0, "top": 0, "width": 1920, "height": 1080},  # Monitor 1
            {"left": 1920, "top": 0, "width": 1920, "height": 1080},  # Monitor 2
        ]
        mock_mss.return_value = mock_sct

        capture = MSSCapture()
        monitors = capture.get_monitors()

        assert len(monitors) == 2
        assert monitors[0].width == 1920
        assert monitors[1].x == 1920

    @patch('mss.mss')
    def test_capture(self, mock_mss):
        mock_sct = Mock()
        mock_sct.monitors = [
            {"left": 0, "top": 0, "width": 3840, "height": 1080},
            {"left": 0, "top": 0, "width": 1920, "height": 1080},
        ]
        mock_sct.grab.return_value = np.zeros((1080, 1920, 4), dtype=np.uint8)
        mock_mss.return_value = mock_sct

        capture = MSSCapture()
        result = capture.capture(monitor_index=0)

        assert isinstance(result, CaptureResult)
        assert result.frame.shape == (1080, 1920, 3)  # BGR
        assert result.monitor_index == 0


class TestWindowsCapture:
    @patch('dxcam.create')
    def test_initialization(self, mock_dxcam_create):
        mock_camera = Mock()
        mock_camera.get_monitor_list.return_value = [
            (0, 0, 1920, 1080),
        ]
        mock_dxcam_create.return_value = mock_camera

        capture = WindowsCapture()
        assert capture._camera is not None
        monitors = capture.get_monitors()
        assert len(monitors) == 1


class TestGDICapture:
    @patch('mss.mss')
    def test_capture(self, mock_mss):
        mock_sct = Mock()
        mock_sct.monitors = [
            {"left": 0, "top": 0, "width": 3840, "height": 1080},
            {"left": 0, "top": 0, "width": 1920, "height": 1080},
        ]
        mock_sct.grab.return_value = np.zeros((1080, 1920, 4), dtype=np.uint8)
        mock_mss.return_value = mock_sct

        capture = GDICapture()
        result = capture.capture(monitor_index=0)

        assert isinstance(result, CaptureResult)
        assert result.frame.shape == (1080, 1920, 3)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])