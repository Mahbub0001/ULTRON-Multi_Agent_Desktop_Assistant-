//! Chrome Adapter Module
//!
//! Provides high-level interfaces to control Chrome via CDP (Chrome DevTools Protocol).

pub mod cdp;
pub mod session;

use crate::adapters_proto::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Chrome adapter trait
#[async_trait::async_trait]
pub trait ChromeAdapter: Send + Sync {
    // Browser lifecycle
    async fn connect(&self, req: ConnectRequest) -> Result<ConnectResponse>;
    async fn disconnect(&self, req: DisconnectRequest) -> Result<()>;
    async fn get_version(&self) -> Result<VersionResponse>;
    async fn get_targets(&self, req: GetTargetsRequest) -> Result<TargetList>;

    // Tab management
    async fn create_tab(&self, req: CreateTabRequest) -> Result<TabResponse>;
    async fn close_tab(&self, req: CloseTabRequest) -> Result<TabResponse>;
    async fn activate_tab(&self, req: ActivateTabRequest) -> Result<TabResponse>;
    async fn navigate(&self, req: NavigateRequest) -> Result<NavigationResponse>;
    async fn reload(&self, req: ReloadRequest) -> Result<NavigationResponse>;
    async fn go_back(&self, req: GoBackRequest) -> Result<NavigationResponse>;
    async fn go_forward(&self, req: GoForwardRequest) -> Result<NavigationResponse>;
    async fn get_tabs(&self) -> Result<TabList>;

    // DOM interaction
    async fn evaluate(&self, req: EvaluateRequest) -> Result<EvaluateResponse>;
    async fn call_function(&self, req: CallFunctionRequest) -> Result<EvaluateResponse>;
    async fn get_document(&self, req: GetDocumentRequest) -> Result<NodeResponse>;
    async fn query_selector(&self, req: QuerySelectorRequest) -> Result<NodeResponse>;
    async fn query_selector_all(&self, req: QuerySelectorAllRequest) -> Result<NodeList>;
    async fn click_element(&self, req: ClickElementRequest) -> Result<ClickResponse>;
    async fn type_text(&self, req: TypeTextRequest) -> Result<TypeResponse>;
    async fn get_element_bounds(&self, req: GetElementBoundsRequest) -> Result<ElementBoundsResponse>;
    async fn get_element_attributes(&self, req: GetElementAttributesRequest) -> Result<ElementAttributesResponse>;
    async fn set_element_attributes(&self, req: SetElementAttributesRequest) -> Result<ElementAttributesResponse>;
    async fn screenshot_element(&self, req: ScreenshotElementRequest) -> Result<ScreenshotResponse>;
    async fn scroll_into_view(&self, req: ScrollIntoViewRequest) -> Result<()>;

    // Network interception
    async fn enable_network(&self, req: EnableNetworkRequest) -> Result<()>;
    async fn disable_network(&self) -> Result<()>;
    async fn set_request_interception(&self, req: SetRequestInterceptionRequest) -> Result<()>;
    async fn continue_request(&self, req: ContinueRequestRequest) -> Result<()>;
    async fn modify_request(&self, req: ModifyRequestRequest) -> Result<()>;
    async fn block_urls(&self, req: BlockUrlsRequest) -> Result<()>;
    async fn get_network_logs(&self) -> Result<NetworkLogList>;

    // Console
    async fn enable_console(&self) -> Result<()>;
    async fn disable_console(&self) -> Result<()>;
    async fn get_console_logs(&self) -> Result<ConsoleLogList>;
    async fn clear_console(&self) -> Result<()>;

    // Screenshots
    async fn capture_screenshot(&self, req: CaptureScreenshotRequest) -> Result<ScreenshotResponse>;
    async fn capture_full_page_screenshot(&self, req: CaptureFullPageScreenshotRequest) -> Result<ScreenshotResponse>;

    // Cookies/Storage
    async fn get_cookies(&self, req: GetCookiesRequest) -> Result<CookieList>;
    async fn set_cookie(&self, req: SetCookieRequest) -> Result<CookieResponse>;
    async fn delete_cookie(&self, req: DeleteCookieRequest) -> Result<CookieResponse>;
    async fn get_local_storage(&self, req: GetLocalStorageRequest) -> Result<StorageResponse>;
    async fn set_local_storage(&self, req: SetLocalStorageRequest) -> Result<StorageResponse>;
}

/// Chrome adapter factory
pub struct ChromeAdapterFactory;

impl ChromeAdapterFactory {
    /// Create a Chrome adapter based on configuration
    pub async fn create(config: &crate::config::AdaptersConfig) -> Result<Arc<dyn ChromeAdapter>> {
        let adapter = cdp::CdpAdapter::new(config).await?;
        Ok(Arc::new(adapter))
    }
}

/// Mock adapter for testing
#[derive(Debug)]
pub struct MockChromeAdapter {
    tabs: Arc<RwLock<std::collections::HashMap<String, TabInfo>>>,
    console_logs: Arc<RwLock<Vec<ConsoleLogEntry>>>,
    network_logs: Arc<RwLock<Vec<NetworkLogEntry>>>,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl MockChromeAdapter {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(RwLock::new(std::collections::HashMap::new())),
            console_logs: Arc::new(RwLock::new(Vec::new())),
            network_logs: Arc::new(RwLock::new(Vec::new())),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    fn next_target_id(&self) -> String {
        format!("target_{}", self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }

    fn create_tab_info(&self, target_id: &str, url: &str) -> TabInfo {
        TabInfo {
            target_id: target_id.to_string(),
            title: "Mock Page".to_string(),
            url: url.to_string(),
            is_active: true,
            is_loading: false,
            favicon_url: String::new(),
        }
    }
}

#[async_trait::async_trait]
impl ChromeAdapter for MockChromeAdapter {
    async fn connect(&self, req: ConnectRequest) -> Result<ConnectResponse> {
        let session_id = format!("session_{}", self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        let target_id = self.next_target_id();
        let mut tabs = self.tabs.write().await;
        tabs.insert(target_id.clone(), self.create_tab_info(&target_id, "about:blank"));
        Ok(ConnectResponse { success: true, session_id, error: String::new() })
    }

    async fn disconnect(&self, _req: DisconnectRequest) -> Result<()> {
        self.tabs.write().await.clear();
        Ok(())
    }

    async fn get_version(&self) -> Result<VersionResponse> {
        Ok(VersionResponse {
            success: true,
            version: Some(BrowserVersion {
                protocol_version: "1.3".to_string(),
                product: "Chrome/120.0.0".to_string(),
                revision: "123456".to_string(),
                user_agent: "Mozilla/5.0 Mock".to_string(),
                js_version: "V8/12.0".to_string(),
            }),
            error: String::new(),
        })
    }

    async fn get_targets(&self, _req: GetTargetsRequest) -> Result<TargetList> {
        let tabs = self.tabs.read().await;
        Ok(TargetList {
            targets: tabs.values().map(|t| TargetInfo {
                target_id: t.target_id.clone(),
                type_: TargetType::TargetTypePage as i32,
                title: t.title.clone(),
                url: t.url.clone(),
                attached: true,
                opener_id: String::new(),
            }).collect(),
        })
    }

    async fn create_tab(&self, req: CreateTabRequest) -> Result<TabResponse> {
        let target_id = self.next_target_id();
        let tab = self.create_tab_info(&target_id, &req.url);
        self.tabs.write().await.insert(target_id.clone(), tab.clone());
        Ok(TabResponse { success: true, tab: Some(tab), error: String::new() })
    }

    async fn close_tab(&self, req: CloseTabRequest) -> Result<TabResponse> {
        let mut tabs = self.tabs.write().await;
        if tabs.remove(&req.target_id).is_some() {
            Ok(TabResponse { success: true, tab: None, error: String::new() })
        } else {
            Ok(TabResponse { success: false, tab: None, error: "Tab not found".to_string() })
        }
    }

    async fn activate_tab(&self, req: ActivateTabRequest) -> Result<TabResponse> {
        let mut tabs = self.tabs.write().await;
        for tab in tabs.values_mut() {
            tab.is_active = tab.target_id == req.target_id;
        }
        if let Some(tab) = tabs.get(&req.target_id) {
            Ok(TabResponse { success: true, tab: Some(tab.clone()), error: String::new() })
        } else {
            Ok(TabResponse { success: false, tab: None, error: "Tab not found".to_string() })
        }
    }

    async fn navigate(&self, req: NavigateRequest) -> Result<NavigationResponse> {
        let mut tabs = self.tabs.write().await;
        if let Some(tab) = tabs.get_mut(&req.target_id) {
            tab.url = req.url.clone();
            tab.title = format!("Page - {}", req.url);
            tab.is_loading = false;
            Ok(NavigationResponse {
                success: true,
                frame_id: "frame_1".to_string(),
                loader_id: "loader_1".to_string(),
                error: None,
            })
        } else {
            Ok(NavigationResponse {
                success: false,
                frame_id: String::new(),
                loader_id: String::new(),
                error: Some(NavigationError { error_text: "Tab not found".to_string(), error_code: -1 }),
            })
        }
    }

    async fn reload(&self, req: ReloadRequest) -> Result<NavigationResponse> {
        self.navigate(NavigateRequest {
            session_id: req.session_id,
            target_id: req.target_id,
            url: String::new(),
            options: None,
        }).await
    }

    async fn go_back(&self, req: GoBackRequest) -> Result<NavigationResponse> {
        Ok(NavigationResponse {
            success: true,
            frame_id: "frame_1".to_string(),
            loader_id: "loader_1".to_string(),
            error: None,
        })
    }

    async fn go_forward(&self, req: GoForwardRequest) -> Result<NavigationResponse> {
        Ok(NavigationResponse {
            success: true,
            frame_id: "frame_1".to_string(),
            loader_id: "loader_1".to_string(),
            error: None,
        })
    }

    async fn get_tabs(&self) -> Result<TabList> {
        let tabs = self.tabs.read().await;
        Ok(TabList { tabs: tabs.values().cloned().collect() })
    }

    async fn evaluate(&self, req: EvaluateRequest) -> Result<EvaluateResponse> {
        Ok(EvaluateResponse {
            success: true,
            result: Some(RemoteObject {
                type_: RemoteObjectType::RemoteObjectTypeString as i32,
                subtype: String::new(),
                class_name: String::new(),
                value: format!("\"Evaluated: {}\"", req.expression),
                object_id: String::new(),
                description: String::new(),
                preview_length: 0,
            }),
            exception: None,
            error: String::new(),
        })
    }

    async fn call_function(&self, req: CallFunctionRequest) -> Result<EvaluateResponse> {
        self.evaluate(EvaluateRequest {
            session_id: req.session_id,
            target_id: req.target_id,
            expression: req.function_declaration,
            options: req.options,
        }).await
    }

    async fn get_document(&self, req: GetDocumentRequest) -> Result<NodeResponse> {
        Ok(NodeResponse {
            success: true,
            node: Some(NodeInfo {
                node_id: 1,
                node_type: NodeType::NodeTypeDocument as i32,
                node_name: "#document".to_string(),
                local_name: String::new(),
                node_value: String::new(),
                child_count: 1,
                children: vec![NodeInfo {
                    node_id: 2,
                    node_type: NodeType::NodeTypeElement as i32,
                    node_name: "HTML".to_string(),
                    local_name: "html".to_string(),
                    node_value: String::new(),
                    child_count: 2,
                    children: vec![],
                    attributes: std::collections::HashMap::new(),
                    frame_id: req.target_id,
                    depth: 0,
                }],
                attributes: std::collections::HashMap::new(),
                frame_id: req.target_id,
                depth: 0,
            }),
            error: String::new(),
        })
    }

    async fn query_selector(&self, req: QuerySelectorRequest) -> Result<NodeResponse> {
        Ok(NodeResponse {
            success: true,
            node: Some(NodeInfo {
                node_id: 100,
                node_type: NodeType::NodeTypeElement as i32,
                node_name: "DIV".to_string(),
                local_name: "div".to_string(),
                node_value: String::new(),
                child_count: 0,
                children: vec![],
                attributes: {
                    let mut m = std::collections::HashMap::new();
                    m.insert("class".to_string(), "mock-element".to_string());
                    m
                },
                frame_id: req.target_id,
                depth: 1,
            }),
            error: String::new(),
        })
    }

    async fn query_selector_all(&self, req: QuerySelectorAllRequest) -> Result<NodeList> {
        Ok(NodeList {
            nodes: vec![
                NodeInfo {
                    node_id: 100,
                    node_type: NodeType::NodeTypeElement as i32,
                    node_name: "DIV".to_string(),
                    local_name: "div".to_string(),
                    node_value: String::new(),
                    child_count: 0,
                    children: vec![],
                    attributes: {
                        let mut m = std::collections::HashMap::new();
                        m.insert("class".to_string(), "mock-element".to_string());
                        m
                    },
                    frame_id: req.target_id,
                    depth: 1,
                },
                NodeInfo {
                    node_id: 101,
                    node_type: NodeType::NodeTypeElement as i32,
                    node_name: "SPAN".to_string(),
                    local_name: "span".to_string(),
                    node_value: String::new(),
                    child_count: 0,
                    children: vec![],
                    attributes: {
                        let mut m = std::collections::HashMap::new();
                        m.insert("class".to_string(), "mock-element".to_string());
                        m
                    },
                    frame_id: req.target_id,
                    depth: 1,
                },
            ],
        })
    }

    async fn click_element(&self, _req: ClickElementRequest) -> Result<ClickResponse> {
        Ok(ClickResponse { success: true, error: String::new() })
    }

    async fn type_text(&self, _req: TypeTextRequest) -> Result<TypeResponse> {
        Ok(TypeResponse { success: true, error: String::new() })
    }

    async fn get_element_bounds(&self, _req: GetElementBoundsRequest) -> Result<ElementBoundsResponse> {
        Ok(ElementBoundsResponse {
            success: true,
            bounds: Some(ElementBounds {
                x: 100.0, y: 100.0, width: 200.0, height: 50.0,
                top: 100.0, left: 100.0, bottom: 150.0, right: 300.0,
            }),
            error: String::new(),
        })
    }

    async fn get_element_attributes(&self, _req: GetElementAttributesRequest) -> Result<ElementAttributesResponse> {
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("class".to_string(), "mock".to_string());
        attrs.insert("id".to_string(), "test".to_string());
        Ok(ElementAttributesResponse { success: true, attributes: attrs, error: String::new() })
    }

    async fn set_element_attributes(&self, _req: SetElementAttributesRequest) -> Result<ElementAttributesResponse> {
        Ok(ElementAttributesResponse { success: true, attributes: std::collections::HashMap::new(), error: String::new() })
    }

    async fn screenshot_element(&self, _req: ScreenshotElementRequest) -> Result<ScreenshotResponse> {
        Ok(ScreenshotResponse {
            success: true,
            image_data: vec![0, 0, 0, 255],
            format: ImageFormat::ImageFormatPng as i32,
            width: 100,
            height: 100,
            error: String::new(),
        })
    }

    async fn scroll_into_view(&self, _req: ScrollIntoViewRequest) -> Result<()> {
        Ok(())
    }

    async fn enable_network(&self, _req: EnableNetworkRequest) -> Result<()> {
        Ok(())
    }

    async fn disable_network(&self) -> Result<()> {
        Ok(())
    }

    async fn set_request_interception(&self, _req: SetRequestInterceptionRequest) -> Result<()> {
        Ok(())
    }

    async fn continue_request(&self, _req: ContinueRequestRequest) -> Result<()> {
        Ok(())
    }

    async fn modify_request(&self, _req: ModifyRequestRequest) -> Result<()> {
        Ok(())
    }

    async fn block_urls(&self, _req: BlockUrlsRequest) -> Result<()> {
        Ok(())
    }

    async fn get_network_logs(&self) -> Result<NetworkLogList> {
        let logs = self.network_logs.read().await;
        Ok(NetworkLogList { logs: logs.clone() })
    }

    async fn enable_console(&self) -> Result<()> {
        Ok(())
    }

    async fn disable_console(&self) -> Result<()> {
        Ok(())
    }

    async fn get_console_logs(&self) -> Result<ConsoleLogList> {
        let logs = self.console_logs.read().await;
        Ok(ConsoleLogList { logs: logs.clone() })
    }

    async fn clear_console(&self) -> Result<()> {
        self.console_logs.write().await.clear();
        Ok(())
    }

    async fn capture_screenshot(&self, _req: CaptureScreenshotRequest) -> Result<ScreenshotResponse> {
        Ok(ScreenshotResponse {
            success: true,
            image_data: vec![0; 10000],
            format: ImageFormat::ImageFormatPng as i32,
            width: 1920,
            height: 1080,
            error: String::new(),
        })
    }

    async fn capture_full_page_screenshot(&self, _req: CaptureFullPageScreenshotRequest) -> Result<ScreenshotResponse> {
        self.capture_screenshot(CaptureScreenshotRequest {
            session_id: String::new(),
            target_id: String::new(),
            format: ImageFormat::ImageFormatPng as i32,
            quality: 80,
            clip: None,
            from_surface: true,
            capture_beyond_viewport: true,
        }).await
    }

    async fn get_cookies(&self, _req: GetCookiesRequest) -> Result<CookieList> {
        Ok(CookieList { cookies: vec![] })
    }

    async fn set_cookie(&self, _req: SetCookieRequest) -> Result<CookieResponse> {
        Ok(CookieResponse { success: true, error: String::new() })
    }

    async fn delete_cookie(&self, _req: DeleteCookieRequest) -> Result<CookieResponse> {
        Ok(CookieResponse { success: true, error: String::new() })
    }

    async fn get_local_storage(&self, _req: GetLocalStorageRequest) -> Result<StorageResponse> {
        Ok(StorageResponse { success: true, items: std::collections::HashMap::new(), error: String::new() })
    }

    async fn set_local_storage(&self, _req: SetLocalStorageRequest) -> Result<StorageResponse> {
        Ok(StorageResponse { success: true, items: std::collections::HashMap::new(), error: String::new() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_chrome_adapter() {
        let adapter = MockChromeAdapter::new();
        let resp = adapter.connect(ConnectRequest { endpoint: "http://localhost:9222".to_string(), timeout_ms: 5000 }).await.unwrap();
        assert!(resp.success);
        assert!(!resp.session_id.is_empty());

        let tabs = adapter.get_tabs().await.unwrap();
        assert_eq!(tabs.tabs.len(), 1);
    }
}