use futures_util::StreamExt;
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, Pixels, RenderOnce, Styled,
    Window, div, px,
};
use gpui_component::{
    ActiveTheme, Sizable, h_flex,
    table::{Column, DataTable, TableDelegate, TableState},
};
use migris::data::{QueryData, QueryResult};
use tokio::runtime::Handle;

use crate::components::text_ellipsis;

const INIT_BATCH_SIZE: usize = 1_000;
const LOAD_BATCH_SIZE: usize = 100;

const MIN_COLUMN_WIDTH: Pixels = px(100.0);
const MAX_COLUMN_WIDTH: Pixels = px(250.0);

struct QueryTableDelegate {
    /// The query result to display in the table.
    result: Option<QueryResult>,

    /// The columns for the table.
    columns: Vec<Column>,

    /// Whether more data is available to load into the table.
    has_more_data: bool,
}

impl QueryTableDelegate {
    /// Creates a new [`QueryTableDelegate`].
    fn new() -> Self {
        Self {
            result: None,
            columns: Vec::new(),
            has_more_data: false,
        }
    }

    /// Initializes the table with the given [`QueryResult`].
    fn init(&mut self, cx: &mut App, result: QueryResult) {
        self.result = Some(result);
        self.has_more_data = true;
        self.load(INIT_BATCH_SIZE);
        self.build_columns(cx);
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
            Handle::current().block_on(async {
                if let Some(stream) = &mut result.stream {
                    let mut data_stream = stream.take(rows);

                    while let Some(row) = data_stream.next().await {
                        if let Ok(row) = row {
                            result.data.push_row(row);
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

        h_flex()
            .w_full()
            .gap_1()
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .text_color(cx.theme().muted_foreground)
                    .text_xs()
                    .child((col_ix + 1).to_string()),
            )
            .child(text_ellipsis(column.name.clone()))
    }
}

/// The state used with a [`QueryTable`].
pub struct QueryTableState {
    /// The state for the table.
    table: Entity<TableState<QueryTableDelegate>>,
}

impl QueryTableState {
    /// Creates a new [`QueryTableState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = QueryTableDelegate::new();
        let table = cx.new(|cx| TableState::new(delegate, window, cx).cell_selectable(true));
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
