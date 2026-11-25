use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryOperator {
    Equals,
    NotEquals,
    Contains,
    StartsWith,
    EndsWith,
    GreaterThan,
    LessThan,
    Between,
    Regex,
    IsEmpty,
    IsNotEmpty,
}

impl QueryOperator {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Equals => "==",
            Self::NotEquals => "!=",
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::EndsWith => "ends_with",
            Self::GreaterThan => ">",
            Self::LessThan => "<",
            Self::Between => "between",
            Self::Regex => "regex",
            Self::IsEmpty => "is_empty",
            Self::IsNotEmpty => "is_not_empty",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "==" | "equals" | "eq" => Some(Self::Equals),
            "!=" | "not_equals" | "ne" => Some(Self::NotEquals),
            "contains" => Some(Self::Contains),
            "starts_with" => Some(Self::StartsWith),
            "ends_with" => Some(Self::EndsWith),
            ">" | "gt" => Some(Self::GreaterThan),
            "<" | "lt" => Some(Self::LessThan),
            "between" => Some(Self::Between),
            "regex" => Some(Self::Regex),
            "is_empty" => Some(Self::IsEmpty),
            "is_not_empty" => Some(Self::IsNotEmpty),
            _ => None,
        }
    }
}

impl fmt::Display for QueryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterCondition {
    pub field: String,
    pub operator: QueryOperator,
    pub value: String,
    pub value_to: Option<String>,
}

impl FilterCondition {
    pub fn new(
        field: impl Into<String>,
        operator: QueryOperator,
        value: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            operator,
            value: value.into(),
            value_to: None,
        }
    }

    pub fn new_between(
        field: impl Into<String>,
        val_from: impl Into<String>,
        val_to: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            operator: QueryOperator::Between,
            value: val_from.into(),
            value_to: Some(val_to.into()),
        }
    }

    pub fn evaluate(&self, actual_val: &str) -> bool {
        match self.operator {
            QueryOperator::Equals => actual_val == self.value,
            QueryOperator::NotEquals => actual_val != self.value,
            QueryOperator::Contains => actual_val.contains(&self.value),
            QueryOperator::StartsWith => actual_val.starts_with(&self.value),
            QueryOperator::EndsWith => actual_val.ends_with(&self.value),
            QueryOperator::GreaterThan => {
                if let (Ok(act_num), Ok(val_num)) =
                    (actual_val.parse::<f64>(), self.value.parse::<f64>())
                {
                    act_num > val_num
                } else {
                    actual_val > &self.value
                }
            }
            QueryOperator::LessThan => {
                if let (Ok(act_num), Ok(val_num)) =
                    (actual_val.parse::<f64>(), self.value.parse::<f64>())
                {
                    act_num < val_num
                } else {
                    actual_val < &self.value
                }
            }
            QueryOperator::Between => {
                if let Some(to) = &self.value_to {
                    if let (Ok(act_num), Ok(f_num), Ok(t_num)) = (
                        actual_val.parse::<f64>(),
                        self.value.parse::<f64>(),
                        to.parse::<f64>(),
                    ) {
                        act_num >= f_num && act_num <= t_num
                    } else {
                        actual_val >= &self.value && actual_val <= to.as_str()
                    }
                } else {
                    false
                }
            }
            QueryOperator::Regex => {
                // Defensive: simple substring check or basic matching if regex crate is not loaded
                actual_val.contains(&self.value)
            }
            QueryOperator::IsEmpty => actual_val.trim().is_empty(),
            QueryOperator::IsNotEmpty => !actual_val.trim().is_empty(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AggregationType {
    Count,
    CountDistinct,
    Sum,
    Avg,
    Min,
    Max,
    Quantile(f64),
    RatePerSecond,
}

impl AggregationType {
    pub fn name(&self) -> String {
        match self {
            Self::Count => "count".to_string(),
            Self::CountDistinct => "count_distinct".to_string(),
            Self::Sum => "sum".to_string(),
            Self::Avg => "avg".to_string(),
            Self::Min => "min".to_string(),
            Self::Max => "max".to_string(),
            Self::Quantile(q) => format!("p{}", (q * 100.0) as u32),
            Self::RatePerSecond => "rate_per_sec".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AggregateField {
    pub field: String,
    pub agg_type: AggregationType,
    pub alias: String,
}

impl AggregateField {
    pub fn new(field: impl Into<String>, agg_type: AggregationType) -> Self {
        let f = field.into();
        let alias = format!("{}_{}", agg_type.name(), f);
        Self {
            field: f,
            agg_type,
            alias,
        }
    }

    pub fn with_alias(
        field: impl Into<String>,
        agg_type: AggregationType,
        alias: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            agg_type,
            alias: alias.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeRangeFilter {
    pub start_nanos: Option<i64>,
    pub end_nanos: Option<i64>,
}

impl TimeRangeFilter {
    pub fn all() -> Self {
        Self {
            start_nanos: None,
            end_nanos: None,
        }
    }

    pub fn between(start: i64, end: i64) -> Self {
        Self {
            start_nanos: Some(start),
            end_nanos: Some(end),
        }
    }

    pub fn contains(&self, ts_nanos: i64) -> bool {
        if let Some(s) = self.start_nanos {
            if ts_nanos < s {
                return false;
            }
        }
        if let Some(e) = self.end_nanos {
            if ts_nanos > e {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvaluationMode {
    Exact,
    Approximate { max_error_rate: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyticsQuery {
    pub title: String,
    pub target_doc_id: usize,
    pub time_range: TimeRangeFilter,
    pub filters: Vec<FilterCondition>,
    pub group_by: Vec<String>,
    pub aggregates: Vec<AggregateField>,
    pub sort_by: Option<(String, bool)>, // (field, is_ascending)
    pub limit: usize,
    pub mode: EvaluationMode,
}

impl Default for AnalyticsQuery {
    fn default() -> Self {
        Self {
            title: "Default Query".to_string(),
            target_doc_id: 0,
            time_range: TimeRangeFilter::all(),
            filters: Vec::new(),
            group_by: Vec::new(),
            aggregates: vec![AggregateField::new("*", AggregationType::Count)],
            sort_by: None,
            limit: 100,
            mode: EvaluationMode::Exact,
        }
    }
}

impl AnalyticsQuery {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn add_filter(&mut self, cond: FilterCondition) -> &mut Self {
        self.filters.push(cond);
        self
    }

    pub fn add_group_by(&mut self, field: impl Into<String>) -> &mut Self {
        self.group_by.push(field.into());
        self
    }

    pub fn add_aggregate(&mut self, agg: AggregateField) -> &mut Self {
        self.aggregates.push(agg);
        self
    }

    pub fn set_limit(&mut self, limit: usize) -> &mut Self {
        self.limit = limit;
        self
    }

    pub fn set_mode(&mut self, mode: EvaluationMode) -> &mut Self {
        self.mode = mode;
        self
    }

    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str(&format!("  \"title\": \"{}\",\n", self.title));
        out.push_str(&format!("  \"target_doc_id\": {},\n", self.target_doc_id));
        out.push_str(&format!("  \"limit\": {},\n", self.limit));
        out.push_str(&format!(
            "  \"mode\": \"{}\",\n",
            match self.mode {
                EvaluationMode::Exact => "Exact".to_string(),
                EvaluationMode::Approximate { max_error_rate } =>
                    format!("Approximate:{}", max_error_rate),
            }
        ));
        out.push_str("  \"group_by\": [");
        for (i, gb) in self.group_by.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("\"{}\"", gb));
        }
        out.push_str("],\n");
        out.push_str("  \"filters_count\": ");
        out.push_str(&self.filters.len().to_string());
        out.push_str(",\n  \"aggregates_count\": ");
        out.push_str(&self.aggregates.len().to_string());
        out.push_str("\n}");
        out
    }
}
