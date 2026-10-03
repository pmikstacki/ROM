//! Predicates deliberately vary selectivity and conjunction order.
use crate::fixture::{Dataset, Distribution};
use rom::{CompareOp, Direction, QuerySpec, json};

pub(crate) struct Case {
    pub name: &'static str,
    pub query: QuerySpec,
}
pub(crate) fn cases(dataset: Dataset, size: usize) -> Vec<Case> {
    let common = if dataset.distribution == Distribution::Skewed {
        "hot"
    } else {
        "category-00"
    };
    let narrow = size.saturating_sub(8) as u64;
    let selective = QuerySpec::all().compare("amount", CompareOp::Ge, json!(narrow));
    vec![
        Case {
            name: "selective_eq",
            query: QuerySpec::equal("amount", json!(size - 1)),
        },
        Case {
            name: "common_eq",
            query: QuerySpec::equal("category", json!(common)),
        },
        Case {
            name: "missing_eq",
            query: QuerySpec::equal("category", json!("absent-category")),
        },
        Case {
            name: "narrow_range",
            query: selective.clone(),
        },
        Case {
            name: "broad_range",
            query: QuerySpec::all().compare("amount", CompareOp::Ge, json!(0)),
        },
        Case {
            name: "sorted_page",
            query: QuerySpec::all()
                .compare("amount", CompareOp::Ge, json!(0))
                .order_by("title", Direction::Desc)
                .order_by("amount", Direction::Asc)
                .limit(20.min(size)),
        },
        Case {
            name: "conjunction_selective_first",
            query: selective.compare("category", CompareOp::Eq, json!(common)),
        },
        Case {
            name: "conjunction_common_first",
            query: QuerySpec::all()
                .compare("category", CompareOp::Eq, json!(common))
                .compare("amount", CompareOp::Ge, json!(narrow)),
        },
    ]
}
