//! 화면 캡처 모듈.
//!
//! - 1차: DXGI Desktop Duplication (Exclusive Fullscreen 지원)
//! - 2차: Windows Graphics Capture (Borderless/Windowed)

pub mod detector;
pub mod dxgi;
pub mod wgc;
