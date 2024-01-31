use anyhow::Context as _;
use chrono::{DateTime, Utc};
use flexim_data_type::FlData;
use rand::random;
use std::collections::{BTreeMap, HashMap};
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
    pub generation: u64,
    pub name: String,
    pub data: FlData,
}

#[derive(Debug)]
pub struct Bag {
    pub id: BagId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub data_list: Vec<ManagedData>,
    pub generation_counter: HashMap<String, u64>,
}

impl Bag {
    pub fn data_groups(&self) -> BTreeMap<String, Vec<&ManagedData>> {
        let mut data_groups = BTreeMap::new();
        for data in &self.data_list {
            let data_groups = data_groups.entry(data.name.clone()).or_insert(vec![]);
            data_groups.push(data);
        }
        for data_groups in data_groups.values_mut() {
            data_groups.sort_by_key(|data| data.generation);
        }
        data_groups
    }
}

#[derive(Default)]
pub struct Storage {
    bags: RwLock<HashMap<BagId, Arc<RwLock<Bag>>>>,
}

type BagGroups = HashMap<String, Vec<Arc<RwLock<Bag>>>>;

impl Storage {
    pub fn create_bag(&self, name: String) -> BagId {
        log::info!("create_bag: name={}", name);
        let id = BagId::new(gen_id());
        let bag = Bag {
            id,
            name,
            created_at: Utc::now(),
            data_list: vec![],
            generation_counter: HashMap::new(),
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
        let generation_mut = bag.generation_counter.entry(name.clone()).or_insert(0);
        let generation = *generation_mut;
        *generation_mut += 1;
        bag.data_list.push(ManagedData {
            generation,
            name,
            data,
        });
        Ok(())
    }

    pub fn bag_groups(&self) -> anyhow::Result<BagGroups> {
        let bags = self.bags.read().unwrap();
        let mut bag_groups = HashMap::new();

        for bag in bags.values() {
            let bag_guard = bag.read().unwrap();
            let bag_groups = bag_groups.entry(bag_guard.name.clone()).or_insert(vec![]);
            bag_groups.push(bag.clone());
        }
        for bag_groups in bag_groups.values_mut() {
            bag_groups.sort_by_key(|bag| bag.read().unwrap().created_at);
        }

        Ok(bag_groups)
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
