//! Per-job data sharding: which client trains which rows in each epoch.
//!
//! `CLUSTER_PLAN.md` listed this as not yet built: the transport and the
//! gradient math assumed each client's shard was pre-assigned. A `ShardPlan`
//! makes that assignment explicit, deterministic and checkable:
//!
//! * every row is assigned to exactly one client per epoch (disjoint and
//!   complete), so the averaged gradient covers the whole dataset once;
//! * the assignment depends only on `(seed, epoch, rows, the set of clients
//!   and their weights)`, never on the order clients joined, so the host and
//!   every client compute the same plan independently;
//! * shard sizes follow client weights (for example a GPU node can take more
//!   rows than a laptop CPU) by the largest-remainder method, so sizes differ
//!   from the exact proportion by less than one row;
//! * rows are shuffled per epoch with a seeded permutation, so a client does
//!   not see the same rows every epoch.
//!
//! Pure data, no networking or tensor dependency, like `GradientAggregator`.
use std::collections::BTreeMap;
use std::fmt::Debug;

/// SplitMix64: a tiny, well-known, seedable generator. Enough for a shuffle
/// that must be identical on every machine; not for cryptography.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` without modulo bias (Lemire's method).
    fn below(&mut self, n: u64) -> u64 {
        loop {
            let m = (self.next() as u128) * (n as u128);
            let low = m as u64;
            if low >= n.wrapping_neg() % n {
                return (m >> 64) as u64;
            }
        }
    }
}

/// Why a plan could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShardError {
    NoClients,
    /// A weight that is zero, negative or not finite.
    BadWeight(String),
}

impl std::fmt::Display for ShardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardError::NoClients => write!(f, "a shard plan needs at least one client"),
            ShardError::BadWeight(c) => write!(f, "client {c} has a weight that is not a positive finite number"),
        }
    }
}

impl std::error::Error for ShardError {}

/// The row assignment for one epoch.
#[derive(Debug, Clone, PartialEq)]
pub struct ShardPlan<K: Ord + Clone + Debug> {
    pub epoch: u64,
    pub rows: usize,
    shards: BTreeMap<K, Vec<usize>>,
}

impl<K: Ord + Clone + Debug> ShardPlan<K> {
    /// Equal-weight plan.
    pub fn new(clients: &[K], rows: usize, epoch: u64, seed: u64) -> Result<Self, ShardError> {
        let weighted: Vec<(K, f64)> = clients.iter().cloned().map(|c| (c, 1.0)).collect();
        Self::weighted(&weighted, rows, epoch, seed)
    }

    /// Plan with per-client weights. Duplicate client ids are merged by
    /// adding their weights.
    pub fn weighted(clients: &[(K, f64)], rows: usize, epoch: u64, seed: u64) -> Result<Self, ShardError> {
        let mut weights: BTreeMap<K, f64> = BTreeMap::new();
        for (c, w) in clients {
            if !(w.is_finite() && *w > 0.0) {
                return Err(ShardError::BadWeight(format!("{c:?}")));
            }
            *weights.entry(c.clone()).or_insert(0.0) += *w;
        }
        if weights.is_empty() {
            return Err(ShardError::NoClients);
        }

        // Largest-remainder apportionment of `rows` by weight. Ties go to the
        // smaller client id (BTreeMap order), so the result is deterministic.
        let total: f64 = weights.values().sum();
        let mut sizes: Vec<(K, usize, f64)> = weights
            .iter()
            .map(|(c, w)| {
                let exact = rows as f64 * w / total;
                (c.clone(), exact.floor() as usize, exact - exact.floor())
            })
            .collect();
        let assigned: usize = sizes.iter().map(|s| s.1).sum();
        let mut order: Vec<usize> = (0..sizes.len()).collect();
        order.sort_by(|&a, &b| sizes[b].2.partial_cmp(&sizes[a].2).unwrap_or(std::cmp::Ordering::Equal).then(a.cmp(&b)));
        for &i in order.iter().take(rows.saturating_sub(assigned)) {
            sizes[i].1 += 1;
        }

        // Seeded Fisher-Yates shuffle of row indices for this epoch.
        let mut perm: Vec<usize> = (0..rows).collect();
        let mut rng = SplitMix64(seed ^ epoch.wrapping_mul(0xD1B5_4A32_D192_ED03));
        for i in (1..rows).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            perm.swap(i, j);
        }

        let mut shards = BTreeMap::new();
        let mut start = 0;
        for (c, n, _) in sizes {
            let mut mine = perm[start..start + n].to_vec();
            mine.sort_unstable();
            shards.insert(c, mine);
            start += n;
        }
        Ok(Self { epoch, rows, shards })
    }

    /// The rows `client` trains on this epoch (sorted), or `None` if it is
    /// not part of the plan.
    pub fn rows_for(&self, client: &K) -> Option<&[usize]> {
        self.shards.get(client).map(|v| v.as_slice())
    }

    /// The client that owns `row`.
    pub fn owner_of(&self, row: usize) -> Option<&K> {
        self.shards.iter().find(|(_, rows)| rows.binary_search(&row).is_ok()).map(|(c, _)| c)
    }

    pub fn clients(&self) -> impl Iterator<Item = &K> {
        self.shards.keys()
    }

    pub fn shard_sizes(&self) -> BTreeMap<K, usize> {
        self.shards.iter().map(|(c, r)| (c.clone(), r.len())).collect()
    }

    /// The batches a client runs this epoch, in a fixed order. The last batch
    /// may be short.
    pub fn batches_for(&self, client: &K, batch_size: usize) -> Vec<Vec<usize>> {
        let size = batch_size.max(1);
        self.rows_for(client).map(|rows| rows.chunks(size).map(|c| c.to_vec()).collect()).unwrap_or_default()
    }

    /// Checks the invariant the gradient average relies on: every row appears
    /// in exactly one shard.
    pub fn is_partition(&self) -> bool {
        let mut seen = vec![false; self.rows];
        for rows in self.shards.values() {
            for &r in rows {
                if r >= self.rows || seen[r] {
                    return false;
                }
                seen[r] = true;
            }
        }
        seen.into_iter().all(|s| s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_is_a_disjoint_complete_partition() {
        for rows in [0, 1, 7, 100, 1001] {
            for n in 1..6 {
                let clients: Vec<u32> = (0..n).collect();
                let plan = ShardPlan::new(&clients, rows, 3, 42).unwrap();
                assert!(plan.is_partition(), "rows={rows} clients={n}");
                let sizes = plan.shard_sizes();
                let (lo, hi) = (sizes.values().min().unwrap(), sizes.values().max().unwrap());
                assert!(hi - lo <= 1, "equal weights must differ by at most one row: {sizes:?}");
            }
        }
    }

    #[test]
    fn plan_does_not_depend_on_join_order() {
        let a = ShardPlan::new(&["laptop", "desktop", "nixos"], 500, 1, 7).unwrap();
        let b = ShardPlan::new(&["nixos", "laptop", "desktop"], 500, 1, 7).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn epochs_reshuffle_and_seeds_reproduce() {
        let c = ["a", "b"];
        let e0 = ShardPlan::new(&c, 200, 0, 1).unwrap();
        let e1 = ShardPlan::new(&c, 200, 1, 1).unwrap();
        assert_ne!(e0.rows_for(&"a"), e1.rows_for(&"a"));
        assert_eq!(e1, ShardPlan::new(&c, 200, 1, 1).unwrap());
        assert_ne!(e1, ShardPlan::new(&c, 200, 1, 2).unwrap());
    }

    #[test]
    fn weights_set_shard_sizes() {
        let plan = ShardPlan::weighted(&[("gpu", 3.0), ("cpu", 1.0)], 1000, 0, 9).unwrap();
        assert_eq!(plan.shard_sizes()[&"gpu"], 750);
        assert_eq!(plan.shard_sizes()[&"cpu"], 250);
        let odd = ShardPlan::weighted(&[("a", 1.0), ("b", 1.0), ("c", 1.0)], 10, 0, 9).unwrap();
        assert_eq!(odd.shard_sizes().values().sum::<usize>(), 10);
        assert!(odd.is_partition());
    }

    #[test]
    fn more_clients_than_rows_leaves_some_idle() {
        let clients: Vec<u8> = (0..5).collect();
        let plan = ShardPlan::new(&clients, 3, 0, 0).unwrap();
        assert!(plan.is_partition());
        assert_eq!(plan.shard_sizes().values().filter(|&&n| n == 0).count(), 2);
    }

    #[test]
    fn bad_inputs_are_errors() {
        assert_eq!(ShardPlan::<u8>::new(&[], 10, 0, 0), Err(ShardError::NoClients));
        assert!(matches!(ShardPlan::weighted(&[(1u8, 0.0)], 10, 0, 0), Err(ShardError::BadWeight(_))));
        assert!(matches!(ShardPlan::weighted(&[(1u8, f64::NAN)], 10, 0, 0), Err(ShardError::BadWeight(_))));
    }

    #[test]
    fn owner_and_batches_agree_with_rows() {
        let plan = ShardPlan::new(&[1u8, 2, 3], 50, 2, 5).unwrap();
        for row in 0..50 {
            let owner = plan.owner_of(row).unwrap();
            assert!(plan.rows_for(owner).unwrap().contains(&row));
        }
        assert_eq!(plan.owner_of(50), None);
        let batches = plan.batches_for(&1, 4);
        assert_eq!(batches.concat(), plan.rows_for(&1).unwrap());
        assert!(batches.iter().all(|b| b.len() <= 4));
        assert!(plan.batches_for(&9, 4).is_empty());
    }

    #[test]
    fn shuffle_is_unbiased_enough() {
        // Over many epochs each row should land with client "a" about half the time.
        let mut hits = vec![0u32; 20];
        for epoch in 0..2000 {
            let plan = ShardPlan::new(&["a", "b"], 20, epoch, 11).unwrap();
            for &r in plan.rows_for(&"a").unwrap() {
                hits[r] += 1;
            }
        }
        for h in hits {
            assert!((800..1200).contains(&h), "row went to 'a' {h} times of 2000");
        }
    }
}
