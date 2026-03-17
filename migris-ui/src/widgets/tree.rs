use iced::{
    Element, Length,
    widget::{Column, button, row, space},
};

pub struct TreeState<T> {
    items: Vec<TreeItem<T>>,
    visible: Vec<VisibleTreeItem>,

    filter: String,
    on_filter: Option<FilterFn<T>>,
}

type FilterFn<T> = Box<dyn Fn(&TreeItem<T>, &str) -> bool>;

impl<T> TreeState<T> {
    /// Creates a new [`TreeState`], initialized with the given items.
    pub fn new<I>(items: I) -> Self
    where
        I: IntoIterator<Item = TreeItem<T>>,
    {
        let items = items.into_iter().collect();
        let mut state = Self {
            items,
            visible: Vec::new(),
            filter: String::from(""),
            on_filter: None,
        };

        state.build_visible();
        state
    }

    /// Sets the function to be used when filtering the tree items.
    pub fn on_filter(mut self, on_filter: FilterFn<T>) -> Self {
        self.on_filter = Some(on_filter);
        self
    }

    /// Returns the current filter within the state.
    pub fn current_filter(&self) -> &str {
        &self.filter
    }

    /// Filters the items in the state using the given filter string.
    pub fn filter(&mut self, filter: String) {
        self.filter = filter;
        self.build_visible();
    }

    /// Loads the state with the given items.
    pub fn load<I>(&mut self, items: I)
    where
        I: IntoIterator<Item = TreeItem<T>>,
    {
        self.items = items.into_iter().collect();
        self.build_visible();
    }

    /// Toggles the expanded state of the item with the given [`TreeItemId`].
    pub fn toggle(&mut self, id: &TreeItemId) {
        let item = self.item_mut(id);
        item.expanded = !item.expanded;

        // Rebuild visible items with item's new expansion state.
        // TODO: change this to only toggle necessary items
        self.build_visible();
    }

    fn build_visible(&mut self) {
        self.visible.clear();

        fn inner<T>(
            items: &[TreeItem<T>],
            visible: &mut Vec<VisibleTreeItem>,
            path: &mut Vec<usize>,
            filter: &str,
            on_filter: Option<&FilterFn<T>>,
        ) {
            for (idx, item) in items.iter().enumerate() {
                if !filter.is_empty()
                    && let Some(on_filter) = on_filter
                    && !on_filter(item, filter)
                {
                    continue;
                }

                path.push(idx);

                visible.push(VisibleTreeItem {
                    id: TreeItemId(path.clone()),
                    depth: path.len() as u32 - 1,
                });

                if item.expanded {
                    inner(&item.children, visible, path, filter, on_filter);
                }

                //
                path.pop();
            }
        }

        let mut path = Vec::new();
        inner(
            &self.items,
            &mut self.visible,
            &mut path,
            &self.filter,
            self.on_filter.as_ref(),
        );
    }

    fn item(&self, id: &TreeItemId) -> &TreeItem<T> {
        let root_idx = id.0.first().expect("empty item path found");
        let mut item = &self.items[*root_idx];

        for idx in &id.0[1..] {
            item = &item.children[*idx];
        }

        item
    }

    fn item_mut(&mut self, id: &TreeItemId) -> &mut TreeItem<T> {
        let root_idx = id.0.first().expect("empty item path found");
        let mut item = &mut self.items[*root_idx];

        for idx in &id.0[1..] {
            item = &mut item.children[*idx];
        }

        item
    }
}

#[derive(Debug, Clone)]
pub struct TreeItemId(Vec<usize>);

#[derive(Debug)]
pub struct TreeItem<T> {
    value: T,
    expanded: bool,
    children: Vec<TreeItem<T>>,
}

impl<T> TreeItem<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            expanded: false,
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: TreeItem<T>) -> Self {
        self.children.push(child);
        self
    }

    pub fn children<I>(mut self, children: I) -> Self
    where
        I: IntoIterator<Item = TreeItem<T>>,
    {
        self.children.extend(children);
        self
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

#[derive(Debug)]
struct VisibleTreeItem {
    id: TreeItemId,
    depth: u32,
}

#[allow(clippy::type_complexity)]
pub struct Tree<'a, T, Message> {
    state: &'a TreeState<T>,
    width: Length,
    height: Length,
    on_view: Box<dyn Fn(&TreeItem<T>) -> Element<'_, Message>>,
    on_select: Option<Box<dyn Fn(TreeItemId) -> Message>>,
    on_toggle: Option<Box<dyn Fn(TreeItemId) -> Message>>,
}

impl<'a, T, Message> Tree<'a, T, Message>
where
    Message: 'a + Clone,
{
    // TODO: calculate this depth width based on application-wide font size and element padding
    const DEFAULT_DEPTH_WIDTH: u32 = 17;

    pub fn new<F>(state: &'a TreeState<T>, on_view: F) -> Self
    where
        F: 'static + Fn(&TreeItem<T>) -> Element<'_, Message>,
    {
        Self {
            state,
            width: Length::Fill,
            height: Length::Fill,
            on_view: Box::new(on_view),
            on_select: None,
            on_toggle: None,
        }
    }

    pub fn on_select<F>(mut self, on_select: F) -> Self
    where
        F: 'static + Fn(TreeItemId) -> Message,
    {
        self.on_select = Some(Box::new(on_select));
        self
    }

    pub fn on_toggle<F>(mut self, on_toggle: F) -> Self
    where
        F: 'static + Fn(TreeItemId) -> Message,
    {
        self.on_toggle = Some(Box::new(on_toggle));
        self
    }

    fn view(&self) -> Element<'a, Message> {
        let elements: Vec<Element<'a, Message>> = self
            .state
            .visible
            .iter()
            .map(|visible_item| self.tree_element(visible_item))
            .collect();

        Column::with_children(elements)
            .width(self.width)
            .height(self.height)
            .clip(true)
            .into()
    }

    fn tree_element(&self, visible_item: &VisibleTreeItem) -> Element<'a, Message> {
        let item = self.state.item(&visible_item.id);
        let spacing = space::horizontal().width(visible_item.depth * Self::DEFAULT_DEPTH_WIDTH);
        let content = (self.on_view)(item);
        let message = if item.children.is_empty()
            && let Some(on_select) = &self.on_select
        {
            Some((on_select)(visible_item.id.clone()))
        } else if !item.children.is_empty()
            && let Some(on_toggle) = &self.on_toggle
        {
            Some((on_toggle)(visible_item.id.clone()))
        } else {
            None
        };

        button(row![spacing, content])
            .style(button::text)
            .width(self.width)
            .padding(2.5)
            .on_press_maybe(message)
            .into()
    }
}

impl<'a, T, Message> From<Tree<'a, T, Message>> for Element<'a, Message>
where
    Message: 'a + Clone,
{
    fn from(tree: Tree<'a, T, Message>) -> Self {
        tree.view()
    }
}
