use std::{cmp::Ordering, collections::HashMap};

use futures_util::StreamExt;
use gpui::{
    Action, App, AppContext, Context, DispatchPhase, Entity, EventEmitter, InteractiveElement,
    IntoElement, KeyBinding, ParentElement, Pixels, RenderOnce, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::FluentBuilder, px,
};
use gpui_component::{
    ActiveTheme, Sizable, h_flex,
    progress::Progress,
    table::{Column, ColumnSort, DataTable, TableDelegate, TableEvent, TableState},
};
use indexmap::IndexMap;
use migris::{
    Index, IndexKind, Value,
    data::{QueryData, QueryResult},
};

use crate::{
    components::{
        icon::{Icon, IconName},
        text_ellipsis,
    },
    settings::{Setting, SettingsManager},
    size::Size,
};

const TABLE_ID: &str = "QUERY_TABLE";

const INIT_BATCH_SIZE: usize = 1_000;
const LOAD_BATCH_SIZE: usize = 100;

const MIN_COLUMN_WIDTH: Pixels = px(100.0);
const MAX_COLUMN_WIDTH: Pixels = px(250.0);

const ROW_NUMBER_COLUMN_IDX: usize = 0;
const ROW_NUMBER_COLUMN_KEY: SharedString = SharedString::new_static("#");

/// Initializes configuration for the table component.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl--", QueryTableAction::DecreaseSize, Some(TABLE_ID)),
        KeyBinding::new("ctrl-=", QueryTableAction::IncreaseSize, Some(TABLE_ID)),
    ]);
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
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);
        let table = state.table.read(cx);

        // Handle scrolling events within the table for the purpose of zoom in/out.
        window.on_mouse_event({
            let state = self.state.clone();
            move |event: &ScrollWheelEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture
                    || !event.secondary()
                    || !state.read(cx).is_hovered
                {
                    return;
                }

                let delta_y = event.delta.pixel_delta(px(1.0)).y;
                let action = if delta_y < Pixels::ZERO {
                    QueryTableAction::DecreaseSize
                } else {
                    QueryTableAction::IncreaseSize
                };

                state.update(cx, |state, cx| {
                    state.handle_action(window, cx, &action);
                });
                cx.stop_propagation();
            }
        });

        div()
            .id(TABLE_ID)
            .key_context(TABLE_ID)
            .relative()
            .size_full()
            .child(div().absolute().top_0().left_0().size_full().child(
                DataTable::new(&state.table).bordered(false).map(|this| {
                    match SettingsManager::table_size(cx) {
                        Size::XSmall | Size::Small | Size::Medium => this.xsmall(),
                        Size::Large | Size::XLarge => this.small(),
                        Size::XXLarge => this,
                        Size::XXXLarge => this.large(),
                    }
                }),
            ))
            .when(table.delegate().loading, |this| {
                this.child(
                    div().absolute().top_0().left_0().w_full().child(
                        Progress::new("table-loading")
                            .color(cx.theme().primary)
                            .loading(true)
                            .xsmall(),
                    ),
                )
            })
            .on_action(
                window.listener_for(&self.state, |state, action, window, cx| {
                    state.handle_action(window, cx, action);
                }),
            )
            .on_hover(
                window.listener_for(&self.state, |state, is_hovered, _, cx| {
                    state.is_hovered = *is_hovered;
                    cx.notify();
                }),
            )
    }
}

#[derive(Action, Clone, Copy, PartialEq, Eq)]
#[action(no_json)]
enum QueryTableAction {
    DecreaseSize,
    IncreaseSize,
}

pub enum QueryTableEvent {
    Sort,
}

/// The state used with a [`QueryTable`].
pub struct QueryTableState {
    /// The state for the table.
    table: Entity<TableState<QueryTableDelegate>>,

    /// Whether the table is hovered over.
    is_hovered: bool,

    /// The subscription used to listen to any settings updates.
    _settings_subscription: Subscription,
}

impl EventEmitter<QueryTableEvent> for QueryTableState {}

impl QueryTableState {
    /// Creates a new [`QueryTableState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = QueryTableDelegate::new();
        let table = cx.new(|cx| {
            TableState::new(delegate, window, cx)
                .cell_selectable(true)
                .row_header(false)
        });

        let _settings_subscription = SettingsManager::subscribe(cx, {
            let table = table.clone();
            move |cx, setting| {
                if let Setting::TableSize = setting {
                    // Resize row number column after table resize.
                    table.update(cx, |table, cx| {
                        table.delegate_mut().build_row_number_column(cx);
                        table.refresh(cx);
                    });
                }
            }
        });

        cx.subscribe(&table, |this, _, event, cx| match event {
            TableEvent::ColumnWidthsChanged(widths) => {
                this.table.update(cx, |table, _| {
                    table.delegate_mut().resize_columns(widths);
                });
            }
            TableEvent::SelectColumn(column_idx) => this.sort_column(cx, *column_idx),
            _ => {}
        })
        .detach();

        Self {
            table,
            is_hovered: false,
            _settings_subscription,
        }
    }

    /// Creates a new [`QueryTableState`] initialized with the given [`QueryResult`].
    pub fn with_result(window: &mut Window, cx: &mut Context<Self>, result: QueryResult) -> Self {
        let mut state = Self::new(window, cx);
        state.init(cx, result);
        state
    }

    /// Handles actions originating from the table.
    fn handle_action(&mut self, _: &mut Window, cx: &mut Context<Self>, action: &QueryTableAction) {
        match action {
            QueryTableAction::DecreaseSize | QueryTableAction::IncreaseSize => {
                let current_size = SettingsManager::table_size(cx);
                let new_size = match action {
                    QueryTableAction::DecreaseSize => current_size.decrease(),
                    QueryTableAction::IncreaseSize => current_size.increase(),
                };

                if current_size != new_size {
                    SettingsManager::set_table_size(cx, new_size);
                    SettingsManager::save(cx);
                    cx.notify();
                }
            }
        }
    }

    /// Initializes the table with the given [`QueryResult`].
    pub fn init(&mut self, cx: &mut Context<Self>, result: QueryResult) {
        self.table.update(cx, |table, cx| {
            let is_result_stream = result.stream.is_some();
            table.delegate_mut().init(cx, result);

            // If the query result is not using a stream for its data, we want to refresh
            // the table here after initialization. Otherwise, the table refresh is delegated
            // to after initial data is loaded from the stream inside the table delegate.
            if !is_result_stream {
                table.refresh(cx);
            }
        });
    }

    /// Builds the column index map from the given indexes.
    pub fn build_column_index_map(&mut self, cx: &mut Context<Self>, indexes: &[Index]) {
        self.table.update(cx, |table, cx| {
            table.delegate_mut().build_column_index_map(indexes);
            cx.notify();
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
        // Only perform the work for sorting the column if the
        // table is not already doing work that would conflict.
        if self.table.read(cx).delegate().loading {
            return;
        }

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
            table.delegate_mut().loading = true;

            cx.spawn(async move |table, cx| {
                let order = tokio::task::spawn_blocking(move || {
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

                    order
                })
                .await
                .unwrap();

                _ = table.update(cx, move |table, cx| {
                    table.delegate_mut().loading = false;
                    table.delegate_mut().row_display_order = Some(order);
                    table.refresh(cx);
                    cx.notify();
                });
            })
            .detach();
        });
    }
}

struct QueryTableDelegate {
    /// The query result to display in the table.
    result: Option<QueryResult>,

    /// The buffered query result to initialize before displaying in the table.
    ///
    /// This will be moved to [`Self::result`] after it is initialized with data.
    result_buffer: Option<QueryResult>,

    /// The columns for the table.
    columns: Vec<Column>,

    /// Tracks the index kind to display for columns.
    column_index_map: HashMap<SharedString, IndexKind>,

    /// Tracks the sort direction and order for columns being actively sorted.
    column_sorts: IndexMap<SharedString, ColumnSort>,

    /// Whether more data is available to load into the table.
    has_more_data: bool,

    /// Whether loading work, such as sorting, is being performed.
    loading: bool,

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
            result_buffer: None,
            columns: Vec::new(),
            column_index_map: HashMap::new(),
            column_sorts: IndexMap::new(),
            has_more_data: false,
            loading: false,
            row_display_order: None,
        }
    }

    /// Initializes the table with the given [`QueryResult`].
    fn init(&mut self, cx: &mut Context<TableState<Self>>, result: QueryResult) {
        let is_first_load = self.result.is_none();
        let is_result_stream = result.stream.is_some();

        if is_result_stream {
            // For results using a stream, we need to load initial data to show in the table.
            // We place the result into a buffer here so the data loading can be performed
            // before we show the result inside the table.
            self.result_buffer = Some(result);
            self.has_more_data = true;
            self.load(cx, INIT_BATCH_SIZE, is_first_load);
        } else {
            // Results without a stream can be shown directly inside the table without pre-loading.
            self.result = Some(result);

            if is_first_load {
                self.build_columns(cx);
            }
        }
    }

    /// Builds the columns for the table.
    fn build_columns(&mut self, cx: &mut App) {
        let Some(data) = self.data() else {
            return;
        };

        let font_size = SettingsManager::table_size(cx).font_size(cx);
        let mut columns: Vec<Column> = data
            .columns()
            .iter()
            .map(|column| {
                let name = column.name().to_owned();
                let width =
                    (name.len() * font_size * 0.60).clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH);

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
                let width =
                    (value.len() * font_size * 0.60).clamp(MIN_COLUMN_WIDTH, MAX_COLUMN_WIDTH);

                // Set the column's new width.
                if width > column.width {
                    column.width = width;
                }
            }
        }

        self.columns = columns;
        self.build_row_number_column(cx);
    }

    /// Builds the column index map using the given indexes.
    fn build_column_index_map(&mut self, indexes: &[Index]) {
        // Retrieve all index kinds for each column.
        let column_indexes: HashMap<&str, Vec<IndexKind>> =
            indexes.iter().fold(HashMap::new(), |mut map, index| {
                for column in index.columns() {
                    map.entry(column).or_default().push(index.kind());
                }
                map
            });

        // Using the previous map, find the highest-priority index kind to display for each column.
        self.column_index_map = column_indexes
            .iter()
            .map(|(key, value)| (SharedString::new(key), *value.iter().min().unwrap()))
            .collect();
    }

    /// Builds the row number column for the table.
    fn build_row_number_column(&mut self, cx: &mut App) {
        let Some(data) = self.data() else {
            return;
        };

        if self.columns.is_empty() {
            return;
        }

        let data_len = data.rows().len().to_string();
        let font_size = SettingsManager::table_size(cx).font_size(cx);
        let width =
            (data_len.len() * font_size * 0.75).clamp(MIN_COLUMN_WIDTH / 2.0, MAX_COLUMN_WIDTH);

        if self.columns[ROW_NUMBER_COLUMN_IDX].key == ROW_NUMBER_COLUMN_KEY {
            self.columns[ROW_NUMBER_COLUMN_IDX].width = width;
        } else {
            let column = Column::new(ROW_NUMBER_COLUMN_KEY, ROW_NUMBER_COLUMN_KEY)
                .movable(false)
                .resizable(false)
                .selectable(false)
                .width(width);

            self.columns.insert(ROW_NUMBER_COLUMN_IDX, column);
        }
    }

    /// Returns a reference to the query data.
    fn data(&self) -> Option<&QueryData> {
        let Some(result) = &self.result else {
            return None;
        };

        Some(&result.data)
    }

    /// Loads a number of rows from the query result's stream.
    ///
    /// This will prioritize loading from the buffered result if one exists.
    fn load(&mut self, cx: &mut Context<TableState<Self>>, num_rows: usize, is_first_load: bool) {
        let result = if let Some(result) = &mut self.result_buffer {
            result
        } else if let Some(result) = &mut self.result {
            result
        } else {
            return;
        };

        let Some(stream) = result.stream.take() else {
            return;
        };

        self.loading = true;
        cx.spawn(async move |table, cx| {
            let mut stream = stream;
            let mut rows = Vec::with_capacity(num_rows);

            for _ in 0..num_rows {
                if let Some(Ok(row)) = stream.next().await {
                    rows.push(row);
                }
            }

            // Determine if the stream has more data to load after.
            let mut peekable = Box::pin(stream.peekable());
            let has_more_data = peekable.as_mut().peek().await.is_some();

            _ = table.update(cx, move |table, cx| {
                if let Some(mut result) = table.delegate_mut().result_buffer.take() {
                    result.extend(rows);
                    result.stream = Some(peekable);
                    table.delegate_mut().result = Some(result);
                } else if let Some(result) = &mut table.delegate_mut().result {
                    result.extend(rows);
                    result.stream = Some(peekable);
                }

                if is_first_load {
                    table.delegate_mut().build_columns(cx);
                }

                table.delegate_mut().build_row_number_column(cx);
                table.delegate_mut().has_more_data = has_more_data;
                table.delegate_mut().loading = false;
                table.refresh(cx);
                cx.notify();
            });
        })
        .detach();
    }

    /// Resizes the columns with the given widths.
    fn resize_columns(&mut self, widths: &[Pixels]) {
        for (idx, width) in widths.iter().enumerate() {
            self.columns[idx].width = *width;
        }
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
        !self.loading && self.has_more_data
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<TableState<Self>>) {
        self.load(cx, LOAD_BATCH_SIZE, false);
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
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(data) = self.data() else {
            return div();
        };

        let font_size = SettingsManager::table_size(cx).font_size(cx);
        if col_ix == ROW_NUMBER_COLUMN_IDX {
            return div()
                .size_full()
                .pr_1p5()
                .border_r_1()
                .border_color(cx.theme().foreground)
                .content_center()
                .text_color(cx.theme().muted_foreground)
                .text_right()
                .text_size(font_size)
                .child((row_ix + 1).to_string());
        }

        let row_ix = if let Some(display_order) = &self.row_display_order {
            display_order[row_ix]
        } else {
            row_ix
        };

        let row = &data.rows()[row_ix];
        let value = &row.values[col_ix - 1];
        let color = match value {
            Value::Bytes(_) => cx.theme().magenta,
            Value::Date(_) | Value::Time(_) => cx.theme().red,
            Value::Decimal(_)
            | Value::F32(_)
            | Value::F64(_)
            | Value::I8(_)
            | Value::I16(_)
            | Value::I32(_)
            | Value::I64(_)
            | Value::U8(_)
            | Value::U16(_)
            | Value::U32(_)
            | Value::U64(_) => cx.theme().blue,
            Value::String(_) => cx.theme().green,
            _ => cx.theme().foreground,
        };

        div()
            .size_full()
            .content_center()
            .text_color(color)
            .text_size(font_size)
            .child(text_ellipsis(value.to_string()))
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let table_size = SettingsManager::table_size(cx);
        let font_size = table_size.font_size(cx);
        if col_ix == ROW_NUMBER_COLUMN_IDX {
            return div()
                .w_full()
                .text_color(cx.theme().muted_foreground)
                .text_size(font_size)
                .child("#");
        }

        let column = &self.columns[col_ix];
        let column_index_kind = self.column_index_map.get(&column.key);
        let column_sort = self.column_sorts.get_full(&column.key);

        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .text_size(font_size)
            .child(
                h_flex()
                    .gap_1()
                    .min_w_0()
                    .items_center()
                    .text_color(cx.theme().foreground)
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(col_ix.to_string()),
                    )
                    .child(text_ellipsis(column.name.clone())),
            )
            .child(
                h_flex()
                    .gap_1()
                    .pl_0p5()
                    .items_center()
                    .when_some(column_index_kind, |this, kind| {
                        this.child(match kind {
                            IndexKind::Primary => Icon::yellow(cx, IconName::KeyRound),
                            IndexKind::Regular => Icon::green(cx, IconName::KeyRound),
                            IndexKind::Unique => Icon::red(cx, IconName::KeyRound),
                        })
                    })
                    .when_some(column_sort, |this, (sort_idx, _, sort)| {
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
                                        .map(|this| match table_size {
                                            Size::XSmall | Size::Small | Size::Medium => {
                                                this.text_xs()
                                            }
                                            Size::Large => this.text_sm(),
                                            Size::XLarge => this.text_base(),
                                            Size::XXLarge => this.text_lg(),
                                            Size::XXXLarge => this.text_xl(),
                                        })
                                        .child((sort_idx + 1).to_string()),
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
                                        .map(|this| match table_size {
                                            Size::XSmall | Size::Small | Size::Medium => {
                                                this.text_xs()
                                            }
                                            Size::Large => this.text_sm(),
                                            Size::XLarge => this.text_base(),
                                            Size::XXLarge => this.text_lg(),
                                            Size::XXXLarge => this.text_xl(),
                                        })
                                        .child((sort_idx + 1).to_string()),
                                )
                        })
                    }),
            )
    }
}
