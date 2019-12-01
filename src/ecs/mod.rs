use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use std::sync::Arc;
use ammolite_math::Mat4;
use ammolite::model::Model;
use ammolite::WorldSpaceModel;
use ammolite::camera::Camera;
use specs::join;
use specs::prelude::*;
use specs_hierarchy::{Hierarchy, HierarchySystem};

pub struct ComponentParent {
    pub entity: Entity,
}

impl Component for ComponentParent {
    type Storage = FlaggedStorage<Self, DenseVecStorage<Self>>;
}

impl specs_hierarchy::Parent for ComponentParent {
    fn parent_entity(&self) -> Entity {
        self.entity
    }
}

/// Specifies an affine transformation of an object
pub struct ComponentTransformRelative {
    pub matrix: Mat4,
}

impl Component for ComponentTransformRelative {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

#[derive(Default)]
pub struct ComponentTransformAbsolute {
    pub matrix: Mat4,
}

impl Component for ComponentTransformAbsolute {
    type Storage = VecStorage<Self>;
}

pub struct ComponentModel {
    pub model: Arc<Model>,
}

impl Component for ComponentModel {
    type Storage = VecStorage<Self>;
}

pub struct ResourceSceneRoot(pub Entity);

#[derive(Default)]
pub struct ResourceTimeElapsed(pub Duration);

#[derive(Default)]
pub struct ResourceTimeElapsedDelta(pub Duration);

#[derive(Default)]
pub struct ResourceRenderData {
    pub world_space_models: Vec<(Mat4, Arc<Model>)>,
}

pub struct SystemTransformInheritance;

impl<'a> System<'a> for SystemTransformInheritance {
    type SystemData = (
        ReadExpect<'a, Hierarchy<ComponentParent>>,
        ReadExpect<'a, ResourceSceneRoot>,
        ReadStorage<'a, ComponentParent>,
        ReadStorage<'a, ComponentTransformRelative>,
        WriteStorage<'a, ComponentTransformAbsolute>,
    );

    fn run(&mut self, (hierarchy, scene_root, parent, transform_rel, mut transform_abs): Self::SystemData) {
        // Note: This does not include entities that are parents.
        for entity in hierarchy.all_children_iter(scene_root.0) {
            // FIXME: Only update when rel or abs were updated
            if let Some(transform_rel) = transform_rel.get(entity) {
                let mut matrix_abs_new = transform_rel.matrix.clone();

                if let Some(parent) = parent.get(entity) {
                    if let Some(parent_transform_abs) = transform_abs.get(parent.entity) {
                        matrix_abs_new = matrix_abs_new * &parent_transform_abs.matrix;
                    }
                }

                transform_abs.insert(entity, ComponentTransformAbsolute {
                    matrix: matrix_abs_new,
                }).unwrap();
            }
        }
    }
}

pub struct SystemRender;

impl<'a> System<'a> for SystemRender {
    type SystemData = (
        Write<'a, ResourceRenderData>,
        ReadStorage<'a, ComponentTransformAbsolute>,
        ReadStorage<'a, ComponentModel>,
    );

    fn run(&mut self, (mut render_data, transform, model): Self::SystemData) {
        render_data.world_space_models.clear();

        for (transform, model) in (&transform, &model).join() {
            render_data.world_space_models.push((transform.matrix.clone(), model.model.clone()));
        }
    }
}
