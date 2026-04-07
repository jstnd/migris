use gpui::{App, AppContext, Context, Entity, IntoElement, RenderOnce, Window};
use gpui_component::table::{Column, DataTable, TableDelegate, TableState};
use migris::QueryData;

struct QueryTableDelegate {
    data: QueryData,
    columns: Vec<Column>,
}

impl QueryTableDelegate {
    fn new(data: QueryData) -> Self {
        let columns = data
            .columns()
            .iter()
            .map(|column| {
                let name = column.name().to_owned();
                Column::new(&name, &name)
            })
            .collect();

        Self { data, columns }
    }
}

impl TableDelegate for QueryTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.data.rows().len()
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
        let row = &self.data.rows()[row_ix];
        row.values[col_ix].to_string()
    }
}

pub struct QueryTableState {
    table_state: Entity<TableState<QueryTableDelegate>>,
}

impl QueryTableState {
    pub fn new(window: &mut Window, cx: &mut Context<Self>, data: QueryData) -> Self {
        let delegate = QueryTableDelegate::new(data);
        let table_state = cx.new(|cx| TableState::new(delegate, window, cx).cell_selectable(true));

        Self { table_state }
    }
}

#[derive(IntoElement)]
pub struct QueryTable {
    state: Entity<QueryTableState>,
}

impl QueryTable {
    pub fn new(state: &Entity<QueryTableState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for QueryTable {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        DataTable::new(&state.table_state).bordered(false)
    }
}
