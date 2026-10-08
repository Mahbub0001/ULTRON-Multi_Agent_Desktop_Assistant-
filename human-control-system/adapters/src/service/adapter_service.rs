//! Adapter gRPC Service Implementation

use crate::adapters_proto::{
    photoshop_service_server::{PhotoshopService, PhotoshopServiceServer},
    chrome_service_server::{ChromeService, ChromeServiceServer},
    game_service_server::{GameService, GameServiceServer},
    window_service_server::{WindowService, WindowServiceServer},
    *,
};
use crate::photoshop::PhotoshopAdapterFactory;
use crate::chrome::ChromeAdapterFactory;
use crate::game::GameAdapterFactory;
use crate::window::WindowAdapterFactory;
use crate::config::AdaptersConfig;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tonic::{Request, Response, Status};
use tracing::{debug, info, warn, error};
use std::time::Instant;

/// Adapter service state
pub struct AdapterServiceState {
    pub photoshop: Arc<dyn crate::photoshop::PhotoshopAdapter>,
    pub chrome: Arc<dyn crate::chrome::ChromeAdapter>,
    pub game: Arc<dyn crate::game::GameAdapter>,
    pub window: Arc<dyn crate::window::WindowAdapter>,
    pub config: AdaptersConfig,
    pub start_time: Instant,
}

/// Main adapter gRPC service
pub struct AdapterServiceImpl {
    state: Arc<AdapterServiceState>,
}

impl AdapterServiceImpl {
    pub async fn new(config: AdaptersConfig) -> Result<Self> {
        info!("Initializing adapter services");

        let photoshop = if config.photoshop.enabled {
            PhotoshopAdapterFactory::create(&config).await?
        } else {
            Arc::new(crate::photoshop::MockPhotoshopAdapter::new())
        };

        let chrome = if config.chrome.enabled {
            ChromeAdapterFactory::create(&config).await?
        } else {
            Arc::new(crate::chrome::MockChromeAdapter::new())
        };

        let game = if config.game.enabled {
            GameAdapterFactory::create(&config).await?
        } else {
            Arc::new(crate::game::MockGameAdapter::new())
        };

        let window = if config.window.enabled {
            WindowAdapterFactory::create(&config).await?
        } else {
            Arc::new(crate::window::MockWindowAdapter::new())
        };

        let state = AdapterServiceState {
            photoshop,
            chrome,
            game,
            window,
            config,
            start_time: Instant::now(),
        };

        Ok(Self {
            state: Arc::new(state),
        })
    }

    pub fn state(&self) -> Arc<AdapterServiceState> {
        self.state.clone()
    }

    /// Build all four gRPC services sharing one state.
    pub fn into_servers(self) -> (
        PhotoshopServiceServer<Self>,
        ChromeServiceServer<Self>,
        GameServiceServer<Self>,
        WindowServiceServer<Self>,
    ) {
        let s = Arc::new(self);
        (
            PhotoshopServiceServer::from_arc(s.clone()),
            ChromeServiceServer::from_arc(s.clone()),
            GameServiceServer::from_arc(s.clone()),
            WindowServiceServer::from_arc(s),
        )
    }
}

#[tonic::async_trait]
impl PhotoshopService for AdapterServiceImpl {
    // ============================================
    // Photoshop Service
    // ============================================

    async fn create_document(&self, request: Request<CreateDocumentRequest>) -> Result<Response<DocumentResponse>, Status> {
        let resp = self.state.photoshop.create_document(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn open_document(&self, request: Request<OpenDocumentRequest>) -> Result<Response<DocumentResponse>, Status> {
        let resp = self.state.photoshop.open_document(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn save_document(&self, request: Request<SaveDocumentRequest>) -> Result<Response<DocumentResponse>, Status> {
        let resp = self.state.photoshop.save_document(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn close_document(&self, request: Request<CloseDocumentRequest>) -> Result<Response<DocumentResponse>, Status> {
        let resp = self.state.photoshop.close_document(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_document_info(&self, request: Request<GetDocumentInfoRequest>) -> Result<Response<DocumentResponse>, Status> {
        let resp = self.state.photoshop.get_document_info(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn list_documents(&self, _request: Request<Empty>) -> Result<Response<DocumentList>, Status> {
        let resp = self.state.photoshop.list_documents().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_layers(&self, request: Request<GetLayersRequest>) -> Result<Response<LayerList>, Status> {
        let resp = self.state.photoshop.get_layers(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn create_layer(&self, request: Request<CreateLayerRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.create_layer(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn delete_layer(&self, request: Request<DeleteLayerRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.delete_layer(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn duplicate_layer(&self, request: Request<DuplicateLayerRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.duplicate_layer(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn move_layer(&self, request: Request<MoveLayerRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.move_layer(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_layer_visibility(&self, request: Request<SetLayerVisibilityRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.set_layer_visibility(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_layer_opacity(&self, request: Request<SetLayerOpacityRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.set_layer_opacity(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_layer_blend_mode(&self, request: Request<SetLayerBlendModeRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.set_layer_blend_mode(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_layer_bounds(&self, request: Request<GetLayerBoundsRequest>) -> Result<Response<LayerBoundsResponse>, Status> {
        let resp = self.state.photoshop.get_layer_bounds(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn apply_layer_style(&self, request: Request<ApplyLayerStyleRequest>) -> Result<Response<LayerResponse>, Status> {
        let resp = self.state.photoshop.apply_layer_style(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn apply_adjustment(&self, request: Request<ApplyAdjustmentRequest>) -> Result<Response<AdjustmentResponse>, Status> {
        let resp = self.state.photoshop.apply_adjustment(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn apply_filter(&self, request: Request<ApplyFilterRequest>) -> Result<Response<FilterResponse>, Status> {
        let resp = self.state.photoshop.apply_filter(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn play_action(&self, request: Request<PlayActionRequest>) -> Result<Response<ActionResponse>, Status> {
        let resp = self.state.photoshop.play_action(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn list_actions(&self, request: Request<ListActionsRequest>) -> Result<Response<ActionList>, Status> {
        let resp = self.state.photoshop.list_actions(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn batch_process(&self, request: Request<BatchProcessRequest>) -> Result<Response<BatchProcessResponse>, Status> {
        let resp = self.state.photoshop.batch_process(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn export_document(&self, request: Request<ExportDocumentRequest>) -> Result<Response<ExportResponse>, Status> {
        let resp = self.state.photoshop.export_document(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn import_file(&self, request: Request<ImportFileRequest>) -> Result<Response<ImportResponse>, Status> {
        let resp = self.state.photoshop.import_file(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }


    async fn health_check(&self, _request: Request<Empty>) -> Result<Response<HealthCheckResponse>, Status> {
        let uptime = self.state.start_time.elapsed().as_secs();
        let mut components = std::collections::HashMap::new();
        components.insert("photoshop".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("chrome".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("game".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("window".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });

        Ok(Response::new(HealthCheckResponse {
            status: health_check_response::ServingStatus::ServingStatusServing as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            components,
        }))
    }

}

#[tonic::async_trait]
impl ChromeService for AdapterServiceImpl {
    // ============================================
    // Chrome Service
    // ============================================

    async fn connect(&self, request: Request<ConnectRequest>) -> Result<Response<ConnectResponse>, Status> {
        let resp = self.state.chrome.connect(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn disconnect(&self, request: Request<DisconnectRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.disconnect(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn get_version(&self, _request: Request<Empty>) -> Result<Response<VersionResponse>, Status> {
        let resp = self.state.chrome.get_version().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_targets(&self, request: Request<GetTargetsRequest>) -> Result<Response<TargetList>, Status> {
        let resp = self.state.chrome.get_targets(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn create_tab(&self, request: Request<CreateTabRequest>) -> Result<Response<TabResponse>, Status> {
        let resp = self.state.chrome.create_tab(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn close_tab(&self, request: Request<CloseTabRequest>) -> Result<Response<TabResponse>, Status> {
        let resp = self.state.chrome.close_tab(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn activate_tab(&self, request: Request<ActivateTabRequest>) -> Result<Response<TabResponse>, Status> {
        let resp = self.state.chrome.activate_tab(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn navigate(&self, request: Request<NavigateRequest>) -> Result<Response<NavigationResponse>, Status> {
        let resp = self.state.chrome.navigate(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn reload(&self, request: Request<ReloadRequest>) -> Result<Response<NavigationResponse>, Status> {
        let resp = self.state.chrome.reload(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn go_back(&self, request: Request<GoBackRequest>) -> Result<Response<NavigationResponse>, Status> {
        let resp = self.state.chrome.go_back(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn go_forward(&self, request: Request<GoForwardRequest>) -> Result<Response<NavigationResponse>, Status> {
        let resp = self.state.chrome.go_forward(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_tabs(&self, _request: Request<Empty>) -> Result<Response<TabList>, Status> {
        let resp = self.state.chrome.get_tabs().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn evaluate(&self, request: Request<EvaluateRequest>) -> Result<Response<EvaluateResponse>, Status> {
        let resp = self.state.chrome.evaluate(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn call_function(&self, request: Request<CallFunctionRequest>) -> Result<Response<EvaluateResponse>, Status> {
        let resp = self.state.chrome.call_function(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_document(&self, request: Request<GetDocumentRequest>) -> Result<Response<NodeResponse>, Status> {
        let resp = self.state.chrome.get_document(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn query_selector(&self, request: Request<QuerySelectorRequest>) -> Result<Response<NodeResponse>, Status> {
        let resp = self.state.chrome.query_selector(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn query_selector_all(&self, request: Request<QuerySelectorAllRequest>) -> Result<Response<NodeList>, Status> {
        let resp = self.state.chrome.query_selector_all(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn click_element(&self, request: Request<ClickElementRequest>) -> Result<Response<ClickResponse>, Status> {
        let resp = self.state.chrome.click_element(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn type_text(&self, request: Request<TypeTextRequest>) -> Result<Response<TypeResponse>, Status> {
        let resp = self.state.chrome.type_text(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_element_bounds(&self, request: Request<GetElementBoundsRequest>) -> Result<Response<ElementBoundsResponse>, Status> {
        let resp = self.state.chrome.get_element_bounds(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_element_attributes(&self, request: Request<GetElementAttributesRequest>) -> Result<Response<ElementAttributesResponse>, Status> {
        let resp = self.state.chrome.get_element_attributes(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_element_attributes(&self, request: Request<SetElementAttributesRequest>) -> Result<Response<ElementAttributesResponse>, Status> {
        let resp = self.state.chrome.set_element_attributes(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn screenshot_element(&self, request: Request<ScreenshotElementRequest>) -> Result<Response<ScreenshotResponse>, Status> {
        let resp = self.state.chrome.screenshot_element(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn scroll_into_view(&self, request: Request<ScrollIntoViewRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.scroll_into_view(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn enable_network(&self, request: Request<EnableNetworkRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.enable_network(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn disable_network(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.state.chrome.disable_network().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn set_request_interception(&self, request: Request<SetRequestInterceptionRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.set_request_interception(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn continue_request(&self, request: Request<ContinueRequestRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.continue_request(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn modify_request(&self, request: Request<ModifyRequestRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.modify_request(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn block_urls(&self, request: Request<BlockUrlsRequest>) -> Result<Response<Empty>, Status> {
        self.state.chrome.block_urls(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn get_network_logs(&self, _request: Request<Empty>) -> Result<Response<NetworkLogList>, Status> {
        let resp = self.state.chrome.get_network_logs().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn enable_console(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.state.chrome.enable_console().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn disable_console(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.state.chrome.disable_console().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn get_console_logs(&self, _request: Request<Empty>) -> Result<Response<ConsoleLogList>, Status> {
        let resp = self.state.chrome.get_console_logs().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn clear_console(&self, _request: Request<Empty>) -> Result<Response<Empty>, Status> {
        self.state.chrome.clear_console().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn capture_screenshot(&self, request: Request<CaptureScreenshotRequest>) -> Result<Response<ScreenshotResponse>, Status> {
        let resp = self.state.chrome.capture_screenshot(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn capture_full_page_screenshot(&self, request: Request<CaptureFullPageScreenshotRequest>) -> Result<Response<ScreenshotResponse>, Status> {
        let resp = self.state.chrome.capture_full_page_screenshot(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_cookies(&self, request: Request<GetCookiesRequest>) -> Result<Response<CookieList>, Status> {
        let resp = self.state.chrome.get_cookies(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_cookie(&self, request: Request<SetCookieRequest>) -> Result<Response<CookieResponse>, Status> {
        let resp = self.state.chrome.set_cookie(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn delete_cookie(&self, request: Request<DeleteCookieRequest>) -> Result<Response<CookieResponse>, Status> {
        let resp = self.state.chrome.delete_cookie(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_local_storage(&self, request: Request<GetLocalStorageRequest>) -> Result<Response<StorageResponse>, Status> {
        let resp = self.state.chrome.get_local_storage(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_local_storage(&self, request: Request<SetLocalStorageRequest>) -> Result<Response<StorageResponse>, Status> {
        let resp = self.state.chrome.set_local_storage(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }


    async fn health_check(&self, _request: Request<Empty>) -> Result<Response<HealthCheckResponse>, Status> {
        let uptime = self.state.start_time.elapsed().as_secs();
        let mut components = std::collections::HashMap::new();
        components.insert("photoshop".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("chrome".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("game".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("window".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });

        Ok(Response::new(HealthCheckResponse {
            status: health_check_response::ServingStatus::ServingStatusServing as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            components,
        }))
    }

}

#[tonic::async_trait]
impl GameService for AdapterServiceImpl {
    // ============================================
    // Game Service
    // ============================================

    async fn attach_process(&self, request: Request<AttachProcessRequest>) -> Result<Response<AttachResponse>, Status> {
        let resp = self.state.game.attach_process(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn detach_process(&self, request: Request<DetachProcessRequest>) -> Result<Response<Empty>, Status> {
        self.state.game.detach_process(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(Empty {}))
    }

    async fn get_process_info(&self, request: Request<GetProcessInfoRequest>) -> Result<Response<ProcessInfoResponse>, Status> {
        let resp = self.state.game.get_process_info(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn list_processes(&self, request: Request<ListProcessesRequest>) -> Result<Response<ProcessList>, Status> {
        let resp = self.state.game.list_processes(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn find_process_by_name(&self, request: Request<FindProcessByNameRequest>) -> Result<Response<ProcessInfoResponse>, Status> {
        let resp = self.state.game.find_process_by_name(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn find_process_by_window(&self, request: Request<FindProcessByWindowRequest>) -> Result<Response<ProcessInfoResponse>, Status> {
        let resp = self.state.game.find_process_by_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn read_memory(&self, _request: Request<ReadMemoryRequest>) -> Result<Response<ReadMemoryResponse>, Status> {
        Err(Status::unimplemented("read_memory is disabled: memory access and injection are not supported by this build"))
    }

    async fn read_pointer_chain(&self, _request: Request<ReadPointerChainRequest>) -> Result<Response<ReadMemoryResponse>, Status> {
        Err(Status::unimplemented("read_pointer_chain is disabled: memory access and injection are not supported by this build"))
    }

    async fn read_string(&self, _request: Request<ReadStringRequest>) -> Result<Response<ReadStringResponse>, Status> {
        Err(Status::unimplemented("read_string is disabled: memory access and injection are not supported by this build"))
    }

    async fn read_struct(&self, _request: Request<ReadStructRequest>) -> Result<Response<ReadStructResponse>, Status> {
        Err(Status::unimplemented("read_struct is disabled: memory access and injection are not supported by this build"))
    }

    async fn write_memory(&self, _request: Request<WriteMemoryRequest>) -> Result<Response<WriteMemoryResponse>, Status> {
        Err(Status::unimplemented("write_memory is disabled: memory access and injection are not supported by this build"))
    }

    async fn write_pointer_chain(&self, _request: Request<WritePointerChainRequest>) -> Result<Response<WriteMemoryResponse>, Status> {
        Err(Status::unimplemented("write_pointer_chain is disabled: memory access and injection are not supported by this build"))
    }

    async fn write_string(&self, _request: Request<WriteStringRequest>) -> Result<Response<WriteMemoryResponse>, Status> {
        Err(Status::unimplemented("write_string is disabled: memory access and injection are not supported by this build"))
    }

    async fn write_struct(&self, _request: Request<WriteStructRequest>) -> Result<Response<WriteMemoryResponse>, Status> {
        Err(Status::unimplemented("write_struct is disabled: memory access and injection are not supported by this build"))
    }

    async fn scan_pattern(&self, _request: Request<ScanPatternRequest>) -> Result<Response<ScanPatternResponse>, Status> {
        Err(Status::unimplemented("scan_pattern is disabled: memory access and injection are not supported by this build"))
    }

    async fn scan_pattern_module(&self, _request: Request<ScanPatternModuleRequest>) -> Result<Response<ScanPatternResponse>, Status> {
        Err(Status::unimplemented("scan_pattern_module is disabled: memory access and injection are not supported by this build"))
    }

    async fn find_pattern(&self, _request: Request<FindPatternRequest>) -> Result<Response<FindPatternResponse>, Status> {
        Err(Status::unimplemented("find_pattern is disabled: memory access and injection are not supported by this build"))
    }

    async fn allocate_memory(&self, _request: Request<AllocateMemoryRequest>) -> Result<Response<AllocateMemoryResponse>, Status> {
        Err(Status::unimplemented("allocate_memory is disabled: memory access and injection are not supported by this build"))
    }

    async fn free_memory(&self, _request: Request<FreeMemoryRequest>) -> Result<Response<Empty>, Status> {
        Err(Status::unimplemented("free_memory is disabled: memory access and injection are not supported by this build"))
    }

    async fn inject_code(&self, _request: Request<InjectCodeRequest>) -> Result<Response<InjectCodeResponse>, Status> {
        Err(Status::unimplemented("inject_code is disabled: memory access and injection are not supported by this build"))
    }

    async fn create_thread(&self, _request: Request<CreateThreadRequest>) -> Result<Response<CreateThreadResponse>, Status> {
        Err(Status::unimplemented("create_thread is disabled: memory access and injection are not supported by this build"))
    }

    async fn inject_dll(&self, _request: Request<InjectDllRequest>) -> Result<Response<InjectDllResponse>, Status> {
        Err(Status::unimplemented("inject_dll is disabled: memory access and injection are not supported by this build"))
    }

    async fn set_hook(&self, _request: Request<SetHookRequest>) -> Result<Response<HookResponse>, Status> {
        Err(Status::unimplemented("set_hook is disabled: memory access and injection are not supported by this build"))
    }

    async fn remove_hook(&self, _request: Request<RemoveHookRequest>) -> Result<Response<HookResponse>, Status> {
        Err(Status::unimplemented("remove_hook is disabled: memory access and injection are not supported by this build"))
    }

    async fn get_modules(&self, _request: Request<Empty>) -> Result<Response<ModuleList>, Status> {
        let resp = self.state.game.get_modules().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_module_base(&self, request: Request<GetModuleBaseRequest>) -> Result<Response<ModuleBaseResponse>, Status> {
        let resp = self.state.game.get_module_base(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_module_export(&self, _request: Request<GetModuleExportRequest>) -> Result<Response<ModuleExportResponse>, Status> {
        Err(Status::unimplemented("get_module_export is disabled: memory access and injection are not supported by this build"))
    }


    async fn health_check(&self, _request: Request<Empty>) -> Result<Response<HealthCheckResponse>, Status> {
        let uptime = self.state.start_time.elapsed().as_secs();
        let mut components = std::collections::HashMap::new();
        components.insert("photoshop".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("chrome".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("game".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("window".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });

        Ok(Response::new(HealthCheckResponse {
            status: health_check_response::ServingStatus::ServingStatusServing as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            components,
        }))
    }

}

#[tonic::async_trait]
impl WindowService for AdapterServiceImpl {
    // ============================================
    // Window Service
    // ============================================

    async fn list_windows(&self, request: Request<ListWindowsRequest>) -> Result<Response<WindowList>, Status> {
        let resp = self.state.window.list_windows(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window(&self, request: Request<GetWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.get_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn find_window(&self, request: Request<FindWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.find_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_foreground_window(&self, _request: Request<Empty>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.get_foreground_window().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_desktop_window(&self, _request: Request<Empty>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.get_desktop_window().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_shell_window(&self, _request: Request<Empty>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.get_shell_window().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn focus_window(&self, request: Request<FocusWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.focus_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn activate_window(&self, request: Request<ActivateWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.activate_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn show_window(&self, request: Request<ShowWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.show_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn hide_window(&self, request: Request<HideWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.hide_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn minimize_window(&self, request: Request<MinimizeWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.minimize_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn maximize_window(&self, request: Request<MaximizeWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.maximize_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn restore_window(&self, request: Request<RestoreWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.restore_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn close_window(&self, request: Request<CloseWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.close_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn is_window_visible(&self, request: Request<IsWindowVisibleRequest>) -> Result<Response<WindowVisibilityResponse>, Status> {
        let resp = self.state.window.is_window_visible(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn is_window_enabled(&self, request: Request<IsWindowEnabledRequest>) -> Result<Response<WindowEnabledResponse>, Status> {
        let resp = self.state.window.is_window_enabled(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_state(&self, request: Request<GetWindowStateRequest>) -> Result<Response<WindowStateResponse>, Status> {
        let resp = self.state.window.get_window_state(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn move_window(&self, request: Request<MoveWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.move_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn resize_window(&self, request: Request<ResizeWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.resize_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn move_resize_window(&self, request: Request<MoveResizeWindowRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.move_resize_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_rect(&self, request: Request<GetWindowRectRequest>) -> Result<Response<WindowRectResponse>, Status> {
        let resp = self.state.window.get_window_rect(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_client_rect(&self, request: Request<GetClientRectRequest>) -> Result<Response<WindowRectResponse>, Status> {
        let resp = self.state.window.get_client_rect(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_window_pos(&self, request: Request<SetWindowPosRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.set_window_pos(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn capture_window(&self, request: Request<CaptureWindowRequest>) -> Result<Response<ScreenshotResponse>, Status> {
        let resp = self.state.window.capture_window(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn capture_client_area(&self, request: Request<CaptureClientAreaRequest>) -> Result<Response<ScreenshotResponse>, Status> {
        let resp = self.state.window.capture_client_area(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn capture_region(&self, request: Request<CaptureRegionRequest>) -> Result<Response<ScreenshotResponse>, Status> {
        let resp = self.state.window.capture_region(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_title(&self, request: Request<GetWindowTitleRequest>) -> Result<Response<WindowTitleResponse>, Status> {
        let resp = self.state.window.get_window_title(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_window_title(&self, request: Request<SetWindowTitleRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.set_window_title(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_class(&self, request: Request<GetWindowClassRequest>) -> Result<Response<WindowClassResponse>, Status> {
        let resp = self.state.window.get_window_class(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_process_id(&self, request: Request<GetWindowProcessIdRequest>) -> Result<Response<WindowProcessIdResponse>, Status> {
        let resp = self.state.window.get_window_process_id(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_styles(&self, request: Request<GetWindowStylesRequest>) -> Result<Response<WindowStylesResponse>, Status> {
        let resp = self.state.window.get_window_styles(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_window_styles(&self, request: Request<SetWindowStylesRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.set_window_styles(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_ex_styles(&self, request: Request<GetWindowExStylesRequest>) -> Result<Response<WindowStylesResponse>, Status> {
        let resp = self.state.window.get_window_ex_styles(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_window_ex_styles(&self, request: Request<SetWindowExStylesRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.set_window_ex_styles(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn bring_to_top(&self, request: Request<BringToTopRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.bring_to_top(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn send_to_bottom(&self, request: Request<SendToBottomRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.send_to_bottom(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_window_z_order(&self, request: Request<SetWindowZOrderRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.set_window_z_order(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn set_focus(&self, request: Request<SetFocusRequest>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.set_focus(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_focus(&self, _request: Request<Empty>) -> Result<Response<WindowResponse>, Status> {
        let resp = self.state.window.get_focus().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn post_message(&self, request: Request<PostMessageRequest>) -> Result<Response<MessageResponse>, Status> {
        let resp = self.state.window.post_message(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn send_message(&self, request: Request<SendMessageRequest>) -> Result<Response<MessageResponse>, Status> {
        let resp = self.state.window.send_message(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_monitors(&self, _request: Request<Empty>) -> Result<Response<MonitorList>, Status> {
        let resp = self.state.window.get_monitors().await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }

    async fn get_window_monitor(&self, request: Request<GetWindowMonitorRequest>) -> Result<Response<MonitorResponse>, Status> {
        let resp = self.state.window.get_window_monitor(request.into_inner()).await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(resp))
    }


    async fn health_check(&self, _request: Request<Empty>) -> Result<Response<HealthCheckResponse>, Status> {
        let uptime = self.state.start_time.elapsed().as_secs();
        let mut components = std::collections::HashMap::new();
        components.insert("photoshop".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("chrome".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("game".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });
        components.insert("window".to_string(), ComponentHealth {
            status: component_health::ComponentStatus::ComponentStatusHealthy as i32,
            message: "OK".to_string(),
            details: std::collections::HashMap::new(),
        });

        Ok(Response::new(HealthCheckResponse {
            status: health_check_response::ServingStatus::ServingStatusServing as i32,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime,
            components,
        }))
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AdaptersConfig;

    #[tokio::test]
    async fn test_service_creation() {
        let config = AdaptersConfig::default();
        let service = AdapterServiceImpl::new(config).await;
        assert!(service.is_ok());
    }
}