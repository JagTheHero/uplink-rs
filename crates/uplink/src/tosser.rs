//! Port of the bits of `tosser` that cannot be replaced by plain std collections.
//!
//! - `LList<T>`  -> `Vec<T>`
//! - `BTree<T>`  -> [`BTree<T>`] below: the C++ unbalanced binary search tree, kept because the
//!   game iterates it in pre-order (tree-shape order), which depends on the insertion history.
//! - `DArray<T>` -> [`DArray<T>`] below: a growable array whose slots can be empty, so indices
//!   stay stable when items are removed (C++ code stores these indices, e.g. computer screens).

use std::fmt;
use std::marker::PhantomData;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DArray<T> {
    slots: Vec<Option<T>>,
}

impl<T> Default for DArray<T> {
    fn default() -> Self {
        DArray { slots: Vec::new() }
    }
}

impl<T> DArray<T> {
    pub fn new() -> Self {
        Self::default()
    }

    /// `SetSize`: grow (with empty slots) or truncate.
    pub fn set_size(&mut self, newsize: i32) {
        let n = newsize.max(0) as usize;
        if n > self.slots.len() {
            self.slots.resize_with(n, || None);
        } else {
            self.slots.truncate(n);
        }
    }

    /// `PutData(data)`: store in the first empty slot (growing if needed). Returns the index.
    pub fn put_data(&mut self, data: T) -> i32 {
        if let Some(i) = self.slots.iter().position(|s| s.is_none()) {
            self.slots[i] = Some(data);
            i as i32
        } else {
            self.slots.push(Some(data));
            (self.slots.len() - 1) as i32
        }
    }

    /// `PutData(data, index)`: store at a specific index, growing the array if needed
    /// (the C++ asserted the index was in range; growing is a strict superset).
    pub fn put_data_at(&mut self, data: T, index: i32) {
        assert!(index >= 0, "DArray::put_data_at negative index");
        let i = index as usize;
        if i >= self.slots.len() {
            self.slots.resize_with(i + 1, || None);
        }
        self.slots[i] = Some(data);
    }

    /// `ChangeData(data, index)`
    pub fn change_data(&mut self, data: T, index: i32) {
        self.put_data_at(data, index);
    }

    /// `GetData(index)` / `operator[]`: `None` for an empty or out-of-range slot.
    pub fn get_data(&self, index: i32) -> Option<&T> {
        if index < 0 {
            return None;
        }
        self.slots.get(index as usize).and_then(|s| s.as_ref())
    }

    pub fn get_data_mut(&mut self, index: i32) -> Option<&mut T> {
        if index < 0 {
            return None;
        }
        self.slots.get_mut(index as usize).and_then(|s| s.as_mut())
    }

    /// `RemoveData(index)`: empties the slot and returns what was there.
    pub fn remove_data(&mut self, index: i32) -> Option<T> {
        if index < 0 {
            return None;
        }
        self.slots.get_mut(index as usize).and_then(|s| s.take())
    }

    /// `ValidIndex(index)`: in range and occupied.
    pub fn valid_index(&self, index: i32) -> bool {
        self.get_data(index).is_some()
    }

    /// `Size()`: total number of slots (used or not).
    pub fn size(&self) -> i32 {
        self.slots.len() as i32
    }

    /// `NumUsed()`
    pub fn num_used(&self) -> i32 {
        self.slots.iter().filter(|s| s.is_some()).count() as i32
    }

    /// `Empty()`
    pub fn empty(&mut self) {
        self.slots.clear();
    }

    /// Iterate `(index, &item)` over occupied slots.
    pub fn iter(&self) -> impl Iterator<Item = (i32, &T)> {
        self.slots.iter().enumerate().filter_map(|(i, s)| s.as_ref().map(|v| (i as i32, v)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (i32, &mut T)> {
        self.slots.iter_mut().enumerate().filter_map(|(i, s)| s.as_mut().map(|v| (i as i32, v)))
    }

    /// Occupied values in index order.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().filter_map(|s| s.as_ref())
    }
}

impl<T: PartialEq> DArray<T> {
    /// `FindData(data)`: index of the first equal item or -1.
    pub fn find_data(&self, data: &T) -> i32 {
        self.iter().find(|(_, v)| *v == data).map(|(i, _)| i).unwrap_or(-1)
    }
}

// ===================================================================
// BTree

/// Port of tosser `BTree<T>`: an unbalanced binary search tree keyed by `strcmp`, duplicates
/// allowed (a key equal to a node's goes to its left), with the C++ `RemoveData` restructuring.
/// Iteration (`iter`, `keys`, `values`, `ConvertToDArray`, `ConvertIndexToDArray`) is a PRE-ORDER
/// walk of the tree shape, exactly like the C++.
///
/// PORT: the nodes live in an arena (`nodes[0]` is the root node, which the C++ object itself was);
/// nodes unlinked by `remove_data` stay in the arena as garbage (the C++ leaked them too).
/// PORT: a node may be a *placeholder*: a key without data (`put_placeholder`). The World uses one
/// for "PLAYER", because the Rust world keeps the Player outside `people` but the C++ tree held it
/// there, and its position shapes the tree and the order of every walk. Placeholders count in
/// `size` and `keys_all` (they were real entries in C++) but `get`/`iter`/`keys`/`values` skip them.
///
/// Saved as a list of `[key, data]` pairs in pre-order; loading re-inserts them in that order,
/// which rebuilds the same tree shape (the C++ `SaveBTree`/`LoadBTree` did the same). Older saves
/// that stored a JSON map are still read (inserted in the map's order).
#[derive(Debug, Clone)]
pub struct BTree<T> {
    nodes: Vec<BTreeNode<T>>,
}

#[derive(Debug, Clone)]
struct BTreeNode<T> {
    id: Option<String>,
    data: Option<T>,
    ltree: Option<usize>,
    rtree: Option<usize>,
}

const BTREE_ROOT: usize = 0;

impl<T> Default for BTree<T> {
    fn default() -> Self {
        BTree { nodes: Vec::new() }
    }
}

impl<T> BTree<T> {
    pub fn new() -> Self {
        Self::default()
    }

    fn new_node(&mut self, id: &str, data: Option<T>) -> usize {
        self.nodes.push(BTreeNode { id: Some(id.to_string()), data, ltree: None, rtree: None });
        self.nodes.len() - 1
    }

    fn is_leaf(&self, n: usize) -> bool {
        self.nodes[n].ltree.is_none() && self.nodes[n].rtree.is_none()
    }

    /// `PutData(id, data)`: duplicates are allowed (they go to the left of the equal key).
    /// Returns the stored data (the new entry, even when the id was already there).
    pub fn put_data(&mut self, newid: &str, newdata: T) -> &mut T {
        let n = self.put_entry(newid, Some(newdata));
        self.nodes[n].data.as_mut().expect("just stored")
    }

    /// A key without data, holding its place in the tree (see the type's PORT note).
    pub fn put_placeholder(&mut self, newid: &str) {
        self.put_entry(newid, None);
    }

    fn put_entry(&mut self, newid: &str, newdata: Option<T>) -> usize {
        if self.nodes.is_empty() {
            self.nodes.push(BTreeNode { id: None, data: None, ltree: None, rtree: None });
        }
        let mut n = BTREE_ROOT;
        loop {
            let goleft = match &self.nodes[n].id {
                None => {
                    self.nodes[n].id = Some(newid.to_string());
                    self.nodes[n].data = newdata;
                    return n;
                }
                Some(id) => newid <= id.as_str(),
            };
            let child = if goleft { self.nodes[n].ltree } else { self.nodes[n].rtree };
            match child {
                Some(c) => n = c,
                None => {
                    let c = self.new_node(newid, newdata);
                    if goleft {
                        self.nodes[n].ltree = Some(c);
                    } else {
                        self.nodes[n].rtree = Some(c);
                    }
                    return c;
                }
            }
        }
    }

    /// `LookupTree(searchid)` starting at node `n`: the first node with that id on the search path.
    fn lookup_from(&self, mut n: usize, searchid: &str) -> Option<usize> {
        loop {
            let node = self.nodes.get(n)?;
            let id = node.id.as_deref()?;
            match searchid.cmp(id) {
                std::cmp::Ordering::Equal => return Some(n),
                std::cmp::Ordering::Less => n = node.ltree?,
                std::cmp::Ordering::Greater => n = node.rtree?,
            }
        }
    }

    /// `LookupTree(id) != NULL`: true for placeholders too.
    pub fn contains_key(&self, searchid: &str) -> bool {
        self.lookup_from(BTREE_ROOT, searchid).is_some()
    }

    /// `GetData(id)`: the data of the first match (`None` if absent or a placeholder).
    pub fn get(&self, searchid: &str) -> Option<&T> {
        let n = self.lookup_from(BTREE_ROOT, searchid)?;
        self.nodes[n].data.as_ref()
    }

    pub fn get_mut(&mut self, searchid: &str) -> Option<&mut T> {
        let n = self.lookup_from(BTREE_ROOT, searchid)?;
        self.nodes[n].data.as_mut()
    }

    /// Every entry with this id, in the order the C++ found them by repeating `LookupTree` on
    /// the left subtree of the last match ("copies with identical names will always be on the
    /// left branch", vlocation.cpp). Placeholders are skipped.
    pub fn get_all(&self, searchid: &str) -> Vec<&T> {
        let mut result = Vec::new();
        let mut tree = Some(BTREE_ROOT);
        while let Some(t) = tree {
            match self.lookup_from(t, searchid) {
                Some(found) => {
                    if let Some(d) = self.nodes[found].data.as_ref() {
                        result.push(d);
                    }
                    tree = self.nodes[found].ltree;
                }
                None => tree = None,
            }
        }
        result
    }

    /// `RemoveData(id)`, faithfully: the matched node takes over its left child (whose right
    /// subtree gets the old right subtree appended at its extreme right), or else its right child.
    /// Returns the removed data.
    ///
    /// PORT: the C++ has a dangling `else` (btree.cpp): when the id sorts AFTER a node's id the
    /// removal does nothing at all, and when it sorts before a node without a left child the
    /// search goes on in the right subtree (where it cannot be found). Both are kept: they decide
    /// what is left in the tree.
    pub fn remove_data(&mut self, newid: &str) -> Option<T> {
        let mut n = BTREE_ROOT;
        loop {
            // C++ would strcmp a NULL id here (empty tree): treated as nothing to remove.
            let id = self.nodes.get(n)?.id.as_deref()?;
            match newid.cmp(id) {
                std::cmp::Ordering::Equal => return self.remove_here(n),
                std::cmp::Ordering::Less => {
                    let next = match (self.nodes[n].ltree, self.nodes[n].rtree) {
                        (Some(l), _) => (l, true),
                        (None, Some(r)) => (r, false),
                        (None, None) => return None,
                    };
                    let (c, isleft) = next;
                    if self.nodes[c].id.as_deref() == Some(newid) && self.is_leaf(c) {
                        if isleft {
                            self.nodes[n].ltree = None;
                        } else {
                            self.nodes[n].rtree = None;
                        }
                        return self.nodes[c].data.take();
                    }
                    n = c;
                }
                std::cmp::Ordering::Greater => return None,
            }
        }
    }

    fn remove_here(&mut self, n: usize) -> Option<T> {
        let tempright = self.nodes[n].rtree;
        let old = self.nodes[n].data.take();
        if let Some(l) = self.nodes[n].ltree {
            let id = self.nodes[l].id.clone();
            let data = self.nodes[l].data.take();
            let (ll, lr) = (self.nodes[l].ltree, self.nodes[l].rtree);
            let node = &mut self.nodes[n];
            node.id = id;
            node.data = data;
            node.rtree = lr;
            node.ltree = ll;
            self.append_right(n, tempright);
        } else if let Some(r) = tempright {
            let id = self.nodes[r].id.clone();
            let data = self.nodes[r].data.take();
            let (rl, rr) = (self.nodes[r].ltree, self.nodes[r].rtree);
            let node = &mut self.nodes[n];
            node.id = id;
            node.data = data;
            node.ltree = rl;
            node.rtree = rr;
        } else {
            self.nodes[n].id = None; // Hopefully this is the root node
        }
        old
    }

    /// `AppendRight(tempright)`: hang `tempright` at the extreme right of `n`.
    fn append_right(&mut self, mut n: usize, tempright: Option<usize>) {
        while let Some(r) = self.nodes[n].rtree {
            n = r;
        }
        self.nodes[n].rtree = tempright;
    }

    /// `Empty()`
    pub fn empty(&mut self) {
        self.nodes.clear();
    }

    /// `Size()`: number of keys, placeholders included.
    pub fn size(&self) -> i32 {
        self.preorder().len() as i32
    }

    /// No keys at all (`Size () == 0`).
    pub fn is_empty(&self) -> bool {
        self.nodes.first().is_none_or(|root| root.id.is_none())
    }

    /// Node indices with an id, in pre-order (`RecursiveConvertToDArray`).
    fn preorder(&self) -> Vec<usize> {
        let mut out = Vec::new();
        if self.nodes.is_empty() {
            return out;
        }
        let mut stack = vec![BTREE_ROOT];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n];
            if node.id.is_some() {
                out.push(n);
            }
            if let Some(r) = node.rtree {
                stack.push(r);
            }
            if let Some(l) = node.ltree {
                stack.push(l);
            }
        }
        out
    }

    /// `ConvertIndexToDArray()`: every key in pre-order, placeholders included.
    pub fn keys_all(&self) -> Vec<String> {
        self.preorder().into_iter().filter_map(|n| self.nodes[n].id.clone()).collect()
    }

    /// Every `(key, data)` in pre-order, placeholders included (data `None`).
    pub fn iter_all(&self) -> impl Iterator<Item = (&str, Option<&T>)> {
        self.preorder().into_iter().map(move |n| {
            let node = &self.nodes[n];
            (node.id.as_deref().unwrap_or(""), node.data.as_ref())
        })
    }

    /// `(key, data)` in pre-order (the C++ `ConvertIndexToDArray` + `ConvertToDArray` pair),
    /// placeholders skipped.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &T)> {
        self.iter_all().filter_map(|(k, d)| d.map(|d| (k, d)))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&str, &mut T)> {
        let order = self.preorder();
        let mut slots: Vec<Option<&mut BTreeNode<T>>> = self.nodes.iter_mut().map(Some).collect();
        order.into_iter().filter_map(move |n| {
            let BTreeNode { id, data, .. } = slots[n].take()?;
            let id: &Option<String> = id;
            match (id, data) {
                (Some(id), Some(data)) => Some((id.as_str(), data)),
                _ => None,
            }
        })
    }

    /// Keys of the entries with data, in pre-order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.iter().map(|(k, _)| k)
    }

    /// `ConvertToDArray()` (placeholders skipped).
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.iter().map(|(_, d)| d)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.iter_mut().map(|(_, d)| d)
    }
}

impl<T: Serialize> Serialize for BTree<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let order = self.preorder();
        let mut seq = serializer.serialize_seq(Some(order.len()))?;
        for n in order {
            let node = &self.nodes[n];
            seq.serialize_element(&(node.id.as_deref().unwrap_or(""), &node.data))?;
        }
        seq.end()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for BTree<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BTreeVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for BTreeVisitor<T> {
            type Value = BTree<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a list of [key, data] pairs (or a map)")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<BTree<T>, A::Error> {
                let mut tree = BTree::new();
                while let Some((id, data)) = seq.next_element::<(String, Option<T>)>()? {
                    tree.put_entry(&id, data);
                }
                Ok(tree)
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<BTree<T>, A::Error> {
                let mut tree = BTree::new();
                while let Some((id, data)) = map.next_entry::<String, T>()? {
                    tree.put_data(&id, data);
                }
                Ok(tree)
            }
        }

        deserializer.deserialize_any(BTreeVisitor(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holes_keep_indices() {
        let mut d = DArray::new();
        assert_eq!(d.put_data("a"), 0);
        assert_eq!(d.put_data("b"), 1);
        d.remove_data(0);
        assert!(!d.valid_index(0));
        assert_eq!(d.size(), 2);
        assert_eq!(d.num_used(), 1);
        assert_eq!(d.put_data("c"), 0);
        d.put_data_at("z", 5);
        assert_eq!(d.size(), 6);
        assert_eq!(d.find_data(&"z"), 5);
    }

    // Expected results below come from the C++ lib/tosser/btree.cpp compiled with a small driver.

    fn dump(t: &BTree<i32>) -> String {
        t.iter_all()
            .map(|(k, d)| format!("{}={}", k, d.map_or("-".to_string(), |d| d.to_string())))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn build(items: &[(&str, i32)]) -> BTree<i32> {
        let mut t = BTree::new();
        for (k, v) in items {
            t.put_data(k, *v);
        }
        t
    }

    #[test]
    fn btree_preorder_and_removal() {
        let mut t = build(&[("m", 1), ("c", 2), ("t", 3), ("a", 4), ("e", 5), ("x", 6), ("m", 7)]);
        assert_eq!(dump(&t), "m=1 c=2 a=4 e=5 m=7 t=3 x=6");
        assert_eq!(t.size(), 7);
        // Dangling else in the C++: a key right of the root is never removed
        assert_eq!(t.remove_data("t"), None);
        assert_eq!(dump(&t), "m=1 c=2 a=4 e=5 m=7 t=3 x=6");
        // Inner node with a left child: takes it over, old right appended at its extreme right
        assert_eq!(t.remove_data("c"), Some(2));
        assert_eq!(dump(&t), "m=1 a=4 e=5 m=7 t=3 x=6");
        assert_eq!(t.remove_data("m"), Some(1));
        assert_eq!(dump(&t), "a=4 e=5 m=7 t=3 x=6");
        // No left child: takes over the right child
        assert_eq!(t.remove_data("a"), Some(4));
        assert_eq!(dump(&t), "e=5 m=7 t=3 x=6");
        assert_eq!(t.remove_data("e"), Some(5));
        assert_eq!(dump(&t), "m=7 t=3 x=6");
        assert_eq!(t.size(), 3);
    }

    #[test]
    fn btree_leaf_and_root_removal() {
        let mut t = build(&[("m", 1), ("c", 2)]);
        assert_eq!(t.remove_data("c"), Some(2));
        assert_eq!(dump(&t), "m=1");
        assert_eq!(t.remove_data("m"), Some(1));
        assert_eq!(dump(&t), "");
        assert!(t.is_empty());
        assert_eq!(t.size(), 0);
        assert_eq!(t.get("m"), None);
        t.put_data("q", 3);
        assert_eq!(dump(&t), "q=3");

        // Left of a node without a left child: the C++ searches the right subtree (no-op)
        let mut t = build(&[("m", 1), ("t", 2), ("p", 3)]);
        assert_eq!(t.remove_data("b"), None);
        assert_eq!(dump(&t), "m=1 t=2 p=3");
        assert_eq!(t.remove_data("m"), Some(1));
        assert_eq!(dump(&t), "t=2 p=3");
        assert_eq!(t.remove_data("p"), Some(3));
        assert_eq!(dump(&t), "t=2");
        assert_eq!(BTree::<i32>::new().remove_data("x"), None);
    }

    #[test]
    fn btree_duplicates() {
        let mut t = build(&[("k", 1), ("k", 2), ("k", 3), ("b", 4)]);
        assert_eq!(dump(&t), "k=1 k=2 k=3 b=4");
        assert_eq!(t.get("k"), Some(&1));
        assert_eq!(t.get_all("k"), vec![&1, &2, &3]);
        assert_eq!(t.remove_data("k"), Some(1));
        assert_eq!(dump(&t), "k=2 k=3 b=4");
        assert_eq!(t.get("k"), Some(&2));
        assert_eq!(t.remove_data("k"), Some(2));
        assert_eq!(dump(&t), "k=3 b=4");
    }

    #[test]
    fn btree_placeholder_and_mutation() {
        let mut t = build(&[("m", 1), ("c", 2)]);
        t.put_placeholder("PLAYER");
        t.put_data("z", 9);
        assert_eq!(dump(&t), "m=1 c=2 PLAYER=- z=9");
        assert!(t.contains_key("PLAYER"));
        assert_eq!(t.get("PLAYER"), None);
        assert_eq!(t.size(), 4);
        assert_eq!(t.keys_all(), vec!["m", "c", "PLAYER", "z"]);
        assert_eq!(t.keys().collect::<Vec<_>>(), vec!["m", "c", "z"]);
        for v in t.values_mut() {
            *v *= 10;
        }
        *t.get_mut("c").unwrap() += 1;
        assert_eq!(dump(&t), "m=10 c=21 PLAYER=- z=90");
    }

    #[test]
    fn btree_serde_keeps_shape() {
        let mut t = build(&[("m", 1), ("c", 2), ("t", 3), ("a", 4), ("e", 5), ("x", 6), ("m", 7)]);
        t.put_placeholder("PLAYER");
        t.remove_data("c");
        let json = serde_json::to_string(&t).unwrap();
        assert_eq!(json, r#"[["m",1],["a",4],["PLAYER",null],["e",5],["m",7],["t",3],["x",6]]"#);
        let back: BTree<i32> = serde_json::from_str(&json).unwrap();
        assert_eq!(dump(&back), dump(&t));
        assert_eq!(serde_json::to_string(&back).unwrap(), json);
        // Old saves stored a JSON map
        let old: BTree<i32> = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        assert_eq!(dump(&old), "a=1 b=2");
    }
}
