use antigen_core::{Changed, LazyComponent, Usage};

use winit::{dpi::PhysicalSize, event::WindowEvent, window::WindowId};

use std::collections::BTreeMap;
use hecs::Entity;

// Winit window
pub type WindowComponent = LazyComponent<winit::window::Window>;

// Tag component for a window that redraws unconditionally
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedrawUnconditionally;

// Window ID -> Entity ID map for winit event handling
pub type WindowEntityMap = BTreeMap<WindowId, Entity>;

/// Window event wrapper
pub type WindowEventComponent = (Option<WindowId>, Option<WindowEvent<'static>>);

/// Usage tag for SizeComponent
pub enum WindowSize {}
pub type WindowSizeComponent = Usage<WindowSize, Changed<PhysicalSize<u32>>>;

/// Usage tag for NameComponent
pub enum WindowTitle {}
pub type WindowTitleComponent = Usage<WindowTitle, Changed<&'static str>>;
