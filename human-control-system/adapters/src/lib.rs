//! App Adapters Library
//!
//! High-level interfaces to control specific applications (Photoshop, Chrome, Games) and window management.

pub mod config;
pub mod photoshop;
pub mod chrome;
pub mod game;
pub mod window;
pub mod proto;
pub mod service;

pub use config::AdaptersConfig;
pub use photoshop::{PhotoshopAdapter, PhotoshopAdapterFactory, MockPhotoshopAdapter, PhotoshopApi, PhotoshopApiBuilder};
pub use chrome::{ChromeAdapter, ChromeAdapterFactory, MockChromeAdapter};
pub use game::{GameAdapter, GameAdapterFactory, MockGameAdapter};
pub use window::{WindowAdapter, WindowAdapterFactory, MockWindowAdapter};
pub use service::{AdapterServiceImpl, AdapterServiceState};
pub use proto::adapters_proto;
