use std::collections::{HashMap, HashSet, VecDeque};

use tui::layout::Rect;

use super::{FrameType, HandlesInput, UI};

pub trait Window: HandlesInput {
    // Return some kind of name
    fn name(&self) -> String;

    // Main draw function
    fn draw(&self, f: FrameType, area: Rect, ui: &UI);

    /// Draw function, except the window is not the actively focused one
    ///
    /// By default, this just calls the regular draw function but the window may choose to perform
    /// a less intensive draw if it's known to not be active
    fn draw_inactive(&mut self, f: FrameType, area: Rect, ui: &UI) {
        self.draw(f, area, ui);
    }

    /// Update function
    fn update(&mut self) {}
}

downcast_rs::impl_downcast!(Window);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayoutId(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PageId(usize);

#[derive(Default, Debug)]
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
        self.ids.insert(id, pid);

        if let None = self.currently_active {
            self.currently_active = Some(id);
        }

        (id, pid)
    }

    pub fn list_pages(&self) -> &[PageId] {
        &self.page_order
    }

    /// Get a set of all windows visible on the current page, given the size of the allotted space
    pub fn visible_windows(&self, area: Rect) -> HashMap<LayoutId, Rect> {
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
                // TODO: how to subdivide properly?
                map.insert(front, area);
            }
        }
        map
    }
}

#[derive(Debug)]
struct PageGraph {
    root: LayoutId,
    adj: HashMap<LayoutId, HashSet<(LayoutId, Dir)>>,
}

#[derive(Debug)]
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
