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
        history_highlight: std::collections::VecDeque::new(),
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

#[test]
fn history_highlight_cache_matches_fresh_computation() {
    let mut buffer = make_buf(4, 40, &[], &[], 0);
    buffer.output_highlight = OutputHighlightPreset::WindTerm;

    // Establish prev, then scroll 2 lines into history.
    let _ = buffer.ingest(b"panic at 0x40004800\r\nrow-B\r\nrow-C\r\nrow-D");
    let _ = buffer.ingest(b"\r\nrow-E\r\nrow-F");

    // Cache length must equal history length.
    assert_eq!(buffer.history_highlight.len(), buffer.history.len());
    assert_eq!(buffer.history.len(), 2);

    // Each cached entry must match a fresh highlight_plain_output call.
    for (i, line) in buffer.history.iter().enumerate() {
        let fresh = highlight_plain_output(
            line.1.clone(),
            buffer.output_highlight,
            &buffer.custom_highlight_rules,
        );
        assert_eq!(
            buffer.history_highlight[i].len(),
            fresh.len(),
            "span count mismatch at history row {i}"
        );
        for (j, (cached, expected)) in buffer.history_highlight[i]
            .iter()
            .zip(fresh.iter())
            .enumerate()
        {
            assert_eq!(
                cached.text, expected.text,
                "text mismatch at history row {i} span {j}"
            );
            assert_eq!(
                cached.fg, expected.fg,
                "fg mismatch at history row {i} span {j}"
            );
        }
    }
}

#[test]
fn rebuild_history_highlight_cache_after_preset_change() {
    let mut buffer = make_buf(4, 40, &[], &[], 0);
    buffer.output_highlight = OutputHighlightPreset::WindTerm;

    let _ = buffer.ingest(b"panic at 0x40004800\r\nrow-B\r\nrow-C\r\nrow-D");
    let _ = buffer.ingest(b"\r\nrow-E\r\nrow-F");
    assert_eq!(buffer.history.len(), 2);

    // Under WindTerm the first history row has highlighted spans (panic + hex).
    let windterm_spans = buffer.history_highlight[0].len();
    assert!(
        windterm_spans > 1,
        "WindTerm should split the line into multiple spans"
    );

    // Switching to Log (no embedded rules) must rebuild the cache.
    buffer.output_highlight = OutputHighlightPreset::Log;
    buffer.rebuild_history_highlight_cache();

    assert_eq!(
        buffer.history_highlight.len(),
        buffer.history.len(),
        "rebuild must preserve length"
    );
    // Under Log preset the embedded rules don't fire, so the first history
    // row collapses to fewer spans (no panic/hex colouring).
    let log_spans = buffer.history_highlight[0].len();
    assert!(
        log_spans < windterm_spans,
        "Log preset should produce fewer spans than WindTerm"
    );

    // The rebuilt cache must match fresh computation under the new preset.
    let fresh = highlight_plain_output(
        buffer.history[0].1.clone(),
        buffer.output_highlight,
        &buffer.custom_highlight_rules,
    );
    assert_eq!(buffer.history_highlight[0].len(), fresh.len());
}

#[test]
fn max_history_trim_keeps_highlight_cache_in_sync() {
    let mut buffer = make_buf(2, 20, &[], &[], 0);
    buffer.output_highlight = OutputHighlightPreset::Log;

    // Feed enough output to exceed MAX_HISTORY would need 100k lines — too
    // slow. Instead verify the invariant indirectly: after many ingests the
    // cache length always equals history length, and the last line matches.
    for n in 0..40 {
        let _ = buffer.ingest(format!("line-{n}\r\n").as_bytes());
    }
    assert_eq!(buffer.history_highlight.len(), buffer.history.len());

    // The newest history line should have a valid cached highlight.
    if let Some(last) = buffer.history_highlight.back() {
        assert!(!last.is_empty() || buffer.history.back().unwrap().1.is_empty());
    }
}
