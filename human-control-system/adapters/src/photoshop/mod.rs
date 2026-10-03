//! Photoshop Adapter Module
//!
//! Provides high-level interfaces to control Adobe Photoshop via UXP/CEP APIs.

pub mod uxp;
pub mod cep;
pub mod api;

use crate::adapters_proto::*;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Photoshop adapter trait
#[async_trait::async_trait]
pub trait PhotoshopAdapter: Send + Sync {
    /// Create a new document
    async fn create_document(&self, req: CreateDocumentRequest) -> Result<DocumentResponse>;

    /// Open an existing document
    async fn open_document(&self, req: OpenDocumentRequest) -> Result<DocumentResponse>;

    /// Save a document
    async fn save_document(&self, req: SaveDocumentRequest) -> Result<DocumentResponse>;

    /// Close a document
    async fn close_document(&self, req: CloseDocumentRequest) -> Result<DocumentResponse>;

    /// Get document info
    async fn get_document_info(&self, req: GetDocumentInfoRequest) -> Result<DocumentResponse>;

    /// List all open documents
    async fn list_documents(&self) -> Result<DocumentList>;

    // Layer operations
    async fn get_layers(&self, req: GetLayersRequest) -> Result<LayerList>;
    async fn create_layer(&self, req: CreateLayerRequest) -> Result<LayerResponse>;
    async fn delete_layer(&self, req: DeleteLayerRequest) -> Result<LayerResponse>;
    async fn duplicate_layer(&self, req: DuplicateLayerRequest) -> Result<LayerResponse>;
    async fn move_layer(&self, req: MoveLayerRequest) -> Result<LayerResponse>;
    async fn set_layer_visibility(&self, req: SetLayerVisibilityRequest) -> Result<LayerResponse>;
    async fn set_layer_opacity(&self, req: SetLayerOpacityRequest) -> Result<LayerResponse>;
    async fn set_layer_blend_mode(&self, req: SetLayerBlendModeRequest) -> Result<LayerResponse>;
    async fn get_layer_bounds(&self, req: GetLayerBoundsRequest) -> Result<LayerBoundsResponse>;
    async fn apply_layer_style(&self, req: ApplyLayerStyleRequest) -> Result<LayerResponse>;

    // Adjustments
    async fn apply_adjustment(&self, req: ApplyAdjustmentRequest) -> Result<AdjustmentResponse>;

    // Filters
    async fn apply_filter(&self, req: ApplyFilterRequest) -> Result<FilterResponse>;

    // Actions
    async fn play_action(&self, req: PlayActionRequest) -> Result<ActionResponse>;
    async fn list_actions(&self, req: ListActionsRequest) -> Result<ActionList>;

    // Batch processing
    async fn batch_process(&self, req: BatchProcessRequest) -> Result<BatchProcessResponse>;

    // Export/Import
    async fn export_document(&self, req: ExportDocumentRequest) -> Result<ExportResponse>;
    async fn import_file(&self, req: ImportFileRequest) -> Result<ImportResponse>;
}

/// Photoshop adapter factory
pub struct PhotoshopAdapterFactory;

impl PhotoshopAdapterFactory {
    /// Create a Photoshop adapter based on configuration
    pub async fn create(config: &crate::config::AdaptersConfig) -> Result<Arc<dyn PhotoshopAdapter>> {
        if config.photoshop.uxp_enabled {
            let adapter = uxp::UxpAdapter::new(config).await?;
            Ok(Arc::new(adapter))
        } else if config.photoshop.cep_enabled {
            let adapter = cep::CepAdapter::new(config).await?;
            Ok(Arc::new(adapter))
        } else {
            // Return mock adapter for testing
            Ok(Arc::new(MockPhotoshopAdapter::new()))
        }
    }
}

/// Mock adapter for testing
#[derive(Debug)]
pub struct MockPhotoshopAdapter {
    documents: Arc<RwLock<std::collections::HashMap<String, DocumentInfo>>>,
    layers: Arc<RwLock<std::collections::HashMap<String, Vec<LayerInfo>>>>,
    counter: Arc<std::sync::atomic::AtomicU64>,
}

impl MockPhotoshopAdapter {
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(std::collections::HashMap::new())),
            layers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    fn next_id(&self) -> String {
        format!("doc_{}", self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst))
    }
}

#[async_trait::async_trait]
impl PhotoshopAdapter for MockPhotoshopAdapter {
    async fn create_document(&self, req: CreateDocumentRequest) -> Result<DocumentResponse> {
        let id = self.next_id();
        let doc = DocumentInfo {
            id: id.clone(),
            name: req.name.clone(),
            path: String::new(),
            width: req.width,
            height: req.height,
            resolution: req.resolution,
            color_mode: req.color_mode,
            layer_count: 1,
            is_modified: true,
            is_background_layer: true,
        };
        self.documents.write().await.insert(id.clone(), doc.clone());
        self.layers.write().await.insert(id, vec![LayerInfo {
            id: "background".to_string(),
            name: "Background".to_string(),
            kind: LayerKind::LayerKindNormal,
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::BlendModeNormal,
            index: 0,
            is_background: true,
            is_locked: true,
            bounds: Some(LayerBounds { x: 0, y: 0, width: req.width, height: req.height }),
        }]);
        Ok(DocumentResponse { success: true, document: Some(doc), error: String::new() })
    }

    async fn open_document(&self, req: OpenDocumentRequest) -> Result<DocumentResponse> {
        let id = self.next_id();
        let doc = DocumentInfo {
            id: id.clone(),
            name: std::path::Path::new(&req.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Unknown")
                .to_string(),
            path: req.path,
            width: 1920,
            height: 1080,
            resolution: 72.0,
            color_mode: ColorMode::ColorModeRgb,
            layer_count: 1,
            is_modified: false,
            is_background_layer: true,
        };
        self.documents.write().await.insert(id.clone(), doc.clone());
        self.layers.write().await.insert(id, vec![LayerInfo {
            id: "background".to_string(),
            name: "Background".to_string(),
            kind: LayerKind::LayerKindNormal,
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::BlendModeNormal,
            index: 0,
            is_background: true,
            is_locked: true,
            bounds: Some(LayerBounds { x: 0, y: 0, width: 1920, height: 1080 }),
        }]);
        Ok(DocumentResponse { success: true, document: Some(doc), error: String::new() })
    }

    async fn save_document(&self, req: SaveDocumentRequest) -> Result<DocumentResponse> {
        let mut docs = self.documents.write().await;
        if let Some(doc) = docs.get_mut(&req.document_id) {
            doc.path = req.path;
            doc.is_modified = false;
            Ok(DocumentResponse { success: true, document: Some(doc.clone()), error: String::new() })
        } else {
            Ok(DocumentResponse { success: false, document: None, error: "Document not found".to_string() })
        }
    }

    async fn close_document(&self, req: CloseDocumentRequest) -> Result<DocumentResponse> {
        let mut docs = self.documents.write().await;
        let mut layers = self.layers.write().await;
        if docs.remove(&req.document_id).is_some() {
            layers.remove(&req.document_id);
            Ok(DocumentResponse { success: true, document: None, error: String::new() })
        } else {
            Ok(DocumentResponse { success: false, document: None, error: "Document not found".to_string() })
        }
    }

    async fn get_document_info(&self, req: GetDocumentInfoRequest) -> Result<DocumentResponse> {
        let docs = self.documents.read().await;
        if let Some(doc) = docs.get(&req.document_id) {
            Ok(DocumentResponse { success: true, document: Some(doc.clone()), error: String::new() })
        } else {
            Ok(DocumentResponse { success: false, document: None, error: "Document not found".to_string() })
        }
    }

    async fn list_documents(&self) -> Result<DocumentList> {
        let docs = self.documents.read().await;
        Ok(DocumentList { documents: docs.values().cloned().collect() })
    }

    async fn get_layers(&self, req: GetLayersRequest) -> Result<LayerList> {
        let layers = self.layers.read().await;
        if let Some(layers_vec) = layers.get(&req.document_id) {
            Ok(LayerList { layers: layers_vec.clone() })
        } else {
            Ok(LayerList { layers: vec![] })
        }
    }

    async fn create_layer(&self, req: CreateLayerRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            let layer_id = format!("layer_{}", layers_vec.len());
            let layer = LayerInfo {
                id: layer_id.clone(),
                name: req.name,
                kind: req.kind,
                visible: true,
                opacity: 1.0,
                blend_mode: BlendMode::BlendModeNormal,
                index: if req.insert_at >= 0 { req.insert_at as usize } else { layers_vec.len() },
                is_background: false,
                is_locked: false,
                bounds: Some(LayerBounds { x: 0, y: 0, width: 1920, height: 1080 }),
            };
            if req.insert_at >= 0 && (req.insert_at as usize) < layers_vec.len() {
                layers_vec.insert(req.insert_at as usize, layer.clone());
            } else {
                layers_vec.push(layer.clone());
            }
            // Update indices
            for (i, l) in layers_vec.iter_mut().enumerate() {
                l.index = i as i32;
            }
            Ok(LayerResponse { success: true, layer: Some(layer), error: String::new() })
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn delete_layer(&self, req: DeleteLayerRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(pos) = layers_vec.iter().position(|l| l.id == req.layer_id) {
                let layer = layers_vec.remove(pos);
                for (i, l) in layers_vec.iter_mut().enumerate() {
                    l.index = i as i32;
                }
                Ok(LayerResponse { success: true, layer: Some(layer), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn duplicate_layer(&self, req: DuplicateLayerRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(pos) = layers_vec.iter().position(|l| l.id == req.layer_id) {
                let mut layer = layers_vec[pos].clone();
                layer.id = format!("layer_{}", layers_vec.len());
                layer.name = req.new_name;
                layer.index = if req.insert_at >= 0 { req.insert_at as usize } else { layers_vec.len() };
                if req.insert_at >= 0 && (req.insert_at as usize) <= layers_vec.len() {
                    layers_vec.insert(req.insert_at as usize, layer.clone());
                } else {
                    layers_vec.push(layer.clone());
                }
                for (i, l) in layers_vec.iter_mut().enumerate() {
                    l.index = i as i32;
                }
                Ok(LayerResponse { success: true, layer: Some(layer), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn move_layer(&self, req: MoveLayerRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(pos) = layers_vec.iter().position(|l| l.id == req.layer_id) {
                let layer = layers_vec.remove(pos);
                let new_idx = req.new_index.max(0).min(layers_vec.len() as i32) as usize;
                layers_vec.insert(new_idx, layer.clone());
                for (i, l) in layers_vec.iter_mut().enumerate() {
                    l.index = i as i32;
                }
                Ok(LayerResponse { success: true, layer: Some(layer), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn set_layer_visibility(&self, req: SetLayerVisibilityRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(layer) = layers_vec.iter_mut().find(|l| l.id == req.layer_id) {
                layer.visible = req.visible;
                Ok(LayerResponse { success: true, layer: Some(layer.clone()), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn set_layer_opacity(&self, req: SetLayerOpacityRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(layer) = layers_vec.iter_mut().find(|l| l.id == req.layer_id) {
                layer.opacity = req.opacity.clamp(0.0, 1.0);
                Ok(LayerResponse { success: true, layer: Some(layer.clone()), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn set_layer_blend_mode(&self, req: SetLayerBlendModeRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(layer) = layers_vec.iter_mut().find(|l| l.id == req.layer_id) {
                layer.blend_mode = req.mode;
                Ok(LayerResponse { success: true, layer: Some(layer.clone()), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn get_layer_bounds(&self, req: GetLayerBoundsRequest) -> Result<LayerBoundsResponse> {
        let layers = self.layers.read().await;
        if let Some(layers_vec) = layers.get(&req.document_id) {
            if let Some(layer) = layers_vec.iter().find(|l| l.id == req.layer_id) {
                Ok(LayerBoundsResponse { success: true, bounds: layer.bounds.clone(), error: String::new() })
            } else {
                Ok(LayerBoundsResponse { success: false, bounds: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerBoundsResponse { success: false, bounds: None, error: "Document not found".to_string() })
        }
    }

    async fn apply_layer_style(&self, req: ApplyLayerStyleRequest) -> Result<LayerResponse> {
        let mut layers = self.layers.write().await;
        if let Some(layers_vec) = layers.get_mut(&req.document_id) {
            if let Some(layer) = layers_vec.iter_mut().find(|l| l.id == req.layer_id) {
                // In a real implementation, we'd apply the style
                Ok(LayerResponse { success: true, layer: Some(layer.clone()), error: String::new() })
            } else {
                Ok(LayerResponse { success: false, layer: None, error: "Layer not found".to_string() })
            }
        } else {
            Ok(LayerResponse { success: false, layer: None, error: "Document not found".to_string() })
        }
    }

    async fn apply_adjustment(&self, req: ApplyAdjustmentRequest) -> Result<AdjustmentResponse> {
        Ok(AdjustmentResponse {
            success: true,
            adjustment_id: format!("adj_{}", uuid::Uuid::new_v4()),
            error: String::new(),
        })
    }

    async fn apply_filter(&self, req: ApplyFilterRequest) -> Result<FilterResponse> {
        Ok(FilterResponse {
            success: true,
            filter_id: format!("flt_{}", uuid::Uuid::new_v4()),
            error: String::new(),
        })
    }

    async fn play_action(&self, req: PlayActionRequest) -> Result<ActionResponse> {
        Ok(ActionResponse { success: true, error: String::new() })
    }

    async fn list_actions(&self, req: ListActionsRequest) -> Result<ActionList> {
        Ok(ActionList { actions: vec![] })
    }

    async fn batch_process(&self, req: BatchProcessRequest) -> Result<BatchProcessResponse> {
        Ok(BatchProcessResponse {
            success: true,
            processed: req.source_files.len() as i32,
            failed: 0,
            errors: vec![],
        })
    }

    async fn export_document(&self, req: ExportDocumentRequest) -> Result<ExportResponse> {
        Ok(ExportResponse {
            success: true,
            path: req.path,
            error: String::new(),
        })
    }

    async fn import_file(&self, req: ImportFileRequest) -> Result<ImportResponse> {
        Ok(ImportResponse {
            success: true,
            layer_id: format!("imported_{}", uuid::Uuid::new_v4()),
            error: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_adapter_create_document() {
        let adapter = MockPhotoshopAdapter::new();
        let req = CreateDocumentRequest {
            width: 800,
            height: 600,
            resolution: 72.0,
            color_mode: ColorMode::ColorModeRgb as i32,
            background: BackgroundContents::BackgroundContentsWhite as i32,
            name: "Test Doc".to_string(),
        };
        let resp = adapter.create_document(req).await.unwrap();
        assert!(resp.success);
        assert!(resp.document.is_some());
    }

    #[tokio::test]
    async fn test_mock_adapter_layer_operations() {
        let adapter = MockPhotoshopAdapter::new();
        let req = CreateDocumentRequest {
            width: 800,
            height: 600,
            resolution: 72.0,
            color_mode: ColorMode::ColorModeRgb as i32,
            background: BackgroundContents::BackgroundContentsWhite as i32,
            name: "Test Doc".to_string(),
        };
        let resp = adapter.create_document(req).await.unwrap();
        let doc_id = resp.document.unwrap().id;

        // Create layer
        let create_req = CreateLayerRequest {
            document_id: doc_id.clone(),
            name: "Test Layer".to_string(),
            kind: LayerKind::LayerKindNormal as i32,
            insert_at: -1,
        };
        let layer_resp = adapter.create_layer(create_req).await.unwrap();
        assert!(layer_resp.success);
        let layer_id = layer_resp.layer.unwrap().id;

        // Set visibility
        let vis_req = SetLayerVisibilityRequest {
            document_id: doc_id.clone(),
            layer_id: layer_id.clone(),
            visible: false,
        };
        let vis_resp = adapter.set_layer_visibility(vis_req).await.unwrap();
        assert!(vis_resp.success);
        assert!(!vis_resp.layer.unwrap().visible);

        // Set opacity
        let op_req = SetLayerOpacityRequest {
            document_id: doc_id.clone(),
            layer_id: layer_id.clone(),
            opacity: 0.5,
        };
        let op_resp = adapter.set_layer_opacity(op_req).await.unwrap();
        assert!(op_resp.success);
        assert_eq!(op_resp.layer.unwrap().opacity, 0.5);
    }
}