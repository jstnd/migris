use iced::{
    Element, Length,
    widget::{Column, button, row, space},
};

#[derive(Debug)]
pub struct TreeState<T> {
    items: Vec<TreeItem<T>>,
    visible: Vec<VisibleTreeItem>,
}

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
        };

        state.build_visible();
        state
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
            visible: &mut Vec<VisibleTreeItem>,
            items: &[TreeItem<T>],
            path: &mut Vec<usize>,
        ) {
            for (idx, item) in items.iter().enumerate() {
                path.push(idx);

                visible.push(VisibleTreeItem {
                    id: TreeItemId(path.clone()),
                    depth: path.len() as u32 - 1,
                });

                if item.expanded {
                    inner(visible, &item.children, path);
                }

                //
                path.pop();
            }
        }

        let mut path = Vec::new();
        inner(&mut self.visible, &self.items, &mut path);
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
            expanded: true,
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
    on_view: Box<dyn Fn(&T) -> Element<'_, Message>>,
    on_select: Option<Box<dyn Fn(TreeItemId) -> Message>>,
    on_toggle: Option<Box<dyn Fn(TreeItemId) -> Message>>,
}

impl<'a, T, Message> Tree<'a, T, Message>
where
    Message: 'a + Clone,
{
    const DEFAULT_DEPTH_WIDTH: u32 = 16;

    pub fn new<F>(state: &'a TreeState<T>, on_view: F) -> Self
    where
        F: 'static + Fn(&T) -> Element<'_, Message>,
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
        let content = (self.on_view)(&item.value);
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
