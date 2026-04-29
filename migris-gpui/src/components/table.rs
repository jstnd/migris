use futures_lite::StreamExt;
use gpui::{App, AppContext, Context, Entity, IntoElement, RenderOnce, Window};
use gpui_component::{
    Sizable,
    table::{Column, DataTable, TableDelegate, TableState},
};
use migris::data::{QueryData, QueryResult};
use tokio::runtime::Handle;

struct QueryTableDelegate {
    result: QueryResult,
    columns: Vec<Column>,
}

impl QueryTableDelegate {
    /// Creates a new [`QueryTableDelegate`].
    fn new(result: QueryResult) -> Self {
        let columns = result
            .data
            .columns()
            .iter()
            .map(|column| {
                let name = column.name().to_owned();
                Column::new(&name, &name)
            })
            .collect();

        Self { result, columns }
    }

    /// Returns a reference to the query data.
    fn data(&self) -> &QueryData {
        &self.result.data
    }

    /// Loads a number of rows from the query result's stream, determined by the given size parameter.
    fn load(&mut self, size: usize) {
        //
        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                if let Some(stream) = &mut self.result.stream {
                    let mut stream = stream.take(size);

                    while let Some(row) = stream.next().await {
                        if let Ok(row) = row {
                            self.result.data.push_row(row);
                        }
                    }
                }
            });
        });
    }
}

impl TableDelegate for QueryTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.data().rows().len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let row = &self.data().rows()[row_ix];
        row.values[col_ix].to_string()
    }
}

/// The state used with a [`QueryTable`].
pub struct QueryTableState {
    /// The state for the table.
    table: Entity<TableState<QueryTableDelegate>>,
}

impl QueryTableState {
    /// Creates a new [`QueryTableState`].
    pub fn new(window: &mut Window, cx: &mut Context<Self>, result: QueryResult) -> Self {
        let delegate = QueryTableDelegate::new(result);
        let table = cx.new(|cx| TableState::new(delegate, window, cx).cell_selectable(true));

        Self { table }
    }

    /// Loads a number of rows from the query result's stream, determined by the given size parameter.
    pub fn load(&mut self, cx: &mut Context<Self>, size: usize) {
        self.table.update(cx, |table, _| {
            table.delegate_mut().load(size);
        })
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
