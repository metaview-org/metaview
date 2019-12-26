//! TODO:
//! * Convert command message passing using the exports to exports/imports
//!   (see wasmtime-api and wasmtime-interface-types)
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
use winit::{EventsLoop, WindowBuilder};
use std::time::Instant;
use winit::dpi::PhysicalSize;
use ammolite::{Ammolite, WorldSpaceModel, UninitializedWindowMedium, UninitializedStereoHmdMedium};
use ammolite_math::*;
use ammolite::camera::PitchYawCamera3;
use lazy_static::lazy_static;
use specs::prelude::*;
use specs_hierarchy::HierarchySystem;
use crate::medium::{MediumData, SpecializedMediumData};
use crate::ecs::*;
use crate::vm::{Mapp, MappExports, MappContainer};

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
    let mapp_path = std::env::args().nth(1)
        .expect("Path to a Metaview application not provided.");

    // Build Ammolite
    let events_loop = EventsLoop::new();
    let primary_monitor = events_loop.get_primary_monitor();
    let events_loop = Rc::new(RefCell::new(events_loop));
    let camera = Rc::new(RefCell::new(PitchYawCamera3::new()));
    let mut hmd_poses: Vec<(Rc<RefCell<Vec3>>, Rc<RefCell<Vec3>>)> = Vec::new();
    let uwm = UninitializedWindowMedium {
        events_loop: events_loop.clone(),
        window_builder: WindowBuilder::new()
            .with_title("metaview")
            .with_dimensions(
                PhysicalSize::new(1280.0, 720.0)
                .to_logical(primary_monitor.get_hidpi_factor())
            ),
        window_handler: Some(Box::new(|window, data| {
            if let MediumData { specialized: SpecializedMediumData::Window { window: current_window, .. }, .. } = data {
                *current_window = Some(window.clone());
            }

            window.window().hide_cursor(true);
        })),
        data: MediumData::new_window(camera.clone(), events_loop),
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
                let data = MediumData::new_stereo_hmd(camera.clone());
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

    // Load Mapp
    let mapp_exports = MappExports::load_file(mapp_path)
        .expect("Could not load the Example MApp.");
    let mapp = Mapp::initialize(mapp_exports);
    let mut mappc = MappContainer::new(mapp, &mut world);
    // println!("{:?}", mapp.test("1".to_string()));
    // println!("{:?}", mapp.test("2".to_string()));
    // println!("{:?}", mapp.test("3".to_string()));

    mappc.process_io();
    mappc.process_commands(&mut ammolite, &mut world, &camera, true);

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

        mappc.mapp.update(elapsed);
        mappc.process_io();
        mappc.process_commands(&mut ammolite, &mut world, &camera, true);

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
