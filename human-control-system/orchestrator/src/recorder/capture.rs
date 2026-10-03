use crate::client::grpc_client::GrpcClient;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecorderConfig {
    pub capture_keyboard: bool,
    pub capture_mouse: bool,
    pub capture_delays: bool,
    pub max_duration: Option<Duration>,
    pub stop_key: Option<String>,
}

impl Default for RecorderConfig {
    fn default() -> Self {
        Self {
            capture_keyboard: true,
            capture_mouse: true,
            capture_delays: true,
            max_duration: None,
            stop_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub duration_us: u64,
    pub events: Vec<RecordedEvent>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedEvent {
    #[serde(flatten)]
    pub event: EventType,
    pub timestamp_us: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventType {
    KeyDown { code: u32, scan_code: u32, extended: bool },
    KeyUp { code: u32, scan_code: u32, extended: bool },
    MouseMove { x: i32, y: i32, absolute: bool },
    MouseClick { button: MouseButton, x: i32, y: i32 },
    MouseDown { button: MouseButton, x: i32, y: i32 },
    MouseUp { button: MouseButton, x: i32, y: i32 },
    MouseScroll { dx: i32, dy: i32 },
    Delay { microseconds: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

pub struct Recorder {
    client: Arc<GrpcClient>,
    config: RecorderConfig,
    events: Arc<Mutex<Vec<RecordedEvent>>>,
    start_time: Option<Instant>,
    speed: f32,
    jitter: bool,
    loop_playback: bool,
    variable_overrides: HashMap<String, serde_json::Value>,
    recording_rx: Option<mpsc::Receiver<RecordedEvent>>,
}

impl Recorder {
    pub fn new(client: Arc<GrpcClient>) -> Self {
        Self {
            client,
            config: RecorderConfig::default(),
            events: Arc::new(Mutex::new(Vec::new())),
            start_time: None,
            speed: 1.0,
            jitter: true,
            loop_playback: false,
            variable_overrides: HashMap::new(),
            recording_rx: None,
        }
    }

    pub fn set_config(&mut self, config: RecorderConfig) {
        self.config = config;
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

    pub fn set_variable_overrides(&mut self, overrides: HashMap<String, serde_json::Value>) {
        self.variable_overrides = overrides;
    }

    pub async fn start(&mut self) -> Result<Recording> {
        info!("Starting input recording");
        
        self.events.lock().unwrap().clear();
        self.start_time = Some(Instant::now());

        // Start keyboard/mouse listener
        let (tx, rx) = mpsc::channel(1000);
        self.recording_rx = Some(rx);

        let events = self.events.clone();
        let config = self.config.clone();
        let start_time = self.start_time.unwrap();

        // Spawn input listener task
        tokio::spawn(async move {
            #[cfg(target_os = "windows")]
            {
                Self::listen_windows(tx, config, start_time).await;
            }
            #[cfg(target_os = "linux")]
            {
                Self::listen_linux(tx, config, start_time).await;
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            {
                warn!("Recording not supported on this platform");
            }
        });

        // Wait for max duration or stop signal
        if let Some(max_duration) = self.config.max_duration {
            sleep(max_duration).await;
        } else {
            // Wait indefinitely until stop signal
            loop {
                sleep(Duration::from_secs(1)).await;
                if self.recording_rx.is_none() {
                    break;
                }
            }
        }

        self.stop().await
    }

    pub async fn stop(&mut self) -> Result<Recording> {
        info!("Stopping recording");
        self.recording_rx = None;
        
        let events = self.events.lock().unwrap().clone();
        let duration_us = self.start_time.map(|t| t.elapsed().as_micros() as u64).unwrap_or(0);

        let recording = Recording {
            id: uuid::Uuid::new_v4().to_string(),
            name: format!("recording_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S")),
            created_at: chrono::Utc::now(),
            duration_us,
            events,
            metadata: HashMap::new(),
        };

        Ok(recording)
    }

    #[cfg(target_os = "windows")]
    async fn listen_windows(
        tx: mpsc::Sender<RecordedEvent>,
        config: RecorderConfig,
        start_time: Instant,
    ) {
        use rdev::{listen, Event, EventType as RdevEventType, Key, Button};
        
        if let Err(e) = listen(move |event| {
            let timestamp_us = start_time.elapsed().as_micros() as u64;
            
            let recorded_event = match event.event_type {
                RdevEventType::KeyPress(key) if config.capture_keyboard => {
                    Some(RecordedEvent {
                        event: EventType::KeyDown {
                            code: key_to_code(key),
                            scan_code: 0,
                            extended: false,
                        },
                        timestamp_us,
                    })
                }
                RdevEventType::KeyRelease(key) if config.capture_keyboard => {
                    Some(RecordedEvent {
                        event: EventType::KeyUp {
                            code: key_to_code(key),
                            scan_code: 0,
                            extended: false,
                        },
                        timestamp_us,
                    })
                }
                RdevEventType::MouseMove { x, y } if config.capture_mouse => {
                    Some(RecordedEvent {
                        event: EventType::MouseMove {
                            x: x as i32,
                            y: y as i32,
                            absolute: true,
                        },
                        timestamp_us,
                    })
                }
                RdevEventType::ButtonPress(button) if config.capture_mouse => {
                    Some(RecordedEvent {
                        event: EventType::MouseDown {
                            button: button_to_mouse_button(button),
                            x: 0, // Would need to track position
                            y: 0,
                        },
                        timestamp_us,
                    })
                }
                RdevEventType::ButtonRelease(button) if config.capture_mouse => {
                    Some(RecordedEvent {
                        event: EventType::MouseUp {
                            button: button_to_mouse_button(button),
                            x: 0,
                            y: 0,
                        },
                        timestamp_us,
                    })
                }
                RdevEventType::Wheel { delta_x, delta_y } if config.capture_mouse => {
                    Some(RecordedEvent {
                        event: EventType::MouseScroll {
                            dx: delta_x as i32,
                            dy: delta_y as i32,
                        },
                        timestamp_us,
                    })
                }
                _ => None,
            };

            if let Some(event) = recorded_event {
                let _ = tx.try_send(event);
            }
        }) {
            error!("Failed to start listener: {}", e);
        }
    }

    #[cfg(target_os = "linux")]
    async fn listen_linux(
        tx: mpsc::Sender<RecordedEvent>,
        config: RecorderConfig,
        start_time: Instant,
    ) {
        // Linux implementation would use evdev or similar
        warn!("Linux recording not yet implemented");
    }

    pub async fn replay(&mut self, recording: &Recording) -> Result<crate::dsl::interpreter::ExecutionResult> {
        info!("Replaying recording with {} events", recording.events.len());

        let mut interpreter = crate::dsl::interpreter::Interpreter::new(
            self.client.clone(),
            crate::dsl::interpreter::InterpreterConfig::default(),
        ).with_variables(self.variable_overrides.clone());

        let start_time = Instant::now();
        let mut last_timestamp = 0u64;
        let mut step_results = Vec::new();
        let mut success = true;
        let mut error = None;

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
                let step_result = self.replay_event(event).await;
                step_results.push(step_result.clone());

                if step_result.status == crate::dsl::types::StepStatus::Failed {
                    success = false;
                    error = step_result.error.clone();
                    break;
                }
            }

            if !self.loop_playback || !success {
                break;
            }
            last_timestamp = 0;
        }

        Ok(crate::dsl::interpreter::ExecutionResult {
            success,
            task_id: recording.id.clone(),
            error,
            result: None,
            execution_time_ms: start_time.elapsed().as_millis() as u64,
            step_results,
        })
    }

    async fn replay_event(&self, event: &RecordedEvent) -> crate::dsl::types::StepResult {
        use crate::dsl::types::{StepResult, StepStatus};
        use hcs_agent_proto::{KeyEvent, MouseEvent, KeyState, MouseButton};

        let step_id = format!("replay_{}", event.timestamp_us);
        let start_time = chrono::Utc::now();

        let result = match &event.event {
            EventType::KeyDown { code, scan_code, extended } => {
                let key_event = KeyEvent {
                    code: *code,
                    state: KeyState::KeyStateDown as i32,
                    scan_code: *scan_code,
                    extended: *extended,
                    timestamp_us: 0,
                };
                self.client.inject_key(key_event).await
            }
            EventType::KeyUp { code, scan_code, extended } => {
                let key_event = KeyEvent {
                    code: *code,
                    state: KeyState::KeyStateUp as i32,
                    scan_code: *scan_code,
                    extended: *extended,
                    timestamp_us: 0,
                };
                self.client.inject_key(key_event).await
            }
            EventType::MouseMove { x, y, absolute } => {
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
            EventType::MouseDown { button, x, y } => {
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
            EventType::MouseUp { button, x, y } => {
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
            EventType::MouseClick { button, x, y } => {
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
            EventType::MouseScroll { dx, dy } => {
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
            EventType::Delay { microseconds } => {
                sleep(Duration::from_micros(*microseconds)).await;
                Ok(())
            }
        };

        let end_time = chrono::Utc::now();
        
        match result {
            Ok(_) => StepResult {
                step_id,
                status: StepStatus::Completed,
                output: None,
                error: None,
                start_time,
                end_time: Some(end_time),
                sub_steps: vec![],
            },
            Err(e) => StepResult {
                step_id,
                status: StepStatus::Failed,
                output: None,
                error: Some(e.to_string()),
                start_time,
                end_time: Some(end_time),
                sub_steps: vec![],
            },
        }
    }
}

fn key_to_code(key: rdev::Key) -> u32 {
    use rdev::Key;
    match key {
        Key::KeyA => 0x41, Key::KeyB => 0x42, Key::KeyC => 0x43, Key::KeyD => 0x44,
        Key::KeyE => 0x45, Key::KeyF => 0x46, Key::KeyG => 0x47, Key::KeyH => 0x48,
        Key::KeyI => 0x49, Key::KeyJ => 0x4A, Key::KeyK => 0x4B, Key::KeyL => 0x4C,
        Key::KeyM => 0x4D, Key::KeyN => 0x4E, Key::KeyO => 0x4F, Key::KeyP => 0x50,
        Key::KeyQ => 0x51, Key::KeyR => 0x52, Key::KeyS => 0x53, Key::KeyT => 0x54,
        Key::KeyU => 0x55, Key::KeyV => 0x56, Key::KeyW => 0x57, Key::KeyX => 0x58,
        Key::KeyY => 0x59, Key::KeyZ => 0x5A,
        Key::Num0 => 0x30, Key::Num1 => 0x31, Key::Num2 => 0x32, Key::Num3 => 0x33,
        Key::Num4 => 0x34, Key::Num5 => 0x35, Key::Num6 => 0x36, Key::Num7 => 0x37,
        Key::Num8 => 0x38, Key::Num9 => 0x39,
        Key::F1 => 0x70, Key::F2 => 0x71, Key::F3 => 0x72, Key::F4 => 0x73,
        Key::F5 => 0x74, Key::F6 => 0x75, Key::F7 => 0x76, Key::F8 => 0x77,
        Key::F9 => 0x78, Key::F10 => 0x79, Key::F11 => 0x7A, Key::F12 => 0x7B,
        Key::Space => 0x20, Key::Return => 0x0D, Key::Tab => 0x09, Key::Escape => 0x1B,
        Key::Backspace => 0x08, Key::Delete => 0x2E, Key::Insert => 0x2D,
        Key::Home => 0x24, Key::End => 0x23, Key::PageUp => 0x21, Key::PageDown => 0x22,
        Key::LeftArrow => 0x25, Key::UpArrow => 0x26, Key::RightArrow => 0x27, Key::DownArrow => 0x28,
        Key::ShiftLeft | Key::ShiftRight => 0x10,
        Key::ControlLeft | Key::ControlRight => 0x11,
        Key::AltLeft | Key::AltRight => 0x12,
        Key::MetaLeft | Key::MetaRight => 0x5B,
        _ => 0,
    }
}

fn button_to_mouse_button(button: rdev::Button) -> MouseButton {
    use rdev::Button;
    match button {
        Button::Left => MouseButton::Left,
        Button::Right => MouseButton::Right,
        Button::Middle => MouseButton::Middle,
        Button::Other(b) => match b {
            1 => MouseButton::X1,
            2 => MouseButton::X2,
            _ => MouseButton::Left,
        },
        _ => MouseButton::Left,
    }
}

fn mouse_button_to_proto(button: &MouseButton) -> hcs_agent_proto::MouseButton {
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