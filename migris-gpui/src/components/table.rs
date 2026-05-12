use futures_util::StreamExt;
use gpui::{
    App, AppContext, Context, Entity, IntoElement, ParentElement, RenderOnce, Styled, Window, div,
};
use gpui_component::{
    ActiveTheme, Sizable,
    table::{Column, DataTable, TableDelegate, TableState},
};
use migris::data::{QueryData, QueryResult};
use tokio::runtime::Handle;

use crate::components::text_ellipsis;

const INIT_BATCH_SIZE: usize = 1_000;
const LOAD_BATCH_SIZE: usize = 100;

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
    fn init(&mut self, result: QueryResult) {
        self.columns = result
            .data
            .columns()
            .iter()
            .map(|column| {
                let name = column.name().to_owned();
                Column::new(&name, &name)
            })
            .collect();

        self.result = Some(result);
        self.has_more_data = true;
        self.load(INIT_BATCH_SIZE);
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

        //
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

        div()
            .w_full()
            .text_color(cx.theme().foreground)
            .child(text_ellipsis(column.name.clone()))
    }

    fn rows_count(&self, _: &App) -> usize {
        let Some(data) = self.data() else {
            return 0;
        };

        data.rows().len()
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
            table.delegate_mut().init(result);
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
