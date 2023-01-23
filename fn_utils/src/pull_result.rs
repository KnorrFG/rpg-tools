use std::collections::HashMap;

pub trait PullResult {
    type T;
    fn pull_result(self) -> Self::T;
}

impl<K, V, E> PullResult for HashMap<K, Result<V, E>>
where
    K: std::hash::Hash + std::cmp::Eq,
{
    type T = Result<HashMap<K, V>, E>;
    fn pull_result(self) -> Self::T {
        let mut res = HashMap::new();
        for (k, v) in self {
            res.insert(k, v?);
        }
        Ok(res)
    }
}

impl<V, E> PullResult for Vec<Result<V, E>> {
    type T = Result<Vec<V>, E>;
    fn pull_result(self) -> Self::T {
        let mut res = Vec::with_capacity(self.len());
        for r in self {
            res.push(r?);
        }
        Ok(res)
    }
}

pub struct PullWrapper<Wrapped>(Wrapped);

pub trait WrapIter: Sized {
    fn wrap_iter(self) -> PullWrapper<Self>;
}

impl<ItemType, ErrType, IterType> WrapIter for IterType
where
    IterType: Sized + Iterator<Item = Result<ItemType, ErrType>>,
{
    fn wrap_iter(self) -> PullWrapper<Self> {
        PullWrapper(self)
    }
}

impl<ItemType, ErrType, IterType> PullResult for PullWrapper<IterType>
where
    IterType: Iterator<Item = Result<ItemType, ErrType>>,
{
    type T = Result<Vec<ItemType>, ErrType>;
    fn pull_result(self) -> Self::T {
        let mut res = vec![];
        for elem in self.0 {
            res.push(elem?)
        }
        Ok(res)
    }
}
