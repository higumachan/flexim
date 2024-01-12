use anyhow::Context as _;
use chrono::{DateTime, Utc};
use flexim_data_type::FlData;
use rand::random;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub trait StorageQuery {
    fn list_bags(&self) -> anyhow::Result<Vec<Arc<RwLock<Bag>>>>;
    fn get_bag(&self, bag_id: BagId) -> anyhow::Result<Arc<RwLock<Bag>>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BagId(u64);

impl BagId {
    pub fn new(field0: u64) -> Self {
        Self(field0)
    }

    pub fn into_inner(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct ManagedData {
    pub name: String,
    pub data: FlData,
}

#[derive(Debug)]
pub struct Bag {
    pub id: BagId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub data_list: Vec<ManagedData>,
}

#[derive(Default)]
pub struct Storage {
    bags: RwLock<HashMap<BagId, Arc<RwLock<Bag>>>>,
}

impl Storage {
    pub fn create_bag(&self, name: String) -> BagId {
        log::info!("create_bag: name={}", name);
        let id = BagId::new(gen_id());
        let bag = Bag {
            id,
            name,
            created_at: Utc::now(),
            data_list: vec![],
        };

        let bag = Arc::new(RwLock::new(bag));

        let mut bags = self.bags.write().unwrap();
        bags.insert(id, bag);

        id
    }

    pub fn insert_data(&self, bag_id: BagId, name: String, data: FlData) -> anyhow::Result<()> {
        log::info!("insert_data: bag_id={:?}, name={}", bag_id, name);

        let bags = self.bags.read().unwrap();
        let bag = bags.get(&bag_id).context("bag not found")?;
        let mut bag = bag.write().unwrap();
        bag.data_list.push(ManagedData { name, data });
        Ok(())
    }
}

impl StorageQuery for Storage {
    fn list_bags(&self) -> anyhow::Result<Vec<Arc<RwLock<Bag>>>> {
        let bags = self.bags.read().unwrap();
        Ok(bags.values().cloned().collect())
    }

    fn get_bag(&self, bag_id: BagId) -> anyhow::Result<Arc<RwLock<Bag>>> {
        let bags = self.bags.read().unwrap();
        let bag = bags.get(&bag_id).context("bag not found")?;
        Ok(bag.clone())
    }
}

fn gen_id() -> u64 {
    loop {
        let id = random();
        if id != 0 {
            return id;
        }
    }
}
