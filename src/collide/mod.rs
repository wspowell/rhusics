//! Generic data structures and algorithms for collision detection

pub use collision::{CollisionStrategy, Contact};
pub use collision::prelude::Primitive;

pub mod narrow;
pub mod ecs;
pub mod broad;

use std::fmt::Debug;

use cgmath::prelude::*;
use collision::algorithm::broad_phase::HasBound;
use collision::dbvt::TreeValue;
use collision::prelude::*;

use {Pose, Real};

/// Contains all the contacts found between two bodies in a single pass.
///
/// # Type parameters
///
/// - `ID`: The ID type of the body. This is supplied by the user of the library. In the ECS case,
///         this will be [`Entity`](https://docs.rs/specs/0.9.5/specs/struct.Entity.html).
/// - `V`: cgmath vector type
#[derive(Debug, Clone)]
pub struct ContactEvent<ID, P>
where
    P: EuclideanSpace,
    P::Diff: Debug,
{
    /// The ids of the two colliding bodies
    pub bodies: (ID, ID),

    /// The contact between the colliding bodies
    pub contact: Contact<P>,
}

impl<ID, P> ContactEvent<ID, P>
where
    ID: Clone + Debug,
    P: EuclideanSpace,
    P::Diff: VectorSpace + Zero + Debug,
{
    /// Create a new contact set
    pub fn new(bodies: (ID, ID), contact: Contact<P>) -> Self {
        Self { bodies, contact }
    }

    /// Convenience function to create a contact set with a single [`Contact`](struct.Contact.html).
    pub fn new_single(strategy: CollisionStrategy, bodies: (ID, ID)) -> Self {
        Self::new(bodies, Contact::new(strategy))
    }
}

/// Collision shape describing a complete collision object in the collision world.
///
/// Can handle both convex shapes, and concave shapes, by subdividing the concave shapes into
/// multiple convex shapes. This task is up to the user of the library to perform, no subdivision is
/// done automatically in the library.
///
/// Contains cached information about the base bounding box containing all primitives,
/// in model space coordinates. Also contains a cached version of the transformed bounding box,
/// in world space coordinates.
///
/// Also have details about what collision strategy to use for contact resolution with this shape.
#[derive(Debug, Clone)]
pub struct CollisionShape<P, T>
where
    P: Primitive,
{
    /// Enable/Disable collision detection for this shape
    pub enabled: bool,
    base_bound: P::Aabb,
    transformed_bound: P::Aabb,
    primitives: Vec<(P, T)>,
    strategy: CollisionStrategy,
}

impl<P, T> CollisionShape<P, T>
where
    P: Primitive,
    P::Aabb: Aabb<Scalar = Real>,
    T: Pose<P::Point>,
{
    /// Create a new collision shape, with multiple collision primitives.
    ///
    /// Will compute and cache the base bounding box that contains all the given primitives,
    /// in model space coordinates.
    ///
    /// # Parameters
    ///
    /// - `strategy`: The collision strategy to use for this shape.
    /// - `primitives`: List of all primitives that make up this shape.
    pub fn new_complex(strategy: CollisionStrategy, primitives: Vec<(P, T)>) -> Self {
        let bound = get_bound(&primitives);
        Self {
            base_bound: bound.clone(),
            primitives,
            enabled: true,
            transformed_bound: bound,
            strategy,
        }
    }

    /// Convenience function to create a simple collision shape with only a single given primitive,
    /// with no local-to-model transform.
    ///
    /// # Parameters
    ///
    /// - `strategy`: The collision strategy to use for this shape.
    /// - `primitive`: The collision primitive.
    pub fn new_simple(strategy: CollisionStrategy, primitive: P) -> Self {
        Self::new_complex(strategy, vec![(primitive, T::one())])
    }

    /// Convenience function to create a simple collision shape with only a single given primitive,
    /// with a given local-to-model transform.
    ///
    /// # Parameters
    ///
    /// - `strategy`: The collision strategy to use for this shape.
    /// - `primitive`: The collision primitive.
    /// - `transform`: Local-to-model transform of the primitive.
    pub fn new_simple_offset(strategy: CollisionStrategy, primitive: P, transform: T) -> Self {
        Self::new_complex(strategy, vec![(primitive, transform)])
    }

    /// Update the cached transformed bounding box in world space coordinates.
    ///
    /// # Parameters
    ///
    /// - `transform`: Current model-to-world transform of the shape.
    pub fn update(&mut self, transform: &T) {
        self.transformed_bound = self.base_bound.transform(transform);
    }

    /// Return the current transformed bound for the shape
    ///
    pub fn bound(&self) -> &P::Aabb {
        &self.transformed_bound
    }
}

/// Shape wrapper for use with containers
#[derive(Debug, Clone)]
pub struct ContainerShapeWrapper<ID, P>
where
    P: Primitive,
    P::Aabb: Aabb<Scalar = Real>,
    <P::Point as EuclideanSpace>::Diff: Debug,
{
    /// The id
    pub id: ID,

    /// The bounding volume
    pub bound: P::Aabb,
    fat_factor: <P::Point as EuclideanSpace>::Diff,
}

impl<ID, P> ContainerShapeWrapper<ID, P>
where
    P: Primitive,
    P::Aabb: Clone + Aabb<Scalar = Real>,
    <P::Point as EuclideanSpace>::Diff: Debug,
{
    /// Create a new shape
    pub fn new_impl(
        id: ID,
        bound: &P::Aabb,
        fat_factor: <P::Point as EuclideanSpace>::Diff,
    ) -> Self {
        Self {
            id,
            bound: bound.clone(),
            fat_factor,
        }
    }

    /// Create a new shape
    pub fn new(id: ID, bound: &P::Aabb) -> Self {
        Self::new_impl(
            id,
            bound,
            <P::Point as EuclideanSpace>::Diff::from_value(Real::one()),
        )
    }
}

impl<ID, P> TreeValue for ContainerShapeWrapper<ID, P>
where
    ID: Clone + Debug,
    P: Primitive,
    P::Aabb: Aabb<Scalar = Real>,
    P::Point: Debug,
    <P::Point as EuclideanSpace>::Diff: Debug,
{
    type Bound = P::Aabb;

    fn bound(&self) -> &Self::Bound {
        &self.bound
    }

    fn fat_bound(&self) -> Self::Bound {
        self.bound.add_margin(self.fat_factor)
    }
}

impl<ID, P> HasBound for ContainerShapeWrapper<ID, P>
where
    P: Primitive,
    P::Aabb: Aabb<Scalar = Real>,
    <P::Point as EuclideanSpace>::Diff: Debug,
{
    type Bound = P::Aabb;

    fn get_bound(&self) -> &P::Aabb {
        &self.bound
    }
}

fn get_bound<P, T>(primitives: &Vec<(P, T)>) -> P::Aabb
where
    P: Primitive,
    P::Aabb: Aabb<Scalar = Real>,
    T: Pose<P::Point>,
{
    primitives
        .iter()
        .map(|&(ref p, ref t)| p.get_bound().transform(t))
        .fold(P::Aabb::zero(), |bound, b| bound.union(&b))
}
