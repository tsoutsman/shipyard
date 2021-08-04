//! Iterators types and traits.

mod abstract_mut;
mod into_abstract;
mod into_iter;
#[allow(clippy::module_inception)]
mod iter;
mod mixed;
#[cfg(feature = "parallel")]
mod par_iter;
#[cfg(feature = "parallel")]
mod par_mixed;
#[cfg(feature = "parallel")]
mod par_tight;
mod tight;
mod with_id;

pub use into_iter::IntoIter;
pub use iter::Iter;
pub use mixed::Mixed;
#[cfg(feature = "parallel")]
pub use par_iter::ParIter;
#[cfg(feature = "parallel")]
pub use par_mixed::ParMixed;
#[cfg(feature = "parallel")]
pub use par_tight::ParTight;
pub use tight::Tight;
pub use with_id::{IntoWithId, LastId, WithId};

use crate::{track, Component, EntityId, Mut, SparseSet, View, ViewMut};

#[allow(missing_docs)]
pub trait LendingIterator {
    type Item<'a>
    where
        Self: 'a;

    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

#[allow(missing_docs)]
pub trait GetData {
    type Out<'a>;

    fn id_at(&self, index: usize) -> Option<EntityId>;
    fn get_data(&mut self, entity: EntityId) -> Option<Self::Out<'_>>;
}

impl<T: Component> GetData for View<'_, T> {
    type Out<'a> = &'a T;

    fn id_at(&self, index: usize) -> Option<EntityId> {
        self.sparse_set.id_at(index)
    }
    fn get_data(&mut self, entity: EntityId) -> Option<Self::Out<'_>> {
        self.sparse_set.private_get(entity)
    }
}

impl<T: Component<Tracking = track::Untracked>> GetData for ViewMut<'_, T, track::Untracked> {
    type Out<'a> = &'a mut T;

    fn id_at(&self, index: usize) -> Option<EntityId> {
        self.sparse_set.id_at(index)
    }
    fn get_data(&mut self, entity: EntityId) -> Option<Self::Out<'_>> {
        let index = self.index_of(entity)?;

        Some(unsafe { self.data.get_unchecked_mut(index) })
    }
}

impl<T: Component<Tracking = track::Modification>> GetData for ViewMut<'_, T, track::Modification> {
    type Out<'a> = Mut<'a, T>;

    fn id_at(&self, index: usize) -> Option<EntityId> {
        self.sparse_set.id_at(index)
    }
    fn get_data(&mut self, entity: EntityId) -> Option<Self::Out<'_>> {
        let index = self.index_of(entity)?;

        let SparseSet { dense, data, .. } = &mut self.sparse_set;

        let entity = unsafe { dense.get_unchecked_mut(index) };

        Some(Mut {
            flag: Some(entity),
            data: unsafe { data.get_unchecked_mut(index) },
        })
    }
}

impl<T: GetData, U: GetData> GetData for (T, U) {
    type Out<'a> = (T::Out<'a>, U::Out<'a>);

    fn id_at(&self, index: usize) -> Option<EntityId> {
        self.0.id_at(index)
    }

    fn get_data(&mut self, entity: EntityId) -> Option<Self::Out<'_>> {
        Some((self.0.get_data(entity)?, self.1.get_data(entity)?))
    }
}

#[allow(missing_docs)]
pub struct LendingIter<T: GetData> {
    pub(crate) index: usize,
    pub(crate) view: T,
}

impl<'b, T: GetData> LendingIterator for LendingIter<T> {
    type Item<'a>
    where
        Self: 'a,
    = T::Out<'a>;

    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>> {
        let entity = self.view.id_at(self.index)?;

        self.index += 1;

        self.view.get_data(entity)
    }
}
