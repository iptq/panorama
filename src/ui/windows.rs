use std::collections::{HashMap, HashSet, VecDeque};

use tui::layout::Rect;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct LayoutId(usize);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PageId(usize);

#[derive(Default)]
pub struct WindowLayout {
    ctr: usize,
    currently_active: Option<LayoutId>,

    ids: HashMap<LayoutId, PageId>,
    page_order: Vec<PageId>,
    pages: HashMap<PageId, PageGraph>,

    layout_cache: HashMap<PageId, HashMap<LayoutId, Rect>>,
}

impl WindowLayout {
    /// Adds a new page to the list
    pub fn new_page(&mut self) -> (LayoutId, PageId) {
        let id = LayoutId(self.ctr);
        self.ctr += 1;

        let pg = PageGraph::new(id);
        let pid = PageId(self.ctr);
        self.ctr += 1;
        self.pages.insert(pid, pg);
        self.page_order.push(pid);
        (id, pid)
    }

    pub fn list_pages(&self) -> &[PageId] {
        &self.page_order
    }

    /// Get a set of all windows visible on the current page
    pub fn visible_windows(&self) -> HashMap<LayoutId, Rect> {
        let mut map = HashMap::new();
        if let Some(page) = self
            .currently_active
            .as_ref()
            .and_then(|id| self.ids.get(id))
            .and_then(|pid| self.pages.get(pid))
        {
            let mut q = VecDeque::new();
            q.push_back(page.root);

            while !q.is_empty() {
                let front = q.pop_front().expect("not empty");
            }
        }
        map
    }
}

struct PageGraph {
    root: LayoutId,
    adj: HashMap<LayoutId, HashSet<(LayoutId, Dir)>>,
}

enum Dir {
    H,
    V,
}

impl PageGraph {
    pub fn new(id: LayoutId) -> Self {
        PageGraph {
            root: id,
            adj: HashMap::new(),
        }
    }
}
