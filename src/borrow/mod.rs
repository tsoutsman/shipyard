mod all_storages;
mod borrow_info;
#[cfg(feature = "thread_local")]
mod non_send;
#[cfg(feature = "thread_local")]
mod non_send_sync;
#[cfg(feature = "thread_local")]
mod non_sync;

pub use all_storages::AllStoragesBorrow;
pub use borrow_info::BorrowInfo;
#[cfg(feature = "thread_local")]
pub use non_send::NonSend;
#[cfg(feature = "thread_local")]
pub use non_send_sync::NonSendSync;
#[cfg(feature = "thread_local")]
pub use non_sync::NonSync;

use crate::all_storages::CustomStorageAccess;
use crate::atomic_refcell::{Ref, RefMut};
use crate::component::Component;
use crate::sparse_set::SparseSet;
use crate::view::{
    AllStoragesViewMut, EntitiesView, EntitiesViewMut, UniqueView, UniqueViewMut, View, ViewMut,
};
use crate::world::World;
use crate::{error, track};
use core::marker::PhantomData;

/// Describes if a storage is borrowed exlusively or not.  
/// It is used to display workloads' borrowing information.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mutability {
    #[allow(missing_docs)]
    Shared,
    #[allow(missing_docs)]
    Exclusive,
}

/// Allows a type to be borrowed by [`World::borrow`], [`World::run`] and worklaods.
pub trait Borrow {
    #[allow(missing_docs)]
    type View<'a>;

    /// This function is where the actual borrowing happens.
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage>;
}

impl Borrow for AllStoragesViewMut<'_> {
    type View<'a> = AllStoragesViewMut<'a>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        world
            .all_storages
            .borrow_mut()
            .map(AllStoragesViewMut)
            .map_err(error::GetStorage::AllStoragesBorrow)
    }
}

impl Borrow for () {
    type View<'a> = ();

    #[inline]
    fn borrow(_: &World) -> Result<Self::View<'_>, error::GetStorage>
    where
        Self: Sized,
    {
        Ok(())
    }
}

impl Borrow for EntitiesView<'_> {
    type View<'a> = EntitiesView<'a>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let entities = all_storages.entities()?;

        let (entities, borrow) = unsafe { Ref::destructure(entities) };

        Ok(EntitiesView {
            entities,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
        })
    }
}

impl Borrow for EntitiesViewMut<'_> {
    type View<'a> = EntitiesViewMut<'a>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let entities = all_storages.entities_mut()?;

        let (entities, borrow) = unsafe { RefMut::destructure(entities) };

        Ok(EntitiesViewMut {
            entities,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
        })
    }
}

impl<T: Send + Sync + Component> Borrow for View<'_, T>
where
    <T::Tracking as track::Tracking<T>>::DeletionData: Send + Sync,
{
    type View<'a> = View<'a, T>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { Ref::destructure(view) };

        Ok(View {
            sparse_set,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
        })
    }
}

#[cfg(feature = "thread_local")]
impl<T: Sync + Component> Borrow for NonSend<View<'_, T>>
where
    <T::Tracking as track::Tracking<T>>::DeletionData: Sync,
{
    type View<'a> = NonSend<View<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_non_send(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { Ref::destructure(view) };

        Ok(NonSend(View {
            sparse_set,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Send + Component> Borrow for NonSync<View<'_, T>>
where
    <T::Tracking as track::Tracking<T>>::DeletionData: Send,
{
    type View<'a> = NonSync<View<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_non_sync(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { Ref::destructure(view) };

        Ok(NonSync(View {
            sparse_set,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Component> Borrow for NonSendSync<View<'_, T>> {
    type View<'a> = NonSendSync<View<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_non_send_sync(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { Ref::destructure(view) };

        Ok(NonSendSync(View {
            sparse_set,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
        }))
    }
}

impl<T: Send + Sync + Component> Borrow for ViewMut<'_, T>
where
    <T::Tracking as track::Tracking<T>>::DeletionData: Send + Sync,
{
    type View<'a> = ViewMut<'a, T>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_mut(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { RefMut::destructure(view) };

        Ok(ViewMut {
            sparse_set,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
        })
    }
}

#[cfg(feature = "thread_local")]
impl<T: Sync + Component> Borrow for NonSend<ViewMut<'_, T>>
where
    <T::Tracking as track::Tracking<T>>::DeletionData: Sync,
{
    type View<'a> = NonSend<ViewMut<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_non_send_mut(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { RefMut::destructure(view) };

        Ok(NonSend(ViewMut {
            sparse_set,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Send + Component> Borrow for NonSync<ViewMut<'_, T>>
where
    <T::Tracking as track::Tracking<T>>::DeletionData: Send,
{
    type View<'a> = NonSync<ViewMut<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_non_sync_mut(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { RefMut::destructure(view) };

        Ok(NonSync(ViewMut {
            sparse_set,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Component> Borrow for NonSendSync<ViewMut<'_, T>> {
    type View<'a> = NonSendSync<ViewMut<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_or_insert_non_send_sync_mut(SparseSet::new)?;

        let (sparse_set, borrow) = unsafe { RefMut::destructure(view) };

        Ok(NonSendSync(ViewMut {
            sparse_set,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
        }))
    }
}

impl<T: Send + Sync + Component> Borrow for UniqueView<'_, T> {
    type View<'a> = UniqueView<'a, T>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage()?;

        let (unique, borrow) = unsafe { Ref::destructure(view) };

        Ok(UniqueView {
            unique,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        })
    }
}

#[cfg(feature = "thread_local")]
impl<T: Sync + Component> Borrow for NonSend<UniqueView<'_, T>> {
    type View<'a> = NonSend<UniqueView<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage()?;

        let (unique, borrow) = unsafe { Ref::destructure(view) };

        Ok(NonSend(UniqueView {
            unique,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Send + Component> Borrow for NonSync<UniqueView<'_, T>> {
    type View<'a> = NonSync<UniqueView<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage()?;

        let (unique, borrow) = unsafe { Ref::destructure(view) };

        Ok(NonSync(UniqueView {
            unique,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Component> Borrow for NonSendSync<UniqueView<'_, T>> {
    type View<'a> = NonSendSync<UniqueView<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage()?;

        let (unique, borrow) = unsafe { Ref::destructure(view) };

        Ok(NonSendSync(UniqueView {
            unique,
            borrow: Some(borrow),
            all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        }))
    }
}

impl<T: Send + Sync + Component> Borrow for UniqueViewMut<'_, T> {
    type View<'a> = UniqueViewMut<'a, T>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_mut()?;

        let (unique, borrow) = unsafe { RefMut::destructure(view) };

        Ok(UniqueViewMut {
            unique,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        })
    }
}

#[cfg(feature = "thread_local")]
impl<T: Sync + Component> Borrow for NonSend<UniqueViewMut<'_, T>> {
    type View<'a> = NonSend<UniqueViewMut<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_mut()?;

        let (unique, borrow) = unsafe { RefMut::destructure(view) };

        Ok(NonSend(UniqueViewMut {
            unique,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        }))
    }
}
#[cfg(feature = "thread_local")]
impl<T: Send + Component> Borrow for NonSync<UniqueViewMut<'_, T>> {
    type View<'a> = NonSync<UniqueViewMut<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_mut()?;

        let (unique, borrow) = unsafe { RefMut::destructure(view) };

        Ok(NonSync(UniqueViewMut {
            unique,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        }))
    }
}

#[cfg(feature = "thread_local")]
impl<T: Component> Borrow for NonSendSync<UniqueViewMut<'_, T>> {
    type View<'a> = NonSendSync<UniqueViewMut<'a, T>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        let (all_storages, all_borrow) = unsafe {
            Ref::destructure(
                world
                    .all_storages
                    .borrow()
                    .map_err(error::GetStorage::AllStoragesBorrow)?,
            )
        };

        let view = all_storages.custom_storage_mut()?;

        let (unique, borrow) = unsafe { RefMut::destructure(view) };

        Ok(NonSendSync(UniqueViewMut {
            unique,
            _borrow: Some(borrow),
            _all_borrow: Some(all_borrow),
            _phantom: PhantomData,
        }))
    }
}

impl<T: Borrow> Borrow for Option<T> {
    type View<'a> = Option<T::View<'a>>;

    #[inline]
    fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
        Ok(T::borrow(world).ok())
    }
}

macro_rules! impl_world_borrow {
    ($(($type: ident, $index: tt))+) => {
        impl<$($type: Borrow),+> Borrow for ($($type,)+) {
            type View<'a> = ($($type::View<'a>,)+);

            #[inline]
            fn borrow(world: &World) -> Result<Self::View<'_>, error::GetStorage> {
                Ok(($($type::borrow(world)?,)+))
            }
        }
    }
}

macro_rules! world_borrow {
    ($(($type: ident, $index: tt))*;($type1: ident, $index1: tt) $(($queue_type: ident, $queue_index: tt))*) => {
        impl_world_borrow![$(($type, $index))*];
        world_borrow![$(($type, $index))* ($type1, $index1); $(($queue_type, $queue_index))*];
    };
    ($(($type: ident, $index: tt))*;) => {
        impl_world_borrow![$(($type, $index))*];
    }
}

world_borrow![(A, 0); (B, 1) (C, 2) (D, 3) (E, 4) (F, 5) (G, 6) (H, 7) (I, 8) (J, 9)];
