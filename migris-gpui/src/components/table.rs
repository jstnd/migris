use futures_lite::StreamExt;
use gpui::{App, AppContext, Context, Entity, IntoElement, ParentElement, RenderOnce, Window, div};
use gpui_component::{
    Sizable,
    table::{Column, DataTable, TableDelegate, TableState},
};
use migris::data::{QueryData, QueryResult};
use tokio::runtime::Handle;

const INIT_BATCH_SIZE: usize = 1_000;

struct QueryTableDelegate {
    /// The query result to display in the table.
    result: Option<QueryResult>,

    /// The columns for the table.
    columns: Vec<Column>,
}

impl QueryTableDelegate {
    /// Creates a new [`QueryTableDelegate`].
    fn new() -> Self {
        Self {
            result: None,
            columns: Vec::new(),
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
                    let mut stream = stream.take(rows);

                    while let Some(row) = stream.next().await {
                        if let Ok(row) = row {
                            result.data.push_row(row);
                        }
                    }
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
        div().child(row.values[col_ix].to_string())
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
        let mut delegate = QueryTableDelegate::new();
        delegate.init(result);

        let table = cx.new(|cx| TableState::new(delegate, window, cx).cell_selectable(true));
        Self { table }
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
