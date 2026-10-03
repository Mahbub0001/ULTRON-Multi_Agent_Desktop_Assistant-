//! Windows Window Adapter
//!
//! Implements window management using Windows API.

use crate::window::{WindowAdapter, MockWindowAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::*;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Gdi::*;
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::*;
#[cfg(target_os = "windows")]
use windows::Win32::UI::HiDpi::*;

/// Windows-specific window adapter
#[derive(Debug)]
pub struct WindowsWindowAdapter {
    config: AdaptersConfig,
    use_dwm: bool,
    capture_cursor: bool,
    // For now, delegate to mock
    mock: Arc<MockWindowAdapter>,
}

impl WindowsWindowAdapter {
    /// Create a new Windows window adapter
    pub async fn new(config: &AdaptersConfig) -> Result<Self> {
        info!("Initializing Windows Window adapter");

        let use_dwm = config.window.windows.use_dwm;
        let capture_cursor = config.window.windows.capture_cursor;

        let mock = Arc::new(MockWindowAdapter::new());

        Ok(Self {
            config: config.clone(),
            use_dwm,
            capture_cursor,
            mock,
        })
    }

    /// Enum all top-level windows
    #[cfg(target_os = "windows")]
    pub fn enum_windows() -> Result<Vec<WindowInfo>> {
        let mut windows = Vec::new();

        unsafe {
            EnumWindows(Some(enum_windows_callback), LPARAM(&mut windows as *mut _ as isize))?;
        }

        Ok(windows)
    }

    /// Get window info from HWND
    #[cfg(target_os = "windows")]
    fn get_window_info(hwnd: HWND) -> Result<WindowInfo> {
        unsafe {
            // Get title
            let mut title = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, &mut title);
            let title = String::from_utf16_lossy(&title[..title_len as usize]);

            // Get class name
            let mut class_name = [0u16; 256];
            let class_len = GetClassNameW(hwnd, &mut class_name);
            let class_name = String::from_utf16_lossy(&class_name[..class_len as usize]);

            // Get process ID
            let mut process_id = 0u32;
            let thread_id = GetWindowThreadProcessId(hwnd, Some(&mut process_id));

            // Get rect
            let mut rect = RECT::default();
            GetWindowRect(hwnd, &mut rect)?;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            // Get client rect
            let mut client_rect = RECT::default();
            GetClientRect(hwnd, &mut client_rect)?;
            let client_width = client_rect.right - client_rect.left;
            let client_height = client_rect.bottom - client_rect.top;

            // Get styles
            let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;

            // Check visibility
            let is_visible = IsWindowVisible(hwnd).as_bool();
            let is_enabled = IsWindowEnabled(hwnd).as_bool();

            // Check minimized/maximized
            let mut placement = WINDOWPLACEMENT::default();
            placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            GetWindowPlacement(hwnd, &mut placement)?;
            let is_minimized = placement.showCmd == SW_SHOWMINIMIZED;
            let is_maximized = placement.showCmd == SW_SHOWMAXIMIZED;

            // Check foreground
            let is_foreground = GetForegroundWindow() == hwnd;

            // Get parent/owner
            let parent = GetParent(hwnd);
            let owner = GetWindow(hwnd, GW_OWNER);

            Ok(WindowInfo {
                handle: hwnd.0 as u64,
                title,
                class_name,
                process_id,
                thread_id,
                rect: Some(WindowRect {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                    width,
                    height,
                }),
                client_rect: Some(WindowRect {
                    left: client_rect.left,
                    top: client_rect.top,
                    right: client_rect.right,
                    bottom: client_rect.bottom,
                    width: client_width,
                    height: client_height,
                }),
                is_visible,
                is_enabled,
                is_minimized,
                is_maximized,
                is_foreground,
                styles: Some(WindowStyles {
                    style,
                    ws_overlapped: style & WS_OVERLAPPED.0 != 0,
                    ws_popup: style & WS_POPUP.0 != 0,
                    ws_child: style & WS_CHILD.0 != 0,
                    ws_minimize: style & WS_MINIMIZE.0 != 0,
                    ws_maximize: style & WS_MAXIMIZE.0 != 0,
                    ws_caption: style & WS_CAPTION.0 != 0,
                    ws_border: style & WS_BORDER.0 != 0,
                    ws_dlgframe: style & WS_DLGFRAME.0 != 0,
                    ws_vscroll: style & WS_VSCROLL.0 != 0,
                    ws_hscroll: style & WS_HSCROLL.0 != 0,
                    ws_sysmenu: style & WS_SYSMENU.0 != 0,
                    ws_thickframe: style & WS_THICKFRAME.0 != 0,
                    ws_group: style & WS_GROUP.0 != 0,
                    ws_tabstop: style & WS_TABSTOP.0 != 0,
                    ws_minimizebox: style & WS_MINIMIZEBOX.0 != 0,
                    ws_maximizebox: style & WS_MAXIMIZEBOX.0 != 0,
                    ws_disabled: style & WS_DISABLED.0 != 0,
                    ws_visible: style & WS_VISIBLE.0 != 0,
                    ws_clipchildren: style & WS_CLIPCHILDREN.0 != 0,
                    ws_clipsiblings: style & WS_CLIPSIBLINGS.0 != 0,
                }),
                ex_styles: Some(WindowStyles {
                    style: ex_style,
                    ws_overlapped: false,
                    ws_popup: false,
                    ws_child: false,
                    ws_minimize: false,
                    ws_maximize: false,
                    ws_caption: false,
                    ws_border: false,
                    ws_dlgframe: false,
                    ws_vscroll: false,
                    ws_hscroll: false,
                    ws_sysmenu: false,
                    ws_thickframe: false,
                    ws_group: false,
                    ws_tabstop: false,
                    ws_minimizebox: false,
                    ws_maximizebox: false,
                    ws_disabled: false,
                    ws_visible: false,
                    ws_clipchildren: false,
                    ws_clipsiblings: false,
                }),
                z_order: 0, // Would need GetWindow(hwnd, GW_HWNDPREV) loop
                parent_handle: parent.0 as u64,
                owner_handle: owner.0 as u64,
            })
        }
    }

    /// Capture window using PrintWindow or BitBlt
    #[cfg(target_os = "windows")]
    async fn capture_window_impl(&self, hwnd: HWND, format: ImageFormat, quality: i32, include_nonclient: bool) -> Result<ScreenshotResponse> {
        use windows::Win32::Graphics::Gdi::*;
        use windows::Win32::Foundation::*;
        use windows::Win32::UI::WindowsAndMessaging::*;

        unsafe {
            let mut rect = RECT::default();
            if include_nonclient {
                GetWindowRect(hwnd, &mut rect)?;
            } else {
                GetClientRect(hwnd, &mut rect)?;
                let mut pt = POINT { x: rect.left, y: rect.top };
                ClientToScreen(hwnd, &mut pt)?;
                rect.left = pt.x;
                rect.top = pt.y;
                pt.x = rect.right;
                pt.y = rect.bottom;
                ClientToScreen(hwnd, &mut pt)?;
                rect.right = pt.x;
                rect.bottom = pt.y;
            }

            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            let hdc_screen = GetDC(None);
            let hdc_mem = CreateCompatibleDC(hdc_screen);
            let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
            let hbitmap_old = SelectObject(hdc_mem, hbitmap);

            // Use PrintWindow for better results with DWM
            let flags = if self.use_dwm { PW_RENDERFULLCONTENT } else { PW_CLIENTONLY };
            PrintWindow(hwnd, hdc_mem, flags)?;

            // Get bitmap data
            let mut bmi = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            };

            let mut bits = vec![0u8; (width * height * 4) as usize];
            GetDIBits(hdc_mem, hbitmap, 0, height as u32, Some(bits.as_mut_ptr() as _), &mut BITMAPINFO { bmiHeader: bmi, bmiColors: [] }, DIB_RGB_COLORS);

            // Cleanup
            SelectObject(hdc_mem, hbitmap_old);
            DeleteObject(hbitmap);
            DeleteDC(hdc_mem);
            ReleaseDC(None, hdc_screen);

            // Convert format if needed
            let image_data = match format {
                ImageFormat::ImageFormatBgr => {
                    // Convert BGRA to BGR
                    let mut bgr = Vec::with_capacity((width * height * 3) as usize);
                    for chunk in bits.chunks_exact(4) {
                        bgr.extend_from_slice(&[chunk[0], chunk[1], chunk[2]]);
                    }
                    bgr
                }
                ImageFormat::ImageFormatRgb => {
                    // Convert BGRA to RGB
                    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
                    for chunk in bits.chunks_exact(4) {
                        rgb.extend_from_slice(&[chunk[2], chunk[1], chunk[0]]);
                    }
                    rgb
                }
                ImageFormat::ImageFormatRgba => {
                    // Already BGRA, swap R and B
                    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                    for chunk in bits.chunks_exact(4) {
                        rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
                    }
                    rgba
                }
                ImageFormat::ImageFormatGray => {
                    let mut gray = Vec::with_capacity((width * height) as usize);
                    for chunk in bits.chunks_exact(4) {
                        let y = (0.299 * chunk[2] as f32 + 0.587 * chunk[1] as f32 + 0.114 * chunk[0] as f32) as u8;
                        gray.push(y);
                    }
                    gray
                }
                _ => bits, // PNG/JPEG would need encoding
            };

            Ok(ScreenshotResponse {
                success: true,
                image_data,
                format: format as i32,
                width,
                height,
                error: String::new(),
            })
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let windows = &mut *(lparam.0 as *mut Vec<WindowInfo>);
    if let Ok(info) = WindowsWindowAdapter::get_window_info(hwnd) {
        windows.push(info);
    }
    BOOL::from(true)
}

#[async_trait::async_trait]
impl WindowAdapter for WindowsWindowAdapter {
    async fn list_windows(&self, req: ListWindowsRequest) -> Result<WindowList> {
        #[cfg(target_os = "windows")]
        {
            let mut windows = Self::enum_windows()?;

            if req.visible_only {
                windows.retain(|w| w.is_visible);
            }
            if req.enabled_only {
                windows.retain(|w| w.is_enabled);
            }
            if !req.class_filter.is_empty() {
                windows.retain(|w| w.class_name.contains(&req.class_filter));
            }
            if !req.title_filter.is_empty() {
                windows.retain(|w| w.title.contains(&req.title_filter));
            }
            if req.process_id_filter > 0 {
                windows.retain(|w| w.process_id == req.process_id_filter);
            }

            return Ok(WindowList { windows });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.list_windows(req).await;
        }
    }

    async fn get_window(&self, req: GetWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            match Self::get_window_info(hwnd) {
                Ok(info) => return Ok(WindowResponse { success: true, window: Some(info), error: String::new() }),
                Err(e) => return Ok(WindowResponse { success: false, window: None, error: e.to_string() }),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window(req).await;
        }
    }

    async fn find_window(&self, req: FindWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
            use windows::core::PCWSTR;

            let class = if req.class_name.is_empty() { None } else {
                Some(PCWSTR(std::ffi::OsStr::new(&req.class_name).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>().as_ptr()))
            };
            let title = if req.window_title.is_empty() { None } else {
                Some(PCWSTR(std::ffi::OsStr::new(&req.window_title).encode_wide().chain(std::iter::once(0)).collect::<Vec<_>>().as_ptr()))
            };

            let hwnd = FindWindowW(class, title);
            if hwnd.0.is_null() {
                return Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() });
            }

            match Self::get_window_info(hwnd) {
                Ok(info) => Ok(WindowResponse { success: true, window: Some(info), error: String::new() }),
                Err(e) => Ok(WindowResponse { success: false, window: None, error: e.to_string() }),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.find_window(req).await;
        }
    }

    async fn get_foreground_window(&self) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = unsafe { GetForegroundWindow() };
            if hwnd.0.is_null() {
                return Ok(WindowResponse { success: false, window: None, error: "No foreground window".to_string() });
            }
            match Self::get_window_info(hwnd) {
                Ok(info) => Ok(WindowResponse { success: true, window: Some(info), error: String::new() }),
                Err(e) => Ok(WindowResponse { success: false, window: None, error: e.to_string() }),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_foreground_window().await;
        }
    }

    async fn get_desktop_window(&self) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = unsafe { GetDesktopWindow() };
            match Self::get_window_info(hwnd) {
                Ok(info) => Ok(WindowResponse { success: true, window: Some(info), error: String::new() }),
                Err(e) => Ok(WindowResponse { success: false, window: None, error: e.to_string() }),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_desktop_window().await;
        }
    }

    async fn get_shell_window(&self) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = unsafe { GetShellWindow() };
            if hwnd.0.is_null() {
                return Ok(WindowResponse { success: false, window: None, error: "No shell window".to_string() });
            }
            match Self::get_window_info(hwnd) {
                Ok(info) => Ok(WindowResponse { success: true, window: Some(info), error: String::new() }),
                Err(e) => Ok(WindowResponse { success: false, window: None, error: e.to_string() }),
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_shell_window().await;
        }
    }

    async fn focus_window(&self, req: FocusWindowRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn activate_window(&self, req: ActivateWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            unsafe {
                // Use SetForegroundWindow with attachment
                let fg_hwnd = GetForegroundWindow();
                let fg_thread = GetWindowThreadProcessId(fg_hwnd, None);
                let cur_thread = GetCurrentThreadId();
                AttachThreadInput(fg_thread, cur_thread, true);
                SetForegroundWindow(hwnd);
                AttachThreadInput(fg_thread, cur_thread, false);
            }
            return self.get_window(req).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.activate_window(req).await;
        }
    }

    async fn show_window(&self, req: ShowWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let cmd = match req.command() {
                ShowCommand::ShowCommandHide => SW_HIDE,
                ShowCommand::ShowCommandNormal => SW_NORMAL,
                ShowCommand::ShowCommandMinimized => SW_MINIMIZE,
                ShowCommand::ShowCommandMaximized => SW_MAXIMIZE,
                ShowCommand::ShowCommandShowNoactivate => SW_SHOWNOACTIVATE,
                ShowCommand::ShowCommandShow => SW_SHOW,
                ShowCommand::ShowCommandMinimize => SW_MINIMIZE,
                ShowCommand::ShowCommandShowMinnoactive => SW_SHOWMINNOACTIVE,
                ShowCommand::ShowCommandShowNa => SW_SHOWNA,
                ShowCommand::ShowCommandRestore => SW_RESTORE,
                ShowCommand::ShowCommandShowDefault => SW_SHOWDEFAULT,
                ShowCommand::ShowCommandForceMinimize => SW_FORCEMINIMIZE,
                _ => SW_NORMAL,
            };
            unsafe { ShowWindow(hwnd, cmd) };
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.show_window(req).await;
        }
    }

    async fn hide_window(&self, req: HideWindowRequest) -> Result<WindowResponse> {
        self.show_window(ShowWindowRequest { handle: req.handle, command: ShowCommand::ShowCommandHide as i32 }).await
    }

    async fn minimize_window(&self, req: MinimizeWindowRequest) -> Result<WindowResponse> {
        self.show_window(ShowWindowRequest { handle: req.handle, command: ShowCommand::ShowCommandMinimized as i32 }).await
    }

    async fn maximize_window(&self, req: MaximizeWindowRequest) -> Result<WindowResponse> {
        self.show_window(ShowWindowRequest { handle: req.handle, command: ShowCommand::ShowCommandMaximized as i32 }).await
    }

    async fn restore_window(&self, req: RestoreWindowRequest) -> Result<WindowResponse> {
        self.show_window(ShowWindowRequest { handle: req.handle, command: ShowCommand::ShowCommandRestore as i32 }).await
    }

    async fn close_window(&self, req: CloseWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            unsafe { PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0)) };
            return Ok(WindowResponse { success: true, window: None, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.close_window(req).await;
        }
    }

    async fn is_window_visible(&self, req: IsWindowVisibleRequest) -> Result<WindowVisibilityResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
            return Ok(WindowVisibilityResponse { success: true, visible, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.is_window_visible(req).await;
        }
    }

    async fn is_window_enabled(&self, req: IsWindowEnabledRequest) -> Result<WindowEnabledResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let enabled = unsafe { IsWindowEnabled(hwnd).as_bool() };
            return Ok(WindowEnabledResponse { success: true, enabled, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.is_window_enabled(req).await;
        }
    }

    async fn get_window_state(&self, req: GetWindowStateRequest) -> Result<WindowStateResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut placement = WINDOWPLACEMENT::default();
            placement.length = std::mem::size_of::<WINDOWPLACEMENT>() as u32;
            unsafe { GetWindowPlacement(hwnd, &mut placement) }?;

            let state = match placement.showCmd {
                SW_SHOWMINIMIZED => WindowState::WindowStateMinimized,
                SW_SHOWMAXIMIZED => WindowState::WindowStateMaximized,
                SW_HIDE => WindowState::WindowStateHidden,
                _ => WindowState::WindowStateNormal,
            };

            return Ok(WindowStateResponse { success: true, state: state as i32, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_state(req).await;
        }
    }

    async fn move_window(&self, req: MoveWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut rect = RECT::default();
            unsafe { GetWindowRect(hwnd, &mut rect) }?;
            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;
            unsafe { MoveWindow(hwnd, req.x, req.y, width, height, req.repaint) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.move_window(req).await;
        }
    }

    async fn resize_window(&self, req: ResizeWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut rect = RECT::default();
            unsafe { GetWindowRect(hwnd, &mut rect) }?;
            unsafe { MoveWindow(hwnd, rect.left, rect.top, req.width, req.height, req.repaint) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.resize_window(req).await;
        }
    }

    async fn move_resize_window(&self, req: MoveResizeWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            unsafe { MoveWindow(hwnd, req.x, req.y, req.width, req.height, req.repaint) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.move_resize_window(req).await;
        }
    }

    async fn get_window_rect(&self, req: GetWindowRectRequest) -> Result<WindowRectResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut rect = RECT::default();
            unsafe { GetWindowRect(hwnd, &mut rect) }?;
            return Ok(WindowRectResponse {
                success: true,
                rect: Some(WindowRect {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                }),
                error: String::new(),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_rect(req).await;
        }
    }

    async fn get_client_rect(&self, req: GetClientRectRequest) -> Result<WindowRectResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut rect = RECT::default();
            unsafe { GetClientRect(hwnd, &mut rect) }?;
            return Ok(WindowRectResponse {
                success: true,
                rect: Some(WindowRect {
                    left: rect.left,
                    top: rect.top,
                    right: rect.right,
                    bottom: rect.bottom,
                    width: rect.right - rect.left,
                    height: rect.bottom - rect.top,
                }),
                error: String::new(),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_client_rect(req).await;
        }
    }

    async fn set_window_pos(&self, req: SetWindowPosRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut flags = SWP_NOZORDER | SWP_NOACTIVATE;
            if req.flags & SetWindowPosFlags::SetWindowPosFlagNosize as i32 != 0 { flags |= SWP_NOSIZE; }
            if req.flags & SetWindowPosFlags::SetWindowPosFlagNomove as i32 != 0 { flags |= SWP_NOMOVE; }
            if req.flags & SetWindowPosFlags::SetWindowPosFlagNoredraw as i32 != 0 { flags |= SWP_NOREDRAW; }
            if req.flags & SetWindowPosFlags::SetWindowPosFlagNoactivate as i32 != 0 { flags |= SWP_NOACTIVATE; }
            if req.flags & SetWindowPosFlags::SetWindowPosFlagFramechanged as i32 != 0 { flags |= SWP_FRAMECHANGED; }
            if req.flags & SetWindowPosFlags::SetWindowPosFlagShowwindow as i32 != 0 { flags |= SWP_SHOWWINDOW; }
            if req.flags & SetWindowPosFlags::SetWindowPosFlagHidewindow as i32 != 0 { flags |= SWP_HIDEWINDOW; }

            unsafe { SetWindowPos(hwnd, HWND(0), req.x, req.y, req.width, req.height, flags) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.set_window_pos(req).await;
        }
    }

    async fn capture_window(&self, req: CaptureWindowRequest) -> Result<ScreenshotResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            return self.capture_window_impl(hwnd, req.format(), req.quality, req.include_nonclient).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.capture_window(req).await;
        }
    }

    async fn capture_client_area(&self, req: CaptureClientAreaRequest) -> Result<ScreenshotResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            return self.capture_window_impl(hwnd, req.format(), req.quality, false).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.capture_client_area(req).await;
        }
    }

    async fn capture_region(&self, req: CaptureRegionRequest) -> Result<ScreenshotResponse> {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Graphics::Gdi::*;
            use windows::Win32::Foundation::*;

            let rect = req.region;
            let width = rect.width;
            let height = rect.height;

            unsafe {
                let hdc_screen = GetDC(None);
                let hdc_mem = CreateCompatibleDC(hdc_screen);
                let hbitmap = CreateCompatibleBitmap(hdc_screen, width, height);
                let hbitmap_old = SelectObject(hdc_mem, hbitmap);

                BitBlt(hdc_mem, 0, 0, width, height, hdc_screen, rect.left, rect.top, SRCCOPY);

                let mut bmi = BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: 0,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                };

                let mut bits = vec![0u8; (width * height * 4) as usize];
                GetDIBits(hdc_mem, hbitmap, 0, height as u32, Some(bits.as_mut_ptr() as _), &mut BITMAPINFO { bmiHeader: bmi, bmiColors: [] }, DIB_RGB_COLORS);

                SelectObject(hdc_mem, hbitmap_old);
                DeleteObject(hbitmap);
                DeleteDC(hdc_mem);
                ReleaseDC(None, hdc_screen);

                return Ok(ScreenshotResponse {
                    success: true,
                    image_data: bits,
                    format: req.format as i32,
                    width,
                    height,
                    error: String::new(),
                });
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.capture_region(req).await;
        }
    }

    async fn get_window_title(&self, req: GetWindowTitleRequest) -> Result<WindowTitleResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut title = [0u16; 512];
            let len = unsafe { GetWindowTextW(hwnd, &mut title) };
            return Ok(WindowTitleResponse { success: true, title: String::from_utf16_lossy(&title[..len as usize]), error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_title(req).await;
        }
    }

    async fn set_window_title(&self, req: SetWindowTitleRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let title: Vec<u16> = std::ffi::OsStr::new(&req.title).encode_wide().chain(std::iter::once(0)).collect();
            unsafe { SetWindowTextW(hwnd, PCWSTR(title.as_ptr())) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.set_window_title(req).await;
        }
    }

    async fn get_window_class(&self, req: GetWindowClassRequest) -> Result<WindowClassResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut class = [0u16; 256];
            let len = unsafe { GetClassNameW(hwnd, &mut class) };
            return Ok(WindowClassResponse { success: true, class_name: String::from_utf16_lossy(&class[..len as usize]), error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_class(req).await;
        }
    }

    async fn get_window_process_id(&self, req: GetWindowProcessIdRequest) -> Result<WindowProcessIdResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let mut pid = 0u32;
            let tid = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
            return Ok(WindowProcessIdResponse { success: true, process_id: pid, thread_id: tid, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_process_id(req).await;
        }
    }

    async fn get_window_styles(&self, req: GetWindowStylesRequest) -> Result<WindowStylesResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
            let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;

            return Ok(WindowStylesResponse {
                success: true,
                styles: Some(WindowStyles {
                    style,
                    ws_overlapped: style & WS_OVERLAPPED.0 != 0,
                    ws_popup: style & WS_POPUP.0 != 0,
                    ws_child: style & WS_CHILD.0 != 0,
                    ws_minimize: style & WS_MINIMIZE.0 != 0,
                    ws_maximize: style & WS_MAXIMIZE.0 != 0,
                    ws_caption: style & WS_CAPTION.0 != 0,
                    ws_border: style & WS_BORDER.0 != 0,
                    ws_dlgframe: style & WS_DLGFRAME.0 != 0,
                    ws_vscroll: style & WS_VSCROLL.0 != 0,
                    ws_hscroll: style & WS_HSCROLL.0 != 0,
                    ws_sysmenu: style & WS_SYSMENU.0 != 0,
                    ws_thickframe: style & WS_THICKFRAME.0 != 0,
                    ws_group: style & WS_GROUP.0 != 0,
                    ws_tabstop: style & WS_TABSTOP.0 != 0,
                    ws_minimizebox: style & WS_MINIMIZEBOX.0 != 0,
                    ws_maximizebox: style & WS_MAXIMIZEBOX.0 != 0,
                    ws_disabled: style & WS_DISABLED.0 != 0,
                    ws_visible: style & WS_VISIBLE.0 != 0,
                    ws_clipchildren: style & WS_CLIPCHILDREN.0 != 0,
                    ws_clipsiblings: style & WS_CLIPSIBLINGS.0 != 0,
                }),
                ex_styles: Some(WindowStyles { style: ex_style, ..Default::default() }),
                error: String::new(),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_styles(req).await;
        }
    }

    async fn set_window_styles(&self, req: SetWindowStylesRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            if let Some(styles) = req.styles {
                unsafe { SetWindowLongW(hwnd, GWL_STYLE, styles.style as i32) };
            }
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.set_window_styles(req).await;
        }
    }

    async fn get_window_ex_styles(&self, req: GetWindowExStylesRequest) -> Result<WindowStylesResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
            return Ok(WindowStylesResponse {
                success: true,
                styles: None,
                ex_styles: Some(WindowStyles { style: ex_style, ..Default::default() }),
                error: String::new(),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_ex_styles(req).await;
        }
    }

    async fn set_window_ex_styles(&self, req: SetWindowExStylesRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            if let Some(styles) = req.ex_styles {
                unsafe { SetWindowLongW(hwnd, GWL_EXSTYLE, styles.style as i32) };
            }
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.set_window_ex_styles(req).await;
        }
    }

    async fn bring_to_top(&self, req: BringToTopRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            unsafe { BringWindowToTop(hwnd) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.bring_to_top(req).await;
        }
    }

    async fn send_to_bottom(&self, req: SendToBottomRequest) -> Result<WindowResponse> {
        self.mock.send_to_bottom(req).await
    }

    async fn set_window_z_order(&self, req: SetWindowZOrderRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let insert_after = if req.is_handle {
                HWND(req.insert_after as *mut _)
            } else {
                match req.insert_after {
                    0 => HWND_TOP,
                    1 => HWND_BOTTOM,
                    2 => HWND_TOPMOST,
                    3 => HWND_NOTOPMOST,
                    _ => HWND_TOP,
                }
            };
            unsafe { SetWindowPos(hwnd, insert_after, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) }?;
            return self.get_window(GetWindowRequest { handle: req.handle }).await;
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.set_window_z_order(req).await;
        }
    }

    async fn set_focus(&self, req: SetFocusRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn get_focus(&self) -> Result<WindowResponse> {
        self.get_foreground_window().await
    }

    async fn post_message(&self, req: PostMessageRequest) -> Result<MessageResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let result = unsafe { PostMessageW(hwnd, req.msg, WPARAM(req.wparam), LPARAM(req.lparam as isize)) };
            return Ok(MessageResponse { success: result.as_bool(), result: 0, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.post_message(req).await;
        }
    }

    async fn send_message(&self, req: SendMessageRequest) -> Result<MessageResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let result = unsafe { SendMessageW(hwnd, req.msg, WPARAM(req.wparam), LPARAM(req.lparam as isize)) };
            return Ok(MessageResponse { success: true, result: result.0 as i64, error: String::new() });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.send_message(req).await;
        }
    }

    async fn get_monitors(&self) -> Result<MonitorList> {
        #[cfg(target_os = "windows")]
        {
            let mut monitors = Vec::new();
            unsafe {
                EnumDisplayMonitors(None, None, Some(enum_monitors_callback), LPARAM(&mut monitors as *mut _ as isize))?;
            }
            return Ok(MonitorList { monitors });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_monitors().await;
        }
    }

    async fn get_window_monitor(&self, req: GetWindowMonitorRequest) -> Result<MonitorResponse> {
        #[cfg(target_os = "windows")]
        {
            let hwnd = HWND(req.handle as *mut _);
            let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
            let mut info = MONITORINFOEXW::default();
            info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            unsafe { GetMonitorInfoW(hmonitor, &mut info.monitorInfo) }?;

            let rect = info.monitorInfo.rcMonitor;
            let work = info.monitorInfo.rcWork;
            let name = String::from_utf16_lossy(&info.szDevice).trim_end_matches('\0').to_string();

            return Ok(MonitorResponse {
                success: true,
                monitor: Some(MonitorInfo {
                    name,
                    rect: Some(WindowRect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.right - rect.left, height: rect.bottom - rect.top }),
                    work_area: Some(WindowRect { left: work.left, top: work.top, right: work.right, bottom: work.bottom, width: work.right - work.left, height: work.bottom - work.top }),
                    is_primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY.0 != 0,
                    scale_factor: 1.0, // Would need GetDpiForMonitor
                    dpi_x: 96,
                    dpi_y: 96,
                }),
                error: String::new(),
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            return self.mock.get_window_monitor(req).await;
        }
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn enum_monitors_callback(hmonitor: HMONITOR, _hdc: HDC, _rect: *mut RECT, lparam: LPARAM) -> BOOL {
    let monitors = &mut *(lparam.0 as *mut Vec<MonitorInfo>);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hmonitor, &mut info.monitorInfo).is_ok() {
        let rect = info.monitorInfo.rcMonitor;
        let work = info.monitorInfo.rcWork;
        let name = String::from_utf16_lossy(&info.szDevice).trim_end_matches('\0').to_string();
        monitors.push(MonitorInfo {
            name,
            rect: Some(WindowRect { left: rect.left, top: rect.top, right: rect.right, bottom: rect.bottom, width: rect.right - rect.left, height: rect.bottom - rect.top }),
            work_area: Some(WindowRect { left: work.left, top: work.top, right: work.right, bottom: work.bottom, width: work.right - work.left, height: work.bottom - work.top }),
            is_primary: info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY.0 != 0,
            scale_factor: 1.0,
            dpi_x: 96,
            dpi_y: 96,
        });
    }
    BOOL::from(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdaptersConfig;

    #[tokio::test]
    async fn test_windows_adapter_creation() {
        let config = AdaptersConfig::default();
        let adapter = WindowsWindowAdapter::new(&config).await;
        assert!(adapter.is_ok());
    }
}