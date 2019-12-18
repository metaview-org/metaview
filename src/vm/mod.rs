use std::io::Write;
use std::sync::Arc;
use ammolite_math::*;
use ammolite::Ammolite;
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

    pub fn process_commands(&mut self, ammolite: &mut Ammolite<MediumData>, world: &mut World, process_io: bool) {
        while let Some(command) = self.mapp.send_command() {
            if process_io {
                self.process_io();
            }

            match command.kind {
                CommandKind::ModelCreate { .. } => (),
                _ => {
                    dbg!(&command);
                },
            }

            let Command { id, kind } = command;
            let response_kind = match kind {
                CommandKind::ModelCreate { data } => {
                    let model = Arc::new(ammolite.load_model_slice(&data[..]));
                    let model_index = self.models.len();
                    self.models.push(model);

                    CommandResponseKind::ModelCreate {
                        model: Model(model_index),
                    }
                },
                CommandKind::EntityRootGet => {
                    CommandResponseKind::EntityRootGet {
                        // FIXME
                        root_entity: Entity(self.root_entity.id() as usize),
                    }
                },
                CommandKind::EntityCreate => {
                    let entity = world.create_entity()
                        .build();

                    CommandResponseKind::EntityCreate {
                        entity: Entity(entity.id() as usize),
                    }
                },
                CommandKind::EntityParentSet { entity, parent_entity } => {
                    let entities = world.fetch::<EntitiesRes>();
                    // FIXME
                    let entity = entities.entity(entity.0 as u32);
                    let parent_entity = parent_entity.map(|parent_entity| entities.entity(parent_entity.0 as u32));
                    let mut storage = world.write_storage::<ComponentParent>();
                    let previous_component = parent_entity.and_then(|parent_entity| {
                        storage.insert(entity, ComponentParent {
                            entity: parent_entity,
                        }).expect("An error occurred while inserting a component into storage.")
                    });
                    let previous_value = previous_component.map(|component| {
                        Entity(component.entity.id() as usize)
                    });

                    CommandResponseKind::EntityParentSet {
                        previous_parent_entity: previous_value,
                    }
                },
                CommandKind::EntityModelSet { entity, model } => {
                    let entities = world.fetch::<EntitiesRes>();
                    // FIXME
                    let entity = entities.entity(entity.0 as u32);
                    let model = model.map(|model| self.models[model.0].clone());

                    let mut storage = world.write_storage::<ComponentModel>();
                    let previous_component = model.and_then(|model| {
                        storage.insert(entity, ComponentModel {
                            model,
                        }).expect("An error occurred while inserting a component into storage.")
                    });
                    let previous_value = previous_component.and_then(|component| {
                        let mut index_found = None;

                        for (index, model) in self.models.iter().enumerate() {
                            if Arc::ptr_eq(model, &component.model) {
                                index_found = Some(index);
                                break;
                            }
                        }

                        index_found.map(|index_found| Model(index_found))
                    });

                    CommandResponseKind::EntityModelSet {
                        previous_model: previous_value,
                    }
                },
                CommandKind::EntityTransformSet { entity, transform } => {
                    let entities = world.fetch::<EntitiesRes>();
                    // FIXME
                    let entity = entities.entity(entity.0 as u32);
                    let mut storage = world.write_storage::<ComponentTransformRelative>();
                    let previous_component = transform.and_then(|transform| {
                        storage.insert(entity, ComponentTransformRelative {
                            matrix: transform,
                        }).expect("An error occurred while inserting a component into storage.")
                    });
                    let previous_value = previous_component.map(|component| {
                        component.matrix
                    });

                    CommandResponseKind::EntityTransformSet {
                        previous_transform: previous_value,
                    }
                },
            };

            self.mapp.receive_command_response(CommandResponse {
                command_id: id,
                kind: response_kind,
            });

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
