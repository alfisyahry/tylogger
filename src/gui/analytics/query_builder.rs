use crate::core::analytics::query::{
    AggregateField, AggregationType, AnalyticsQuery, FilterCondition, QueryOperator,
};

pub struct QueryBuilderDialog {
    pub query: AnalyticsQuery,
    pub available_fields: Vec<String>,
}

impl QueryBuilderDialog {
    pub fn new(available_fields: Vec<String>) -> Self {
        Self {
            query: AnalyticsQuery::default(),
            available_fields,
        }
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.query.title = title.into();
    }

    pub fn add_condition(&mut self, field: &str, op: QueryOperator, val: &str) {
        self.query.add_filter(FilterCondition::new(field, op, val));
    }

    pub fn set_group_by(&mut self, field: &str) {
        self.query.group_by.clear();
        if !field.is_empty() {
            self.query.add_group_by(field);
        }
    }

    pub fn set_aggregation(&mut self, field: &str, agg_type: AggregationType) {
        self.query.aggregates.clear();
        self.query
            .add_aggregate(AggregateField::new(field, agg_type));
    }

    pub fn build(self) -> AnalyticsQuery {
        self.query
    }
}
