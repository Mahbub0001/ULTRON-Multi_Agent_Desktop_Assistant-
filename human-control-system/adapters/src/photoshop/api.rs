//! Photoshop API - High-level convenience functions
//!
//! Provides simplified APIs for common Photoshop operations.

use crate::photoshop::{PhotoshopAdapter, MockPhotoshopAdapter};
use crate::adapters_proto::*;
use anyhow::Result;
use std::sync::Arc;

/// High-level Photoshop API
#[derive(Clone)]
pub struct PhotoshopApi {
    adapter: Arc<dyn PhotoshopAdapter>,
}

impl PhotoshopApi {
    /// Create a new Photoshop API wrapper
    pub fn new(adapter: Arc<dyn PhotoshopAdapter>) -> Self {
        Self { adapter }
    }

    /// Create a new document with sensible defaults
    pub async fn new_document(&self, width: u32, height: u32, name: &str) -> Result<DocumentInfo> {
        let req = CreateDocumentRequest {
            width: width as i32,
            height: height as i32,
            resolution: 72.0,
            color_mode: ColorMode::ColorModeRgb as i32,
            background: BackgroundContents::BackgroundContentsWhite as i32,
            name: name.to_string(),
        };
        let resp = self.adapter.create_document(req).await?;
        resp.document.ok_or_else(|| anyhow::anyhow!("Failed to create document: {}", resp.error))
    }

    /// Open an existing document
    pub async fn open(&self, path: &str) -> Result<DocumentInfo> {
        let req = OpenDocumentRequest { path: path.to_string() };
        let resp = self.adapter.open_document(req).await?;
        resp.document.ok_or_else(|| anyhow::anyhow!("Failed to open document: {}", resp.error))
    }

    /// Save the current document
    pub async fn save(&self, document_id: &str, path: Option<&str>) -> Result<()> {
        let req = SaveDocumentRequest {
            document_id: document_id.to_string(),
            path: path.map(|s| s.to_string()).unwrap_or_default(),
            options: None,
        };
        let resp = self.adapter.save_document(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to save: {}", resp.error));
        }
        Ok(())
    }

    /// Close a document
    pub async fn close(&self, document_id: &str, force: bool) -> Result<()> {
        let req = CloseDocumentRequest {
            document_id: document_id.to_string(),
            force,
        };
        let resp = self.adapter.close_document(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to close: {}", resp.error));
        }
        Ok(())
    }

    /// Get all layers in a document
    pub async fn get_layers(&self, document_id: &str) -> Result<Vec<LayerInfo>> {
        let req = GetLayersRequest { document_id: document_id.to_string() };
        let resp = self.adapter.get_layers(req).await?;
        Ok(resp.layers)
    }

    /// Create a new layer
    pub async fn create_layer(&self, document_id: &str, name: &str, layer_type: LayerKind) -> Result<LayerInfo> {
        let req = CreateLayerRequest {
            document_id: document_id.to_string(),
            name: name.to_string(),
            kind: layer_type as i32,
            insert_at: -1,
        };
        let resp = self.adapter.create_layer(req).await?;
        resp.layer.ok_or_else(|| anyhow::anyhow!("Failed to create layer: {}", resp.error))
    }

    /// Delete a layer
    pub async fn delete_layer(&self, document_id: &str, layer_id: &str) -> Result<()> {
        let req = DeleteLayerRequest {
            document_id: document_id.to_string(),
            layer_id: layer_id.to_string(),
        };
        let resp = self.adapter.delete_layer(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to delete layer: {}", resp.error));
        }
        Ok(())
    }

    /// Set layer visibility
    pub async fn set_layer_visible(&self, document_id: &str, layer_id: &str, visible: bool) -> Result<()> {
        let req = SetLayerVisibilityRequest {
            document_id: document_id.to_string(),
            layer_id: layer_id.to_string(),
            visible,
        };
        let resp = self.adapter.set_layer_visibility(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to set visibility: {}", resp.error));
        }
        Ok(())
    }

    /// Set layer opacity (0.0 - 1.0)
    pub async fn set_layer_opacity(&self, document_id: &str, layer_id: &str, opacity: f32) -> Result<()> {
        let req = SetLayerOpacityRequest {
            document_id: document_id.to_string(),
            layer_id: layer_id.to_string(),
            opacity: opacity.clamp(0.0, 1.0),
        };
        let resp = self.adapter.set_layer_opacity(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to set opacity: {}", resp.error));
        }
        Ok(())
    }

    /// Set layer blend mode
    pub async fn set_layer_blend_mode(&self, document_id: &str, layer_id: &str, mode: BlendMode) -> Result<()> {
        let req = SetLayerBlendModeRequest {
            document_id: document_id.to_string(),
            layer_id: layer_id.to_string(),
            mode: mode as i32,
        };
        let resp = self.adapter.set_layer_blend_mode(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to set blend mode: {}", resp.error));
        }
        Ok(())
    }

    /// Apply a brightness/contrast adjustment
    pub async fn adjust_brightness_contrast(&self, document_id: &str, layer_id: Option<&str>, brightness: i32, contrast: i32) -> Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("brightness".to_string(), brightness.to_string());
        params.insert("contrast".to_string(), contrast.to_string());

        let req = ApplyAdjustmentRequest {
            document_id: document_id.to_string(),
            layer_id: layer_id.map(|s| s.to_string()).unwrap_or_default(),
            type_: AdjustmentType::AdjustmentTypeBrightnessContrast as i32,
            parameters: params,
        };
        let resp = self.adapter.apply_adjustment(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to apply adjustment: {}", resp.error));
        }
        Ok(())
    }

    /// Apply Gaussian blur filter
    pub async fn gaussian_blur(&self, document_id: &str, layer_id: &str, radius: f32) -> Result<()> {
        let mut params = std::collections::HashMap::new();
        params.insert("radius".to_string(), radius.to_string());

        let req = ApplyFilterRequest {
            document_id: document_id.to_string(),
            layer_id: layer_id.to_string(),
            type_: FilterType::FilterTypeGaussianBlur as i32,
            parameters: params,
            smart_filter: true,
        };
        let resp = self.adapter.apply_filter(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to apply filter: {}", resp.error));
        }
        Ok(())
    }

    /// Play an action
    pub async fn play_action(&self, action_name: &str, action_set: &str, document_id: Option<&str>) -> Result<()> {
        let req = PlayActionRequest {
            action_name: action_name.to_string(),
            action_set: action_set.to_string(),
            document_id: document_id.map(|s| s.to_string()).unwrap_or_default(),
            parameters: std::collections::HashMap::new(),
        };
        let resp = self.adapter.play_action(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to play action: {}", resp.error));
        }
        Ok(())
    }

    /// Export document to file
    pub async fn export(&self, document_id: &str, path: &str, format: ExportFormat, quality: u8) -> Result<()> {
        let req = ExportDocumentRequest {
            document_id: document_id.to_string(),
            path: path.to_string(),
            format: format as i32,
            options: Some(ExportOptions {
                quality: quality as i32,
                include_icc_profile: true,
                convert_to_srgb: true,
                transparency: true,
                png_compression: 6,
            }),
        };
        let resp = self.adapter.export_document(req).await?;
        if !resp.success {
            return Err(anyhow::anyhow!("Failed to export: {}", resp.error));
        }
        Ok(())
    }

    /// Batch process files with an action
    pub async fn batch(&self, action_name: &str, action_set: &str, files: Vec<String>, output_dir: &str) -> Result<BatchProcessResponse> {
        let req = BatchProcessRequest {
            action_name: action_name.to_string(),
            action_set: action_set.to_string(),
            source_files: files,
            destination_folder: output_dir.to_string(),
            options: Some(BatchOptions {
                override_open: true,
                include_subfolders: false,
                suppress_warnings: true,
                file_naming: "document_name".to_string(),
                save_options: None,
            }),
        };
        self.adapter.batch_process(req).await
    }
}

/// Builder for creating Photoshop API with mock adapter (for testing)
pub struct PhotoshopApiBuilder {
    use_mock: bool,
}

impl PhotoshopApiBuilder {
    pub fn new() -> Self {
        Self { use_mock: true }
    }

    pub fn with_mock(mut self, use_mock: bool) -> Self {
        self.use_mock = use_mock;
        self
    }

    pub fn build(self) -> PhotoshopApi {
        if self.use_mock {
            PhotoshopApi::new(Arc::new(MockPhotoshopAdapter::new()))
        } else {
            // In real implementation, create actual adapter
            PhotoshopApi::new(Arc::new(MockPhotoshopAdapter::new()))
        }
    }
}

impl Default for PhotoshopApiBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_photoshop_api_basic() {
        let api = PhotoshopApiBuilder::new().build();

        // Create document
        let doc = api.new_document(800, 600, "Test").await.unwrap();
        assert_eq!(doc.width, 800);
        assert_eq!(doc.height, 600);

        // Create layer
        let layer = api.create_layer(&doc.id, "Test Layer", LayerKind::LayerKindNormal).await.unwrap();
        assert_eq!(layer.name, "Test Layer");

        // Set opacity
        api.set_layer_opacity(&doc.id, &layer.id, 0.5).await.unwrap();

        // Set visibility
        api.set_layer_visible(&doc.id, &layer.id, false).await.unwrap();

        // Delete layer
        api.delete_layer(&doc.id, &layer.id).await.unwrap();
    }
}