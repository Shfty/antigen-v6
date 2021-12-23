use antigen_core::{ChangedFlag, Construct, With};
use winit::dpi::PhysicalSize;

use crate::{
    WindowComponent, WindowEntityMap, WindowEventComponent, WindowSizeComponent,
    WindowTitleComponent,
};

#[derive(Default, hecs::Bundle)]
pub struct BackendBundle {
    window_entity_map: WindowEntityMap,
    window_event: WindowEventComponent,
}

#[derive(hecs::Bundle)]
pub struct WindowBundle {
    window: WindowComponent,
    size: WindowSizeComponent,
}

impl Default for WindowBundle {
    fn default() -> Self {
        let size =
            WindowSizeComponent::construct(PhysicalSize::<u32>::default()).with(ChangedFlag(false));

        WindowBundle {
            window: Default::default(),
            size,
        }
    }
}

#[derive(hecs::Bundle)]
pub struct WindowTitleBundle {
    title: WindowTitleComponent,
}

impl WindowTitleBundle {
    pub fn new(title: &'static str) -> Self {
        let title = WindowTitleComponent::construct(title).with(ChangedFlag(true));
        WindowTitleBundle { title }
    }
}
