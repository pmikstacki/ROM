//! Standard codecs use the same Resource declaration and runtime as application fields.
use crate::studio_choices::Category;
use rom::{Action, Input, Resource, ResourceRef};
use rom_fields::{
    Color, Date, DateTime, Decimal, Email, JsonDocument, Multiline, Time, UnitValue, Url,
};

#[derive(Clone, Resource)]
#[resource(
    name = "field-showcase",
    label = "Field showcase",
    title_field = "name"
)]
pub struct FieldShowcase {
    #[resource(label = "Name")]
    pub name: String,
    #[resource(label = "Scheduled date")]
    pub date: Date,
    #[resource(label = "Local time")]
    pub time: Time,
    #[resource(label = "Recorded at")]
    pub recorded_at: DateTime,
    pub color: Color,
    pub email: Email,
    #[resource(label = "Website")]
    pub website: Url,
    #[resource(label = "Notes")]
    pub notes: Multiline,
    #[resource(label = "JSON document")]
    pub document: JsonDocument,
    #[resource(help = "Exact decimal. Ordering compares canonical text, not numeric magnitude.")]
    pub decimal: Decimal,
    #[resource(label = "Measurement")]
    pub measurement: UnitValue,
    pub tags: Vec<String>,
    #[resource(label = "Related task")]
    pub task: ResourceRef<crate::Task>,
    #[resource(label = "Attachment")]
    pub attachment: Option<ResourceRef<rom_blob::Blob>>,
    pub related: Option<ResourceRef<FieldShowcase>>,
    pub category: Option<Category>,
    pub categories: Option<Vec<Category>>,
}

pub fn example() -> rom::Result<FieldShowcase> {
    Ok(FieldShowcase {
        name: "Workshop sample".into(),
        date: Date::new("2026-10-05")?,
        time: Time::new("09:30:00")?,
        recorded_at: DateTime::new("2026-10-05T09:30:00+02:00")?,
        color: Color::new("#6366f1")?,
        email: Email::new("workshop@example.com")?,
        website: Url::new("https://example.com/workshop")?,
        notes: Multiline::new("Inspect the pump.\nRecord the result.")?,
        document: JsonDocument::new(r#"{"sequence":9007199254740993,"approved":false}"#)?,
        decimal: Decimal::new("12345678901234567890.123456789")?,
        measurement: UnitValue::new(Decimal::new("12.5")?, "kg")?,
        tags: vec!["Inspection".into(), "Workshop".into()],
        task: ResourceRef::new("task-a")?,
        attachment: None,
        related: None,
        category: Some(Category::Inspection),
        categories: Some(vec![Category::Inspection, Category::Maintenance]),
    })
}

/// The action form uses the same semantic input descriptor as ordinary fields.
pub const MEASURE: Action<FieldShowcase, UnitValue> = Action::new("measure", |resource, value| {
    resource.measurement = value;
    Ok(vec![])
});

/// Named action inputs retain the same Field contracts as persisted values.
#[derive(Clone, Input)]
pub struct ShowcaseValues {
    pub date: Date,
    pub time: Time,
    pub recorded_at: DateTime,
    pub color: Color,
    pub email: Email,
    pub website: Url,
    pub notes: Multiline,
    pub document: JsonDocument,
    pub decimal: Decimal,
    pub measurement: UnitValue,
}
pub const SET_VALUES: Action<FieldShowcase, ShowcaseValues> =
    Action::new("set_values", |resource, values| {
        resource.date = values.date;
        resource.time = values.time;
        resource.recorded_at = values.recorded_at;
        resource.color = values.color;
        resource.email = values.email;
        resource.website = values.website;
        resource.notes = values.notes;
        resource.document = values.document;
        resource.decimal = values.decimal;
        resource.measurement = values.measurement;
        Ok(vec![])
    });

/// Enum display labels also reach standalone action forms without another schema.
pub const SET_CATEGORY: Action<FieldShowcase, Category> =
    Action::new("set_category", |resource, category| {
        resource.category = Some(category);
        Ok(vec![])
    });
