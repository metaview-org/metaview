use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;
use vulkano::swapchain::{PresentMode, SurfaceTransform, AcquireError, SwapchainCreationError, Surface};
use winit::{ElementState, MouseButton, Event, DeviceEvent, WindowEvent, KeyboardInput, VirtualKeyCode, EventsLoop, WindowBuilder, Window};
use ammolite::{Ammolite, XrInstance, XrVkSession, HandleEventsCommand};
use ammolite::swapchain::Swapchain;
use ammolite::camera::{Camera, PitchYawCamera3};
use smallvec::SmallVec;

pub enum MediumData {
    Window {
        window: Arc<Surface<Window>>,
        window_events_loop: Rc<RefCell<EventsLoop>>,
        mouse_delta: [f64; 2],
        camera: Box<dyn Camera>,
        pressed_keys: HashSet<VirtualKeyCode>,
        pressed_mouse_buttons: HashSet<MouseButton>,
        cursor_capture: bool,
    },
    Xr {
        xr_instance: Arc<XrInstance>,
        xr_session: Arc<XrVkSession>,
    },
}

impl MediumData {
    pub fn new_window(
        window: Arc<Surface<Window>>,
        window_events_loop: Rc<RefCell<EventsLoop>>,
        swapchain: Arc<dyn Swapchain>,
    ) -> Self {
        Self::Window {
            window,
            window_events_loop,
            mouse_delta: [0.0; 2],
            camera: Box::new(PitchYawCamera3::new()),
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            cursor_capture: true,
        }
    }
}

impl ammolite::MediumData for MediumData {
    fn handle_events(&mut self) -> SmallVec<[HandleEventsCommand; 8]> {
        match self {
            Self::Window {
                window,
                window_events_loop,
                mouse_delta,
                camera,
                pressed_keys,
                pressed_mouse_buttons,
                cursor_capture,
            } => {
                let mut result = SmallVec::new();

                window_events_loop.clone().as_ref().borrow_mut().poll_events(|ev| {
                    match ev {
                        Event::WindowEvent {
                            event: WindowEvent::KeyboardInput {
                                input: KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                                ..
                            },
                            ..
                        } |
                        Event::WindowEvent {
                            event: WindowEvent::CloseRequested,
                            ..
                        } => result.push(HandleEventsCommand::Quit),

                        Event::WindowEvent {
                            event: WindowEvent::KeyboardInput {
                                input: KeyboardInput {
                                    state: ElementState::Released,
                                    virtual_keycode: Some(VirtualKeyCode::LAlt),
                                    ..
                                },
                                ..
                            },
                            ..
                        } => *cursor_capture ^= true,

                        Event::DeviceEvent {
                            event: DeviceEvent::Motion { axis, value },
                            ..
                        } if *cursor_capture => {
                            match axis {
                                0 => mouse_delta[0] += value,
                                1 => mouse_delta[1] += value,
                                _ => (),
                            }
                        },

                        Event::DeviceEvent {
                            event: DeviceEvent::MouseMotion { .. },
                            ..
                        } if *cursor_capture => {
                            result.push(HandleEventsCommand::CenterCursor);
                            // window.window().set_cursor_position(
                            //     (ammolite.window_dimensions[0].get() as f64 / 2.0, ammolite.window_dimensions[1].get() as f64 / 2.0).into()
                            // ).expect("Could not center the cursor position.");
                        }

                        Event::WindowEvent {
                            event: WindowEvent::KeyboardInput {
                                input: KeyboardInput {
                                    state,
                                    virtual_keycode: Some(virtual_code),
                                    ..
                                },
                                ..
                            },
                            ..
                        } => {
                            match state {
                                ElementState::Pressed => { pressed_keys.insert(virtual_code); }
                                ElementState::Released => { pressed_keys.remove(&virtual_code); }
                            }
                        },

                        Event::WindowEvent {
                            event: WindowEvent::MouseInput {
                                state,
                                button,
                                ..
                            },
                            ..
                        } => {
                            match state {
                                ElementState::Pressed => { pressed_mouse_buttons.insert(button); }
                                ElementState::Released => { pressed_mouse_buttons.remove(&button); }
                            }
                        }

                        Event::WindowEvent {
                            event: WindowEvent::Resized(_),
                            ..
                        } => {
                            result.push(HandleEventsCommand::RecreateSwapchains);
                            // TODO: recreate only actual window swapchains, not HMD ones?
                            // for view_swapchain in &ammolite.view_swapchains.inner[..] {
                            //     let mut view_swapchain = view_swapchain.write().unwrap();
                            //     view_swapchain.recreate = true;
                            // }
                        }

                        _ => ()
                    }
                });

                result
            },
            _ => unimplemented!(),
        }
    }
}

