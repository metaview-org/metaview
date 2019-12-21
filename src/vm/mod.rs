use std::rc::Rc;
use std::cell::RefCell;
use std::io::Write;
use std::sync::Arc;
use ammolite_math::*;
use ammolite::{Ammolite, Ray, WorldSpaceModel};
use ammolite::camera::{Camera, PitchYawCamera3};
use specs::{World, WorldExt, world::{Builder, EntitiesRes}};
use serde::{Deserialize, Serialize};
use json5::{from_str, to_string};
use mlib::*;
use crate::ecs::*;
use crate::medium::MediumData;

#[mapp(host)]
pub struct Mapp;

pub struct MappContainer {
    pub mapp: Mapp,
    pub models: Vec<Arc<ammolite::model::Model>>,
    pub root_entity: specs::Entity,
}

impl MappContainer {
    pub fn new(mapp: Mapp, world: &mut World) -> Self {
        let resource_scene_root = world.fetch::<ResourceSceneRoot>().0;
        let root_entity = world.create_entity()
            .with(ComponentParent {
                entity: resource_scene_root,
            })
            .with(ComponentTransformRelative {
                matrix: Mat4::identity(),
            })
            .build();
        Self {
            mapp,
            models: Vec::new(),
            root_entity,
        }
    }

    pub fn process_commands(&mut self, ammolite: &mut Ammolite<MediumData>, world: &mut World, camera: &Rc<RefCell<PitchYawCamera3>>, process_io: bool) {
        while let Some(command) = self.mapp.send_command() {
            if process_io {
                self.process_io();
            }

            // match command.kind {
            //     CommandKind::ModelCreate { .. } => (),
            //     _ => {
            //         dbg!(&command);
            //     },
            // }

            let Command { id, kind } = command;
            let response_kind = match kind {
                CommandKind::ModelCreate { data } => {
                    let model = Arc::new(ammolite.load_model_slice(&data[..]));
                    let model_index = self.models.len();
                    self.models.push(model);

                    Some(CommandResponseKind::ModelCreate {
                        model: Model(model_index),
                    })
                },
                CommandKind::EntityRootGet => {
                    Some(CommandResponseKind::EntityRootGet {
                        // FIXME
                        root_entity: Entity(self.root_entity.id() as usize),
                    })
                },
                CommandKind::EntityCreate => {
                    let entity = world.create_entity()
                        .build();

                    Some(CommandResponseKind::EntityCreate {
                        entity: Entity(entity.id() as usize),
                    })
                },
                CommandKind::EntityParentSet { entity, parent_entity } => {
                    let entities = world.fetch::<EntitiesRes>();
                    // FIXME
                    let entity = entities.entity(entity.0 as u32);
                    let parent_entity = parent_entity.map(|parent_entity| entities.entity(parent_entity.0 as u32));
                    let mut storage = world.write_storage::<ComponentParent>();
                    let previous_component = if let Some(parent_entity) = parent_entity {
                        storage.insert(entity, ComponentParent {
                            entity: parent_entity,
                        }).expect("An error occurred while inserting a component into storage.")
                    } else {
                        storage.remove(entity)
                    };
                    let previous_value = previous_component.map(|component| {
                        Entity(component.entity.id() as usize)
                    });

                    Some(CommandResponseKind::EntityParentSet {
                        previous_parent_entity: previous_value,
                    })
                },
                CommandKind::EntityModelSet { entity, model } => {
                    dbg!(&entity);
                    dbg!(&model);
                    let entities = world.fetch::<EntitiesRes>();
                    // FIXME
                    let entity = entities.entity(entity.0 as u32);
                    let model = model.map(|model| self.models[model.0].clone());

                    let mut storage = world.write_storage::<ComponentModel>();
                    let previous_component = if let Some(model) = model {
                        storage.insert(entity, ComponentModel {
                            model,
                        }).expect("An error occurred while inserting a component into storage.")
                    } else {
                        storage.remove(entity)
                    };
                    let previous_value = previous_component.and_then(|component| {
                        let mut index_found = None;

                        // FIXME use something better than an O(n) search
                        for (index, model) in self.models.iter().enumerate() {
                            if Arc::ptr_eq(model, &component.model) {
                                index_found = Some(index);
                                break;
                            }
                        }

                        index_found.map(|index_found| Model(index_found))
                    });

                    Some(CommandResponseKind::EntityModelSet {
                        previous_model: previous_value,
                    })
                },
                CommandKind::EntityTransformSet { entity, transform } => {
                    let entities = world.fetch::<EntitiesRes>();
                    // FIXME
                    let entity = entities.entity(entity.0 as u32);
                    let mut storage = world.write_storage::<ComponentTransformRelative>();
                    let previous_component = if let Some(transform) = transform {
                        storage.insert(entity, ComponentTransformRelative {
                            matrix: transform,
                        }).expect("An error occurred while inserting a component into storage.")
                    } else {
                        storage.remove(entity)
                    };
                    let previous_value = previous_component.map(|component| {
                        component.matrix
                    });

                    Some(CommandResponseKind::EntityTransformSet {
                        previous_transform: previous_value,
                    })
                },
                CommandKind::GetViewOrientation {} => {
                    let views_per_medium = ammolite.views().map(|views|
                        views.map(|views|
                            views.iter().map(|view| {
                                mlib::View {
                                    pose: {
                                        (view.pose.orientation.clone().to_homogeneous()
                                            * Mat4::translation((&view.pose.position).into())
                                            * camera.borrow().get_view_matrix()).inverse()
                                    },
                                    fov: mlib::ViewFov {
                                        angle_left: view.fov.angle_left,
                                        angle_right: view.fov.angle_right,
                                        angle_up: view.fov.angle_up,
                                        angle_down: view.fov.angle_down,
                                    },
                                }
                            }).collect::<Vec<_>>()
                        )
                    ).collect::<Vec<_>>();

                    Some(CommandResponseKind::GetViewOrientation {
                        views_per_medium,
                    })
                },
                CommandKind::RayTrace { origin, direction } => {
                    dbg!(&origin);
                    dbg!(&direction);
                    // unreachable!();
                    let render_data = world.fetch::<ResourceRenderData>();
                    let mut world_space_models: Vec<(Entity, WorldSpaceModel)> = Vec::with_capacity(render_data.world_space_models.len());

                    for (id, matrix, model) in &render_data.world_space_models {
                        world_space_models.push((Entity(*id as usize), WorldSpaceModel {
                            matrix: matrix.clone(),
                            model: &model,
                        }))
                    }

                    let ray = Ray { origin, direction };
                    let mut closest_intersection: Option<Intersection> = None;

                    for (entity, world_space_model) in &world_space_models {
                        let ray_intersection = ammolite::raytrace_distance(world_space_model, &ray);

                        if let Some(ray_intersection) = ray_intersection {
                            if closest_intersection.is_none() || (ray_intersection.distance < closest_intersection.as_ref().unwrap().distance_from_origin) {
                                closest_intersection = Some(Intersection {
                                    distance_from_origin: ray_intersection.distance,
                                    position: &ray.origin + (&ray.direction * ray_intersection.distance),
                                    entity: *entity,
                                });
                            }
                        }
                    }

                    Some(CommandResponseKind::RayTrace {
                        closest_intersection
                    })
                }
            };

            if let Some(response_kind) = response_kind {
                self.mapp.receive_command_response(CommandResponse {
                    command_id: id,
                    kind: response_kind,
                });
            }

            if process_io {
                self.process_io();
            }
        }

        if process_io {
            self.process_io();
        }
    }

    pub fn process_io(&mut self) {
        let IO { out, err } = self.mapp.flush_io();

        {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();

            handle.write_all(&out[..])
                .expect("Could not redirect the module's stdout to the host stdout.");
        }

        {
            let stderr = std::io::stderr();
            let mut handle = stderr.lock();

            handle.write_all(&err[..])
                .expect("Could not redirect the module's stderr to the host stderr.");
        }
    }
}

pub fn example() {
    let mut mapp_exports = MappExports::load_file("../example-mapp/pkg/example_mapp.wasm")
        .expect("Could not load the Example MApp.");
    let mut mapp = Mapp::initialize(mapp_exports);
    // println!("{:?}", mapp.test("1".to_string()));
    // println!("{:?}", mapp.test("2".to_string()));
    // println!("{:?}", mapp.test("3".to_string()));
    // println!("{:#?}", mapp.get_model_matrices(3.14));
}
