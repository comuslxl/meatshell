use super::*;

fn hist_line(s: &str) -> Line {
    (s.to_string(), Vec::new(), false)
}

fn wrapped_hist_line(s: &str) -> Line {
    (s.to_string(), Vec::new(), true)
}

fn make_buf(
    rows: u16,
    cols: u16,
    history: &[&str],
    live_lines: &[&str],
    view_offset: usize,
) -> TermBuffer {
    let mut parser = vt100::Parser::new(rows, cols, 0);
    parser.process(live_lines.join("\r\n").as_bytes());
    TermBuffer {
        parser,
        find_query: String::new(),
        is_dark: false,
        output_highlight: OutputHighlightPreset::Log,
        custom_highlight_rules: Vec::new(),
        sel_anchor: None,
        sel_focus: None,
        sel_ranges: Vec::new(),
        history: history.iter().map(|s| hist_line(s)).collect(),
        prev: Vec::new(),
        view_offset,
        displayed_text: Vec::new(),
        csi_state: CsiState::Normal,
        csi_pending: Vec::new(),
        raw: std::collections::VecDeque::new(),
    }
}

mod colors;
mod protocol;
mod selection;
mod sftp_sorting;

#[test]
fn scrolled_viewport_stays_pinned_as_new_output_streams_in() {
    let mut buffer = make_buf(4, 20, &[], &[], 0);

    // Fill the screen; the first ingest sets `prev`, no scrollback yet.
    let _ = buffer.ingest(b"row-A\r\nrow-B\r\nrow-C\r\nrow-D");
    // Second ingest scrolls 2 lines (row-A, row-B) into history.
    let _ = buffer.ingest(b"\r\nrow-E\r\nrow-F");
    assert_eq!(buffer.history.len(), 2);

    // Simulate the user scrolling up one row.
    buffer.view_offset = 1;
    let _ = buffer.render();
    let displayed_before: Vec<String> = buffer.displayed_text.clone();

    // Stream more output — this scrolls 2 more lines into history.
    let _ = buffer.ingest(b"\r\nrow-G\r\nrow-H");
    let _ = buffer.render();
    let displayed_after: Vec<String> = buffer.displayed_text.clone();

    // The visible content must not shift.
    assert_eq!(displayed_before, displayed_after);
    // view_offset grew by exactly the 2 lines that entered history.
    assert_eq!(buffer.view_offset, 3);
}
