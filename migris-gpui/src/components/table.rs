use std::{cmp::Ordering, sync::Arc};

use futures_util::StreamExt;
use gpui::{
    App, AppContext, Context, DefiniteLength, Entity, EventEmitter, IntoElement, ParentElement,
    Pixels, RenderOnce, SharedString, Styled, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Sizable, h_flex,
    table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState},
};
use indexmap::IndexMap;
use migris::data::{QueryData, QueryResult};

use crate::components::{
    icon::{Icon, IconName},
    text_ellipsis,
};

const INIT_BATCH_SIZE: usize = 1_000;
const LOAD_BATCH_SIZE: usize = 100;

const MIN_COLUMN_WIDTH: Pixels = px(100.0);
const MAX_COLUMN_WIDTH: Pixels = px(250.0);

struct QueryTableDelegate {
    /// The query result to display in the table.
    result: Option<QueryResult>,

    /// The columns for the table.
    columns: Vec<Column>,

    /// Tracks the sort direction and order for columns being actively sorted.
    column_sorts: IndexMap<SharedString, ColumnSort>,

    /// Whether more data is available to load into the table.
    has_more_data: bool,

    /// The order to display rows inside the table.
    ///
    /// This is used for an efficient way of visually sorting the data within the table without
    /// having to expensively move row data around, and also to preserve the original data order.
    row_display_order: Option<Vec<usize>>,
}

impl QueryTableDelegate {
    /// Creates a new [`QueryTableDelegate`].
    fn new() -> Self {
        Self {
            result: None,
            columns: Vec::new(),
            column_sorts: IndexMap::new(),
            has_more_data: false,
            row_display_order: None,
        }
    }

    /// Initializes the table with the given [`QueryResult`].
    fn init(&mut self, cx: &mut App, result: QueryResult) {
        let is_reinit = self.result.is_some();
        self.result = Some(result);
        self.has_more_data = true;
        self.load(INIT_BATCH_SIZE);

        if !is_reinit {
            self.build_columns(cx);
        }
    }

    /// Builds the columns for the table.
    fn build_columns(&mut self, cx: &mut App) {
        let Some(data) = self.data() else {
            return;
        };

        let mut columns: Vec<Column> = data
            .columns()
            .iter()
            .map(|column| {
                let name = column.name().to_owned();
                let width = (name.len() * cx.theme().font_size * 0.60)
                    .clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH);

                Column::new(&name, &name).width(width)
            })
            .collect();

        // Determine default column widths from a small batch of data.
        for row in data.rows().iter().take(LOAD_BATCH_SIZE) {
            for (idx, column) in columns.iter_mut().enumerate() {
                // Skip if column has already reached the max default width.
                if column.width >= MAX_COLUMN_WIDTH {
                    continue;
                }

                // Determine the width from the value's length.
                let value = row.values[idx].to_string();
                let width = (value.len() * cx.theme().font_size * 0.60)
                    .clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH);

                // Set the column's new width.
                if width > column.width {
                    column.width = width;
                }
            }
        }

        self.columns = columns;
    }

    /// Returns a reference to the query data.
    fn data(&self) -> Option<&QueryData> {
        let Some(result) = &self.result else {
            return None;
        };

        Some(&result.data)
    }

    /// Loads a number of rows from the query result's stream.
    fn load(&mut self, rows: usize) {
        let Some(result) = &mut self.result else {
            return;
        };

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Some(stream) = &mut result.stream
                    && let Some(data) = Arc::get_mut(&mut result.data)
                {
                    let mut data_stream = stream.take(rows);

                    while let Some(row) = data_stream.next().await {
                        if let Ok(row) = row {
                            data.push_row(row);
                        }
                    }

                    // Determine if the stream has more data to load after.
                    let peekable = stream.peekable();
                    futures_util::pin_mut!(peekable);
                    self.has_more_data = peekable.peek().await.is_some();
                }
            });
        });
    }

    /// Sorts the column with the given index.
    ///
    /// This will only update the sort direction of the column.
    fn sort_column(&mut self, column_idx: usize) {
        let column = &self.columns[column_idx];
        let current_sort = if let Some(sort) = self.column_sorts.get(&column.key) {
            *sort
        } else {
            ColumnSort::Default
        };

        match current_sort {
            ColumnSort::Default => {
                // Move unsorted (default) column to ascending order.
                self.column_sorts
                    .insert(column.key.clone(), ColumnSort::Ascending);
            }
            ColumnSort::Ascending => {
                // Move ascending order column to descending order.
                self.column_sorts[&column.key] = ColumnSort::Descending;
            }
            ColumnSort::Descending => {
                // Move descending order column to unsorted (default).
                self.column_sorts.shift_remove(&column.key);
            }
        }
    }
}

impl TableDelegate for QueryTableDelegate {
    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        let Some(data) = self.data() else {
            return 0;
        };

        data.rows().len()
    }

    fn has_more(&self, _: &App) -> bool {
        self.has_more_data
    }

    fn load_more(&mut self, _: &mut Window, _: &mut Context<TableState<Self>>) {
        self.load(LOAD_BATCH_SIZE);
    }

    fn load_more_threshold(&self) -> usize {
        LOAD_BATCH_SIZE
    }

    fn loading(&self, _: &App) -> bool {
        self.data().is_none()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(data) = self.data() else {
            return div();
        };

        let row_ix = if let Some(display_order) = &self.row_display_order {
            display_order[row_ix]
        } else {
            row_ix
        };

        let row = &data.rows()[row_ix];

        div()
            .w_full()
            .child(text_ellipsis(row.values[col_ix].to_string()))
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let column = &self.columns[col_ix];
        let column_sort = self.column_sorts.get_full(&column.key);

        h_flex()
            .w_full()
            .justify_between()
            .child(
                h_flex()
                    .w(DefiniteLength::Fraction(0.9))
                    .gap_1()
                    .text_color(cx.theme().foreground)
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .text_xs()
                            .child((col_ix + 1).to_string()),
                    )
                    .child(text_ellipsis(column.name.clone())),
            )
            .when_some(column_sort, |this, (idx, _, sort)| {
                this.child(if *sort == ColumnSort::Ascending {
                    div()
                        .relative()
                        .pt_1()
                        .child(Icon::new(cx, IconName::ArrowUpNarrowWide))
                        .child(
                            div()
                                .absolute()
                                .top(px(-4.0))
                                .right(px(-2.0))
                                .text_color(cx.theme().muted_foreground)
                                .text_xs()
                                .child((idx + 1).to_string()),
                        )
                } else {
                    div()
                        .relative()
                        .pb_1()
                        .child(Icon::new(cx, IconName::ArrowDownWideNarrow))
                        .child(
                            div()
                                .absolute()
                                .bottom(px(-4.0))
                                .right(px(-2.0))
                                .text_color(cx.theme().muted_foreground)
                                .text_xs()
                                .child((idx + 1).to_string()),
                        )
                })
            })
    }
}

pub enum QueryTableEvent {
    Sort,
}

/// The state used with a [`QueryTable`].
pub struct QueryTableState {
    /// The state for the table.
    table: Entity<TableState<QueryTableDelegate>>,
}

impl EventEmitter<QueryTableEvent> for QueryTableState {}

impl QueryTableState {
    /// Creates a new [`QueryTableState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = QueryTableDelegate::new();
        let table = cx.new(|cx| TableState::new(delegate, window, cx).cell_selectable(true));

        cx.subscribe(&table, |this, _, event, cx| {
            if let TableEvent::SelectColumn(column_idx) = event {
                this.sort_column(cx, *column_idx);
            }
        })
        .detach();

        Self { table }
    }

    /// Creates a new [`QueryTableState`], initialized with the given [`QueryResult`].
    pub fn with_result(window: &mut Window, cx: &mut Context<Self>, result: QueryResult) -> Self {
        let mut state = Self::new(window, cx);
        state.init(cx, result);
        state
    }

    /// Initializes the table with the given [`QueryResult`].
    pub fn init(&mut self, cx: &mut Context<Self>, result: QueryResult) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().init(cx, result);
            table.refresh(cx);
        });
    }

    /// Returns an SQL ORDER BY string generated from the currently sorted columns.
    pub fn order_by(&self, cx: &App) -> String {
        let column_sorts = &self.table.read(cx).delegate().column_sorts;
        if column_sorts.is_empty() {
            return String::new();
        }

        let mut orders = Vec::new();
        for (name, sort) in self.table.read(cx).delegate().column_sorts.iter() {
            orders.push(format!(
                "{} {}",
                name,
                if *sort == ColumnSort::Ascending {
                    "ASC"
                } else {
                    "DESC"
                }
            ));
        }

        format!("ORDER BY {}", orders.join(", "))
    }

    /// Sorts the column with the given index.
    ///
    /// This will handle updating the column's sort direction and
    /// emit an event for implementers of this component to handle.
    fn sort_column(&mut self, cx: &mut Context<Self>, column_idx: usize) {
        self.table.update(cx, |table, _| {
            table.delegate_mut().sort_column(column_idx);
        });

        cx.emit(QueryTableEvent::Sort);
    }

    /// Sorts the data currently loaded within the table using the currently sorted columns.
    ///
    /// Note that this function sorts the data in-memory, so slowdowns may be a worry for larger amounts of data.
    pub fn sort_data(&mut self, cx: &mut Context<Self>) {
        self.table.update(cx, |table, cx| {
            let Some(result) = &table.delegate().result else {
                return;
            };

            // Remove any saved display order if there are no currently sorted columns.
            if table.delegate().column_sorts.is_empty() {
                table.delegate_mut().row_display_order = None;
                return;
            }

            let column_sorts = table.delegate().column_sorts.clone();
            let data = result.data.clone();
            cx.spawn(async move |table, cx| {
                let mut order: Vec<usize> = (0..data.rows().len()).collect();
                order.sort_unstable_by(|a, b| {
                    let mut ordering = Ordering::Equal;
                    let a_row = &data.rows()[*a];
                    let b_row = &data.rows()[*b];

                    for (name, sort) in column_sorts.iter() {
                        let column_idx = data.column_index(name);
                        ordering = ordering.then_with(|| {
                            let a_value = &a_row.values[column_idx];
                            let b_value = &b_row.values[column_idx];
                            let partial_ord = if let ColumnSort::Ascending = sort {
                                a_value.partial_cmp(b_value)
                            } else {
                                b_value.partial_cmp(a_value)
                            };

                            partial_ord.unwrap_or(Ordering::Equal)
                        });

                        if ordering != Ordering::Equal {
                            break;
                        }
                    }

                    ordering
                });

                _ = table.update(cx, move |table, cx| {
                    table.delegate_mut().row_display_order = Some(order);
                    table.refresh(cx);
                    cx.notify();
                });
            })
            .detach();
        });
    }
}

#[derive(IntoElement)]
pub struct QueryTable {
    /// The state for the query table.
    state: Entity<QueryTableState>,
}

impl QueryTable {
    /// Creates a new [`QueryTable`].
    pub fn new(state: &Entity<QueryTableState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for QueryTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        DataTable::new(&state.table).bordered(false).small()
    }
}
