use std::collections::hash_map::{
    HashMap,
    Entry,
};
use winit::event as we;
use winit::window as ww;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::rc::Rc;
use std::cell::RefCell;
use specs::{World, WorldExt, world::{Builder, EntitiesRes}};
use ammolite::camera::{Camera, PitchYawCamera3};
use ammolite::Ammolite;
use openxr as xr;
use crate::medium::MediumData;
use crate::vm::MappContainer;

pub use mlib::event::*;

pub struct EventDistributor {
    events: Receiver<Event>,
    sender_to_clone: Sender<Event>,
}

impl EventDistributor {
    pub fn new() -> Self {
        let (sender, receiver) = channel();

        Self {
            events: receiver,
            sender_to_clone: sender,
        }
    }

    pub fn create_sender(&self) -> Sender<Event> {
        self.sender_to_clone.clone()
    }

    pub fn distribute_events(&self, mappcs: &mut [MappContainer], ammolite: &mut Ammolite<MediumData>, world: &mut World, camera: &Rc<RefCell<PitchYawCamera3>>) {
        while let Ok(event) = self.events.try_recv() {
            for mappc in &mut mappcs[..] {
                mappc.send_event(event.clone(), ammolite, world, camera);
            }
        }
    }
}

/// Assigns each device a unique ID to identify it with, in a cross-platform way
pub struct DeviceStore {
    map: HashMap<we::DeviceId, mlib::Device>,
    count: usize,
}

impl DeviceStore {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            count: 0,
        }
    }

    fn register_device(&mut self, id: we::DeviceId) -> mlib::Device {
        match self.map.entry(id) {
            Entry::Occupied(entry) => {
                *entry.get()
            },
            Entry::Vacant(entry) => {
                let device = mlib::Device(self.count);
                self.count += 1;
                entry.insert(device);
                device
            },
        }
    }
}

pub trait IntoWithDeviceStore<Output> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> Output;
}

impl IntoWithDeviceStore<mlib::Device> for we::DeviceId {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::Device {
        device_store.register_device(self)
    }
}

impl IntoWithDeviceStore<mlib::Force> for we::Force {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::Force {
        match self {
            we::Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => {
                mlib::Force::Calibrated {
                    force,
                    max_possible_force,
                    altitude_angle,
                }
            },
            we::Force::Normalized(force) => mlib::Force::Normalized(force),
        }
    }
}

impl IntoWithDeviceStore<mlib::TouchPhase> for we::TouchPhase {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::TouchPhase {
        match self {
            we::TouchPhase::Started => mlib::TouchPhase::Started,
            we::TouchPhase::Moved => mlib::TouchPhase::Moved,
            we::TouchPhase::Ended => mlib::TouchPhase::Ended,
            we::TouchPhase::Cancelled => mlib::TouchPhase::Cancelled,
        }
    }
}

impl IntoWithDeviceStore<mlib::Touch> for we::Touch {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::Touch {
        let we::Touch {
            device_id,
            phase,
            location,
            force,
            id,
        } = self;

        mlib::Touch {
            device_id: device_id.into_with_device_store(device_store),
            phase: phase.into_with_device_store(device_store),
            physical_location: location.into(),
            force: force.map(|force| force.into_with_device_store(device_store)),
            id,
        }
    }
}

impl IntoWithDeviceStore<mlib::Theme> for ww::Theme {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::Theme {
        match self {
            ww::Theme::Light => mlib::Theme::Light,
            ww::Theme::Dark => mlib::Theme::Dark,
        }
    }
}

impl IntoWithDeviceStore<mlib::ElementState> for we::ElementState {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::ElementState {
        match self {
            we::ElementState::Pressed => mlib::ElementState::Pressed,
            we::ElementState::Released => mlib::ElementState::Released,
        }
    }
}

impl IntoWithDeviceStore<mlib::VirtualKeyCode> for we::VirtualKeyCode {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::VirtualKeyCode {
        match self {
            we::VirtualKeyCode::Key1 => mlib::VirtualKeyCode::Key1,
            we::VirtualKeyCode::Key2 => mlib::VirtualKeyCode::Key2,
            we::VirtualKeyCode::Key3 => mlib::VirtualKeyCode::Key3,
            we::VirtualKeyCode::Key4 => mlib::VirtualKeyCode::Key4,
            we::VirtualKeyCode::Key5 => mlib::VirtualKeyCode::Key5,
            we::VirtualKeyCode::Key6 => mlib::VirtualKeyCode::Key6,
            we::VirtualKeyCode::Key7 => mlib::VirtualKeyCode::Key7,
            we::VirtualKeyCode::Key8 => mlib::VirtualKeyCode::Key8,
            we::VirtualKeyCode::Key9 => mlib::VirtualKeyCode::Key9,
            we::VirtualKeyCode::Key0 => mlib::VirtualKeyCode::Key0,
            we::VirtualKeyCode::A => mlib::VirtualKeyCode::A,
            we::VirtualKeyCode::B => mlib::VirtualKeyCode::B,
            we::VirtualKeyCode::C => mlib::VirtualKeyCode::C,
            we::VirtualKeyCode::D => mlib::VirtualKeyCode::D,
            we::VirtualKeyCode::E => mlib::VirtualKeyCode::E,
            we::VirtualKeyCode::F => mlib::VirtualKeyCode::F,
            we::VirtualKeyCode::G => mlib::VirtualKeyCode::G,
            we::VirtualKeyCode::H => mlib::VirtualKeyCode::H,
            we::VirtualKeyCode::I => mlib::VirtualKeyCode::I,
            we::VirtualKeyCode::J => mlib::VirtualKeyCode::J,
            we::VirtualKeyCode::K => mlib::VirtualKeyCode::K,
            we::VirtualKeyCode::L => mlib::VirtualKeyCode::L,
            we::VirtualKeyCode::M => mlib::VirtualKeyCode::M,
            we::VirtualKeyCode::N => mlib::VirtualKeyCode::N,
            we::VirtualKeyCode::O => mlib::VirtualKeyCode::O,
            we::VirtualKeyCode::P => mlib::VirtualKeyCode::P,
            we::VirtualKeyCode::Q => mlib::VirtualKeyCode::Q,
            we::VirtualKeyCode::R => mlib::VirtualKeyCode::R,
            we::VirtualKeyCode::S => mlib::VirtualKeyCode::S,
            we::VirtualKeyCode::T => mlib::VirtualKeyCode::T,
            we::VirtualKeyCode::U => mlib::VirtualKeyCode::U,
            we::VirtualKeyCode::V => mlib::VirtualKeyCode::V,
            we::VirtualKeyCode::W => mlib::VirtualKeyCode::W,
            we::VirtualKeyCode::X => mlib::VirtualKeyCode::X,
            we::VirtualKeyCode::Y => mlib::VirtualKeyCode::Y,
            we::VirtualKeyCode::Z => mlib::VirtualKeyCode::Z,
            we::VirtualKeyCode::Escape => mlib::VirtualKeyCode::Escape,
            we::VirtualKeyCode::F1 => mlib::VirtualKeyCode::F1,
            we::VirtualKeyCode::F2 => mlib::VirtualKeyCode::F2,
            we::VirtualKeyCode::F3 => mlib::VirtualKeyCode::F3,
            we::VirtualKeyCode::F4 => mlib::VirtualKeyCode::F4,
            we::VirtualKeyCode::F5 => mlib::VirtualKeyCode::F5,
            we::VirtualKeyCode::F6 => mlib::VirtualKeyCode::F6,
            we::VirtualKeyCode::F7 => mlib::VirtualKeyCode::F7,
            we::VirtualKeyCode::F8 => mlib::VirtualKeyCode::F8,
            we::VirtualKeyCode::F9 => mlib::VirtualKeyCode::F9,
            we::VirtualKeyCode::F10 => mlib::VirtualKeyCode::F10,
            we::VirtualKeyCode::F11 => mlib::VirtualKeyCode::F11,
            we::VirtualKeyCode::F12 => mlib::VirtualKeyCode::F12,
            we::VirtualKeyCode::F13 => mlib::VirtualKeyCode::F13,
            we::VirtualKeyCode::F14 => mlib::VirtualKeyCode::F14,
            we::VirtualKeyCode::F15 => mlib::VirtualKeyCode::F15,
            we::VirtualKeyCode::F16 => mlib::VirtualKeyCode::F16,
            we::VirtualKeyCode::F17 => mlib::VirtualKeyCode::F17,
            we::VirtualKeyCode::F18 => mlib::VirtualKeyCode::F18,
            we::VirtualKeyCode::F19 => mlib::VirtualKeyCode::F19,
            we::VirtualKeyCode::F20 => mlib::VirtualKeyCode::F20,
            we::VirtualKeyCode::F21 => mlib::VirtualKeyCode::F21,
            we::VirtualKeyCode::F22 => mlib::VirtualKeyCode::F22,
            we::VirtualKeyCode::F23 => mlib::VirtualKeyCode::F23,
            we::VirtualKeyCode::F24 => mlib::VirtualKeyCode::F24,
            we::VirtualKeyCode::Snapshot => mlib::VirtualKeyCode::Snapshot,
            we::VirtualKeyCode::Scroll => mlib::VirtualKeyCode::Scroll,
            we::VirtualKeyCode::Pause => mlib::VirtualKeyCode::Pause,
            we::VirtualKeyCode::Insert => mlib::VirtualKeyCode::Insert,
            we::VirtualKeyCode::Home => mlib::VirtualKeyCode::Home,
            we::VirtualKeyCode::Delete => mlib::VirtualKeyCode::Delete,
            we::VirtualKeyCode::End => mlib::VirtualKeyCode::End,
            we::VirtualKeyCode::PageDown => mlib::VirtualKeyCode::PageDown,
            we::VirtualKeyCode::PageUp => mlib::VirtualKeyCode::PageUp,
            we::VirtualKeyCode::Left => mlib::VirtualKeyCode::Left,
            we::VirtualKeyCode::Up => mlib::VirtualKeyCode::Up,
            we::VirtualKeyCode::Right => mlib::VirtualKeyCode::Right,
            we::VirtualKeyCode::Down => mlib::VirtualKeyCode::Down,
            we::VirtualKeyCode::Back => mlib::VirtualKeyCode::Back,
            we::VirtualKeyCode::Return => mlib::VirtualKeyCode::Return,
            we::VirtualKeyCode::Space => mlib::VirtualKeyCode::Space,
            we::VirtualKeyCode::Compose => mlib::VirtualKeyCode::Compose,
            we::VirtualKeyCode::Caret => mlib::VirtualKeyCode::Caret,
            we::VirtualKeyCode::Numlock => mlib::VirtualKeyCode::Numlock,
            we::VirtualKeyCode::Numpad0 => mlib::VirtualKeyCode::Numpad0,
            we::VirtualKeyCode::Numpad1 => mlib::VirtualKeyCode::Numpad1,
            we::VirtualKeyCode::Numpad2 => mlib::VirtualKeyCode::Numpad2,
            we::VirtualKeyCode::Numpad3 => mlib::VirtualKeyCode::Numpad3,
            we::VirtualKeyCode::Numpad4 => mlib::VirtualKeyCode::Numpad4,
            we::VirtualKeyCode::Numpad5 => mlib::VirtualKeyCode::Numpad5,
            we::VirtualKeyCode::Numpad6 => mlib::VirtualKeyCode::Numpad6,
            we::VirtualKeyCode::Numpad7 => mlib::VirtualKeyCode::Numpad7,
            we::VirtualKeyCode::Numpad8 => mlib::VirtualKeyCode::Numpad8,
            we::VirtualKeyCode::Numpad9 => mlib::VirtualKeyCode::Numpad9,
            we::VirtualKeyCode::AbntC1 => mlib::VirtualKeyCode::AbntC1,
            we::VirtualKeyCode::AbntC2 => mlib::VirtualKeyCode::AbntC2,
            we::VirtualKeyCode::Add => mlib::VirtualKeyCode::Add,
            we::VirtualKeyCode::Apostrophe => mlib::VirtualKeyCode::Apostrophe,
            we::VirtualKeyCode::Apps => mlib::VirtualKeyCode::Apps,
            we::VirtualKeyCode::At => mlib::VirtualKeyCode::At,
            we::VirtualKeyCode::Ax => mlib::VirtualKeyCode::Ax,
            we::VirtualKeyCode::Backslash => mlib::VirtualKeyCode::Backslash,
            we::VirtualKeyCode::Calculator => mlib::VirtualKeyCode::Calculator,
            we::VirtualKeyCode::Capital => mlib::VirtualKeyCode::Capital,
            we::VirtualKeyCode::Colon => mlib::VirtualKeyCode::Colon,
            we::VirtualKeyCode::Comma => mlib::VirtualKeyCode::Comma,
            we::VirtualKeyCode::Convert => mlib::VirtualKeyCode::Convert,
            we::VirtualKeyCode::Decimal => mlib::VirtualKeyCode::Decimal,
            we::VirtualKeyCode::Divide => mlib::VirtualKeyCode::Divide,
            we::VirtualKeyCode::Equals => mlib::VirtualKeyCode::Equals,
            we::VirtualKeyCode::Grave => mlib::VirtualKeyCode::Grave,
            we::VirtualKeyCode::Kana => mlib::VirtualKeyCode::Kana,
            we::VirtualKeyCode::Kanji => mlib::VirtualKeyCode::Kanji,
            we::VirtualKeyCode::LAlt => mlib::VirtualKeyCode::LAlt,
            we::VirtualKeyCode::LBracket => mlib::VirtualKeyCode::LBracket,
            we::VirtualKeyCode::LControl => mlib::VirtualKeyCode::LControl,
            we::VirtualKeyCode::LShift => mlib::VirtualKeyCode::LShift,
            we::VirtualKeyCode::LWin => mlib::VirtualKeyCode::LWin,
            we::VirtualKeyCode::Mail => mlib::VirtualKeyCode::Mail,
            we::VirtualKeyCode::MediaSelect => mlib::VirtualKeyCode::MediaSelect,
            we::VirtualKeyCode::MediaStop => mlib::VirtualKeyCode::MediaStop,
            we::VirtualKeyCode::Minus => mlib::VirtualKeyCode::Minus,
            we::VirtualKeyCode::Multiply => mlib::VirtualKeyCode::Multiply,
            we::VirtualKeyCode::Mute => mlib::VirtualKeyCode::Mute,
            we::VirtualKeyCode::MyComputer => mlib::VirtualKeyCode::MyComputer,
            we::VirtualKeyCode::NavigateForward => mlib::VirtualKeyCode::NavigateForward,
            we::VirtualKeyCode::NavigateBackward => mlib::VirtualKeyCode::NavigateBackward,
            we::VirtualKeyCode::NextTrack => mlib::VirtualKeyCode::NextTrack,
            we::VirtualKeyCode::NoConvert => mlib::VirtualKeyCode::NoConvert,
            we::VirtualKeyCode::NumpadComma => mlib::VirtualKeyCode::NumpadComma,
            we::VirtualKeyCode::NumpadEnter => mlib::VirtualKeyCode::NumpadEnter,
            we::VirtualKeyCode::NumpadEquals => mlib::VirtualKeyCode::NumpadEquals,
            we::VirtualKeyCode::OEM102 => mlib::VirtualKeyCode::OEM102,
            we::VirtualKeyCode::Period => mlib::VirtualKeyCode::Period,
            we::VirtualKeyCode::PlayPause => mlib::VirtualKeyCode::PlayPause,
            we::VirtualKeyCode::Power => mlib::VirtualKeyCode::Power,
            we::VirtualKeyCode::PrevTrack => mlib::VirtualKeyCode::PrevTrack,
            we::VirtualKeyCode::RAlt => mlib::VirtualKeyCode::RAlt,
            we::VirtualKeyCode::RBracket => mlib::VirtualKeyCode::RBracket,
            we::VirtualKeyCode::RControl => mlib::VirtualKeyCode::RControl,
            we::VirtualKeyCode::RShift => mlib::VirtualKeyCode::RShift,
            we::VirtualKeyCode::RWin => mlib::VirtualKeyCode::RWin,
            we::VirtualKeyCode::Semicolon => mlib::VirtualKeyCode::Semicolon,
            we::VirtualKeyCode::Slash => mlib::VirtualKeyCode::Slash,
            we::VirtualKeyCode::Sleep => mlib::VirtualKeyCode::Sleep,
            we::VirtualKeyCode::Stop => mlib::VirtualKeyCode::Stop,
            we::VirtualKeyCode::Subtract => mlib::VirtualKeyCode::Subtract,
            we::VirtualKeyCode::Sysrq => mlib::VirtualKeyCode::Sysrq,
            we::VirtualKeyCode::Tab => mlib::VirtualKeyCode::Tab,
            we::VirtualKeyCode::Underline => mlib::VirtualKeyCode::Underline,
            we::VirtualKeyCode::Unlabeled => mlib::VirtualKeyCode::Unlabeled,
            we::VirtualKeyCode::VolumeDown => mlib::VirtualKeyCode::VolumeDown,
            we::VirtualKeyCode::VolumeUp => mlib::VirtualKeyCode::VolumeUp,
            we::VirtualKeyCode::Wake => mlib::VirtualKeyCode::Wake,
            we::VirtualKeyCode::WebBack => mlib::VirtualKeyCode::WebBack,
            we::VirtualKeyCode::WebFavorites => mlib::VirtualKeyCode::WebFavorites,
            we::VirtualKeyCode::WebForward => mlib::VirtualKeyCode::WebForward,
            we::VirtualKeyCode::WebHome => mlib::VirtualKeyCode::WebHome,
            we::VirtualKeyCode::WebRefresh => mlib::VirtualKeyCode::WebRefresh,
            we::VirtualKeyCode::WebSearch => mlib::VirtualKeyCode::WebSearch,
            we::VirtualKeyCode::WebStop => mlib::VirtualKeyCode::WebStop,
            we::VirtualKeyCode::Yen => mlib::VirtualKeyCode::Yen,
            we::VirtualKeyCode::Copy => mlib::VirtualKeyCode::Copy,
            we::VirtualKeyCode::Paste => mlib::VirtualKeyCode::Paste,
            we::VirtualKeyCode::Cut => mlib::VirtualKeyCode::Cut,
        }
    }
}

impl IntoWithDeviceStore<mlib::KeyboardInput> for we::KeyboardInput {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::KeyboardInput {
        #[allow(deprecated)]
        let we::KeyboardInput {
            scancode,
            state,
            virtual_keycode,
            modifiers: _,
        } = self;

        mlib::KeyboardInput {
            scancode,
            state: state.into_with_device_store(device_store),
            virtual_keycode: virtual_keycode
                .map(|virtual_keycode| virtual_keycode.into_with_device_store(device_store)),
        }
    }
}

impl IntoWithDeviceStore<mlib::ModifiersState> for we::ModifiersState {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::ModifiersState {
        mlib::ModifiersState {
            shift: self.shift(),
            ctrl: self.ctrl(),
            alt: self.alt(),
            logo: self.logo(),
        }
    }
}

impl IntoWithDeviceStore<mlib::MouseButton> for we::MouseButton {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::MouseButton {
        match self {
            we::MouseButton::Left => mlib::MouseButton::Left,
            we::MouseButton::Right => mlib::MouseButton::Right,
            we::MouseButton::Middle => mlib::MouseButton::Middle,
            we::MouseButton::Other(other) => mlib::MouseButton::Other(other),
        }
    }
}

impl IntoWithDeviceStore<mlib::MouseScrollDelta> for we::MouseScrollDelta {
    fn into_with_device_store(self, _device_store: &mut DeviceStore) -> mlib::MouseScrollDelta {
        match self {
            we::MouseScrollDelta::LineDelta(lines, rows) =>
                mlib::MouseScrollDelta::LineDelta(lines, rows),
            we::MouseScrollDelta::PixelDelta(position) => mlib::MouseScrollDelta::PixelDelta {
                logical_position: position.into(),
            }
        }
    }
}

impl<'a> IntoWithDeviceStore<Option<mlib::WindowEvent>> for we::WindowEvent<'a> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> Option<mlib::WindowEvent> {
        Some(match self {
            we::WindowEvent::Resized(physical_size) => mlib::WindowEvent::Resized {
                physical_size: physical_size.into(),
            },
            we::WindowEvent::Moved(physical_position) => mlib::WindowEvent::Moved {
                physical_position: physical_position.into(),
            },
            we::WindowEvent::CloseRequested => mlib::WindowEvent::CloseRequested,
            we::WindowEvent::Destroyed => mlib::WindowEvent::Destroyed,

            // TODO: Add file handling events
            we::WindowEvent::DroppedFile(_)
            | we::WindowEvent::HoveredFile(_)
            | we::WindowEvent::HoveredFileCancelled => return None,

            we::WindowEvent::ReceivedCharacter(character) => mlib::WindowEvent::ReceivedCharacter(character),
            we::WindowEvent::Focused(focused) => mlib::WindowEvent::Focused(focused),
            we::WindowEvent::KeyboardInput {
                device_id,
                input,
                is_synthetic,
            } => mlib::WindowEvent::KeyboardInput {
                device_id: device_id.into_with_device_store(device_store),
                input: input.into_with_device_store(device_store),
                is_synthetic,
            },
            we::WindowEvent::ModifiersChanged(state) => mlib::WindowEvent::ModifiersChanged(
                state.into_with_device_store(device_store),
            ),
            #[allow(deprecated)]
            we::WindowEvent::CursorMoved {
                device_id,
                position,
                modifiers: _,
            } => mlib::WindowEvent::CursorMoved {
                device_id: device_id.into_with_device_store(device_store),
                physical_position: position.into(),
            },
            we::WindowEvent::CursorEntered {
                device_id,
            } => mlib::WindowEvent::CursorEntered {
                device_id: device_id.into_with_device_store(device_store),
            },
            we::WindowEvent::CursorLeft {
                device_id,
            } => mlib::WindowEvent::CursorLeft {
                device_id: device_id.into_with_device_store(device_store),
            },
            #[allow(deprecated)]
            we::WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
                modifiers: _,
            } => mlib::WindowEvent::MouseWheel {
                device_id: device_id.into_with_device_store(device_store),
                delta: delta.into_with_device_store(device_store),
                phase: phase.into_with_device_store(device_store),
            },
            #[allow(deprecated)]
            we::WindowEvent::MouseInput {
                device_id,
                state,
                button,
                modifiers: _,
            } => mlib::WindowEvent::MouseInput {
                device_id: device_id.into_with_device_store(device_store),
                state: state.into_with_device_store(device_store),
                button: button.into_with_device_store(device_store),
            },
            we::WindowEvent::TouchpadPressure {
                device_id,
                pressure,
                stage,
            } => mlib::WindowEvent::TouchpadPressure {
                device_id: device_id.into_with_device_store(device_store),
                pressure,
                stage,
            },
            we::WindowEvent::AxisMotion {
                device_id,
                axis,
                value,
            } => mlib::WindowEvent::AxisMotion {
                device_id: device_id.into_with_device_store(device_store),
                axis,
                value,
            },
            we::WindowEvent::Touch(touch) => mlib::WindowEvent::Touch(
                touch.into_with_device_store(device_store),
            ),
            we::WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_size,
            } => mlib::WindowEvent::ScaleFactorChanged {
                scale_factor,
                new_inner_physical_size: {
                    [new_inner_size.width, new_inner_size.height]
                },
            },
            we::WindowEvent::ThemeChanged(theme) => mlib::WindowEvent::ThemeChanged(
                theme.into_with_device_store(device_store),
            ),
        })
    }
}

impl IntoWithDeviceStore<Option<mlib::DeviceEvent>> for we::DeviceEvent {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> Option<mlib::DeviceEvent> {
        Some(match self {
            we::DeviceEvent::Added => mlib::DeviceEvent::Added,
            we::DeviceEvent::Removed => mlib::DeviceEvent::Removed,
            we::DeviceEvent::MouseMotion {
                delta,
            } => mlib::DeviceEvent::MouseMotion {
                delta
            },
            we::DeviceEvent::MouseWheel {
                delta,
            } => mlib::DeviceEvent::MouseWheel {
                delta: delta.into_with_device_store(device_store),
            },
            we::DeviceEvent::Motion {
                axis,
                value,
            } => mlib::DeviceEvent::Motion {
                axis,
                value,
            },
            we::DeviceEvent::Button {
                button,
                state,
            } => mlib::DeviceEvent::Button {
                button,
                state: state.into_with_device_store(device_store),
            },
            we::DeviceEvent::Key(input) => mlib::DeviceEvent::Key(
                input.into_with_device_store(device_store),
            ),
            we::DeviceEvent::Text {
                codepoint,
            } => mlib::DeviceEvent::Text {
                codepoint
            },
        })
    }
}

impl<'a, T: 'static> IntoWithDeviceStore<Option<mlib::Event>> for we::Event<'a, T> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> Option<mlib::Event> {
        match self {
            we::Event::WindowEvent {
                window_id: _,
                event,
            } => {
                event.into_with_device_store(device_store)
                    .map(|event| mlib::Event::Window(event))
            },
            we::Event::DeviceEvent {
                device_id,
                event,
            } => {
                event.into_with_device_store(device_store)
                    .map(|event| mlib::Event::Device {
                        device_id: device_id.into_with_device_store(device_store),
                        event,
                    })
            },
            _ => None,
        }
    }
}

impl<'a> IntoWithDeviceStore<mlib::XrSessionState> for xr::SessionState {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::XrSessionState {
        match self {
            xr::SessionState::UNKNOWN => mlib::XrSessionState::Unknown,
            xr::SessionState::IDLE => mlib::XrSessionState::Idle,
            xr::SessionState::READY => mlib::XrSessionState::Ready,
            xr::SessionState::SYNCHRONIZED => mlib::XrSessionState::Synchronized,
            xr::SessionState::VISIBLE => mlib::XrSessionState::Visible,
            xr::SessionState::FOCUSED => mlib::XrSessionState::Focused,
            xr::SessionState::STOPPING => mlib::XrSessionState::Stopping,
            xr::SessionState::LOSS_PENDING => mlib::XrSessionState::LossPending,
            xr::SessionState::EXITING => mlib::XrSessionState::Exiting,
            _ => panic!("Unexpected XrSessionState value."),
        }
    }
}

impl<'a> IntoWithDeviceStore<mlib::XrEvent> for xr::EventsLost<'a> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::XrEvent {
        mlib::XrEvent::EventsLost {
            lost_event_count: self.lost_event_count(),
        }
    }
}

impl<'a> IntoWithDeviceStore<mlib::XrEvent> for xr::InstanceLossPending<'a> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::XrEvent {
        mlib::XrEvent::InstanceLossPending {
            loss_time_nanos: self.loss_time().as_nanos(),
        }
    }
}

impl<'a> IntoWithDeviceStore<mlib::XrEvent> for xr::SessionStateChanged<'a> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> mlib::XrEvent {
        mlib::XrEvent::SessionStateChanged {
            state: self.state().into_with_device_store(device_store),
            time_nanos: self.time().as_nanos(),
        }
    }
}

impl<'a> IntoWithDeviceStore<Option<mlib::XrEvent>> for xr::Event<'a> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> Option<mlib::XrEvent> {
        match self {
            xr::Event::EventsLost(event) => {
                Some(event.into_with_device_store(device_store))
            },
            xr::Event::InstanceLossPending(event) => {
                Some(event.into_with_device_store(device_store))
            },
            xr::Event::SessionStateChanged(event) => {
                Some(event.into_with_device_store(device_store))
            },
            // TODO:
            // xr::Event::ReferenceSpaceChangePending(event) => {
            //     Some(event.into_with_device_store(device_store))
            // },
            // xr::Event::PerfSettingsEXT(event) => {
            //     Some(event.into_with_device_store(device_store))
            // },
            // xr::Event::VisibilityMaskChangedKHR(event) => {
            //     Some(event.into_with_device_store(device_store))
            // },
            // xr::Event::InteractionProfileChanged(event) => {
            //     Some(event.into_with_device_store(device_store))
            // },
            _ => None,
        }
    }
}

impl<'a> IntoWithDeviceStore<Option<mlib::Event>> for xr::Event<'a> {
    fn into_with_device_store(self, device_store: &mut DeviceStore) -> Option<mlib::Event> {
        IntoWithDeviceStore::<Option<mlib::XrEvent>>::into_with_device_store(self, device_store)
            .map(|event| mlib::Event::Xr(event))
    }
}
