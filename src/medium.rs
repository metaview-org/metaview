use std::time::Duration;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;
use vulkano::swapchain::{PresentMode, SurfaceTransform, AcquireError, SwapchainCreationError, Surface};
use winit::{ElementState, MouseButton, Event, DeviceEvent, WindowEvent, KeyboardInput, VirtualKeyCode, EventsLoop, WindowBuilder, Window};
use ammolite::{View, Ammolite, CameraTransforms, XrInstance, XrVkSession, HandleEventsCommand, MediumSpecificHandleEventsCommand};
use ammolite::swapchain::Swapchain;
use ammolite::camera::{self, Camera, PitchYawCamera3};
use ammolite::mat4;
use ammolite::math::Mat4;
use smallvec::SmallVec;
use openxr::{self as xr, ViewConfigurationType, EventDataBuffer};

pub enum MediumData {
    Window {
        window: Option<Arc<Surface<Window>>>,
        window_events_loop: Rc<RefCell<EventsLoop>>,
        camera: Box<dyn Camera>,
        pressed_keys: HashSet<VirtualKeyCode>,
        pressed_mouse_buttons: HashSet<MouseButton>,
        cursor_capture: bool,
    },
    Xr {
        camera: Box<dyn Camera>,
        xr_instance: Option<Arc<XrInstance>>,
        xr_vk_session: Option<XrVkSession>,
    },
}

impl MediumData {
    pub fn new_window(
        window_events_loop: Rc<RefCell<EventsLoop>>,
    ) -> Self {
        Self::Window {
            window: None,
            window_events_loop,
            camera: Box::new(PitchYawCamera3::new()),
            pressed_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            cursor_capture: true,
        }
    }

    pub fn new_stereo_hmd() -> Self {
        Self::Xr {
            xr_instance: None,
            xr_vk_session: None,
            camera: Box::new(PitchYawCamera3::new()),
        }
    }
}

impl ammolite::MediumData for MediumData {
    fn get_camera_transforms(&self, view_index: usize, view: &View, dimensions: [NonZeroU32; 2]) -> CameraTransforms {
        println!("view: {:?}", view);
        match self {
            Self::Window {
                window,
                window_events_loop,
                camera,
                pressed_keys,
                pressed_mouse_buttons,
                cursor_capture,
            } => {
                CameraTransforms {
                    position: camera.get_position(),
                    view_matrix: camera.get_view_matrix(),
                    projection_matrix: camera::construct_perspective_projection_matrix(
                        0.001,
                        1000.0,
                        dimensions[0].get() as f32 / dimensions[1].get() as f32,
                        std::f32::consts::FRAC_PI_2,
                    ),
                }
            },
            Self::Xr {
                camera,
                xr_instance,
                xr_vk_session,
            } => {
                let world_space_display_view_matrix =
                    view.pose.orientation.clone().to_homogeneous()
                  * mat4!([1.0, 0.0, 0.0, -view.pose.position[0],
                           0.0, 1.0, 0.0, view.pose.position[1],
                           0.0, 0.0, 1.0, view.pose.position[2],
                           0.0, 0.0, 0.0, 1.0])
                  * camera.get_view_matrix();

                // dbg!(&world_space_display_position);
                dbg!(&world_space_display_view_matrix);

                CameraTransforms {
                    position: camera.get_position(),
                    view_matrix: world_space_display_view_matrix,
                    projection_matrix: camera::construct_perspective_projection_matrix_asymmetric(
                        0.001,
                        1000.0,
                        view.fov.angle_right,
                        view.fov.angle_up,
                        view.fov.angle_left,
                        view.fov.angle_down,
                    ),
                }
            },
        }
    }

    fn handle_events(&mut self, delta_time: &Duration) -> SmallVec<[HandleEventsCommand; 8]> {
        match self {
            Self::Window {
                window,
                window_events_loop,
                camera,
                pressed_keys,
                pressed_mouse_buttons,
                cursor_capture,
            } => {
                let mut mouse_delta = [0.0, 0.0];
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
                            result.push(HandleEventsCommand::MediumSpecific(MediumSpecificHandleEventsCommand::CenterCursorToWindow));
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
                            result.push(HandleEventsCommand::RecreateSwapchain(0));
                        }

                        _ => ()
                    }
                });

                camera.update(delta_time, &mouse_delta, &pressed_keys, &pressed_mouse_buttons);

                result
            },
            Self::Xr {
                camera,
                xr_instance,
                xr_vk_session,
            } => {
                let xr_instance = xr_instance.as_mut().unwrap();
                let mut result = SmallVec::new();
                let mut event_data_buffer = EventDataBuffer::new();

                while let Some(event) = xr_instance.poll_event(&mut event_data_buffer).unwrap() {
                    match event {
                        xr::Event::EventsLost(_) => println!("XR Event: EventsLost"),
                        xr::Event::InstanceLossPending(_) => println!("XR Event: InstanceLossPending"),
                        xr::Event::SessionStateChanged(_) => println!("XR Event: SessionStateChanged"),
                        xr::Event::ReferenceSpaceChangePending(_) => println!("XR Event: ReferenceSpaceChangePending"),
                        xr::Event::PerfSettingsEXT(_) => println!("XR Event: PerfSettingsEXT"),
                        xr::Event::VisibilityMaskChangedKHR(_) => println!("XR Event: VisibilityMaskChangedKHR"),
                        xr::Event::InteractionProfileChanged(_) => println!("XR Event: InteractionProfileChanged"),
                        _ => (),
                    }
                }

                result
            },
        }
    }
}

