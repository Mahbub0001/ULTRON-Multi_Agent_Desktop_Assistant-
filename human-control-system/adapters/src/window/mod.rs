//! Window Manager Adapter Module
//!
//! Provides cross-platform window management capabilities.

pub mod windows;
pub mod linux;

use crate::adapters_proto::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Window adapter trait
#[async_trait::async_trait]
pub trait WindowAdapter: Send + Sync {
    // Window enumeration
    async fn list_windows(&self, req: ListWindowsRequest) -> Result<WindowList>;
    async fn get_window(&self, req: GetWindowRequest) -> Result<WindowResponse>;
    async fn find_window(&self, req: FindWindowRequest) -> Result<WindowResponse>;
    async fn get_foreground_window(&self) -> Result<WindowResponse>;
    async fn get_desktop_window(&self) -> Result<WindowResponse>;
    async fn get_shell_window(&self) -> Result<WindowResponse>;

    // Window state
    async fn focus_window(&self, req: FocusWindowRequest) -> Result<WindowResponse>;
    async fn activate_window(&self, req: ActivateWindowRequest) -> Result<WindowResponse>;
    async fn show_window(&self, req: ShowWindowRequest) -> Result<WindowResponse>;
    async fn hide_window(&self, req: HideWindowRequest) -> Result<WindowResponse>;
    async fn minimize_window(&self, req: MinimizeWindowRequest) -> Result<WindowResponse>;
    async fn maximize_window(&self, req: MaximizeWindowRequest) -> Result<WindowResponse>;
    async fn restore_window(&self, req: RestoreWindowRequest) -> Result<WindowResponse>;
    async fn close_window(&self, req: CloseWindowRequest) -> Result<WindowResponse>;
    async fn is_window_visible(&self, req: IsWindowVisibleRequest) -> Result<WindowVisibilityResponse>;
    async fn is_window_enabled(&self, req: IsWindowEnabledRequest) -> Result<WindowEnabledResponse>;
    async fn get_window_state(&self, req: GetWindowStateRequest) -> Result<WindowStateResponse>;

    // Position/Size
    async fn move_window(&self, req: MoveWindowRequest) -> Result<WindowResponse>;
    async fn resize_window(&self, req: ResizeWindowRequest) -> Result<WindowResponse>;
    async fn move_resize_window(&self, req: MoveResizeWindowRequest) -> Result<WindowResponse>;
    async fn get_window_rect(&self, req: GetWindowRectRequest) -> Result<WindowRectResponse>;
    async fn get_client_rect(&self, req: GetClientRectRequest) -> Result<WindowRectResponse>;
    async fn set_window_pos(&self, req: SetWindowPosRequest) -> Result<WindowResponse>;

    // Screenshot
    async fn capture_window(&self, req: CaptureWindowRequest) -> Result<ScreenshotResponse>;
    async fn capture_client_area(&self, req: CaptureClientAreaRequest) -> Result<ScreenshotResponse>;
    async fn capture_region(&self, req: CaptureRegionRequest) -> Result<ScreenshotResponse>;

    // Window properties
    async fn get_window_title(&self, req: GetWindowTitleRequest) -> Result<WindowTitleResponse>;
    async fn set_window_title(&self, req: SetWindowTitleRequest) -> Result<WindowResponse>;
    async fn get_window_class(&self, req: GetWindowClassRequest) -> Result<WindowClassResponse>;
    async fn get_window_process_id(&self, req: GetWindowProcessIdRequest) -> Result<WindowProcessIdResponse>;
    async fn get_window_styles(&self, req: GetWindowStylesRequest) -> Result<WindowStylesResponse>;
    async fn set_window_styles(&self, req: SetWindowStylesRequest) -> Result<WindowResponse>;
    async fn get_window_ex_styles(&self, req: GetWindowExStylesRequest) -> Result<WindowStylesResponse>;
    async fn set_window_ex_styles(&self, req: SetWindowExStylesRequest) -> Result<WindowResponse>;

    // Z-order
    async fn bring_to_top(&self, req: BringToTopRequest) -> Result<WindowResponse>;
    async fn send_to_bottom(&self, req: SendToBottomRequest) -> Result<WindowResponse>;
    async fn set_window_z_order(&self, req: SetWindowZOrderRequest) -> Result<WindowResponse>;

    // Input
    async fn set_focus(&self, req: SetFocusRequest) -> Result<WindowResponse>;
    async fn get_focus(&self) -> Result<WindowResponse>;
    async fn post_message(&self, req: PostMessageRequest) -> Result<MessageResponse>;
    async fn send_message(&self, req: SendMessageRequest) -> Result<MessageResponse>;

    // Multi-monitor
    async fn get_monitors(&self) -> Result<MonitorList>;
    async fn get_window_monitor(&self, req: GetWindowMonitorRequest) -> Result<MonitorResponse>;
}

/// Window adapter factory
pub struct WindowAdapterFactory;

impl WindowAdapterFactory {
    /// Create a window adapter based on platform
    pub async fn create(config: &crate::config::AdaptersConfig) -> Result<Arc<dyn WindowAdapter>> {
        #[cfg(target_os = "windows")]
        {
            let adapter = windows::WindowsWindowAdapter::new(config).await?;
            Ok(Arc::new(adapter))
        }

        #[cfg(target_os = "linux")]
        {
            let adapter = linux::LinuxWindowAdapter::new(config).await?;
            Ok(Arc::new(adapter))
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux")))]
        {
            Ok(Arc::new(MockWindowAdapter::new()))
        }
    }
}

/// Mock adapter for testing
#[derive(Debug)]
pub struct MockWindowAdapter {
    windows: Arc<RwLock<std::collections::HashMap<u64, WindowInfo>>>,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl MockWindowAdapter {
    pub fn new() -> Self {
        let mut windows = std::collections::HashMap::new();
        // Add some mock windows
        windows.insert(1, WindowInfo {
            handle: 1,
            title: "Mock Window 1".to_string(),
            class_name: "MockClass".to_string(),
            process_id: 1234,
            thread_id: 5678,
            rect: Some(WindowRect { left: 0, top: 0, right: 800, bottom: 600, width: 800, height: 600 }),
            client_rect: Some(WindowRect { left: 0, top: 0, right: 780, bottom: 580, width: 780, height: 580 }),
            is_visible: true,
            is_enabled: true,
            is_minimized: false,
            is_maximized: false,
            is_foreground: true,
            styles: Some(WindowStyles {
                style: 0x14CA0000,
                ws_overlapped: true,
                ws_popup: false,
                ws_child: false,
                ws_minimize: false,
                ws_maximize: false,
                ws_caption: true,
                ws_border: true,
                ws_dlgframe: false,
                ws_vscroll: false,
                ws_hscroll: false,
                ws_sysmenu: true,
                ws_thickframe: true,
                ws_group: false,
                ws_tabstop: false,
                ws_minimizebox: true,
                ws_maximizebox: true,
                ws_disabled: false,
                ws_visible: true,
                ws_clipchildren: true,
                ws_clipsiblings: true,
            }),
            ex_styles: Some(WindowStyles {
                style: 0x00040100,
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
            z_order: 0,
            parent_handle: 0,
            owner_handle: 0,
        });

        Self {
            windows: Arc::new(RwLock::new(windows)),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(2)),
        }
    }

    fn next_handle(&self) -> u64 {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[async_trait::async_trait]
impl WindowAdapter for MockWindowAdapter {
    async fn list_windows(&self, req: ListWindowsRequest) -> Result<WindowList> {
        let windows = self.windows.read().await;
        let mut result: Vec<WindowInfo> = windows.values().cloned().collect();

        if req.visible_only {
            result.retain(|w| w.is_visible);
        }
        if req.enabled_only {
            result.retain(|w| w.is_enabled);
        }
        if !req.class_filter.is_empty() {
            result.retain(|w| w.class_name.contains(&req.class_filter));
        }
        if !req.title_filter.is_empty() {
            result.retain(|w| w.title.contains(&req.title_filter));
        }
        if req.process_id_filter > 0 {
            result.retain(|w| w.process_id == req.process_id_filter);
        }

        Ok(WindowList { windows: result })
    }

    async fn get_window(&self, req: GetWindowRequest) -> Result<WindowResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn find_window(&self, req: FindWindowRequest) -> Result<WindowResponse> {
        let windows = self.windows.read().await;
        for window in windows.values() {
            let class_match = req.class_name.is_empty() || window.class_name == req.class_name;
            let title_match = req.window_title.is_empty() || window.title == req.window_title;
            if class_match && title_match {
                return Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() });
            }
        }
        Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
    }

    async fn get_foreground_window(&self) -> Result<WindowResponse> {
        let windows = self.windows.read().await;
        for window in windows.values() {
            if window.is_foreground {
                return Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() });
            }
        }
        // Return first window
        if let Some(window) = windows.values().next() {
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "No windows".to_string() })
        }
    }

    async fn get_desktop_window(&self) -> Result<WindowResponse> {
        Ok(WindowResponse {
            success: true,
            window: Some(WindowInfo {
                handle: 0,
                title: "Desktop".to_string(),
                class_name: "Progman".to_string(),
                process_id: 0,
                thread_id: 0,
                rect: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1080, width: 1920, height: 1080 }),
                client_rect: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1080, width: 1920, height: 1080 }),
                is_visible: true,
                is_enabled: true,
                is_minimized: false,
                is_maximized: false,
                is_foreground: false,
                styles: None,
                ex_styles: None,
                z_order: -1,
                parent_handle: 0,
                owner_handle: 0,
            }),
            error: String::new(),
        })
    }

    async fn get_shell_window(&self) -> Result<WindowResponse> {
        self.get_desktop_window().await
    }

    async fn focus_window(&self, req: FocusWindowRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn activate_window(&self, req: ActivateWindowRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        for window in windows.values_mut() {
            window.is_foreground = window.handle == req.handle;
        }
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn show_window(&self, req: ShowWindowRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            window.is_visible = true;
            window.is_minimized = matches!(req.command(), ShowCommand::ShowCommandMinimized);
            window.is_maximized = matches!(req.command(), ShowCommand::ShowCommandMaximized);
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn hide_window(&self, req: HideWindowRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            window.is_visible = false;
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
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
        let mut windows = self.windows.write().await;
        if windows.remove(&req.handle).is_some() {
            Ok(WindowResponse { success: true, window: None, error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn is_window_visible(&self, req: IsWindowVisibleRequest) -> Result<WindowVisibilityResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowVisibilityResponse { success: true, visible: window.is_visible, error: String::new() })
        } else {
            Ok(WindowVisibilityResponse { success: false, visible: false, error: "Window not found".to_string() })
        }
    }

    async fn is_window_enabled(&self, req: IsWindowEnabledRequest) -> Result<WindowEnabledResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowEnabledResponse { success: true, enabled: window.is_enabled, error: String::new() })
        } else {
            Ok(WindowEnabledResponse { success: false, enabled: false, error: "Window not found".to_string() })
        }
    }

    async fn get_window_state(&self, req: GetWindowStateRequest) -> Result<WindowStateResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            let state = if window.is_minimized {
                WindowState::WindowStateMinimized
            } else if window.is_maximized {
                WindowState::WindowStateMaximized
            } else if !window.is_visible {
                WindowState::WindowStateHidden
            } else {
                WindowState::WindowStateNormal
            };
            Ok(WindowStateResponse { success: true, state: state as i32, error: String::new() })
        } else {
            Ok(WindowStateResponse { success: false, state: WindowState::WindowStateUnspecified as i32, error: "Window not found".to_string() })
        }
    }

    async fn move_window(&self, req: MoveWindowRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            if let Some(rect) = window.rect.as_mut() {
                let width = rect.width;
                let height = rect.height;
                rect.left = req.x;
                rect.top = req.y;
                rect.right = req.x + width;
                rect.bottom = req.y + height;
            }
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn resize_window(&self, req: ResizeWindowRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            if let Some(rect) = window.rect.as_mut() {
                rect.right = rect.left + req.width;
                rect.bottom = rect.top + req.height;
                rect.width = req.width;
                rect.height = req.height;
            }
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn move_resize_window(&self, req: MoveResizeWindowRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            if let Some(rect) = window.rect.as_mut() {
                rect.left = req.x;
                rect.top = req.y;
                rect.right = req.x + req.width;
                rect.bottom = req.y + req.height;
                rect.width = req.width;
                rect.height = req.height;
            }
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn get_window_rect(&self, req: GetWindowRectRequest) -> Result<WindowRectResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowRectResponse { success: true, rect: window.rect.clone(), error: String::new() })
        } else {
            Ok(WindowRectResponse { success: false, rect: None, error: "Window not found".to_string() })
        }
    }

    async fn get_client_rect(&self, req: GetClientRectRequest) -> Result<WindowRectResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowRectResponse { success: true, rect: window.client_rect.clone(), error: String::new() })
        } else {
            Ok(WindowRectResponse { success: false, rect: None, error: "Window not found".to_string() })
        }
    }

    async fn set_window_pos(&self, req: SetWindowPosRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            if let Some(rect) = window.rect.as_mut() {
                let flags = req.flags;
                if !flags.contains(SetWindowPosFlags::SetWindowPosFlagNomove as i32) {
                    rect.left = req.x;
                    rect.top = req.y;
                }
                if !flags.contains(SetWindowPosFlags::SetWindowPosFlagNosize as i32) {
                    rect.right = rect.left + req.width;
                    rect.bottom = rect.top + req.height;
                    rect.width = req.width;
                    rect.height = req.height;
                }
            }
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn capture_window(&self, req: CaptureWindowRequest) -> Result<ScreenshotResponse> {
        // Return mock screenshot data
        let (width, height) = {
            let windows = self.windows.read().await;
            if let Some(window) = windows.get(&req.handle) {
                if let Some(rect) = &window.rect {
                    (rect.width, rect.height)
                } else {
                    (800, 600)
                }
            } else {
                (800, 600)
            }
        };
        Ok(ScreenshotResponse {
            success: true,
            image_data: vec![0; (width * height * 4) as usize],
            format: req.format,
            width: width as i32,
            height: height as i32,
            error: String::new(),
        })
    }

    async fn capture_client_area(&self, req: CaptureClientAreaRequest) -> Result<ScreenshotResponse> {
        let (width, height) = {
            let windows = self.windows.read().await;
            if let Some(window) = windows.get(&req.handle) {
                if let Some(rect) = &window.client_rect {
                    (rect.width, rect.height)
                } else {
                    (780, 580)
                }
            } else {
                (780, 580)
            }
        };
        Ok(ScreenshotResponse {
            success: true,
            image_data: vec![0; (width * height * 4) as usize],
            format: req.format,
            width: width as i32,
            height: height as i32,
            error: String::new(),
        })
    }

    async fn capture_region(&self, req: CaptureRegionRequest) -> Result<ScreenshotResponse> {
        let width = req.region.width;
        let height = req.region.height;
        Ok(ScreenshotResponse {
            success: true,
            image_data: vec![0; (width * height * 4) as usize],
            format: req.format,
            width: width as i32,
            height: height as i32,
            error: String::new(),
        })
    }

    async fn get_window_title(&self, req: GetWindowTitleRequest) -> Result<WindowTitleResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowTitleResponse { success: true, title: window.title.clone(), error: String::new() })
        } else {
            Ok(WindowTitleResponse { success: false, title: String::new(), error: "Window not found".to_string() })
        }
    }

    async fn set_window_title(&self, req: SetWindowTitleRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            window.title = req.title;
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn get_window_class(&self, req: GetWindowClassRequest) -> Result<WindowClassResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowClassResponse { success: true, class_name: window.class_name.clone(), error: String::new() })
        } else {
            Ok(WindowClassResponse { success: false, class_name: String::new(), error: "Window not found".to_string() })
        }
    }

    async fn get_window_process_id(&self, req: GetWindowProcessIdRequest) -> Result<WindowProcessIdResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowProcessIdResponse { success: true, process_id: window.process_id, thread_id: window.thread_id, error: String::new() })
        } else {
            Ok(WindowProcessIdResponse { success: false, process_id: 0, thread_id: 0, error: "Window not found".to_string() })
        }
    }

    async fn get_window_styles(&self, req: GetWindowStylesRequest) -> Result<WindowStylesResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            Ok(WindowStylesResponse {
                success: true,
                styles: window.styles.clone(),
                ex_styles: window.ex_styles.clone(),
                error: String::new(),
            })
        } else {
            Ok(WindowStylesResponse { success: false, styles: None, ex_styles: None, error: "Window not found".to_string() })
        }
    }

    async fn set_window_styles(&self, _req: SetWindowStylesRequest) -> Result<WindowResponse> {
        Ok(WindowResponse { success: true, window: None, error: String::new() })
    }

    async fn get_window_ex_styles(&self, req: GetWindowExStylesRequest) -> Result<WindowStylesResponse> {
        self.get_window_styles(GetWindowStylesRequest { handle: req.handle }).await
    }

    async fn set_window_ex_styles(&self, _req: SetWindowExStylesRequest) -> Result<WindowResponse> {
        Ok(WindowResponse { success: true, window: None, error: String::new() })
    }

    async fn bring_to_top(&self, req: BringToTopRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn send_to_bottom(&self, req: SendToBottomRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            window.z_order = -1;
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn set_window_z_order(&self, req: SetWindowZOrderRequest) -> Result<WindowResponse> {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(&req.handle) {
            window.z_order = req.insert_after as i32;
            Ok(WindowResponse { success: true, window: Some(window.clone()), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn set_focus(&self, req: SetFocusRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn get_focus(&self) -> Result<WindowResponse> {
        self.get_foreground_window().await
    }

    async fn post_message(&self, _req: PostMessageRequest) -> Result<MessageResponse> {
        Ok(MessageResponse { success: true, result: 0, error: String::new() })
    }

    async fn send_message(&self, _req: SendMessageRequest) -> Result<MessageResponse> {
        Ok(MessageResponse { success: true, result: 0, error: String::new() })
    }

    async fn get_monitors(&self) -> Result<MonitorList> {
        Ok(MonitorList {
            monitors: vec![
                MonitorInfo {
                    name: "Monitor 1".to_string(),
                    rect: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1080, width: 1920, height: 1080 }),
                    work_area: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1040, width: 1920, height: 1040 }),
                    is_primary: true,
                    scale_factor: 1.0,
                    dpi_x: 96,
                    dpi_y: 96,
                },
                MonitorInfo {
                    name: "Monitor 2".to_string(),
                    rect: Some(WindowRect { left: 1920, top: 0, right: 3840, bottom: 1080, width: 1920, height: 1080 }),
                    work_area: Some(WindowRect { left: 1920, top: 0, right: 3840, bottom: 1040, width: 1920, height: 1040 }),
                    is_primary: false,
                    scale_factor: 1.0,
                    dpi_x: 96,
                    dpi_y: 96,
                },
            ],
        })
    }

    async fn get_window_monitor(&self, req: GetWindowMonitorRequest) -> Result<MonitorResponse> {
        let windows = self.windows.read().await;
        if let Some(window) = windows.get(&req.handle) {
            let monitor = MonitorInfo {
                name: "Monitor 1".to_string(),
                rect: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1080, width: 1920, height: 1080 }),
                work_area: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1040, width: 1920, height: 1040 }),
                is_primary: true,
                scale_factor: 1.0,
                dpi_x: 96,
                dpi_y: 96,
            };
            Ok(MonitorResponse { success: true, monitor: Some(monitor), error: String::new() })
        } else {
            Ok(MonitorResponse { success: false, monitor: None, error: "Window not found".to_string() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_window_adapter() {
        let adapter = MockWindowAdapter::new();

        let list = adapter.list_windows(ListWindowsRequest {
            visible_only: true,
            enabled_only: true,
            class_filter: String::new(),
            title_filter: String::new(),
            process_id_filter: 0,
        }).await.unwrap();
        assert_eq!(list.windows.len(), 1);

        let window = adapter.get_window(GetWindowRequest { handle: 1 }).await.unwrap();
        assert!(window.success);
        assert_eq!(window.window.as_ref().unwrap().title, "Mock Window 1");

        let fg = adapter.get_foreground_window().await.unwrap();
        assert!(fg.success);

        let moved = adapter.move_window(MoveWindowRequest { handle: 1, x: 100, y: 100, repaint: true }).await.unwrap();
        assert!(moved.success);
    }
}