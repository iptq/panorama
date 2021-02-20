use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;
use std::ops::Deref;

pub type StringStore = Store<&'static str>;
pub type StringEntry = Entry<&'static str>;

pub struct Store<T> {
    capacity: usize,
    store: Vec<Entry<T>>,
    index: HashMap<T, usize>,
    head: usize,
    tail: usize,
}

impl<T: Hash + Eq> Store<T> {
    pub fn new(capacity: usize) -> Self {
        Store {
            capacity,
            store: Vec::with_capacity(capacity),
            index: HashMap::new(),
            head: 0,
            tail: 0,
        }
    }

    pub fn insert(&mut self, val: T) {
        if self.index.contains_key(&val) {
            return;
        }

        let entry = Entry {
            val,
            prev: 0,
            next: 0,
        };

        let new_head = if self.store.len() == self.store.capacity() {
            let idx = self.pop_back();
            self.store[idx] = entry;
            idx
        } else {
            self.store.push(entry);
            self.store.len() - 1
        };

        self.push_front(new_head);
    }

    fn pop_back(&mut self) -> usize {
        let old_tail = self.tail;
        let new_tail = self.store[old_tail].prev;
        self.tail = new_tail;
        old_tail
    }

    fn push_front(&mut self, idx: usize) {
        if self.store.len() == 1 {
            self.tail = idx;
        } else {
            self.store[self.head].prev = idx;
            self.store[idx].next = idx;
        }
        self.head = idx;
    }
}

#[derive(Copy, Clone)]
pub struct Entry<T> {
    val: T,
    prev: usize,
    next: usize,
}

impl<T: Copy + Clone> Deref for Entry<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.val
    }
}

impl<T> AsRef<T> for Entry<T> {
    fn as_ref(&self) -> &T {
        &self.val
    }
}
