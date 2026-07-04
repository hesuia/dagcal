use crate::style::{reference_color, warning_color};
use dagcal_app::formatting::{ReferenceSegment, reference_segments};
use iced::{Color, Font};

pub(crate) use dagcal_app::formatting::{
    entry_expression_source, resolved_source, table_state_summary,
};

pub(crate) fn expression_spans(
    source: &str,
    entries: &[dagcal_app::EntryView],
) -> Vec<iced::widget::text::Span<'static, (), Font>> {
    reference_segments(source, entries)
        .into_iter()
        .map(|segment| {
            let color = segment_color(&segment);
            let mut span = iced::widget::span(segment.text().to_string());
            if let Some(color) = color {
                span = span.color(color);
            }
            span
        })
        .collect()
}

fn segment_color(segment: &ReferenceSegment) -> Option<Color> {
    match segment {
        ReferenceSegment::Value(_) => Some(reference_color()),
        ReferenceSegment::Error(_) | ReferenceSegment::Missing(_) => Some(warning_color()),
        ReferenceSegment::Text(_) => None,
    }
}
