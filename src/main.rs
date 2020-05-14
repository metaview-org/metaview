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
use std::time::{Instant, Duration};
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
use metaview_lib::*;

fn main() {
    let mapp_paths = std::env::args().skip(1);
    let mut metaview = Metaview::new();

    // let bench_start = Instant::now();
    metaview.load_mapps(mapp_paths);
    // println!("Duration: {:?}", bench_start.elapsed());
    // return;

    // Event loop
    let init_instant = Instant::now();
    let mut previous_frame_instant = init_instant.clone();

    // println!("Rendering loop entered.");
    // let measurement_count_max = 1100;
    // let mut measurements = Vec::with_capacity(measurement_count_max);

    loop {
        // measurements.push(Instant::now());

        // if measurements.len() >= measurement_count_max {
        //     println!("Frame time measurements:");
        //     for measurement in measurements {
        //         println!("{}", measurement.duration_since(init_instant).as_secs_f64());
        //     }
        //     return;
        // }

        // println!("Frame.");

        let now = Instant::now();
        let elapsed = now.duration_since(init_instant);
        let delta_time = now.duration_since(previous_frame_instant);
        *metaview.world.write_resource::<ResourceTimeElapsed>() = ResourceTimeElapsed(elapsed);
        *metaview.world.write_resource::<ResourceTimeElapsedDelta>() = ResourceTimeElapsedDelta(delta_time);
        previous_frame_instant = now;

        if metaview.ammolite.handle_events(&delta_time) {
            break;
        }

        for mappc in &mut metaview.mappcs {
            mappc.mapp.update(elapsed);
            mappc.process_io();
            mappc.process_commands(&mut metaview.ammolite, &mut metaview.world, &metaview.camera, true);
        }

        metaview.event_distributor.distribute_events(&mut metaview.mappcs[..], &mut metaview.ammolite, &mut metaview.world, &metaview.camera);

        metaview.dispatcher.dispatch(&mut metaview.world);

        {
            let render_data = metaview.world.fetch::<ResourceRenderData>();
            let mut world_space_models: Vec<WorldSpaceModel> = Vec::with_capacity(render_data.world_space_models.len());

            for (_id, matrix, model) in &render_data.world_space_models {
                world_space_models.push(WorldSpaceModel {
                    matrix: matrix.clone(),
                    model: &model,
                })
            }

            metaview.ammolite.render(&elapsed, || &world_space_models[..]);
        };

        metaview.world.maintain();
    }
}
