/// Open addressing with backward-shift deletion. Entries and buckets are cloned
/// together on a COW write; bucket indices never point into another snapshot.
#[derive(Clone, Debug, Default)]
pub(crate) struct MapStorage {
    entries: Vec<(Value, Value)>,
    buckets: Vec<usize>,
}
impl MapStorage {
    pub(crate) fn entries(&self) -> &[(Value, Value)] {
        &self.entries
    }
    fn hash(key: &Value) -> usize {
        let mut h = match key {
            Value::Int(n, _) => *n as u64,
            Value::Bool(b) => u64::from(*b),
            Value::Enum(n, _, _) => *n as u64,
            Value::Float(n, Type::Float(32)) => {
                if *n == 0.0 {
                    0
                } else {
                    (*n as f32).to_bits() as u64
                }
            }
            Value::Float(n, _) => {
                if *n == 0.0 {
                    0
                } else {
                    n.to_bits()
                }
            }
            Value::Str(s) => {
                let mut h = 14695981039346656037u64;
                for byte in s.bytes() {
                    h = (h ^ byte as u64).wrapping_mul(1099511628211);
                }
                h
            }
            _ => unreachable!("validated Map key"),
        };
        h = (h ^ (h >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94d049bb133111eb);
        (h ^ (h >> 31)) as usize
    }
    fn equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(a, _), Value::Int(b, _)) => a == b,
            (Value::Float(a, _), Value::Float(b, _)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Enum(a, _, _), Value::Enum(b, _, _)) => a == b,
            _ => false,
        }
    }
    fn slot(&self, key: &Value) -> usize {
        let mask = self.buckets.len() - 1;
        let mut slot = Self::hash(key) & mask;
        while self.buckets[slot] != 0 && !Self::equal(&self.entries[self.buckets[slot] - 1].0, key)
        {
            slot = (slot + 1) & mask;
        }
        slot
    }
    fn find(&self, key: &Value) -> Option<usize> {
        if self.buckets.is_empty() {
            return None;
        }
        self.buckets[self.slot(key)].checked_sub(1)
    }
    fn rehash(&mut self, capacity: usize) -> Result<(), String> {
        let mut buckets = Vec::new();
        buckets
            .try_reserve_exact(capacity)
            .map_err(|_| "collection allocation failed")?;
        buckets.resize(capacity, 0);
        self.buckets = buckets;
        for i in 0..self.entries.len() {
            let slot = self.slot(&self.entries[i].0);
            self.buckets[slot] = i + 1;
        }
        Ok(())
    }
    fn set(&mut self, key: Value, value: Value) -> Result<(), String> {
        if let Some(i) = self.find(&key) {
            self.entries[i].1 = value;
            return Ok(());
        }
        self.entries
            .try_reserve(1)
            .map_err(|_| "collection allocation failed")?;
        if self.buckets.is_empty() {
            self.rehash(16)?;
        } else if self.entries.len() >= self.buckets.len() / 2 {
            self.rehash(
                self.buckets
                    .len()
                    .checked_mul(2)
                    .ok_or("collection allocation failed")?,
            )?;
        }
        let slot = self.slot(&key);
        self.entries.push((key, value));
        self.buckets[slot] = self.entries.len();
        Ok(())
    }
    fn remove(&mut self, key: &Value) -> bool {
        let Some(i) = self.find(key) else {
            return false;
        };
        let mask = self.buckets.len() - 1;
        let mut hole = self.slot(key);
        let mut scan = (hole + 1) & mask;
        while self.buckets[scan] != 0 {
            let home = Self::hash(&self.entries[self.buckets[scan] - 1].0) & mask;
            if scan.wrapping_sub(home) & mask >= scan.wrapping_sub(hole) & mask {
                self.buckets[hole] = self.buckets[scan];
                hole = scan;
            }
            scan = (scan + 1) & mask;
        }
        self.buckets[hole] = 0;
        let last = self.entries.len() - 1;
        self.entries.swap_remove(i);
        if i != last {
            // Search by index, not equality: IEEE NaN keys never compare equal.
            let mut moved = Self::hash(&self.entries[i].0) & mask;
            while self.buckets[moved] != last + 1 {
                moved = (moved + 1) & mask;
            }
            self.buckets[moved] = i + 1;
        }
        true
    }
}
