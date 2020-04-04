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

fn main() {
    // Check arguments
    let mapp_paths = std::env::args().skip(1)
        .collect::<Vec<_>>();

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
        data: MediumData::new_window(camera.clone(), device_store.clone(), event_distributor.create_sender(), event_loop),
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

    // World
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

    // Load Mapps
    let mut mappcs = mapp_paths.into_iter().map(|mapp_path| {
        let mapp_exports = MappExports::load_file(mapp_path)
            .expect("Could not load the Example MApp.");
        let mapp = Mapp::initialize(mapp_exports);

        MappContainer::from_wasm(mapp, &mut world)
    }).collect::<Vec<_>>();

    #[cfg(feature = "native-example-mapp")]
    mappcs.push({
        use example_mapp::{Mapp, ExampleMapp};
        let mapp = ExampleMapp::new();
        let mapp_interface: Box<dyn MappInterface> = Box::new(mapp);

        MappContainer::new(mapp_interface, &mut world)
    });

    if mappcs.is_empty() {
        eprintln!("At least one metaview application must be specified.");
        return;
    }

    for mappc in &mut mappcs {
        mappc.process_io();
        mappc.process_commands(&mut ammolite, &mut world, &camera, true);
    }

    event_distributor.distribute_events(&mut mappcs[..], &mut ammolite, &mut world, &camera);

    // Event loop
    let init_instant = Instant::now();
    let mut previous_frame_instant = init_instant.clone();

    println!("Rendering loop entered.");

    loop {
        // println!("Frame.");

        let now = Instant::now();
        let elapsed = now.duration_since(init_instant);
        let delta_time = now.duration_since(previous_frame_instant);
        *world.write_resource::<ResourceTimeElapsed>() = ResourceTimeElapsed(elapsed);
        *world.write_resource::<ResourceTimeElapsedDelta>() = ResourceTimeElapsedDelta(delta_time);
        previous_frame_instant = now;

        if ammolite.handle_events(&delta_time) {
            break;
        }

        for mappc in &mut mappcs {
            mappc.mapp.update(elapsed);
            mappc.process_io();
            mappc.process_commands(&mut ammolite, &mut world, &camera, true);
        }

        event_distributor.distribute_events(&mut mappcs[..], &mut ammolite, &mut world, &camera);

        dispatcher.dispatch(&mut world);

        {
            let render_data = world.fetch::<ResourceRenderData>();
            let mut world_space_models: Vec<WorldSpaceModel> = Vec::with_capacity(render_data.world_space_models.len());

            for (_id, matrix, model) in &render_data.world_space_models {
                world_space_models.push(WorldSpaceModel {
                    matrix: matrix.clone(),
                    model: &model,
                })
            }

            ammolite.render(&elapsed, || &world_space_models[..]);
        };

        world.maintain();
    }
}
