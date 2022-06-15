use caches::{WTinyLFUCache};
use caches::lfu::DefaultKeyHasher;
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::secure::entity::permission::{Group};
use crate::model::user::entity::user::User;
use lazy_static::lazy_static;
use crate::model::link::entity::link::LinkType;

pub struct LFU {
    pub user_by_id: WTinyLFUCache<u64, User, DefaultKeyHasher<u64>>,
    pub user_by_login: WTinyLFUCache<String, User, DefaultKeyHasher<String>>,
    pub object_by_id: WTinyLFUCache<u64, Object, DefaultKeyHasher<u64>>,
    pub object_type_by_id: WTinyLFUCache<u64, ObjectType, DefaultKeyHasher<u64>>,
    pub object_type_by_alias: WTinyLFUCache<String, ObjectType, DefaultKeyHasher<String>>,
    pub link_type_by_id: WTinyLFUCache<u64, LinkType, DefaultKeyHasher<u64>>,
    pub link_type_by_alias: WTinyLFUCache<String, LinkType, DefaultKeyHasher<String>>,
    pub group_by_id: WTinyLFUCache<u64, Group, DefaultKeyHasher<u64>>,
}

impl LFU {
    pub fn new(samples: usize) -> Self {
        Self {
            user_by_id: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            user_by_login: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            object_by_id: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            object_type_by_id: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            object_type_by_alias: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            link_type_by_id: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            link_type_by_alias: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
            group_by_id: WTinyLFUCache::with_sizes(1, 2, 2, samples).unwrap(),
        }
    }
}

lazy_static! {
    static ref CACHE:LFU = LFU::new(1000);
}