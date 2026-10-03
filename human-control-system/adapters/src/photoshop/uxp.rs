//! UXP (Unified Extensibility Platform) Adapter for Photoshop
//!
//! Modern Photoshop API using JavaScript/TypeScript plugins.

use crate::photoshop::{PhotoshopAdapter, MockPhotoshopAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// UXP-based Photoshop adapter
#[derive(Debug)]
pub struct UxpAdapter {
    config: AdaptersConfig,
    port: u16,
    host: String,
    connected: Arc<RwLock<bool>>,
    // In a real implementation, this would be a WebSocket connection to Photoshop's UXP server
    // For now, we delegate to the mock adapter
    mock: Arc<MockPhotoshopAdapter>,
}

impl UxpAdapter {
    /// Create a new UXP adapter
    pub async fn new(config: &AdaptersConfig) -> Result<Self> {
        let port = config.photoshop.uxp_port;
        let host = config.photoshop.uxp_host.clone();

        info!(port, host, "Initializing UXP adapter");

        // In a real implementation, we'd connect to Photoshop's UXP WebSocket server
        // For now, we just return a mock adapter
        let mock = Arc::new(MockPhotoshopAdapter::new());

        Ok(Self {
            config: config.clone(),
            port,
            host,
            connected: Arc::new(RwLock::new(false)),
            mock,
        })
    }

    /// Connect to Photoshop UXP server
    pub async fn connect(&self) -> Result<()> {
        // In a real implementation:
        // 1. Connect to ws://host:port
        // 2. Send initialization message
        // 3. Wait for ready event
        // 4. Set up event listeners

        *self.connected.write().await = true;
        info!("Connected to Photoshop UXP server");
        Ok(())
    }

    /// Disconnect from Photoshop UXP server
    pub async fn disconnect(&self) -> Result<()> {
        *self.connected.write().await = false;
        info!("Disconnected from Photoshop UXP server");
        Ok(())
    }

    /// Execute a UXP command
    async fn execute_command(&self, command: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        // In a real implementation, send command over WebSocket and await response
        debug!(command, "Executing UXP command");
        Ok(serde_json::json!({ "success": true }))
    }
}

#[async_trait::async_trait]
impl PhotoshopAdapter for UxpAdapter {
    async fn create_document(&self, req: CreateDocumentRequest) -> Result<DocumentResponse> {
        if !*self.connected.read().await {
            self.connect().await?;
        }
        // Delegate to mock for now
        self.mock.create_document(req).await
    }

    async fn open_document(&self, req: OpenDocumentRequest) -> Result<DocumentResponse> {
        if !*self.connected.read().await {
            self.connect().await?;
        }
        self.mock.open_document(req).await
    }

    async fn save_document(&self, req: SaveDocumentRequest) -> Result<DocumentResponse> {
        self.mock.save_document(req).await
    }

    async fn close_document(&self, req: CloseDocumentRequest) -> Result<DocumentResponse> {
        self.mock.close_document(req).await
    }

    async fn get_document_info(&self, req: GetDocumentInfoRequest) -> Result<DocumentResponse> {
        self.mock.get_document_info(req).await
    }

    async fn list_documents(&self) -> Result<DocumentList> {
        self.mock.list_documents().await
    }

    async fn get_layers(&self, req: GetLayersRequest) -> Result<LayerList> {
        self.mock.get_layers(req).await
    }

    async fn create_layer(&self, req: CreateLayerRequest) -> Result<LayerResponse> {
        self.mock.create_layer(req).await
    }

    async fn delete_layer(&self, req: DeleteLayerRequest) -> Result<LayerResponse> {
        self.mock.delete_layer(req).await
    }

    async fn duplicate_layer(&self, req: DuplicateLayerRequest) -> Result<LayerResponse> {
        self.mock.duplicate_layer(req).await
    }

    async fn move_layer(&self, req: MoveLayerRequest) -> Result<LayerResponse> {
        self.mock.move_layer(req).await
    }

    async fn set_layer_visibility(&self, req: SetLayerVisibilityRequest) -> Result<LayerResponse> {
        self.mock.set_layer_visibility(req).await
    }

    async fn set_layer_opacity(&self, req: SetLayerOpacityRequest) -> Result<LayerResponse> {
        self.mock.set_layer_opacity(req).await
    }

    async fn set_layer_blend_mode(&self, req: SetLayerBlendModeRequest) -> Result<LayerResponse> {
        self.mock.set_layer_blend_mode(req).await
    }

    async fn get_layer_bounds(&self, req: GetLayerBoundsRequest) -> Result<LayerBoundsResponse> {
        self.mock.get_layer_bounds(req).await
    }

    async fn apply_layer_style(&self, req: ApplyLayerStyleRequest) -> Result<LayerResponse> {
        self.mock.apply_layer_style(req).await
    }

    async fn apply_adjustment(&self, req: ApplyAdjustmentRequest) -> Result<AdjustmentResponse> {
        self.mock.apply_adjustment(req).await
    }

    async fn apply_filter(&self, req: ApplyFilterRequest) -> Result<FilterResponse> {
        self.mock.apply_filter(req).await
    }

    async fn play_action(&self, req: PlayActionRequest) -> Result<ActionResponse> {
        self.mock.play_action(req).await
    }

    async fn list_actions(&self, req: ListActionsRequest) -> Result<ActionList> {
        self.mock.list_actions(req).await
    }

    async fn batch_process(&self, req: BatchProcessRequest) -> Result<BatchProcessResponse> {
        self.mock.batch_process(req).await
    }

    async fn export_document(&self, req: ExportDocumentRequest) -> Result<ExportResponse> {
        self.mock.export_document(req).await
    }

    async fn import_file(&self, req: ImportFileRequest) -> Result<ImportResponse> {
        self.mock.import_file(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdaptersConfig;

    #[tokio::test]
    async fn test_uxp_adapter_creation() {
        let config = AdaptersConfig::default();
        let adapter = UxpAdapter::new(&config).await;
        assert!(adapter.is_ok());
    }
}