use anyhow::Context as _;
use chrono::{DateTime, Utc};
use flexim_data_type::FlData;
use rand::random;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

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

#[derive(Debug)]
pub struct Bag {
    id: BagId,
    name: String,
    created_at: DateTime<Utc>,
    data_list: Vec<FlData>,
}

#[derive(Default)]
pub struct Storage {
    bags: RwLock<HashMap<BagId, Arc<RwLock<Bag>>>>,
}

impl Storage {
    pub fn create_bag(&self, name: String) -> BagId {
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

    pub fn insert_data(&self, bag_id: BagId, data: FlData) -> anyhow::Result<()> {
        let bags = self.bags.read().unwrap();
        let bag = bags.get(&bag_id).context("bag not found")?;
        let mut bag = bag.write().unwrap();
        bag.data_list.push(data);
        Ok(())
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
