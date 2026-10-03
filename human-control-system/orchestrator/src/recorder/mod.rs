pub mod capture;
pub mod replay;

pub use capture::{Recorder, RecorderConfig, Recording, RecordedEvent, EventType, MouseButton};
pub use replay::{Replayer, ReplayResult};