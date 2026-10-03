use crate::dsl::types::*;
use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tonic::transport::{Channel, ClientTlsConfig, Endpoint};
use tonic::metadata::MetadataMap;
use tracing::{debug, error, info, warn};

// Generated proto modules
pub mod orchestrator_proto {
    tonic::include_proto!("hcs.orchestrator.v1");
}

pub mod hcs_agent_proto {
    tonic::include_proto!("hcs.agent.v1");
}

pub mod hcs_vision_proto {
    tonic::include_proto!("hcs.vision.v1");
}

pub mod brain_proto {
    tonic::include_proto!("brain");
}

pub mod adapters_proto {
    tonic::include_proto!("hcs.adapters.v1");
}

use orchestrator_proto::orchestrator_service_client::OrchestratorServiceClient;
use hcs_agent_proto::{
    input_service_client::InputServiceClient,
    macro_service_client::MacroServiceClient,
    task_service_client::TaskServiceClient,
    auth_service_client::AuthServiceClient,
    system_service_client::SystemServiceClient,
    KeyEvent, MouseEvent, InputBatch, MacroRequest, MacroDefinition,
    TaskRequest, TaskId, TaskFilter, TokenRequest, Capability, Action,
    HealthCheckRequest, google::protobuf::Empty,
};
use hcs_vision_proto::{
    vision_service_client::VisionServiceClient,
    CaptureRequest, DetectionRequest, OcrRequest, Region, ImageFormat,
};
use brain_proto::{
    brain_service_client::BrainServiceClient,
    ExecuteTreeRequest, TickTreeRequest, LoadTreeRequest, UnloadTreeRequest,
    ListTreesRequest, GetTreeVisualizationRequest, HealthCheckRequest as BrainHealthCheck,
    VisualizationFormat,
};
use adapters_proto::{
    photoshop_service_client::PhotoshopServiceClient,
    chrome_service_client::ChromeServiceClient,
    game_service_client::GameServiceClient,
    window_service_client::WindowServiceClient,
    HealthCheckRequest as AdapterHealthCheck,
    google::protobuf::Empty as AdapterEmpty,
};

pub struct GrpcClient {
    agent_endpoint: String,
    vision_endpoint: String,
    brain_endpoint: String,
    adapters_endpoint: String,
    auth_token: Option<String>,
    
    agent_channel: Option<Channel>,
    vision_channel: Option<Channel>,
    brain_channel: Option<Channel>,
    adapters_channel: Option<Channel>,
    
    input_client: Option<InputServiceClient<Channel>>,
    macro_client: Option<MacroServiceClient<Channel>>,
    task_client: Option<TaskServiceClient<Channel>>,
    auth_client: Option<AuthServiceClient<Channel>>,
    system_client: Option<SystemServiceClient<Channel>>,
    
    vision_client: Option<VisionServiceClient<Channel>>,
    brain_client: Option<BrainServiceClient<Channel>>,
    photoshop_client: Option<PhotoshopServiceClient<Channel>>,
    chrome_client: Option<ChromeServiceClient<Channel>>,
    game_client: Option<GameServiceClient<Channel>>,
    window_client: Option<WindowServiceClient<Channel>>,
}

impl GrpcClient {
    pub async fn new(
        agent_endpoint: String,
        vision_endpoint: String,
        brain_endpoint: String,
        adapters_endpoint: String,
        auth_token: Option<String>,
    ) -> Result<Self> {
        let mut client = Self {
            agent_endpoint,
            vision_endpoint,
            brain_endpoint,
            adapters_endpoint,
            auth_token,
            agent_channel: None,
            vision_channel: None,
            brain_channel: None,
            adapters_channel: None,
            input_client: None,
            macro_client: None,
            task_client: None,
            auth_client: None,
            system_client: None,
            vision_client: None,
            brain_client: None,
            photoshop_client: None,
            chrome_client: None,
            game_client: None,
            window_client: None,
        };
        
        client.connect_all().await?;
        Ok(client)
    }

    async fn connect_all(&mut self) -> Result<()> {
        // Connect to agent
        self.agent_channel = Some(self.connect_endpoint(&self.agent_endpoint).await?);
        let agent_channel = self.agent_channel.as_ref().unwrap().clone();
        self.input_client = Some(InputServiceClient::new(agent_channel.clone()));
        self.macro_client = Some(MacroServiceClient::new(agent_channel.clone()));
        self.task_client = Some(TaskServiceClient::new(agent_channel.clone()));
        self.auth_client = Some(AuthServiceClient::new(agent_channel.clone()));
        self.system_client = Some(SystemServiceClient::new(agent_channel.clone()));

        // Connect to vision
        self.vision_channel = Some(self.connect_endpoint(&self.vision_endpoint).await?);
        let vision_channel = self.vision_channel.as_ref().unwrap().clone();
        self.vision_client = Some(VisionServiceClient::new(vision_channel));

        // Connect to brain
        self.brain_channel = Some(self.connect_endpoint(&self.brain_endpoint).await?);
        let brain_channel = self.brain_channel.as_ref().unwrap().clone();
        self.brain_client = Some(BrainServiceClient::new(brain_channel));

        // Connect to adapters
        self.adapters_channel = Some(self.connect_endpoint(&self.adapters_endpoint).await?);
        let adapters_channel = self.adapters_channel.as_ref().unwrap().clone();
        self.photoshop_client = Some(PhotoshopServiceClient::new(adapters_channel.clone()));
        self.chrome_client = Some(ChromeServiceClient::new(adapters_channel.clone()));
        self.game_client = Some(GameServiceClient::new(adapters_channel.clone()));
        self.window_client = Some(WindowServiceClient::new(adapters_channel));

        Ok(())
    }

    async fn connect_endpoint(&self, endpoint: &str) -> Result<Channel> {
        let endpoint = Endpoint::from_shared(endpoint.to_string())?
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .tcp_keepalive(Duration::from_secs(10));
        
        // Add auth token if available
        let endpoint = if let Some(token) = &self.auth_token {
            endpoint.add_header("authorization", format!("Bearer {}", token).parse()?)
        } else {
            endpoint
        };

        endpoint.connect().await.context("Failed to connect to endpoint")
    }

    fn add_auth(&self, mut metadata: MetadataMap) -> MetadataMap {
        if let Some(token) = &self.auth_token {
            metadata.insert("authorization", format!("Bearer {}", token).parse().unwrap());
        }
        metadata
    }

    // Agent Input Service
    pub async fn inject_key(&self, event: KeyEvent) -> Result<()> {
        let mut client = self.input_client.as_ref().context("Input client not connected")?.clone();
        let mut request = tonic::Request::new(event);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.inject_key(request).await?;
        Ok(())
    }

    pub async fn inject_mouse(&self, event: MouseEvent) -> Result<()> {
        let mut client = self.input_client.as_ref().context("Input client not connected")?.clone();
        let mut request = tonic::Request::new(event);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.inject_mouse(request).await?;
        Ok(())
    }

    pub async fn inject_batch(&self, events: Vec<hcs_agent_proto::InputEvent>) -> Result<()> {
        let mut client = self.input_client.as_ref().context("Input client not connected")?.clone();
        let batch = InputBatch { events };
        let mut request = tonic::Request::new(batch);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.inject_batch(request).await?;
        Ok(())
    }

    pub async fn list_devices(&self) -> Result<Vec<hcs_agent_proto::DeviceInfo>> {
        let mut client = self.input_client.as_ref().context("Input client not connected")?.clone();
        let mut request = tonic::Request::new(Empty {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.list_devices(request).await?;
        Ok(response.into_inner().devices)
    }

    pub async fn get_driver_info(&self) -> Result<hcs_agent_proto::DriverInfo> {
        let mut client = self.input_client.as_ref().context("Input client not connected")?.clone();
        let mut request = tonic::Request::new(Empty {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.get_driver_info(request).await?;
        Ok(response.into_inner())
    }

    // Agent Macro Service
    pub async fn execute_macro(&self, name: &str, parameters: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let mut client = self.macro_client.as_ref().context("Macro client not connected")?.clone();
        let mut request = tonic::Request::new(MacroRequest {
            name: name.to_string(),
            parameters: serde_json::to_value(parameters)?.try_into()?,
            variance_ms: 0,
        });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.execute_macro(request).await?;
        Ok(serde_json::to_value(response.into_inner())?)
    }

    pub async fn list_macros(&self) -> Result<Vec<hcs_agent_proto::MacroInfo>> {
        let mut client = self.macro_client.as_ref().context("Macro client not connected")?.clone();
        let mut request = tonic::Request::new(Empty {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.list_macros(request).await?;
        Ok(response.into_inner().macros)
    }

    pub async fn register_macro(&self, macro_def: MacroDefinition) -> Result<()> {
        let mut client = self.macro_client.as_ref().context("Macro client not connected")?.clone();
        let mut request = tonic::Request::new(macro_def);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.register_macro(request).await?;
        Ok(())
    }

    // Agent Task Service
    pub async fn submit_task(&self, request: TaskRequest) -> Result<hcs_agent_proto::TaskResponse> {
        let mut client = self.task_client.as_ref().context("Task client not connected")?.clone();
        let mut request = tonic::Request::new(request);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.submit_task(request).await?;
        Ok(response.into_inner())
    }

    pub async fn get_task_status(&self, task_id: &str) -> Result<hcs_agent_proto::TaskResponse> {
        let mut client = self.task_client.as_ref().context("Task client not connected")?.clone();
        let mut request = tonic::Request::new(TaskId { task_id: task_id.to_string() });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.get_task_status(request).await?;
        Ok(response.into_inner())
    }

    pub async fn cancel_task(&self, task_id: &str) -> Result<()> {
        let mut client = self.task_client.as_ref().context("Task client not connected")?.clone();
        let mut request = tonic::Request::new(TaskId { task_id: task_id.to_string() });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.cancel_task(request).await?;
        Ok(())
    }

    pub async fn list_tasks(&self, filter: Option<TaskFilter>) -> Result<Vec<hcs_agent_proto::TaskResponse>> {
        let mut client = self.task_client.as_ref().context("Task client not connected")?.clone();
        let mut request = tonic::Request::new(filter.unwrap_or_default());
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.list_tasks(request).await?;
        Ok(response.into_inner().tasks)
    }

    // Agent System Service
    pub async fn get_system_info(&self) -> Result<hcs_agent_proto::SystemInfo> {
        let mut client = self.system_client.as_ref().context("System client not connected")?.clone();
        let mut request = tonic::Request::new(Empty {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.get_system_info(request).await?;
        Ok(response.into_inner())
    }

    pub async fn health_check(&self) -> Result<hcs_agent_proto::HealthCheckResponse> {
        let mut client = self.system_client.as_ref().context("System client not connected")?.clone();
        let mut request = tonic::Request::new(HealthCheckRequest {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.health_check(request).await?;
        Ok(response.into_inner())
    }

    // Vision Service
    pub async fn capture_screen(&self, request: CaptureRequest) -> Result<hcs_vision_proto::CaptureResponse> {
        let mut client = self.vision_client.as_ref().context("Vision client not connected")?.clone();
        let mut request = tonic::Request::new(request);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.capture_screen(request).await?;
        Ok(response.into_inner())
    }

    pub async fn detect_objects(&self, request: DetectionRequest) -> Result<hcs_vision_proto::DetectionResponse> {
        let mut client = self.vision_client.as_ref().context("Vision client not connected")?.clone();
        let mut request = tonic::Request::new(request);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.detect_objects(request).await?;
        Ok(response.into_inner())
    }

    pub async fn recognize_text(&self, request: OcrRequest) -> Result<hcs_vision_proto::OcrResponse> {
        let mut client = self.vision_client.as_ref().context("Vision client not connected")?.clone();
        let mut request = tonic::Request::new(request);
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.recognize_text(request).await?;
        Ok(response.into_inner())
    }

    pub async fn list_monitors(&self) -> Result<Vec<hcs_vision_proto::MonitorInfo>> {
        let mut client = self.vision_client.as_ref().context("Vision client not connected")?.clone();
        let mut request = tonic::Request::new(Empty {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.list_monitors(request).await?;
        Ok(response.into_inner().monitors)
    }

    // Brain Service
    pub async fn execute_brain_tree(
        &self,
        tree_id: &str,
        tree_definition: Option<String>,
        blackboard: HashMap<String, String>,
    ) -> Result<serde_json::Value> {
        let mut client = self.brain_client.as_ref().context("Brain client not connected")?.clone();
        let mut request = tonic::Request::new(ExecuteTreeRequest {
            tree_id: tree_id.to_string(),
            tree_definition: tree_definition.unwrap_or_default(),
            initial_blackboard: blackboard,
            persist_state: false,
        });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.execute_tree(request).await?;
        Ok(serde_json::to_value(response.into_inner())?)
    }

    pub async fn tick_brain_tree(&self, tree_id: &str, blackboard: HashMap<String, String>) -> Result<serde_json::Value> {
        let mut client = self.brain_client.as_ref().context("Brain client not connected")?.clone();
        let mut request = tonic::Request::new(TickTreeRequest {
            tree_id: tree_id.to_string(),
            blackboard_updates: blackboard,
        });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.tick_tree(request).await?;
        Ok(serde_json::to_value(response.into_inner())?)
    }

    pub async fn load_brain_tree(&self, tree_id: &str, tree_definition: String, blackboard: HashMap<String, String>) -> Result<()> {
        let mut client = self.brain_client.as_ref().context("Brain client not connected")?.clone();
        let mut request = tonic::Request::new(LoadTreeRequest {
            tree_id: tree_id.to_string(),
            tree_definition,
            initial_blackboard: blackboard,
        });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.load_tree(request).await?;
        Ok(())
    }

    pub async fn list_brain_trees(&self) -> Result<Vec<brain_proto::TreeInfo>> {
        let mut client = self.brain_client.as_ref().context("Brain client not connected")?.clone();
        let mut request = tonic::Request::new(ListTreesRequest {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.list_trees(request).await?;
        Ok(response.into_inner().trees)
    }

    // Photoshop Adapter
    pub async fn photoshop_operation(&self, operation: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let mut client = self.photoshop_client.as_ref().context("Photoshop client not connected")?.clone();
        
        // This is a simplified dispatch - in practice would use proper method routing
        // For now, just return a placeholder
        Ok(serde_json::json!({"operation": operation, "status": "not_implemented"}))
    }

    // Chrome Adapter
    pub async fn chrome_operation(&self, operation: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let mut client = self.chrome_client.as_ref().context("Chrome client not connected")?.clone();
        
        Ok(serde_json::json!({"operation": operation, "status": "not_implemented"}))
    }

    // Game Adapter
    pub async fn game_operation(&self, operation: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let mut client = self.game_client.as_ref().context("Game client not connected")?.clone();
        
        Ok(serde_json::json!({"operation": operation, "status": "not_implemented"}))
    }

    // Window Adapter
    pub async fn window_operation(&self, operation: &str, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value> {
        let mut client = self.window_client.as_ref().context("Window client not connected")?.clone();
        
        Ok(serde_json::json!({"operation": operation, "status": "not_implemented"}))
    }

    // Orchestrator Service (for recording management)
    pub async fn list_recordings(&self) -> Result<Vec<orchestrator_proto::RecordingMeta>> {
        let mut client = OrchestratorServiceClient::new(
            self.agent_channel.as_ref().context("Agent channel not connected")?.clone()
        );
        let mut request = tonic::Request::new(Empty {});
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.list_recordings(request).await?;
        Ok(response.into_inner().recordings)
    }

    pub async fn get_recording(&self, id: &str) -> Result<orchestrator_proto::Recording> {
        let mut client = OrchestratorServiceClient::new(
            self.agent_channel.as_ref().context("Agent channel not connected")?.clone()
        );
        let mut request = tonic::Request::new(orchestrator_proto::RecordingId { id: id.to_string() });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        let response = client.get_recording(request).await?;
        Ok(response.into_inner())
    }

    pub async fn delete_recording(&self, id: &str) -> Result<()> {
        let mut client = OrchestratorServiceClient::new(
            self.agent_channel.as_ref().context("Agent channel not connected")?.clone()
        );
        let mut request = tonic::Request::new(orchestrator_proto::RecordingId { id: id.to_string() });
        request.metadata_mut().extend(self.add_auth(MetadataMap::new()));
        client.delete_recording(request).await?;
        Ok(())
    }
}

// Helper trait to convert serde_json::Value to prost_types::Struct
trait ToProtoStruct {
    fn try_into(self) -> Result<prost_types::Struct>;
}

impl ToProtoStruct for serde_json::Value {
    fn try_into(self) -> Result<prost_types::Struct> {
        let fields = match self {
            serde_json::Value::Object(map) => {
                let mut fields = std::collections::HashMap::new();
                for (k, v) in map {
                    fields.insert(k, v.try_into()?);
                }
                fields
            }
            _ => return Err(anyhow::anyhow!("Expected object")),
        };
        Ok(prost_types::Struct { fields })
    }
}

impl ToProtoStruct for &serde_json::Value {
    fn try_into(self) -> Result<prost_types::Struct> {
        self.clone().try_into()
    }
}

// Also need for prost_types::Value
trait ToProtoValue {
    fn try_into(self) -> Result<prost_types::Value>;
}

impl ToProtoValue for serde_json::Value {
    fn try_into(self) -> Result<prost_types::Value> {
        Ok(match self {
            serde_json::Value::Null => prost_types::Value { kind: Some(prost_types::value::Kind::NullValue(0)) },
            serde_json::Value::Bool(b) => prost_types::Value { kind: Some(prost_types::value::Kind::BoolValue(b)) },
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    prost_types::Value { kind: Some(prost_types::value::Kind::NumberValue(i as f64)) }
                } else if let Some(f) = n.as_f64() {
                    prost_types::Value { kind: Some(prost_types::value::Kind::NumberValue(f)) }
                } else {
                    prost_types::Value { kind: Some(prost_types::value::Kind::NumberValue(0.0)) }
                }
            }
            serde_json::Value::String(s) => prost_types::Value { kind: Some(prost_types::value::Kind::StringValue(s)) },
            serde_json::Value::Array(arr) => {
                let values: Vec<prost_types::Value> = arr.into_iter().map(|v| v.try_into().unwrap()).collect();
                prost_types::Value { kind: Some(prost_types::value::Kind::ListValue(prost_types::ListValue { values })) }
            }
            serde_json::Value::Object(obj) => {
                let fields: std::collections::HashMap<String, prost_types::Value> = obj.into_iter()
                    .map(|(k, v)| (k, v.try_into().unwrap()))
                    .collect();
                prost_types::Value { kind: Some(prost_types::value::Kind::StructValue(prost_types::Struct { fields })) }
            }
        })
    }
}