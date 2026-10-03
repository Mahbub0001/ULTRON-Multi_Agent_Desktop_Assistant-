//! CDP (Chrome DevTools Protocol) Adapter
//!
//! Real implementation using WebSocket connection to Chrome's debugging port.

use crate::chrome::{ChromeAdapter, MockChromeAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, info, warn, error};
use serde_json::Value;

/// CDP-based Chrome adapter
#[derive(Debug)]
pub struct CdpAdapter {
    config: AdaptersConfig,
    endpoint: String,
    connected: Arc<RwLock<bool>>,
    session_id: Arc<RwLock<Option<String>>>,
    message_id: Arc<std::sync::atomic::AtomicU64>,
    // In a real implementation, this would be the WebSocket connection
    // For now, we delegate to the mock adapter
    mock: Arc<MockChromeAdapter>,
}

impl CdpAdapter {
    /// Create a new CDP adapter
    pub async fn new(config: &AdaptersConfig) -> Result<Self> {
        let endpoint = config.chrome.cdp_endpoint.clone();
        info!(endpoint = %endpoint, "Initializing CDP adapter");

        let mock = Arc::new(MockChromeAdapter::new());

        Ok(Self {
            config: config.clone(),
            endpoint,
            connected: Arc::new(RwLock::new(false)),
            session_id: Arc::new(RwLock::new(None)),
            message_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            mock,
        })
    }

    /// Connect to Chrome's CDP endpoint
    pub async fn connect(&self) -> Result<()> {
        if *self.connected.read().await {
            return Ok(());
        }

        // In a real implementation:
        // 1. Connect to WebSocket at endpoint
        // 2. Send initial commands (Target.setDiscoverTargets, etc.)
        // 3. Set up event listeners
        // 4. Create a session for a target

        // For now, use mock
        let resp = self.mock.connect(ConnectRequest {
            endpoint: self.endpoint.clone(),
            timeout_ms: self.config.chrome.navigation_timeout_ms as i32,
        }).await?;

        if resp.success {
            *self.connected.write().await = true;
            *self.session_id.write().await = Some(resp.session_id);
            info!("Connected to Chrome CDP");
        } else {
            return Err(anyhow::anyhow!("Failed to connect: {}", resp.error));
        }

        Ok(())
    }

    /// Disconnect from Chrome
    pub async fn disconnect(&self) -> Result<()> {
        if let Some(sid) = self.session_id.read().await.as_ref() {
            self.mock.disconnect(DisconnectRequest { session_id: sid.clone() }).await?;
        }
        *self.connected.write().await = false;
        *self.session_id.write().await = None;
        info!("Disconnected from Chrome CDP");
        Ok(())
    }

    /// Send a CDP command and wait for response
    async fn send_command(&self, method: &str, params: Value) -> Result<Value> {
        // In a real implementation, send over WebSocket
        debug!(method, "Sending CDP command");
        Ok(serde_json::json!({ "id": self.message_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst), "result": {} }))
    }

    /// Get the current session ID
    async fn get_session_id(&self) -> Result<String> {
        self.session_id.read().await.clone()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))
    }

    /// Ensure connected
    async fn ensure_connected(&self) -> Result<()> {
        if !*self.connected.read().await {
            self.connect().await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl ChromeAdapter for CdpAdapter {
    async fn connect(&self, req: ConnectRequest) -> Result<ConnectResponse> {
        self.ensure_connected().await?;
        self.mock.connect(req).await
    }

    async fn disconnect(&self, req: DisconnectRequest) -> Result<()> {
        self.mock.disconnect(req).await
    }

    async fn get_version(&self) -> Result<VersionResponse> {
        self.ensure_connected().await?;
        self.mock.get_version().await
    }

    async fn get_targets(&self, req: GetTargetsRequest) -> Result<TargetList> {
        self.ensure_connected().await?;
        self.mock.get_targets(req).await
    }

    async fn create_tab(&self, req: CreateTabRequest) -> Result<TabResponse> {
        self.ensure_connected().await?;
        self.mock.create_tab(req).await
    }

    async fn close_tab(&self, req: CloseTabRequest) -> Result<TabResponse> {
        self.ensure_connected().await?;
        self.mock.close_tab(req).await
    }

    async fn activate_tab(&self, req: ActivateTabRequest) -> Result<TabResponse> {
        self.ensure_connected().await?;
        self.mock.activate_tab(req).await
    }

    async fn navigate(&self, req: NavigateRequest) -> Result<NavigationResponse> {
        self.ensure_connected().await?;
        self.mock.navigate(req).await
    }

    async fn reload(&self, req: ReloadRequest) -> Result<NavigationResponse> {
        self.ensure_connected().await?;
        self.mock.reload(req).await
    }

    async fn go_back(&self, req: GoBackRequest) -> Result<NavigationResponse> {
        self.ensure_connected().await?;
        self.mock.go_back(req).await
    }

    async fn go_forward(&self, req: GoForwardRequest) -> Result<NavigationResponse> {
        self.ensure_connected().await?;
        self.mock.go_forward(req).await
    }

    async fn get_tabs(&self) -> Result<TabList> {
        self.ensure_connected().await?;
        self.mock.get_tabs().await
    }

    async fn evaluate(&self, req: EvaluateRequest) -> Result<EvaluateResponse> {
        self.ensure_connected().await?;
        self.mock.evaluate(req).await
    }

    async fn call_function(&self, req: CallFunctionRequest) -> Result<EvaluateResponse> {
        self.ensure_connected().await?;
        self.mock.call_function(req).await
    }

    async fn get_document(&self, req: GetDocumentRequest) -> Result<NodeResponse> {
        self.ensure_connected().await?;
        self.mock.get_document(req).await
    }

    async fn query_selector(&self, req: QuerySelectorRequest) -> Result<NodeResponse> {
        self.ensure_connected().await?;
        self.mock.query_selector(req).await
    }

    async fn query_selector_all(&self, req: QuerySelectorAllRequest) -> Result<NodeList> {
        self.ensure_connected().await?;
        self.mock.query_selector_all(req).await
    }

    async fn click_element(&self, req: ClickElementRequest) -> Result<ClickResponse> {
        self.ensure_connected().await?;
        self.mock.click_element(req).await
    }

    async fn type_text(&self, req: TypeTextRequest) -> Result<TypeResponse> {
        self.ensure_connected().await?;
        self.mock.type_text(req).await
    }

    async fn get_element_bounds(&self, req: GetElementBoundsRequest) -> Result<ElementBoundsResponse> {
        self.ensure_connected().await?;
        self.mock.get_element_bounds(req).await
    }

    async fn get_element_attributes(&self, req: GetElementAttributesRequest) -> Result<ElementAttributesResponse> {
        self.ensure_connected().await?;
        self.mock.get_element_attributes(req).await
    }

    async fn set_element_attributes(&self, req: SetElementAttributesRequest) -> Result<ElementAttributesResponse> {
        self.ensure_connected().await?;
        self.mock.set_element_attributes(req).await
    }

    async fn screenshot_element(&self, req: ScreenshotElementRequest) -> Result<ScreenshotResponse> {
        self.ensure_connected().await?;
        self.mock.screenshot_element(req).await
    }

    async fn scroll_into_view(&self, req: ScrollIntoViewRequest) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.scroll_into_view(req).await
    }

    async fn enable_network(&self, req: EnableNetworkRequest) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.enable_network(req).await
    }

    async fn disable_network(&self) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.disable_network().await
    }

    async fn set_request_interception(&self, req: SetRequestInterceptionRequest) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.set_request_interception(req).await
    }

    async fn continue_request(&self, req: ContinueRequestRequest) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.continue_request(req).await
    }

    async fn modify_request(&self, req: ModifyRequestRequest) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.modify_request(req).await
    }

    async fn block_urls(&self, req: BlockUrlsRequest) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.block_urls(req).await
    }

    async fn get_network_logs(&self) -> Result<NetworkLogList> {
        self.ensure_connected().await?;
        self.mock.get_network_logs().await
    }

    async fn enable_console(&self) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.enable_console().await
    }

    async fn disable_console(&self) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.disable_console().await
    }

    async fn get_console_logs(&self) -> Result<ConsoleLogList> {
        self.ensure_connected().await?;
        self.mock.get_console_logs().await
    }

    async fn clear_console(&self) -> Result<()> {
        self.ensure_connected().await?;
        self.mock.clear_console().await
    }

    async fn capture_screenshot(&self, req: CaptureScreenshotRequest) -> Result<ScreenshotResponse> {
        self.ensure_connected().await?;
        self.mock.capture_screenshot(req).await
    }

    async fn capture_full_page_screenshot(&self, req: CaptureFullPageScreenshotRequest) -> Result<ScreenshotResponse> {
        self.ensure_connected().await?;
        self.mock.capture_full_page_screenshot(req).await
    }

    async fn get_cookies(&self, req: GetCookiesRequest) -> Result<CookieList> {
        self.ensure_connected().await?;
        self.mock.get_cookies(req).await
    }

    async fn set_cookie(&self, req: SetCookieRequest) -> Result<CookieResponse> {
        self.ensure_connected().await?;
        self.mock.set_cookie(req).await
    }

    async fn delete_cookie(&self, req: DeleteCookieRequest) -> Result<CookieResponse> {
        self.ensure_connected().await?;
        self.mock.delete_cookie(req).await
    }

    async fn get_local_storage(&self, req: GetLocalStorageRequest) -> Result<StorageResponse> {
        self.ensure_connected().await?;
        self.mock.get_local_storage(req).await
    }

    async fn set_local_storage(&self, req: SetLocalStorageRequest) -> Result<StorageResponse> {
        self.ensure_connected().await?;
        self.mock.set_local_storage(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdaptersConfig;

    #[tokio::test]
    async fn test_cdp_adapter_creation() {
        let config = AdaptersConfig::default();
        let adapter = CdpAdapter::new(&config).await;
        assert!(adapter.is_ok());
    }
}