// #[cfg(feature = "serde1")]
// use super::{SparseSet, SparseSetDeserializer};
// #[cfg(feature = "serde1")]
// use crate::atomic_refcell::AtomicRefCell;
// #[cfg(feature = "serde1")]
// use crate::serde_setup::{GlobalDeConfig, GlobalSerConfig, Identifier, SerConfig};
use crate::sparse_set::SparseArray;
use crate::storage::EntityId;
// #[cfg(feature = "serde1")]
// use crate::storage::Storage;
use crate::type_id::TypeId;
use alloc::sync::Arc;
use alloc::vec::Vec;
// #[cfg(feature = "serde1")]
// use hashbrown::HashMap;

pub(crate) const BUCKET_SIZE: usize = 128 / core::mem::size_of::<EntityId>();

pub(crate) enum Pack {
    Tight(TightPack),
    Loose(LoosePack),
    None,
}

pub struct Metadata<T> {
    pub(crate) pack: Pack,
    pub(crate) observer_types: Vec<TypeId>,
    pub(crate) shared: SparseArray<[EntityId; BUCKET_SIZE]>,
    pub(crate) update: Option<UpdatePack<T>>,
    // #[cfg(feature = "serde1")]
    // pub(crate) serde: Option<SerdeInfos<T>>,
}

impl<T> Default for Metadata<T> {
    fn default() -> Self {
        Metadata {
            pack: Pack::None,
            observer_types: Vec::new(),
            shared: SparseArray::new(),
            update: None,
            // #[cfg(feature = "serde1")]
            // serde: None,
        }
    }
}

impl<T> Metadata<T> {
    /// Returns `true` if enough storages were passed in
    pub(crate) fn has_all_storages(&self, components: &[TypeId], additionals: &[TypeId]) -> bool {
        match &self.pack {
            Pack::Tight(tight) => {
                tight.has_all_storages(components, additionals, &self.observer_types)
            }
            Pack::Loose(loose) => {
                loose.has_all_storages(components, additionals, &self.observer_types)
            }
            Pack::None => {
                if components.len() + additionals.len() < self.observer_types.len() {
                    return false;
                }

                // current component index
                let mut comp = 0;
                // current additional index
                let mut add = 0;

                // we know observer types are at most as many as components + additionals so we'll use them to drive the iteration
                for &observer_type in &self.observer_types {
                    // we skip components with a lower TypeId
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < observer_type)
                        .count();

                    // we also skip additional types with a lower TypeId
                    add += additionals[add..]
                        .iter()
                        .take_while(|&&additional| additional < observer_type)
                        .count();

                    // one of them has to be equal to observer_type else not enough storages where passed in
                    match (components.get(comp), additionals.get(add)) {
                        (Some(&component), Some(&additional))
                            if component == observer_type || additional == observer_type => {}
                        (Some(&component), None) if component == observer_type => {}
                        (None, Some(&additional)) if additional == observer_type => {}
                        _ => return false,
                    }
                }

                true
            }
        }
    }
}

pub(crate) struct TightPack {
    pub(crate) types: Arc<[TypeId]>,
    pub(crate) len: usize,
}

impl TightPack {
    pub(crate) fn new(types: Arc<[TypeId]>) -> Self {
        TightPack { types, len: 0 }
    }
    /// Returns `Ok(packed_types)` if `components` contains at least all components in `self.types`
    pub(crate) fn is_packable(&self, components: &[TypeId]) -> bool {
        // the entity doesn't have enough components to be packed
        if components.len() < self.types.len() {
            return false;
        }

        // current component index
        let mut comp = 0;

        // we know packed types are at most as many as components so we'll use them to drive the iteration
        for &packed_type in &*self.types {
            // we skip components with a lower TypeId
            comp += components[comp..]
                .iter()
                .take_while(|&&component| component < packed_type)
                .count();

            // since both slices are sorted, if the types aren't equal it means components is missing a packed type
            if components
                .get(comp)
                .filter(|&&component| component == packed_type)
                .is_none()
            {
                return false;
            }
        }

        true
    }
    /// Returns `true` if enough storages were passed in
    fn has_all_storages(
        &self,
        components: &[TypeId],
        additionals: &[TypeId],
        observer_types: &[TypeId],
    ) -> bool {
        // both pairs can't have duplicates
        if components.len() + additionals.len() < self.types.len() + observer_types.len() {
            return false;
        }

        // current tight type
        let mut tight = 0;
        // current observer type
        let mut observer = 0;
        // current component
        let mut comp = 0;
        // current additional
        let mut add = 0;

        // we use the tight and observer types to drive the iteration since there are at most the same count as components + additionals
        loop {
            // since both arrays are sorted and a value can't be in both we can iterate just once
            // but we have to make sure to not stop the iteration too early when tight or loose ends
            match (self.types.get(tight), observer_types.get(observer)) {
                (Some(&tight_type), observer_type)
                    if observer_type.is_none() || tight_type < *observer_type.unwrap() =>
                {
                    // we skip components with a lower TypeId
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < tight_type)
                        .count();

                    // we also skip additional types with a lower TypeId
                    add += additionals[add..]
                        .iter()
                        .take_while(|&&additional| additional < tight_type)
                        .count();

                    // one of them has to be equal to tight_type else not enough storages where passed in
                    // we also have to update the number of components found in the tight_types
                    match (components.get(comp), additionals.get(add)) {
                        (Some(&component), Some(&additional))
                            if component == tight_type || additional == tight_type =>
                        {
                            tight += 1
                        }
                        (Some(&component), None) if component == tight_type => tight += 1,
                        (None, Some(&additional)) if additional == tight_type => tight += 1,
                        _ => return false,
                    }
                }
                (Some(_), None) => unreachable!(), // the compiler isn't smart enough to see this
                (_, Some(&observer_type)) => {
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < observer_type)
                        .count();
                    add += additionals[add..]
                        .iter()
                        .take_while(|&&additional| additional < observer_type)
                        .count();

                    match (components.get(comp), additionals.get(add)) {
                        (Some(&component), Some(&additional))
                            if component == observer_type || additional == observer_type =>
                        {
                            observer += 1
                        }
                        (Some(&component), None) if component == observer_type => observer += 1,
                        (None, Some(&additional)) if additional == observer_type => observer += 1,
                        _ => return false,
                    }
                }
                (None, None) => break,
            }
        }

        // we check all types were passed in
        tight == self.types.len() && observer == observer_types.len()
    }
}

pub(crate) struct LoosePack {
    pub(crate) tight_types: Arc<[TypeId]>,
    pub(crate) loose_types: Arc<[TypeId]>,
    pub(crate) len: usize,
}

impl LoosePack {
    pub(crate) fn new(tight_types: Arc<[TypeId]>, loose_types: Arc<[TypeId]>) -> Self {
        LoosePack {
            tight_types,
            loose_types,
            len: 0,
        }
    }
    /// Returns `Ok(packed_types)` if `components` contains at least all components in `self.types`
    pub(crate) fn is_packable(&self, components: &[TypeId]) -> bool {
        if components.len() < self.tight_types.len() + self.loose_types.len() {
            // the entity doesn't have enough components to be packed
            return false;
        }

        // current tight type
        let mut tight = 0;
        // current loose type
        let mut loose = 0;
        // current component
        let mut comp = 0;

        // we use the packed types to drive the iteration since there are at most the same count as components
        loop {
            // since both arrays are sorted and a value can't be in both we can iterate just once
            // but we have to make sure to not stop the iteration too early when tight or loose ends
            match (self.tight_types.get(tight), self.loose_types.get(loose)) {
                (Some(&tight_type), loose_type)
                    if loose_type.is_none() || tight_type < *loose_type.unwrap() =>
                {
                    // we skip components with a lower TypeId
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < tight_type)
                        .count();

                    if components
                        .get(comp)
                        .filter(|&&component| component == tight_type)
                        .is_some()
                    {
                        tight += 1;
                    } else {
                        return false;
                    }
                }
                (Some(_), None) => unreachable!(),
                (_, Some(&loose_type)) => {
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < loose_type)
                        .count();

                    if components
                        .get(comp)
                        .filter(|&&component| component == loose_type)
                        .is_some()
                    {
                        loose += 1;
                    } else {
                        return false;
                    }
                }
                (None, None) => break,
            }
        }

        tight == self.tight_types.len() && loose == self.loose_types.len()
    }
    #[allow(clippy::cognitive_complexity)]
    /// Returns `true` if enough storages were passed in
    fn has_all_storages(
        &self,
        components: &[TypeId],
        additionals: &[TypeId],
        observer_types: &[TypeId],
    ) -> bool {
        if components.len() + additionals.len()
            < self.tight_types.len() + self.loose_types.len() + observer_types.len()
        {
            return false;
        }

        // current tight type
        let mut tight = 0;
        // current loose type
        let mut loose = 0;
        // current observer type
        let mut observer = 0;
        // current component
        let mut comp = 0;
        // current additional
        let mut add = 0;

        // we use the packed types to drive the iteration since there are at most the same count as components
        loop {
            // since both arrays are sorted and a value can't be in both we can iterate just once
            // but we have to make sure to not stop the iteration too early when tight or loose ends
            match (
                self.tight_types.get(tight),
                self.loose_types.get(loose),
                observer_types.get(observer),
            ) {
                (Some(&tight_type), Some(&loose_type), Some(&observer_type)) => {
                    if tight_type < loose_type && tight_type < observer_type {
                        // we skip components with a lower TypeId
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < tight_type)
                            .count();

                        // we also skip additional types with a lower TypeId
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < tight_type)
                            .count();

                        // one of them has to be equal to tight_type else not enough storages where passed in
                        // we also have to update the number of components found in the tight_types
                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == tight_type || additional == tight_type =>
                            {
                                tight += 1
                            }
                            (Some(&component), None) if component == tight_type => tight += 1,
                            (None, Some(&additional)) if additional == tight_type => tight += 1,
                            _ => return false,
                        }
                    } else if loose_type < observer_type {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < loose_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < loose_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == loose_type || additional == loose_type =>
                            {
                                loose += 1
                            }
                            (Some(&component), None) if component == loose_type => loose += 1,
                            (None, Some(&additional)) if additional == loose_type => loose += 1,
                            _ => return false,
                        }
                    } else {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < observer_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < observer_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == observer_type || additional == observer_type =>
                            {
                                observer += 1
                            }
                            (Some(&component), None) if component == observer_type => observer += 1,
                            (None, Some(&additional)) if additional == observer_type => {
                                observer += 1
                            }
                            _ => return false,
                        }
                    }
                }
                (Some(&tight_type), Some(&loose_type), None) => {
                    if tight_type < loose_type {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < tight_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < tight_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == tight_type || additional == tight_type =>
                            {
                                tight += 1
                            }
                            (Some(&component), None) if component == tight_type => tight += 1,
                            (None, Some(&additional)) if additional == tight_type => tight += 1,
                            _ => return false,
                        }
                    } else {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < loose_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < loose_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == loose_type || additional == loose_type =>
                            {
                                loose += 1
                            }
                            (Some(&component), None) if component == loose_type => loose += 1,
                            (None, Some(&additional)) if additional == loose_type => loose += 1,
                            _ => return false,
                        }
                    }
                }
                (Some(&tight_type), None, Some(&observer_type)) => {
                    if tight_type < observer_type {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < tight_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < tight_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == tight_type || additional == tight_type =>
                            {
                                tight += 1
                            }
                            (Some(&component), None) if component == tight_type => tight += 1,
                            (None, Some(&additional)) if additional == tight_type => tight += 1,
                            _ => return false,
                        }
                    } else {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < observer_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < observer_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == observer_type || additional == observer_type =>
                            {
                                observer += 1
                            }
                            (Some(&component), None) if component == observer_type => observer += 1,
                            (None, Some(&additional)) if additional == observer_type => {
                                observer += 1
                            }
                            _ => return false,
                        }
                    }
                }
                (None, Some(&loose_type), Some(&observer_type)) => {
                    if loose_type < observer_type {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < loose_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < loose_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == loose_type || additional == loose_type =>
                            {
                                loose += 1
                            }
                            (Some(&component), None) if component == loose_type => loose += 1,
                            (None, Some(&additional)) if additional == loose_type => loose += 1,
                            _ => return false,
                        }
                    } else {
                        comp += components[comp..]
                            .iter()
                            .take_while(|&&component| component < observer_type)
                            .count();
                        add += additionals[add..]
                            .iter()
                            .take_while(|&&additional| additional < observer_type)
                            .count();

                        match (components.get(comp), additionals.get(add)) {
                            (Some(&component), Some(&additional))
                                if component == observer_type || additional == observer_type =>
                            {
                                observer += 1
                            }
                            (Some(&component), None) if component == observer_type => observer += 1,
                            (None, Some(&additional)) if additional == observer_type => {
                                observer += 1
                            }
                            _ => return false,
                        }
                    }
                }
                (Some(&tight_type), None, None) => {
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < tight_type)
                        .count();
                    add += additionals[add..]
                        .iter()
                        .take_while(|&&additional| additional < tight_type)
                        .count();

                    match (components.get(comp), additionals.get(add)) {
                        (Some(&component), Some(&additional))
                            if component == tight_type || additional == tight_type =>
                        {
                            tight += 1
                        }
                        (Some(&component), None) if component == tight_type => tight += 1,
                        (None, Some(&additional)) if additional == tight_type => tight += 1,
                        _ => return false,
                    }
                }
                (None, Some(&loose_type), None) => {
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < loose_type)
                        .count();
                    add += additionals[add..]
                        .iter()
                        .take_while(|&&additional| additional < loose_type)
                        .count();

                    match (components.get(comp), additionals.get(add)) {
                        (Some(&component), Some(&additional))
                            if component == loose_type || additional == loose_type =>
                        {
                            loose += 1
                        }
                        (Some(&component), None) if component == loose_type => loose += 1,
                        (None, Some(&additional)) if additional == loose_type => loose += 1,
                        _ => return false,
                    }
                }
                (None, None, Some(&observer_type)) => {
                    comp += components[comp..]
                        .iter()
                        .take_while(|&&component| component < observer_type)
                        .count();
                    add += additionals[add..]
                        .iter()
                        .take_while(|&&additional| additional < observer_type)
                        .count();

                    match (components.get(comp), additionals.get(add)) {
                        (Some(&component), Some(&additional))
                            if component == observer_type || additional == observer_type =>
                        {
                            observer += 1
                        }
                        (Some(&component), None) if component == observer_type => observer += 1,
                        (None, Some(&additional)) if additional == observer_type => observer += 1,
                        _ => return false,
                    }
                }
                (None, None, None) => break,
            }
        }

        tight == self.tight_types.len()
            && loose == self.loose_types.len()
            && observer == observer_types.len()
    }
}

pub(crate) struct UpdatePack<T> {
    pub(crate) removed: Vec<EntityId>,
    pub(crate) deleted: Vec<(EntityId, T)>,
}

impl<T> Default for UpdatePack<T> {
    fn default() -> Self {
        UpdatePack {
            removed: Vec::new(),
            deleted: Vec::new(),
        }
    }
}

// #[cfg(feature = "serde1")]
// #[allow(unused)]
// pub(crate) struct SerdeInfos<T> {
//     pub(crate) serialization: fn(
//         &SparseSet<T>,
//         GlobalSerConfig,
//         &mut dyn crate::erased_serde::Serializer,
//     )
//         -> Result<crate::erased_serde::Ok, crate::erased_serde::Error>,
//     pub(crate) deserialization: fn(
//         GlobalDeConfig,
//         &HashMap<EntityId, EntityId>,
//         &mut dyn crate::erased_serde::Deserializer<'_>,
//     ) -> Result<Storage, crate::erased_serde::Error>,
//     pub(crate) with_shared: bool,
//     pub(crate) identifier: Option<Identifier>,
// }

// #[cfg(feature = "serde1")]
// impl<T: serde::Serialize + for<'de> serde::Deserialize<'de> + 'static> SerdeInfos<T> {
//     pub(super) fn new(ser_config: SerConfig) -> Self {
//         SerdeInfos {
//             serialization:
//                 |sparse_set: &SparseSet<T>,
//                  ser_config: GlobalSerConfig,
//                  serializer: &mut dyn crate::erased_serde::Serializer| {
//                     crate::erased_serde::Serialize::erased_serialize(
//                         &super::SparseSetSerializer {
//                             sparse_set: &sparse_set,
//                             ser_config,
//                         },
//                         serializer,
//                     )
//                 },
//             deserialization:
//                 |de_config: GlobalDeConfig,
//                  entities_map: &HashMap<EntityId, EntityId>,
//                  deserializer: &mut dyn crate::erased_serde::Deserializer<'_>| {
//                     #[cfg(feature = "std")]
//                     {
//                         Ok(Storage(Box::new(AtomicRefCell::new(
//                             serde::de::DeserializeSeed::deserialize(
//                                 SparseSetDeserializer::<T> {
//                                     de_config,
//                                     _phantom: core::marker::PhantomData,
//                                 },
//                                 deserializer,
//                             )?,
//                             None,
//                             true,
//                         ))))
//                     }
//                     #[cfg(not(feature = "std"))]
//                     {
//                         Ok(Storage(Box::new(AtomicRefCell::new(
//                             serde::de::DeserializeSeed::deserialize(
//                                 SparseSetDeserializer::<T> {
//                                     de_config,
//                                     _phantom: core::marker::PhantomData,
//                                 },
//                                 deserializer,
//                             )?,
//                         ))))
//                     }
//                 },
//             with_shared: true,
//             identifier: ser_config.identifier,
//         }
//     }
// }