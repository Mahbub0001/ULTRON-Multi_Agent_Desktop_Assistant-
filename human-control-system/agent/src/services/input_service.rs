//! Input Service Implementation

use crate::driver_manager::DriverManager;
use crate::proto::hcs::agent::v1::*;
use crate::auth::AuthManager;
use tonic::{Request, Response, Status};
use std::sync::Arc;

pub struct InputServiceImpl {
    driver_manager: Arc<DriverManager>,
    auth_manager: Arc<AuthManager>,
}

impl InputServiceImpl {
    pub fn new(driver_manager: Arc<DriverManager>, auth_manager: Arc<AuthManager>) -> Self {
        Self {
            driver_manager,
            auth_manager,
        }
    }
    
    fn check_auth(&self, request: &Request<()>, resource: &str, action: crate::auth::Action) -> Result<(), Status> {
        if !self.auth_manager.config.enabled {
            return Ok(());
        }
        
        // Extract token from metadata
        let token = request.metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization token"))?;
        
        self.auth_manager.validate_token(token, resource, action)
            .map_err(|e| Status::permission_denied(e.to_string()))?;
        
        Ok(())
    }
}

#[tonic::async_trait]
impl InputService for InputServiceImpl {
    async fn inject_key(&self, request: Request<KeyEvent>) -> Result<Response<()>, Status> {
        self.check_auth(&request, "input:keyboard", crate::auth::Action::Write)?;
        
        let event = request.into_inner();
        let key_event = crate::driver::KeyboardEvent {
            code: event.code as u16,
            state: match event.state {
                KeyState::KeyStateDown => crate::driver::KeyState::Down,
                KeyState::KeyStateUp => crate::driver::KeyState::Up,
                _ => crate::driver::KeyState::Down,
            },
            scan_code: event.scan_code as u16,
            extended: event.extended,
            timestamp: event.timestamp_us,
        };
        
        self.driver_manager.inject_keyboard(key_event).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(()))
    }
    
    async fn inject_mouse(&self, request: Request<MouseEvent>) -> Result<Response<()>, Status> {
        self.check_auth(&request, "input:mouse", crate::auth::Action::Write)?;
        
        let event = request.into_inner();
        let mouse_event = crate::driver::MouseEvent {
            x: event.x,
            y: event.y,
            dx: event.dx,
            dy: event.dy,
            button: match event.button {
                MouseButton::MouseButtonLeft => Some(crate::driver::MouseButton::Left),
                MouseButton::MouseButtonRight => Some(crate::driver::MouseButton::Right),
                MouseButton::MouseButtonMiddle => Some(crate::driver::MouseButton::Middle),
                MouseButton::MouseButtonX1 => Some(crate::driver::MouseButton::X1),
                MouseButton::MouseButtonX2 => Some(crate::driver::MouseButton::X2),
                MouseButton::MouseButtonWheelUp => Some(crate::driver::MouseButton::WheelUp),
                MouseButton::MouseButtonWheelDown => Some(crate::driver::MouseButton::WheelDown),
                MouseButton::MouseButtonWheelLeft => Some(crate::driver::MouseButton::WheelLeft),
                MouseButton::MouseButtonWheelRight => Some(crate::driver::MouseButton::WheelRight),
                _ => None,
            },
            button_state: match event.button_state {
                KeyState::KeyStateDown => Some(crate::driver::KeyState::Down),
                KeyState::KeyStateUp => Some(crate::driver::KeyState::Up),
                _ => None,
            },
            absolute: event.absolute,
            timestamp: event.timestamp_us,
        };
        
        self.driver_manager.inject_mouse(mouse_event).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(()))
    }
    
    async fn inject_batch(&self, request: Request<InputBatch>) -> Result<Response<()>, Status> {
        self.check_auth(&request, "input:*", crate::auth::Action::Write)?;
        
        let batch = request.into_inner();
        let mut events = Vec::with_capacity(batch.events.len());
        
        for event in batch.events {
            match event.event {
                Some(input_event::Event::Key(key)) => {
                    events.push(crate::driver::InputEvent::Keyboard(crate::driver::KeyboardEvent {
                        code: key.code as u16,
                        state: match key.state {
                            KeyState::KeyStateDown => crate::driver::KeyState::Down,
                            KeyState::KeyStateUp => crate::driver::KeyState::Up,
                            _ => crate::driver::KeyState::Down,
                        },
                        scan_code: key.scan_code as u16,
                        extended: key.extended,
                        timestamp: key.timestamp_us,
                    }));
                }
                Some(input_event::Event::Mouse(mouse)) => {
                    events.push(crate::driver::InputEvent::Mouse(crate::driver::MouseEvent {
                        x: mouse.x,
                        y: mouse.y,
                        dx: mouse.dx,
                        dy: mouse.dy,
                        button: match mouse.button {
                            MouseButton::MouseButtonLeft => Some(crate::driver::MouseButton::Left),
                            MouseButton::MouseButtonRight => Some(crate::driver::MouseButton::Right),
                            MouseButton::MouseButtonMiddle => Some(crate::driver::MouseButton::Middle),
                            MouseButton::MouseButtonX1 => Some(crate::driver::MouseButton::X1),
                            MouseButton::MouseButtonX2 => Some(crate::driver::MouseButton::X2),
                            MouseButton::MouseButtonWheelUp => Some(crate::driver::MouseButton::WheelUp),
                            MouseButton::MouseButtonWheelDown => Some(crate::driver::MouseButton::WheelDown),
                            MouseButton::MouseButtonWheelLeft => Some(crate::driver::MouseButton::WheelLeft),
                            MouseButton::MouseButtonWheelRight => Some(crate::driver::MouseButton::WheelRight),
                            _ => None,
                        },
                        button_state: match mouse.button_state {
                            KeyState::KeyStateDown => Some(crate::driver::KeyState::Down),
                            KeyState::KeyStateUp => Some(crate::driver::KeyState::Up),
                            _ => None,
                        },
                        absolute: mouse.absolute,
                        timestamp: mouse.timestamp_us,
                    }));
                }
                Some(input_event::Event::Delay(delay)) => {
                    events.push(crate::driver::InputEvent::Delay(delay.microseconds));
                }
                None => {}
            }
        }
        
        self.driver_manager.inject_batch(events).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(()))
    }
    
    async fn inject_stream(&self, request: Request<tonic::Streaming<InputEvent>>) -> Result<Response<()>, Status> {
        self.check_auth(&request, "input:*", crate::auth::Action::Write)?;
        
        let mut stream = request.into_inner();
        let mut batch = Vec::new();
        
        while let Some(event) = stream.message().await? {
            match event.event {
                Some(input_event::Event::Key(key)) => {
                    batch.push(crate::driver::InputEvent::Keyboard(crate::driver::KeyboardEvent {
                        code: key.code as u16,
                        state: match key.state {
                            KeyState::KeyStateDown => crate::driver::KeyState::Down,
                            KeyState::KeyStateUp => crate::driver::KeyState::Up,
                            _ => crate::driver::KeyState::Down,
                        },
                        scan_code: key.scan_code as u16,
                        extended: key.extended,
                        timestamp: key.timestamp_us,
                    }));
                }
                Some(input_event::Event::Mouse(mouse)) => {
                    batch.push(crate::driver::InputEvent::Mouse(crate::driver::MouseEvent {
                        x: mouse.x,
                        y: mouse.y,
                        dx: mouse.dx,
                        dy: mouse.dy,
                        button: match mouse.button {
                            MouseButton::MouseButtonLeft => Some(crate::driver::MouseButton::Left),
                            MouseButton::MouseButtonRight => Some(crate::driver::MouseButton::Right),
                            MouseButton::MouseButtonMiddle => Some(crate::driver::MouseButton::Middle),
                            MouseButton::MouseButtonX1 => Some(crate::driver::MouseButton::X1),
                            MouseButton::MouseButtonX2 => Some(crate::driver::MouseButton::X2),
                            MouseButton::MouseButtonWheelUp => Some(crate::driver::MouseButton::WheelUp),
                            MouseButton::MouseButtonWheelDown => Some(crate::driver::MouseButton::WheelDown),
                            MouseButton::MouseButtonWheelLeft => Some(crate::driver::MouseButton::WheelLeft),
                            MouseButton::MouseButtonWheelRight => Some(crate::driver::MouseButton::WheelRight),
                            _ => None,
                        },
                        button_state: match mouse.button_state {
                            KeyState::KeyStateDown => Some(crate::driver::KeyState::Down),
                            KeyState::KeyStateUp => Some(crate::driver::KeyState::Up),
                            _ => None,
                        },
                        absolute: mouse.absolute,
                        timestamp: mouse.timestamp_us,
                    }));
                }
                Some(input_event::Event::Delay(delay)) => {
                    batch.push(crate::driver::InputEvent::Delay(delay.microseconds));
                }
                None => {}
            }
            
            // Flush batch periodically
            if batch.len() >= 64 {
                self.driver_manager.inject_batch(batch).await
                    .map_err(|e| Status::internal(e.to_string()))?;
                batch.clear();
            }
        }
        
        // Flush remaining
        if !batch.is_empty() {
            self.driver_manager.inject_batch(batch).await
                .map_err(|e| Status::internal(e.to_string()))?;
        }
        
        Ok(Response::new(()))
    }
    
    async fn list_devices(&self, request: Request<()>) -> Result<Response<DeviceList>, Status> {
        self.check_auth(&request, "input:*", crate::auth::Action::Read)?;
        
        let devices = self.driver_manager.get_devices().await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let proto_devices: Vec<DeviceInfo> = devices.into_iter().map(|d| DeviceInfo {
            id: d.id,
            name: d.name,
            type_: d.device_type as i32,
            vendor_id: d.vendor_id as u32,
            product_id: d.product_id as u32,
            is_keyboard: d.is_keyboard,
            is_mouse: d.is_mouse,
            is_touch: d.is_touch,
        }).collect();
        
        Ok(Response::new(DeviceList { devices: proto_devices }))
    }
    
    async fn configure_driver(&self, request: Request<DriverConfig>) -> Result<Response<()>, Status> {
        self.check_auth(&request, "system:config", crate::auth::Action::Admin)?;
        
        let config = request.into_inner();
        let driver_config = crate::driver::DriverConfig {
            backend: match config.backend {
                BackendType::BackendTypeAuto => crate::driver::BackendType::Auto,
                BackendType::BackendTypeInterception => crate::driver::BackendType::Interception,
                BackendType::BackendTypeUinput => crate::driver::BackendType::UInput,
                BackendType::BackendTypeHid => crate::driver::BackendType::HID,
                BackendType::BackendTypeSynthetic => crate::driver::BackendType::Synthetic,
                _ => crate::driver::BackendType::Auto,
            },
            exclusive_mode: config.exclusive_mode,
            injection_delay_us: config.injection_delay_us,
            max_batch_size: config.max_batch_size as usize,
            device_filter: config.device_filter.map(|f| crate::driver::DeviceFilter {
                vendor_ids: f.vendor_ids.into_iter().map(|v| v as u16).collect(),
                product_ids: f.product_ids.into_iter().map(|v| v as u16).collect(),
                device_names: f.device_names,
            }),
        };
        
        self.driver_manager.reconfigure(driver_config).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(()))
    }
    
    async fn get_driver_info(&self, request: Request<()>) -> Result<Response<DriverInfo>, Status> {
        self.check_auth(&request, "system:info", crate::auth::Action::Read)?;
        
        let devices = self.driver_manager.get_devices().await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let stats = self.driver_manager.get_stats();
        
        let proto_devices: Vec<DeviceInfo> = devices.into_iter().map(|d| DeviceInfo {
            id: d.id,
            name: d.name,
            type_: d.device_type as i32,
            vendor_id: d.vendor_id as u32,
            product_id: d.product_id as u32,
            is_keyboard: d.is_keyboard,
            is_mouse: d.is_mouse,
            is_touch: d.is_touch,
        }).collect();
        
        Ok(Response::new(DriverInfo {
            backend: self.driver_manager.config.backend as i32,
            initialized: true,
            ready: self.driver_manager.is_ready().await,
            devices: proto_devices,
            stats: Some(ControllerStats {
                events_injected: stats.events_injected,
                events_failed: stats.events_failed,
                avg_latency_us: stats.avg_latency_us,
                last_error: stats.last_error.unwrap_or_default(),
            }),
        }))
    }
}