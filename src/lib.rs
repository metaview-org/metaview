//! TODO:
//! * Example App #2 -- Interactions with entities
//! * Asynchronous execution of applications
//! * Convert command message passing using the exports to exports/imports
//!   (see wasmtime-api and wasmtime-interface-types)
//! * Camera movement (App prioritization?)
//! * Applications as libraries? Inter-application communication?
//!
//! Most likely cancelled because of the transition to webgpu:
//! * Add entity deletion
//! * Figure out a way to represent point lights:
//!   - abuse glTF scenes, which you can use to store light sources with;
//!   - or use an explicit representation for light sources
//! * Figure out how to represent cameras/views in the scene graph and how to
//!   render geometry relative to the cameras/views. Ideas:
//!   - Make HMDs proper entities of the scene graph
//!   - Make HMDs' orientations available via a resource

#![feature(test)]
extern crate test;

use std::rc::Rc;
use std::cell::RefCell;
use winit::{
    event_loop::EventLoop,
    window::WindowBuilder,
};
use std::time::Instant;
use winit::dpi::PhysicalSize;
use ammolite::{Ammolite, WorldSpaceModel, UninitializedWindowMedium, UninitializedStereoHmdMedium};
use ammolite_math::*;
use ammolite::camera::PitchYawCamera3;
use lazy_static::lazy_static;
use specs::prelude::*;
use specs_hierarchy::HierarchySystem;
use ::mlib::MappInterface;
use crate::medium::{MediumData, SpecializedMediumData};
use crate::ecs::*;
use crate::vm::{Mapp, MappExports, MappContainer};
use crate::vm::event::{DeviceStore, EventDistributor};

pub mod medium;
pub mod ecs;
pub mod vm;

lazy_static! {
    static ref PACKAGE_VERSION: (u16, u16, u16) = (
        env!("CARGO_PKG_VERSION_MAJOR").parse()
            .expect("Invalid crate major version, must be u16."),
        env!("CARGO_PKG_VERSION_MINOR").parse()
            .expect("Invalid crate minor version, must be u16."),
        env!("CARGO_PKG_VERSION_PATCH").parse()
            .expect("Invalid crate patch version, must be u16."),
    );
    static ref PACKAGE_NAME: &'static str = env!("CARGO_PKG_NAME");
}

pub struct Metaview {
    pub device_store: Rc<RefCell<DeviceStore>>,
    pub event_distributor: EventDistributor,
    pub event_loop: Rc<RefCell<EventLoop<()>>>,
    pub camera: Rc<RefCell<PitchYawCamera3>>,
    pub hmd_poses: Vec<(Rc<RefCell<Vec3>>, Rc<RefCell<Vec3>>)>,
    pub ammolite: Ammolite<MediumData>,
    pub world: World,
    pub dispatcher: Dispatcher<'static, 'static>,
    pub mappcs: Vec<MappContainer>,
}

impl Metaview {
    pub fn new() -> Self {
        // Build Ammolite
        let event_loop = EventLoop::new();
        let device_store = Rc::new(RefCell::new(DeviceStore::new()));
        let event_distributor = EventDistributor::new();
        let primary_monitor = event_loop.primary_monitor();
        let event_loop = Rc::new(RefCell::new(event_loop));
        let camera = Rc::new(RefCell::new(PitchYawCamera3::new()));
        let mut hmd_poses: Vec<(Rc<RefCell<Vec3>>, Rc<RefCell<Vec3>>)> = Vec::new();
        let uwm = UninitializedWindowMedium {
            events_loop: event_loop.clone(),
            window_builder: WindowBuilder::new()
                .with_title("metaview")
                .with_inner_size(
                    PhysicalSize::new(1280.0, 720.0)
                    .to_logical::<f64>(primary_monitor.scale_factor())
                ),
            window_handler: Some(Box::new(|window, data| {
                if let MediumData { specialized: SpecializedMediumData::Window { window: current_window, .. }, .. } = data {
                    *current_window = Some(window.clone());
                }

                window.window().set_cursor_visible(false);
            })),
            data: MediumData::new_window(camera.clone(), device_store.clone(), event_distributor.create_sender(), event_loop.clone()),
        };
        let mut ammolite = Ammolite::<MediumData>::builder(&PACKAGE_NAME, *PACKAGE_VERSION)
            .initialize_openxr()
            .initialize_vulkan()
            /*
             * TODO:
             * `initialize_vulkan` creates the windows already, consider either moving the window
             * creation to this method or to register the windows within `initialize_vulkan`.
             */
            .add_medium_window(uwm)
            .finish_adding_mediums_window()
            .add_medium_stereo_hmd(UninitializedStereoHmdMedium {
                instance_handler: Some(Box::new(|xr_instance, xr_vk_session, data| {
                    if let MediumData { specialized: SpecializedMediumData::Xr {
                        xr_instance: current_xr_instance,
                        xr_vk_session: current_xr_vk_session,
                        ..
                    }, ..} = data {
                        *current_xr_instance = Some(xr_instance.clone());
                        *current_xr_vk_session = Some(xr_vk_session.clone());
                    }
                })),
                data: {
                    let data = MediumData::new_stereo_hmd(camera.clone(), device_store.clone(), event_distributor.create_sender());
                    hmd_poses.push((data.uniform.origin.clone(), data.uniform.forward.clone()));
                    data
                },
            })
            .finish_adding_mediums_stereo_hmd()
            .build();

        let mut world = World::new();
        world.insert(ResourceTimeElapsed::default());
        world.insert(ResourceTimeElapsedDelta::default());
        world.insert(ResourceRenderData::default());

        let mut dispatcher = DispatcherBuilder::new()
            .with(HierarchySystem::<ComponentParent>::new(&mut world), "system_hierarchy", &[])
            .with_barrier()
            .with(SystemTransformInheritance, "system_transform_inheritance", &[])
            .with_thread_local(SystemRender)
            .build();

        dispatcher.setup(&mut world);

        let scene_root = world.create_entity()
            .build();

        world.insert(ResourceSceneRoot(scene_root));

        Self {
            device_store,
            event_distributor,
            event_loop,
            camera,
            hmd_poses,
            ammolite,
            world,
            dispatcher,
            mappcs: Vec::new(),
        }
    }

    pub fn load_mapps<T: AsRef<str>>(&mut self, mapp_paths: impl IntoIterator<Item=T>) {
        // Check arguments
        let mapp_paths: Vec<String> = mapp_paths.into_iter()
            .map(|item| item.as_ref().to_string())
            .collect::<Vec<_>>();

        // Load Mapps
        self.mappcs = mapp_paths.into_iter().map(|mapp_path| {
            let mapp_exports = MappExports::load_file(mapp_path)
                .expect("Could not load the Example MApp.");
            let mapp = Mapp::initialize(mapp_exports);

            MappContainer::from_wasm(mapp, &mut self.world)
        }).collect::<Vec<_>>();

        #[cfg(feature = "native-example-mapp")]
        {
            // mappcs.push({
            //     use example_mapp::{Mapp, ExampleMapp};
            //     let mapp = ExampleMapp::new();
            //     let mapp_interface: Box<dyn MappInterface> = Box::new(mapp);

            //     MappContainer::new(mapp_interface, &mut world)
            // });
            self.mappcs.push({
                use example_mapp_2::{Mapp, ExampleMapp};
                let mapp = ExampleMapp::new();
                let mapp_interface: Box<dyn MappInterface> = Box::new(mapp);

                MappContainer::new(mapp_interface, &mut self.world)
            });
        }

        if self.mappcs.is_empty() {
            eprintln!("At least one metaview application must be specified.");
            return;
        }

        for mappc in &mut self.mappcs {
            mappc.process_io();
            mappc.process_commands(&mut self.ammolite, &mut self.world, &self.camera, true);
        }

        self.event_distributor.distribute_events(&mut self.mappcs[..], &mut self.ammolite, &mut self.world, &self.camera);

        println!("Mapps initialized.");
    }
}
