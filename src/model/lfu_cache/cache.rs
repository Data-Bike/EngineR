use caches::{Cache, WTinyLFUCache};
use caches::lfu::DefaultKeyHasher;
use crate::model::object::entity::object::{Object, ObjectType};
use crate::model::secure::entity::permission::{Group};
use crate::model::user::entity::user::User;
use lazy_static::lazy_static;
use std::sync::Mutex;
use crate::model::link::entity::link::LinkType;

pub struct LFU {
    pub user_by_id: WTinyLFUCache<String, User, DefaultKeyHasher<String>>,
    pub user_by_login: WTinyLFUCache<String, User, DefaultKeyHasher<String>>,
    pub object_by_id: WTinyLFUCache<String, Object, DefaultKeyHasher<String>>,
    pub object_type_by_id: WTinyLFUCache<String, ObjectType, DefaultKeyHasher<String>>,
    pub object_type_by_alias: WTinyLFUCache<String, ObjectType, DefaultKeyHasher<String>>,
    pub link_type_by_id: WTinyLFUCache<String, LinkType, DefaultKeyHasher<String>>,
    pub link_type_by_alias: WTinyLFUCache<String, LinkType, DefaultKeyHasher<String>>,
    pub group_by_id: WTinyLFUCache<String, Group, DefaultKeyHasher<String>>,
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
   pub static ref CACHE:Mutex<LFU> = Mutex::new(LFU::new(1000));
}

pub fn clear_cache() {
    CACHE.lock().unwrap().user_by_id.purge();
    CACHE.lock().unwrap().group_by_id.purge();
    CACHE.lock().unwrap().link_type_by_alias.purge();
    CACHE.lock().unwrap().link_type_by_id.purge();
    CACHE.lock().unwrap().user_by_login.purge();
    CACHE.lock().unwrap().object_type_by_id.purge();
    CACHE.lock().unwrap().object_type_by_alias.purge();
    CACHE.lock().unwrap().object_by_id.purge();
}

#[macro_export]
macro_rules! cache_it {
            ( $key:expr,$cache:ident, $x:block ) => {
                {
                use crate::model::lfu_cache::cache::CACHE;
                use caches::Cache;
                Ok(match CACHE.lock().unwrap().$cache.get($key) {
                    None => {
                        $x
                    }
                    Some(t) => {t.clone()}
                })}
            };
        }

#[macro_export]
macro_rules! remove_it_from_cache {
    ($key:expr,$cache:ident) => {
        {
                use crate::model::lfu_cache::cache::CACHE;
                use caches::Cache;
                CACHE.lock().unwrap().$cache.remove($key) 
        }
    };
}


