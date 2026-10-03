use crate::recorder::capture::{Recording, RecordedEvent};
use crate::client::grpc_client::GrpcClient;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, info};

pub struct Replayer {
    client: Arc<GrpcClient>,
    speed: f32,
    jitter: bool,
    loop_playback: bool,
}

impl Replayer {
    pub fn new(client: Arc<GrpcClient>) -> Self {
        Self {
            client,
            speed: 1.0,
            jitter: true,
            loop_playback: false,
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.max(0.1).min(10.0);
    }

    pub fn set_jitter(&mut self, jitter: bool) {
        self.jitter = jitter;
    }

    pub fn set_loop(&mut self, loop_playback: bool) {
        self.loop_playback = loop_playback;
    }

    pub async fn replay(&mut self, recording: &Recording) -> Result<ReplayResult> {
        info!("Replaying recording: {} ({} events)", recording.name, recording.events.len());

        let start_time = std::time::Instant::now();
        let mut last_timestamp = 0u64;
        let mut executed_events = 0;
        let mut failed_events = 0;
        let mut errors = Vec::new();

        loop {
            for event in &recording.events {
                let delay_us = if event.timestamp_us > last_timestamp {
                    event.timestamp_us - last_timestamp
                } else {
                    0
                };
                last_timestamp = event.timestamp_us;

                // Apply speed factor
                let adjusted_delay = (delay_us as f32 / self.speed) as u64;
                
                // Apply jitter
                let final_delay = if self.jitter && adjusted_delay > 0 {
                    use rand::Rng;
                    let jitter_factor = 0.1;
                    let jitter_range = (adjusted_delay as f32 * jitter_factor) as u64;
                    let jitter = rand::thread_rng().gen_range(0..=jitter_range);
                    adjusted_delay.saturating_add(jitter)
                } else {
                    adjusted_delay
                };

                if final_delay > 0 {
                    sleep(Duration::from_micros(final_delay)).await;
                }

                // Execute event
                if let Err(e) = self.execute_event(event).await {
                    failed_events += 1;
                    errors.push(format!("Event at {}us: {}", event.timestamp_us, e));
                    debug!("Event failed: {}", e);
                } else {
                    executed_events += 1;
                }
            }

            if !self.loop_playback {
                break;
            }
            last_timestamp = 0;
        }

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let success = failed_events == 0;

        info!("Replay completed: {} executed, {} failed, {}ms", 
            executed_events, failed_events, execution_time_ms);

        Ok(ReplayResult {
            success,
            executed_events,
            failed_events,
            execution_time_ms,
            errors,
        })
    }

    async fn execute_event(&self, event: &RecordedEvent) -> Result<()> {
        use hcs_agent_proto::{KeyEvent, MouseEvent, KeyState, MouseButton};

        match &event.event {
            crate::recorder::capture::EventType::KeyDown { code, scan_code, extended } => {
                let key_event = KeyEvent {
                    code: *code,
                    state: KeyState::KeyStateDown as i32,
                    scan_code: *scan_code,
                    extended: *extended,
                    timestamp_us: 0,
                };
                self.client.inject_key(key_event).await
            }
            crate::recorder::capture::EventType::KeyUp { code, scan_code, extended } => {
                let key_event = KeyEvent {
                    code: *code,
                    state: KeyState::KeyStateUp as i32,
                    scan_code: *scan_code,
                    extended: *extended,
                    timestamp_us: 0,
                };
                self.client.inject_key(key_event).await
            }
            crate::recorder::capture::EventType::MouseMove { x, y, absolute } => {
                let mouse_event = MouseEvent {
                    x: *x,
                    y: *y,
                    dx: 0,
                    dy: 0,
                    button: MouseButton::MouseButtonUnspecified as i32,
                    button_state: KeyState::KeyStateUnspecified as i32,
                    absolute: *absolute,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(mouse_event).await
            }
            crate::recorder::capture::EventType::MouseDown { button, x, y } => {
                let mouse_event = MouseEvent {
                    x: *x,
                    y: *y,
                    dx: 0,
                    dy: 0,
                    button: mouse_button_to_proto(button) as i32,
                    button_state: KeyState::KeyStateDown as i32,
                    absolute: true,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(mouse_event).await
            }
            crate::recorder::capture::EventType::MouseUp { button, x, y } => {
                let mouse_event = MouseEvent {
                    x: *x,
                    y: *y,
                    dx: 0,
                    dy: 0,
                    button: mouse_button_to_proto(button) as i32,
                    button_state: KeyState::KeyStateUp as i32,
                    absolute: true,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(mouse_event).await
            }
            crate::recorder::capture::EventType::MouseClick { button, x, y } => {
                // Down
                let down_event = MouseEvent {
                    x: *x,
                    y: *y,
                    dx: 0,
                    dy: 0,
                    button: mouse_button_to_proto(button) as i32,
                    button_state: KeyState::KeyStateDown as i32,
                    absolute: true,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(down_event).await?;
                
                sleep(Duration::from_millis(10)).await;
                
                // Up
                let up_event = MouseEvent {
                    x: *x,
                    y: *y,
                    dx: 0,
                    dy: 0,
                    button: mouse_button_to_proto(button) as i32,
                    button_state: KeyState::KeyStateUp as i32,
                    absolute: true,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(up_event).await
            }
            crate::recorder::capture::EventType::MouseScroll { dx, dy } => {
                let mouse_event = MouseEvent {
                    x: 0,
                    y: 0,
                    dx: *dx,
                    dy: *dy,
                    button: MouseButton::MouseButtonUnspecified as i32,
                    button_state: KeyState::KeyStateUnspecified as i32,
                    absolute: false,
                    timestamp_us: 0,
                };
                self.client.inject_mouse(mouse_event).await
            }
            crate::recorder::capture::EventType::Delay { microseconds } => {
                sleep(Duration::from_micros(*microseconds)).await;
                Ok(())
            }
        }
    }
}

fn mouse_button_to_proto(button: &crate::recorder::capture::MouseButton) -> hcs_agent_proto::MouseButton {
    use crate::recorder::capture::MouseButton;
    match button {
        MouseButton::Left => hcs_agent_proto::MouseButton::MouseButtonLeft,
        MouseButton::Right => hcs_agent_proto::MouseButton::MouseButtonRight,
        MouseButton::Middle => hcs_agent_proto::MouseButton::MouseButtonMiddle,
        MouseButton::X1 => hcs_agent_proto::MouseButton::MouseButtonX1,
        MouseButton::X2 => hcs_agent_proto::MouseButton::MouseButtonX2,
        MouseButton::WheelUp => hcs_agent_proto::MouseButton::MouseButtonWheelUp,
        MouseButton::WheelDown => hcs_agent_proto::MouseButton::MouseButtonWheelDown,
        MouseButton::WheelLeft => hcs_agent_proto::MouseButton::MouseButtonWheelLeft,
        MouseButton::WheelRight => hcs_agent_proto::MouseButton::MouseButtonWheelRight,
    }
}

#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub success: bool,
    pub executed_events: usize,
    pub failed_events: usize,
    pub execution_time_ms: u64,
    pub errors: Vec<String>,
}