use anyhow::Context as _;
use chrono::{DateTime, Utc};
use flexim_data_type::{FlData, FlDataReference, GenerationSelector};
use rand::random;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

pub trait StorageQuery {
    fn list_bags(&self) -> anyhow::Result<Vec<Arc<RwLock<Bag>>>>;
    fn get_bag(&self, bag_id: BagId) -> anyhow::Result<Arc<RwLock<Bag>>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BagId(u64);

impl BagId {
    pub fn new(field0: u64) -> Self {
        Self(field0)
    }

    pub fn into_inner(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedData {
    pub generation: u64,
    pub name: String,
    pub data: FlData,
}

impl From<ManagedData> for FlDataReference {
    fn from(value: ManagedData) -> Self {
        Self::new(
            value.name,
            GenerationSelector::Generation(value.generation),
            value.data.data_type(),
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

    pub fn data_by_reference(&self, reference: &FlDataReference) -> anyhow::Result<FlData> {
        let mut name_filtered = self
            .data_list
            .iter()
            .filter(|data| data.name == reference.name);

        Ok((match reference.generation {
            GenerationSelector::Latest => {
                let data = name_filtered.max_by_key(|data| data.generation);
                data.context("data not found")
            }
            GenerationSelector::Generation(generation) => {
                let data = name_filtered.find(|data| data.generation == generation);
                data.context("data not found")
            }
        })?
        .data
        .clone())
    }
}

#[derive(Default)]
pub struct Storage {
    bags: RwLock<HashMap<BagId, Arc<RwLock<Bag>>>>,
}

type BagVersions = BTreeMap<String, Vec<Arc<RwLock<Bag>>>>;
type BagGroups = BTreeMap<String, BagVersions>;

impl Storage {
    pub fn load_bag(&self, bag: Bag) -> bool {
        let bag = Arc::new(RwLock::new(bag));
        let mut bags = self.bags.write().unwrap();
        let id = bag.read().unwrap().id;
        if bags.contains_key(&id) {
            return false;
        }
        let _ = bags.insert(id, bag);
        true
    }

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

    pub fn clear_bags(&self) {
        let mut bags = self.bags.write().unwrap();
        bags.clear();
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
        let mut bag_versions = BTreeMap::new();

        for bag in bags.values() {
            let bag_guard = bag.read().unwrap();
            let versions = bag_versions.entry(bag_guard.name.clone()).or_insert(vec![]);
            versions.push(bag.clone());
        }
        for bag_groups in bag_versions.values_mut() {
            bag_groups.sort_by_key(|bag| bag.read().unwrap().created_at);
        }

        let mut bag_groups = BTreeMap::new();

        for (name, bag_version) in bag_versions {
            let (group_key, bag_name) = if name.contains('/') {
                let mut parts = name.splitn(2, '/');
                (
                    parts.next().unwrap().to_string(),
                    parts.next().unwrap().to_string(),
                )
            } else {
                (name.to_string(), name.to_string())
            };

            let bag_versions = bag_groups.entry(group_key).or_insert(BTreeMap::new());
            bag_versions.insert(bag_name, bag_version);
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
