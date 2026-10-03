//! CEP (Common Extensibility Platform) Adapter for Photoshop
//!
//! Legacy Photoshop API using HTML5/JS extensions.

use crate::photoshop::{PhotoshopAdapter, MockPhotoshopAdapter};
use crate::adapters_proto::*;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// CEP-based Photoshop adapter
#[derive(Debug)]
pub struct CepAdapter {
    config: AdaptersConfig,
    port: u16,
    host: String,
    connected: Arc<RwLock<bool>>,
    // In a real implementation, this would communicate via CEP's VulcanInterface
    // For now, we delegate to the mock adapter
    mock: Arc<MockPhotoshopAdapter>,
}

impl CepAdapter {
    /// Create a new CEP adapter
    pub async fn new(config: &AdaptersConfig) -> Result<Self> {
        let port = config.photoshop.cep_port;
        let host = config.photoshop.cep_host.clone();

        info!(port, host, "Initializing CEP adapter");

        let mock = Arc::new(MockPhotoshopAdapter::new());

        Ok(Self {
            config: config.clone(),
            port,
            host,
            connected: Arc::new(RwLock::new(false)),
            mock,
        })
    }

    /// Connect to Photoshop CEP engine
    pub async fn connect(&self) -> Result<()> {
        // In a real implementation:
        // 1. Use VulcanInterface to communicate with Photoshop
        // 2. Load the CEP extension
        // 3. Establish message passing

        *self.connected.write().await = true;
        info!("Connected to Photoshop CEP engine");
        Ok(())
    }

    /// Disconnect from Photoshop CEP engine
    pub async fn disconnect(&self) -> Result<()> {
        *self.connected.write().await = false;
        info!("Disconnected from Photoshop CEP engine");
        Ok(())
    }

    /// Evaluate a JSX script in Photoshop
    async fn eval_jsx(&self, script: &str) -> Result<serde_json::Value> {
        // In a real implementation, use CSInterface.evalScript()
        debug!(script = %script.chars().take(100).collect::<String>(), "Evaluating JSX");
        Ok(serde_json::json!({ "success": true }))
    }
}

#[async_trait::async_trait]
impl PhotoshopAdapter for CepAdapter {
    async fn create_document(&self, req: CreateDocumentRequest) -> Result<DocumentResponse> {
        if !*self.connected.read().await {
            self.connect().await?;
        }
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
    async fn test_cep_adapter_creation() {
        let config = AdaptersConfig::default();
        let adapter = CepAdapter::new(&config).await;
        assert!(adapter.is_ok());
    }
}