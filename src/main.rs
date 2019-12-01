//! TODO:
//! * Figure out how to represent cameras/views in the scene graph and how to
//!   render geometry relative to the cameras/views. Ideas:
//!   - Make HMDs proper entities of the scene graph
//!   - Make HMDs' orientations available via a resource

#![feature(type_alias_enum_variants)]

use std::rc::Rc;
use std::cell::RefCell;
use std::time::{Duration, Instant};
use std::num::NonZeroU32;
use std::sync::Arc;
use vulkano as vk;
use openxr as xr;
use winit::{ElementState, MouseButton, Event, DeviceEvent, WindowEvent, KeyboardInput, VirtualKeyCode, EventsLoop, WindowBuilder, Window};
use winit::dpi::PhysicalSize;
use ammolite::{Ammolite, WorldSpaceModel, UninitializedWindowMedium, UninitializedStereoHmdMedium};
use ammolite_math::*;
use ammolite::camera::{Camera, PitchYawCamera3};
use lazy_static::lazy_static;
use specs::prelude::*;
use specs_hierarchy::{Hierarchy, HierarchySystem};
use crate::medium::{MediumData, SpecializedMediumData, UniformMediumData};
use crate::ecs::*;

pub mod medium;
pub mod ecs;

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

fn construct_model_matrix(scale: f32, translation: &Vec3, rotation: &Vec3) -> Mat4 {
    Mat4::translation(translation)
        * Mat4::rotation_roll(rotation[2])
        * Mat4::rotation_yaw(rotation[1])
        * Mat4::rotation_pitch(rotation[0])
        * Mat4::scale(scale)
}

fn main() {
    // let polygon = [
    //     Vec3([ 1.0,  1.0, 1.0]),
    //     Vec3([-1.0,  1.0, 2.0]),
    //     Vec3([-1.0, -1.0, 3.0]),
    //     Vec3([ 1.0, -1.0, 2.0]),
    // ];
    // let ray = ammolite::Ray {
    //     origin: Vec3([ 1.0, -1.0,  0.0]),
    //     direction: Vec3([0.0, 0.0, 1.0]),
    // };
    // let intersection = ammolite::intersect_convex_polygon(&polygon[..], &ray.into());
    // dbg!(intersection);
    // return;

    // Check arguments
    let model_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("No model path provided.");
        std::process::exit(1);
    });

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

    // Load resources
    let model = Arc::new(ammolite.load_model(model_path));
    let sphere = Arc::new(ammolite.load_model("../ammolite/resources/sphere_1m_radius.glb"));

    // World
    let mut world = World::new();
    // world.register::<ComponentParent>();
    // world.register::<ComponentTransformRelative>();
    // world.register::<ComponentTransformAbsolute>();
    // world.register::<ComponentModel>();
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

    let scene_child = world.create_entity()
        .with(ComponentParent {
            entity: scene_root,
        })
        .with(ComponentTransformRelative {
            matrix: construct_model_matrix(
                1.0,
                &[0.0, 0.0, 2.0].into(),
                &[0.0, 0.0, 0.0].into(),
            ),
        })
        .with(ComponentModel {
            model: model.clone(),
        })
        .build();

    let mut previous_child = scene_child;

    for _ in 0..4 {
        previous_child = world.create_entity()
            .with(ComponentParent {
                entity: previous_child,
            })
            .with(ComponentTransformRelative {
                matrix: construct_model_matrix(
                    0.75,
                    &[3.0, 0.0, 0.0].into(),
                    &[0.0, 0.3, 0.0].into(),
                ),
            })
            .with(ComponentModel {
                model: model.clone(),
            })
            .build();
    }

    // Event loop
    let init_instant = Instant::now();
    let mut previous_frame_instant = init_instant.clone();

    loop {
        let now = Instant::now();
        let elapsed = now.duration_since(init_instant);
        let delta_time = now.duration_since(previous_frame_instant);
        *world.write_resource::<ResourceTimeElapsed>() = ResourceTimeElapsed(elapsed);
        *world.write_resource::<ResourceTimeElapsedDelta>() = ResourceTimeElapsedDelta(delta_time);
        previous_frame_instant = now;
        let secs_elapsed = ((elapsed.as_secs() as f64) + (elapsed.as_nanos() as f64) / (1_000_000_000f64)) as f32;
        if ammolite.handle_events(&delta_time) {
            break;
        }

        let model_matrices = [
            construct_model_matrix(
                1.0,
                &[0.0, 0.0, 2.0].into(),
                &[secs_elapsed.sin() * 0.0 * 1.0, std::f32::consts::PI + secs_elapsed.cos() * 0.0 * 3.0 / 2.0, 0.0].into(),
            ),
            construct_model_matrix(
                1.0,
                &[0.0, 1.0, 2.0].into(),
                &[secs_elapsed.sin() * 0.0 * 1.0, std::f32::consts::PI + secs_elapsed.cos() * 0.0 * 3.0 / 2.0, 0.0].into(),
            ),
        ];

        let mut world_space_models = [
            WorldSpaceModel {
                model: &sphere,
                matrix: Mat4::zero(),
            },
            WorldSpaceModel { model: &model, matrix: model_matrices[0].clone() },
            // WorldSpaceModel { model: &model, matrix: model_matrices[1].clone() },
        ];

        dbg!(&hmd_poses);

        let (avg_origin, avg_forward) = if hmd_poses.is_empty() {
            (camera.borrow().get_position(), camera.borrow().get_direction())
        } else {
            let mut avg_origin: Vec3 = Vec3::zero();
            let mut avg_forward: Vec3 = Vec3::zero();

            for hmd_pose in hmd_poses.iter() {
                avg_origin = avg_origin + &*hmd_pose.0.borrow();
                avg_forward = avg_forward + &*hmd_pose.1.borrow();
            }

            (
                avg_origin / hmd_poses.len() as f32,
                avg_forward / hmd_poses.len() as f32,
            )
        };

        let ray = ammolite::Ray {
            origin: avg_origin,
            direction: avg_forward,
        };
        let intersection = ammolite::raytrace_distance(&world_space_models[1], &ray);
        let intersection_point = intersection.as_ref().map(|intersection| ray.origin + ray.direction * intersection.distance).unwrap_or_else(|| Vec3::zero());
        world_space_models[0].matrix = construct_model_matrix(
            0.02 * camera.borrow().get_position().distance_to(&intersection_point),
            &intersection_point,
            &[0.0, 0.0, 0.0].into(),
        );

        dispatcher.dispatch(&mut world);

        {
            let render_data = world.fetch::<ResourceRenderData>();
            let mut world_space_models: Vec<WorldSpaceModel> = Vec::with_capacity(render_data.world_space_models.len());

            for (matrix, model) in &render_data.world_space_models {
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
