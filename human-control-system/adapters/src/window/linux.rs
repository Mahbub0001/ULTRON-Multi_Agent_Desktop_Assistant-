//! Linux Window Adapter
//!
//! Implements window management using X11/Wayland.

use crate::window::{WindowAdapter, MockWindowAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info, warn};

#[cfg(target_os = "linux")]
use x11rb::{connection::Connection, protocol::xproto::*, wrapper::ConnectionExt as _};

/// Linux-specific window adapter
#[derive(Debug)]
pub struct LinuxWindowAdapter {
    config: AdaptersConfig,
    use_x11: bool,
    use_wayland: bool,
    capture_cursor: bool,
    // For now, delegate to mock
    mock: Arc<MockWindowAdapter>,
}

impl LinuxWindowAdapter {
    /// Create a new Linux window adapter
    pub async fn new(config: &AdaptersConfig) -> Result<Self> {
        info!("Initializing Linux Window adapter");

        let use_x11 = config.window.linux.use_x11;
        let use_wayland = config.window.linux.use_wayland;
        let capture_cursor = config.window.linux.capture_cursor;

        let mock = Arc::new(MockWindowAdapter::new());

        Ok(Self {
            config: config.clone(),
            use_x11,
            use_wayland,
            capture_cursor,
            mock,
        })
    }

    /// Connect to X11 server
    #[cfg(target_os = "linux")]
    fn connect_x11() -> Result<x11rb::rust_connection::RustConnection> {
        let (conn, screen_num) = x11rb::connect(None)?;
        Ok(conn)
    }

    /// Get window attributes
    #[cfg(target_os = "linux")]
    async fn get_window_attributes(&self, conn: &x11rb::rust_connection::RustConnection, window: Window) -> Result<WindowInfo> {
        let attrs = conn.get_window_attributes(window)?.reply()?;

        // Get window title
        let title = conn.get_property(
            false,
            window,
            AtomEnum::WM_NAME,
            AtomEnum::STRING,
            0,
            1024,
        )?.reply()?.value;
        let title = String::from_utf8_lossy(&title).to_string();

        // Get window class
        let class = conn.get_property(
            false,
            window,
            AtomEnum::WM_CLASS,
            AtomEnum::STRING,
            0,
            1024,
        )?.reply()?.value;
        let class_name = String::from_utf8_lossy(&class).to_string();

        // Get PID (_NET_WM_PID)
        let pid_atom = conn.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
        let pid_prop = conn.get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)?.reply()?;
        let pid = if pid_prop.value.len() >= 4 {
            u32::from_le_bytes(pid_prop.value[..4].try_into().unwrap())
        } else { 0 };

        Ok(WindowInfo {
            handle: window as u64,
            title,
            class_name,
            process_id: pid,
            thread_id: 0,
            rect: Some(WindowRect {
                left: attrs.x,
                top: attrs.y,
                right: attrs.x + attrs.width as i32,
                bottom: attrs.y + attrs.height as i32,
                width: attrs.width as i32,
                height: attrs.height as i32,
            }),
            client_rect: Some(WindowRect {
                left: 0,
                top: 0,
                right: attrs.width as i32,
                bottom: attrs.height as i32,
                width: attrs.width as i32,
                height: attrs.height as i32,
            }),
            is_visible: attrs.map_state == MapState::VIEWABLE,
            is_enabled: true, // X11 doesn't have explicit enabled state
            is_minimized: false, // Would need to check WM_STATE
            is_maximized: false, // Would need to check _NET_WM_STATE
            is_foreground: false, // Would need to check _NET_ACTIVE_WINDOW
            styles: None,
            ex_styles: None,
            z_order: 0,
            parent_handle: attrs.parent as u64,
            owner_handle: 0,
        })
    }

    /// List all windows on X11
    #[cfg(target_os = "linux")]
    async fn list_windows_x11(&self) -> Result<Vec<WindowInfo>> {
        let conn = Self::connect_x11()?;
        let screen = &conn.setup().roots[conn.setup().roots.len().min(1) - 1];
        let root = screen.root;

        let mut windows = Vec::new();
        self.enum_x11_windows(&conn, root, &mut windows).await?;
        Ok(windows)
    }

    #[cfg(target_os = "linux")]
    fn enum_x11_windows<'a>(&'a self, conn: &'a x11rb::rust_connection::RustConnection, window: Window, windows: &'a mut Vec<WindowInfo>) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            let tree = conn.query_tree(window)?.reply()?;
            for child in tree.children {
                if let Ok(info) = self.get_window_attributes(conn, child).await {
                    windows.push(info);
                }
                self.enum_x11_windows(conn, child, windows).await?;
            }
            Ok(())
        })
    }

    /// Capture window using X11
    #[cfg(target_os = "linux")]
    async fn capture_x11_window(&self, conn: &x11rb::rust_connection::RustConnection, window: Window, format: ImageFormat) -> Result<ScreenshotResponse> {
        let attrs = conn.get_window_attributes(window)?.reply()?;
        let width = attrs.width as u32;
        let height = attrs.height as u32;

        let pixmap = conn.generate_id()?;
        conn.create_pixmap(
            attrs.depth as u8,
            pixmap,
            window,
            width as u16,
            height as u16,
        )?.check()?;

        let gc = conn.generate_id()?;
        conn.create_gc(
            gc,
            pixmap,
            &CreateGCAux::default().graphics_exposures(0),
        )?.check()?;

        conn.copy_area(window, pixmap, gc, 0, 0, 0, 0, width as u16, height as u16)?.check()?;

        let image = conn.get_image(
            ImageFormat::Z_PIXMAP,
            pixmap,
            0, 0,
            width as u16,
            height as u16,
            u32::MAX,
        )?.reply()?;

        conn.free_pixmap(pixmap)?.check()?;
        conn.free_gc(gc)?.check()?;

        let data = image.data;

        // Convert format
        let image_data = match format {
            ImageFormat::ImageFormatBgr => {
                let mut bgr = Vec::with_capacity((width * height * 3) as usize);
                for chunk in data.chunks_exact(4) {
                    bgr.extend_from_slice(&[chunk[0], chunk[1], chunk[2]]);
                }
                bgr
            }
            ImageFormat::ImageFormatRgb => {
                let mut rgb = Vec::with_capacity((width * height * 3) as usize);
                for chunk in data.chunks_exact(4) {
                    rgb.extend_from_slice(&[chunk[2], chunk[1], chunk[0]]);
                }
                rgb
            }
            ImageFormat::ImageFormatRgba => {
                let mut rgba = Vec::with_capacity((width * height * 4) as usize);
                for chunk in data.chunks_exact(4) {
                    rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
                }
                rgba
            }
            _ => data,
        };

        Ok(ScreenshotResponse {
            success: true,
            image_data,
            format: format as i32,
            width: width as i32,
            height: height as i32,
            error: String::new(),
        })
    }
}

#[async_trait::async_trait]
impl WindowAdapter for LinuxWindowAdapter {
    async fn list_windows(&self, req: ListWindowsRequest) -> Result<WindowList> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let mut windows = self.list_windows_x11().await?;

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
        }

        #[cfg(not(target_os = "linux"))]
        {
            return self.mock.list_windows(req).await;
        }

        #[cfg(target_os = "linux")]
        {
            // Wayland or fallback
            return self.mock.list_windows(req).await;
        }
    }

    async fn get_window(&self, req: GetWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                match self.get_window_attributes(&conn, window).await {
                    Ok(info) => return Ok(WindowResponse { success: true, window: Some(info), error: String::new() }),
                    Err(e) => return Ok(WindowResponse { success: false, window: None, error: e.to_string() }),
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            return self.mock.get_window(req).await;
        }

        #[cfg(target_os = "linux")]
        {
            return self.mock.get_window(req).await;
        }
    }

    async fn find_window(&self, req: FindWindowRequest) -> Result<WindowResponse> {
        // X11 doesn't have direct FindWindow, need to enumerate
        let list = self.list_windows(ListWindowsRequest {
            visible_only: false,
            enabled_only: false,
            class_filter: req.class_name.clone(),
            title_filter: req.window_title.clone(),
            process_id_filter: 0,
        }).await?;

        if let Some(window) = list.windows.into_iter().next() {
            Ok(WindowResponse { success: true, window: Some(window), error: String::new() })
        } else {
            Ok(WindowResponse { success: false, window: None, error: "Window not found".to_string() })
        }
    }

    async fn get_foreground_window(&self) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let active_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;
                let root = conn.setup().roots[0].root;
                let prop = conn.get_property(false, root, active_atom, AtomEnum::WINDOW, 0, 1)?.reply()?;

                if prop.value.len() >= 4 {
                    let window = Window(u32::from_le_bytes(prop.value[..4].try_into().unwrap()));
                    return self.get_window(GetWindowRequest { handle: window as u64 }).await;
                }
            }
        }

        self.mock.get_foreground_window().await
    }

    async fn get_desktop_window(&self) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let root = conn.setup().roots[0].root;
                return self.get_window(GetWindowRequest { handle: root as u64 }).await;
            }
        }

        self.mock.get_desktop_window().await
    }

    async fn get_shell_window(&self) -> Result<WindowResponse> {
        self.get_desktop_window().await
    }

    async fn focus_window(&self, req: FocusWindowRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn activate_window(&self, req: ActivateWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let active_atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;
                let root = conn.setup().roots[0].root;

                let mut data = [0u8; 4];
                data.copy_from_slice(&(req.handle as u32).to_le_bytes());
                conn.change_property(
                    PropMode::REPLACE,
                    root,
                    active_atom,
                    AtomEnum::WINDOW,
                    32,
                    1,
                    &data,
                )?.check()?;

                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.activate_window(req).await
    }

    async fn show_window(&self, req: ShowWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);

                match req.command() {
                    ShowCommand::ShowCommandHide => {
                        conn.unmap_window(window)?.check()?;
                    }
                    ShowCommand::ShowCommandMinimized => {
                        let wm_state = conn.intern_atom(false, b"WM_STATE")?.reply()?.atom;
                        conn.change_property(
                            PropMode::REPLACE,
                            window,
                            wm_state,
                            wm_state,
                            32,
                            2,
                            &[0u8, 0, 3, 0, 0, 0, 0, 0], // Iconic state
                        )?.check()?;
                    }
                    ShowCommand::ShowCommandMaximized => {
                        let net_wm_state = conn.intern_atom(false, b"_NET_WM_STATE")?.reply()?.atom;
                        let maximized_horz = conn.intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_HORZ")?.reply()?.atom;
                        let maximized_vert = conn.intern_atom(false, b"_NET_WM_STATE_MAXIMIZED_VERT")?.reply()?.atom;
                        conn.change_property(
                            PropMode::REPLACE,
                            window,
                            net_wm_state,
                            AtomEnum::ATOM,
                            32,
                            2,
                            &[maximized_horz, maximized_vert].iter().flat_map(|a| a.to_le_bytes()).collect::<Vec<_>>(),
                        )?.check()?;
                    }
                    _ => {
                        conn.map_window(window)?.check()?;
                    }
                }
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.show_window(req).await
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
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let wm_delete = conn.intern_atom(false, b"WM_DELETE_WINDOW")?.reply()?.atom;
                let protocols = conn.intern_atom(false, b"WM_PROTOCOLS")?.reply()?.atom;

                // Send WM_DELETE_WINDOW message
                let event = ClientMessageEvent::new(
                    32,
                    window,
                    protocols,
                    [wm_delete, 0, 0, 0, 0],
                );
                conn.send_event(false, window, EventMask::NO_EVENT, event)?.check()?;
                conn.flush()?;
                return Ok(WindowResponse { success: true, window: None, error: String::new() });
            }
        }

        self.mock.close_window(req).await
    }

    async fn is_window_visible(&self, req: IsWindowVisibleRequest) -> Result<WindowVisibilityResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let attrs = conn.get_window_attributes(window)?.reply()?;
                return Ok(WindowVisibilityResponse { success: true, visible: attrs.map_state == MapState::VIEWABLE, error: String::new() });
            }
        }

        self.mock.is_window_visible(req).await
    }

    async fn is_window_enabled(&self, req: IsWindowEnabledRequest) -> Result<WindowEnabledResponse> {
        // X11 doesn't have explicit enabled state
        self.mock.is_window_enabled(req).await
    }

    async fn get_window_state(&self, req: GetWindowStateRequest) -> Result<WindowStateResponse> {
        self.mock.get_window_state(req).await
    }

    async fn move_window(&self, req: MoveWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                conn.configure_window(window, &ConfigureWindowAux::default().x(req.x).y(req.y))?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.move_window(req).await
    }

    async fn resize_window(&self, req: ResizeWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                conn.configure_window(window, &ConfigureWindowAux::default().width(req.width as u16).height(req.height as u16))?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.resize_window(req).await
    }

    async fn move_resize_window(&self, req: MoveResizeWindowRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                conn.configure_window(
                    window,
                    &ConfigureWindowAux::default()
                        .x(req.x)
                        .y(req.y)
                        .width(req.width as u16)
                        .height(req.height as u16),
                )?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.move_resize_window(req).await
    }

    async fn get_window_rect(&self, req: GetWindowRectRequest) -> Result<WindowRectResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let attrs = conn.get_window_attributes(window)?.reply()?;
                return Ok(WindowRectResponse {
                    success: true,
                    rect: Some(WindowRect {
                        left: attrs.x,
                        top: attrs.y,
                        right: attrs.x + attrs.width as i32,
                        bottom: attrs.y + attrs.height as i32,
                        width: attrs.width as i32,
                        height: attrs.height as i32,
                    }),
                    error: String::new(),
                });
            }
        }

        self.mock.get_window_rect(req).await
    }

    async fn get_client_rect(&self, req: GetClientRectRequest) -> Result<WindowRectResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let geom = conn.get_geometry(window)?.reply()?;
                return Ok(WindowRectResponse {
                    success: true,
                    rect: Some(WindowRect {
                        left: 0,
                        top: 0,
                        right: geom.width as i32,
                        bottom: geom.height as i32,
                        width: geom.width as i32,
                        height: geom.height as i32,
                    }),
                    error: String::new(),
                });
            }
        }

        self.mock.get_client_rect(req).await
    }

    async fn set_window_pos(&self, req: SetWindowPosRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let mut aux = ConfigureWindowAux::default();

                if req.flags & SetWindowPosFlags::SetWindowPosFlagNomove as i32 == 0 {
                    aux = aux.x(req.x).y(req.y);
                }
                if req.flags & SetWindowPosFlags::SetWindowPosFlagNosize as i32 == 0 {
                    aux = aux.width(req.width as u16).height(req.height as u16);
                }

                conn.configure_window(window, &aux)?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.set_window_pos(req).await
    }

    async fn capture_window(&self, req: CaptureWindowRequest) -> Result<ScreenshotResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                return self.capture_x11_window(&conn, window, req.format()).await;
            }
        }

        self.mock.capture_window(req).await
    }

    async fn capture_client_area(&self, req: CaptureClientAreaRequest) -> Result<ScreenshotResponse> {
        // Same as capture_window for X11
        self.capture_window(CaptureWindowRequest {
            handle: req.handle,
            format: req.format,
            quality: req.quality,
            include_nonclient: false,
        }).await
    }

    async fn capture_region(&self, req: CaptureRegionRequest) -> Result<ScreenshotResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let root = conn.setup().roots[0].root;
                let rect = req.region;
                let width = rect.width as u32;
                let height = rect.height as u32;

                let pixmap = conn.generate_id()?;
                let screen = &conn.setup().roots[0];
                conn.create_pixmap(screen.root_depth, pixmap, root, width as u16, height as u16)?.check()?;

                let gc = conn.generate_id()?;
                conn.create_gc(gc, pixmap, &CreateGCAux::default().graphics_exposures(0))?.check()?;

                conn.copy_area(root, pixmap, gc, rect.left as u16, rect.top as u16, 0, 0, width as u16, height as u16)?.check()?;

                let image = conn.get_image(
                    ImageFormat::Z_PIXMAP,
                    pixmap,
                    0, 0,
                    width as u16,
                    height as u16,
                    u32::MAX,
                )?.reply()?;

                conn.free_pixmap(pixmap)?.check()?;
                conn.free_gc(gc)?.check()?;

                return Ok(ScreenshotResponse {
                    success: true,
                    image_data: image.data,
                    format: req.format as i32,
                    width: width as i32,
                    height: height as i32,
                    error: String::new(),
                });
            }
        }

        self.mock.capture_region(req).await
    }

    async fn get_window_title(&self, req: GetWindowTitleRequest) -> Result<WindowTitleResponse> {
        let resp = self.get_window(GetWindowRequest { handle: req.handle }).await?;
        if resp.success {
            Ok(WindowTitleResponse { success: true, title: resp.window.unwrap().title, error: String::new() })
        } else {
            Ok(WindowTitleResponse { success: false, title: String::new(), error: resp.error })
        }
    }

    async fn set_window_title(&self, req: SetWindowTitleRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                let title_bytes = req.title.as_bytes();
                conn.change_property(
                    PropMode::REPLACE,
                    window,
                    AtomEnum::WM_NAME,
                    AtomEnum::STRING,
                    8,
                    title_bytes.len() as u32,
                    title_bytes,
                )?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.set_window_title(req).await
    }

    async fn get_window_class(&self, req: GetWindowClassRequest) -> Result<WindowClassResponse> {
        let resp = self.get_window(GetWindowRequest { handle: req.handle }).await?;
        if resp.success {
            Ok(WindowClassResponse { success: true, class_name: resp.window.unwrap().class_name, error: String::new() })
        } else {
            Ok(WindowClassResponse { success: false, class_name: String::new(), error: resp.error })
        }
    }

    async fn get_window_process_id(&self, req: GetWindowProcessIdRequest) -> Result<WindowProcessIdResponse> {
        let resp = self.get_window(GetWindowRequest { handle: req.handle }).await?;
        if resp.success {
            let w = resp.window.unwrap();
            Ok(WindowProcessIdResponse { success: true, process_id: w.process_id, thread_id: w.thread_id, error: String::new() })
        } else {
            Ok(WindowProcessIdResponse { success: false, process_id: 0, thread_id: 0, error: resp.error })
        }
    }

    async fn get_window_styles(&self, req: GetWindowStylesRequest) -> Result<WindowStylesResponse> {
        // X11 doesn't have Windows-style styles
        Ok(WindowStylesResponse { success: true, styles: None, ex_styles: None, error: String::new() })
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
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                conn.configure_window(window, &ConfigureWindowAux::default().stack_mode(StackMode::ABOVE))?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.bring_to_top(req).await
    }

    async fn send_to_bottom(&self, req: SendToBottomRequest) -> Result<WindowResponse> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let window = Window(req.handle as u32);
                conn.configure_window(window, &ConfigureWindowAux::default().stack_mode(StackMode::BELOW))?.check()?;
                conn.flush()?;
                return self.get_window(GetWindowRequest { handle: req.handle }).await;
            }
        }

        self.mock.send_to_bottom(req).await
    }

    async fn set_window_z_order(&self, req: SetWindowZOrderRequest) -> Result<WindowResponse> {
        // X11 doesn't support arbitrary Z-order insertion easily
        self.mock.set_window_z_order(req).await
    }

    async fn set_focus(&self, req: SetFocusRequest) -> Result<WindowResponse> {
        self.activate_window(ActivateWindowRequest { handle: req.handle }).await
    }

    async fn get_focus(&self) -> Result<WindowResponse> {
        self.get_foreground_window().await
    }

    async fn post_message(&self, _req: PostMessageRequest) -> Result<MessageResponse> {
        // X11 uses events, not messages
        Ok(MessageResponse { success: true, result: 0, error: String::new() })
    }

    async fn send_message(&self, _req: SendMessageRequest) -> Result<MessageResponse> {
        Ok(MessageResponse { success: true, result: 0, error: String::new() })
    }

    async fn get_monitors(&self) -> Result<MonitorList> {
        #[cfg(target_os = "linux")]
        {
            if self.use_x11 {
                let conn = Self::connect_x11()?;
                let mut monitors = Vec::new();

                // Use XRandR to get monitors
                // For simplicity, return mock data
                monitors.push(MonitorInfo {
                    name: "Monitor 1".to_string(),
                    rect: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1080, width: 1920, height: 1080 }),
                    work_area: Some(WindowRect { left: 0, top: 0, right: 1920, bottom: 1040, width: 1920, height: 1040 }),
                    is_primary: true,
                    scale_factor: 1.0,
                    dpi_x: 96,
                    dpi_y: 96,
                });

                return Ok(MonitorList { monitors });
            }
        }

        self.mock.get_monitors().await
    }

    async fn get_window_monitor(&self, req: GetWindowMonitorRequest) -> Result<MonitorResponse> {
        let monitors = self.get_monitors().await?;
        if let Some(monitor) = monitors.monitors.first() {
            Ok(MonitorResponse { success: true, monitor: Some(monitor.clone()), error: String::new() })
        } else {
            Ok(MonitorResponse { success: false, monitor: None, error: "No monitors".to_string() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdaptersConfig;

    #[tokio::test]
    async fn test_linux_adapter_creation() {
        let config = AdaptersConfig::default();
        let adapter = LinuxWindowAdapter::new(&config).await;
        assert!(adapter.is_ok());
    }
}