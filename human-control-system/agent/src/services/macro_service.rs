//! Macro Service Implementation

use crate::driver_manager::DriverManager;
use crate::proto::hcs::agent::v1::*;
use crate::auth::AuthManager;
use tonic::{Request, Response, Status};
use std::sync::Arc;

pub struct MacroServiceImpl {
    driver_manager: Arc<DriverManager>,
    auth_manager: Arc<AuthManager>,
}

impl MacroServiceImpl {
    pub fn new(driver_manager: Arc<DriverManager>, auth_manager: Arc<AuthManager>) -> Self {
        Self {
            driver_manager,
            auth_manager,
        }
    }
    
    fn check_auth(&self, request: &Request<()>, action: crate::auth::Action) -> Result<(), Status> {
        if !self.auth_manager.config.enabled {
            return Ok(());
        }
        
        let token = request.metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or_else(|| Status::unauthenticated("Missing authorization token"))?;
        
        self.auth_manager.validate_token(token, "macro:*", action)
            .map_err(|e| Status::permission_denied(e.to_string()))?;
        
        Ok(())
    }
}

#[tonic::async_trait]
impl MacroService for MacroServiceImpl {
    async fn execute_macro(&self, request: Request<MacroRequest>) -> Result<Response<MacroResponse>, Status> {
        self.check_auth(&request, crate::auth::Action::Execute)?;
        
        let req = request.into_inner();
        let variance_ms = req.variance_ms;
        
        match self.driver_manager.execute_macro(&req.name, variance_ms).await {
            Ok(_) => Ok(Response::new(MacroResponse {
                success: true,
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(MacroResponse {
                success: false,
                error: e.to_string(),
            })),
        }
    }
    
    async fn list_macros(&self, request: Request<()>) -> Result<Response<MacroList>, Status> {
        self.check_auth(&request, crate::auth::Action::Read)?;
        
        let macros = self.driver_manager.list_macros().await;
        
        let proto_macros: Vec<MacroInfo> = macros.into_iter().map(|m| MacroInfo {
            name: m.name,
            description: m.description,
            schema: Some(MacroParameterSchema {
                parameters: std::collections::HashMap::new(),
            }),
        }).collect();
        
        Ok(Response::new(MacroList { macros: proto_macros }))
    }
    
    async fn register_macro(&self, request: Request<MacroDefinition>) -> Result<Response<()>, Status> {
        self.check_auth(&request, crate::auth::Action::Admin)?;
        
        let def = request.into_inner();
        let mut events = Vec::new();
        
        for event in def.events {
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
        
        let macro_def = crate::driver_manager::MacroDefinition {
            name: def.name,
            description: def.description,
            events,
            variance_ms: 50,
        };
        
        self.driver_manager.register_macro(macro_def).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(()))
    }
    
    async fn unregister_macro(&self, request: Request<MacroName>) -> Result<Response<()>, Status> {
        self.check_auth(&request, crate::auth::Action::Admin)?;
        
        let name = request.into_inner().name;
        self.driver_manager.unregister_macro(&name).await
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(()))
    }
}