//! Lightweight, reusable GCode editor component (egui-based).
//!
//! Provides: EditorState, SyntaxColors, highlighting helpers and a `show_editor` UI
//! function that can be embedded into any egui `Ui`.

/// Syntax highlighting colors used by the editor
use std::collections::{HashMap, HashSet};

pub struct SyntaxColors {
    pub gcode: [f32; 4],
    pub mcode: [f32; 4],
    pub axis: [f32; 4],
    // Per-axis color overrides (keyed by uppercase axis letter). Allows callers to provide colors for arbitrary axes.
    pub axis_overrides: HashMap<char, [f32; 4]>,
    /// Set of characters that are treated as axis identifiers (upper-case).
    pub axis_chars: HashSet<char>,
    pub parameter: [f32; 4],
    // Color for parameter keys (left-side of `=`)
    pub parameter_key: [f32; 4],
    // Color for parameter values (right-side of `=`)
    pub parameter_value: [f32; 4],
    // Color for P parameter (e.g., loop count / parameter)
    pub p_parameter: [f32; 4],
    pub number: [f32; 4],
    pub comment: [f32; 4],
    pub ocode: [f32; 4],
    pub operator: [f32; 4],
    pub variable: [f32; 4],
    pub error: [f32; 4],
}

impl Default for SyntaxColors {
    fn default() -> Self {
        let mut axis_overrides = HashMap::new();
        axis_overrides.insert('X', [0.2, 0.9, 0.9, 1.0]); // cyan-ish
        axis_overrides.insert('Y', [0.4, 1.0, 0.4, 1.0]); // green
        axis_overrides.insert('Z', [1.0, 0.8, 0.0, 1.0]); // yellow/orange
        axis_overrides.insert('E', [0.8, 0.4, 1.0, 1.0]); // magenta-ish

        let mut axis_chars = HashSet::new();
        axis_chars.insert('X');
        axis_chars.insert('Y');
        axis_chars.insert('Z');
        axis_chars.insert('E');

        Self {
            gcode: [0.4, 0.8, 1.0, 1.0],
            mcode: [1.0, 0.6, 0.2, 1.0],
            axis: [0.4, 1.0, 0.4, 1.0],
            axis_overrides,
            axis_chars,
            parameter: [1.0, 1.0, 0.4, 1.0],
            parameter_key: [1.0, 0.9, 0.6, 1.0], // warm key color
            parameter_value: [0.8, 0.9, 1.0, 1.0], // cool value color
            p_parameter: [1.0, 0.6, 0.2, 1.0],   // distinct orange
            number: [0.8, 0.8, 0.8, 1.0],
            comment: [0.5, 0.5, 0.5, 1.0],
            ocode: [0.8, 0.4, 1.0, 1.0],
            operator: [1.0, 1.0, 1.0, 1.0],
            variable: [0.4, 0.6, 1.0, 1.0],
            error: [1.0, 0.3, 0.3, 1.0],
        }
    }
}

/// Lightweight editor state suitable for embedding in other apps
#[derive(Debug, Clone)]
pub struct EditorState {
    pub cursor_pos: (usize, usize),
    pub selection_start: Option<(usize, usize)>,
    pub selection_end: Option<(usize, usize)>,
    pub selected_lines: Option<(usize, usize)>,
    pub scroll_offset: f32,
    pub focused: bool,
    pub search_text: String,
    pub show_search: bool,

    // Optional gutter line numbers
    pub show_line_numbers: bool,
    // Active line if set (1-based index). External apps can set this to highlight a particular line.
    pub active_line: Option<usize>,

    // Optional active-line background highlight and its color (RGBA floats)
    pub show_active_line_bg: bool,
    pub active_line_bg: [f32; 4],

    // Marked error ranges: vector of `ErrorRange` entries (start_byte, end_byte, optional tooltip)
    // The library does not attempt to maintain these across content edits — the embedder should update ranges accordingly.
    pub error_ranges: Vec<ErrorRange>,

    // Internal monotonic id generator for error ranges
    #[doc(hidden)]
    pub _internal_next_error_id: u64,

    // Internal flag indicating the next UI pass should scroll to `active_line` if set.
    #[doc(hidden)]
    pub _internal_should_scroll_to_active_line: bool,
}

/// Represents an error highlight range in the content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRange {
    pub id: u64,
    pub start: usize,
    pub end: usize,
    pub tooltip: Option<String>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            cursor_pos: (0, 0),
            selection_start: None,
            selection_end: None,
            selected_lines: None,
            scroll_offset: 0.0,
            focused: false,
            search_text: String::new(),
            show_search: false,
            show_line_numbers: false,
            active_line: None,
            // Active line background highlight options
            show_active_line_bg: false,
            active_line_bg: [1.0, 0.95, 0.4, 0.15],
            error_ranges: Vec::new(),
            _internal_next_error_id: 1,
            _internal_should_scroll_to_active_line: false,
        }
    }
}

impl EditorState {
    /// Programmatically select a character range by byte indices into `content`.
    /// `start` and `end` are byte offsets; `scroll` controls whether the editor should
    /// also move the active line to the start of the selection to make it visible.
    pub fn set_selection_range_bytes(
        &mut self,
        start: usize,
        end: usize,
        content: &str,
        scroll: bool,
    ) {
        let s = start.min(content.len());
        let e = end.min(content.len());
        let (s_line, s_col) = byte_index_to_line_col(content, s);
        let (e_line, e_col) = byte_index_to_line_col(content, e);
        // store as 1-based line numbers, columns remain char indices
        self.selection_start = Some((s_line + 1, s_col));
        self.selection_end = Some((e_line + 1, e_col));
        self.selected_lines = Some((s_line + 1, e_line + 1));
        if scroll {
            self.active_line = Some(s_line + 1);
        }
    }

    /// Clear any selection
    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
        self.selected_lines = None;
    }

    /// Add an error range specified by byte indices in the content. `start` <= `end`.
    /// Returns the unique id of the created error range.
    pub fn add_error_range_bytes(&mut self, start: usize, end: usize) -> u64 {
        let s = start.min(end);
        let e = end.max(start);
        let id = self._internal_next_error_id;
        self._internal_next_error_id = self._internal_next_error_id.saturating_add(1);
        self.error_ranges.push(ErrorRange {
            id,
            start: s,
            end: e,
            tooltip: None,
        });
        id
    }

    /// Add an error range with a tooltip message. Returns the id.
    pub fn add_error_range_bytes_with_tooltip<T: Into<String>>(
        &mut self,
        start: usize,
        end: usize,
        tooltip: T,
    ) -> u64 {
        let s = start.min(end);
        let e = end.max(start);
        let id = self._internal_next_error_id;
        self._internal_next_error_id = self._internal_next_error_id.saturating_add(1);
        self.error_ranges.push(ErrorRange {
            id,
            start: s,
            end: e,
            tooltip: Some(tooltip.into()),
        });
        id
    }

    /// Replace all error ranges with the provided list.
    pub fn set_error_ranges(&mut self, ranges: Vec<ErrorRange>) {
        self.error_ranges = ranges;
    }

    /// Remove a specific error range by id. Returns true if removed.
    pub fn remove_error_range(&mut self, id: u64) -> bool {
        let orig = self.error_ranges.len();
        self.error_ranges.retain(|r| r.id != id);
        orig != self.error_ranges.len()
    }

    /// Update an existing error range. Returns true if updated.
    pub fn update_error_range(
        &mut self,
        id: u64,
        start: usize,
        end: usize,
        tooltip: Option<String>,
    ) -> bool {
        for r in &mut self.error_ranges {
            if r.id == id {
                r.start = start.min(end);
                r.end = end.max(start);
                r.tooltip = tooltip;
                return true;
            }
        }
        false
    }

    /// Clear all error ranges
    pub fn clear_error_ranges(&mut self) {
        self.error_ranges.clear();
    }

    /// Add an error range covering the current selection (if any) using the provided `content` string
    /// to map (line,col) to byte indices. Does nothing if no selection is present.
    pub fn add_error_from_selection(&mut self, content: &str) {
        if let (Some((s_line, s_col)), Some((e_line, e_col))) =
            (self.selection_start, self.selection_end)
        {
            let s_byte = line_col_to_byte_index(content, s_line.saturating_sub(1), s_col);
            let e_byte = line_col_to_byte_index(content, e_line.saturating_sub(1), e_col);
            self.add_error_range_bytes(s_byte, e_byte);
        }
    }

    /// Add an error range covering an entire (1-based) line number. If the line is out of range this is a no-op.
    pub fn add_error_for_line(&mut self, line_one_based: usize, content: &str) {
        let line_idx = line_one_based.saturating_sub(1);
        if let Some(line) = content.lines().nth(line_idx) {
            let start = line_col_to_byte_index(content, line_idx, 0);
            let end = start + line.len();
            self.add_error_range_bytes(start, end);
        }
    }

    /// Set selection by (1-based) line and character column indices.
    /// Lines are 1-based, columns are 0-based character indices within the line.
    /// If `scroll` is true the editor will move viewport to make the start visible.
    pub fn set_selection_range_line_col(
        &mut self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
        scroll: bool,
    ) {
        // Normalize ordering so selection_start <= selection_end
        let (s_line, s_col, e_line, e_col) = if (start_line, start_col) <= (end_line, end_col) {
            (start_line, start_col, end_line, end_col)
        } else {
            (end_line, end_col, start_line, start_col)
        };
        self.selection_start = Some((s_line, s_col));
        self.selection_end = Some((e_line, e_col));
        self.selected_lines = Some((s_line, e_line));
        if scroll {
            self.active_line = Some(s_line);
            self._internal_should_scroll_to_active_line = true;
        }
    }

    /// Add an error range specified by (1-based) line and character columns.
    /// Columns are character indices (0-based). If the line is out-of-range nothing happens.
    pub fn add_error_range_line_col(
        &mut self,
        line_one_based: usize,
        start_col: usize,
        end_col: usize,
        content: &str,
    ) {
        if line_one_based == 0 {
            return;
        }
        let line_idx = line_one_based.saturating_sub(1);
        if let Some(line) = content.lines().nth(line_idx) {
            // Clamp columns to the line
            let max_cols = line.chars().count();
            let s_col = start_col.min(max_cols);
            let e_col = end_col.min(max_cols);
            let s_byte = line_col_to_byte_index(content, line_idx, s_col);
            let e_byte = line_col_to_byte_index(content, line_idx, e_col);
            self.add_error_range_bytes(s_byte, e_byte);
        }
    }

    /// Return the selected text slice, if a selection exists. The slice borrows `content`.
    pub fn selected_text<'a>(&self, content: &'a str) -> Option<&'a str> {
        if let (Some((s_line, s_col)), Some((e_line, e_col))) =
            (self.selection_start, self.selection_end)
        {
            let s_byte = line_col_to_byte_index(content, s_line.saturating_sub(1), s_col);
            let e_byte = line_col_to_byte_index(content, e_line.saturating_sub(1), e_col);
            let (start, end) = if s_byte <= e_byte {
                (s_byte, e_byte)
            } else {
                (e_byte, s_byte)
            };
            return Some(&content[start..end]);
        }
        None
    }

    /// Scroll the viewport to a particular (1-based) line number. This sets `active_line` and requests a scroll.
    pub fn scroll_to_line(&mut self, line_one_based: usize) {
        self.active_line = Some(line_one_based);
        self._internal_should_scroll_to_active_line = true;
    }
}

/// Event emitted when editor content changes
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorChangeEvent {
    pub new_content: String,
    pub selected_lines: Option<(usize, usize)>,
}

/// Events emitted by the editor for callers to react to changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorEvent {
    /// Content changed (text modification)
    ContentChanged(EditorChangeEvent),
    /// Active line changed (old -> new). Values are 1-based line numbers.
    ActiveLineChanged {
        old: Option<usize>,
        new: Option<usize>,
    },
    /// Selected lines changed (old -> new)
    SelectionChanged {
        old: Option<(usize, usize)>,
        new: Option<(usize, usize)>,
    },
}

/// Convert a color array to egui Color32
fn array_to_color32(arr: &[f32; 4]) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (arr[0] * 255.0) as u8,
        (arr[1] * 255.0) as u8,
        (arr[2] * 255.0) as u8,
        (arr[3] * 255.0) as u8,
    )
}

/// Helper: return the color `egui::Color32` for a given token type using `SyntaxColors`.
///
/// Example:
/// ```rust
/// # use gcode_editor::{SyntaxColors, TokenType, color_for_token};
/// let colors = SyntaxColors::default();
/// let g_color = color_for_token(&colors, TokenType::GCode);
/// assert_eq!(g_color, color_for_token(&colors, TokenType::GCode));
/// ```
pub fn color_for_token(colors: &SyntaxColors, token_type: TokenType) -> egui::Color32 {
    match token_type {
        TokenType::GCode => array_to_color32(&colors.gcode),
        TokenType::MCode => array_to_color32(&colors.mcode),
        TokenType::Axis => array_to_color32(&colors.axis),
        TokenType::AxisNamed(c) => {
            let upper = c.to_ascii_uppercase();
            if let Some(col) = colors.axis_overrides.get(&upper) {
                array_to_color32(col)
            } else {
                array_to_color32(&colors.axis)
            }
        }
        TokenType::Parameter => array_to_color32(&colors.parameter),
        TokenType::ParameterKey => array_to_color32(&colors.parameter_key),
        TokenType::ParameterValue => array_to_color32(&colors.parameter_value),
        TokenType::ParameterP => array_to_color32(&colors.p_parameter),
        TokenType::Number => array_to_color32(&colors.number),
        TokenType::Comment => array_to_color32(&colors.comment),
        TokenType::OCode => array_to_color32(&colors.ocode),
        TokenType::Operator => array_to_color32(&colors.operator),
        TokenType::Variable => array_to_color32(&colors.variable),
        TokenType::Error => array_to_color32(&colors.error),
        TokenType::Unknown => egui::Color32::WHITE,
    }
}

/// Convert a byte index in `content` into (line_index, column_index) both zero-based.
fn byte_index_to_line_col(content: &str, mut idx: usize) -> (usize, usize) {
    for (i, line) in content.lines().enumerate() {
        let line_len = line.len(); // bytes
        if idx <= line_len {
            // Convert byte offset within this line to character column index
            let mut byte_pos = 0usize;
            for (c, ch) in line.chars().enumerate() {
                let ch_len = ch.len_utf8();
                if idx <= byte_pos {
                    return (i, c);
                }
                byte_pos += ch_len;
            }
            return (i, line.chars().count());
        }
        // Move past this line (+1 for the newline removed by lines())
        idx = idx.saturating_sub(line_len + 1);
    }
    // Out-of-range -> last position
    let last_line = content.lines().count().saturating_sub(1);
    let last_col = content
        .lines()
        .last()
        .map(|l| l.chars().count())
        .unwrap_or(0);
    (last_line, last_col)
}

/// Convert (line-based, col_chars) pair to byte index into content. If the requested line/col
/// is past the end of content it will clamp to content.len().
fn line_col_to_byte_index(content: &str, line_idx: usize, col: usize) -> usize {
    let mut cur = 0usize; // byte offset consumed
    for (i, line) in content.lines().enumerate() {
        if i == line_idx {
            // Walk chars until col reached
            let mut byte_pos = 0usize;
            for (c, ch) in line.chars().enumerate() {
                if c >= col {
                    break;
                }
                byte_pos += ch.len_utf8();
            }
            return cur + byte_pos;
        }
        cur += line.len() + 1; // account for removed newline
    }
    content.len()
}
// Simple tokenizer fallback so the editor can function without the C++ highlighter.
// This is intentionally small and conservative; consumers that want exact parity
// with the main app should pass tokens in via callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    GCode,
    MCode,
    OCode,
    // Generic axis token when a specific axis isn't recognized
    Axis,
    /// Named axis token, e.g., `AxisNamed('X')`.
    AxisNamed(char),
    Parameter,
    // Parameter key (left of '=')
    ParameterKey,
    // Parameter value (right of '=')
    ParameterValue,
    // Parameter-specific token for 'P' which is often semantically distinct
    ParameterP,
    Number,
    Comment,
    Operator,
    Variable,
    Error,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub start: usize,
    pub length: usize,
    pub token_type: TokenType,
}

/// Very small pure-Rust tokenizer used for highlighting when C++ isn't available.
use std::borrow::Cow;

pub fn tokenize_line_pure_rust(line: &str, axis_chars: Option<&HashSet<char>>) -> Vec<Token> {
    let axis_cow: Cow<'_, HashSet<char>> = if let Some(s) = axis_chars {
        Cow::Borrowed(s)
    } else {
        let mut def = HashSet::new();
        def.insert('X');
        def.insert('Y');
        def.insert('Z');
        def.insert('E');
        Cow::Owned(def)
    };
    let axis_chars_ref: &HashSet<char> = axis_cow.as_ref();

    let mut tokens = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == ';' || c == '(' {
            let start = i;
            if c == '(' {
                // find matching )
                while i < chars.len() && chars[i] != ')' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
            } else {
                i = chars.len();
            }
            tokens.push(Token {
                start,
                length: i - start,
                token_type: TokenType::Comment,
            });
            continue;
        }
        if c == 'G' || c == 'g' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(Token {
                start,
                length: i - start,
                token_type: TokenType::GCode,
            });
            continue;
        }
        if c == 'M' || c == 'm' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(Token {
                start,
                length: i - start,
                token_type: TokenType::MCode,
            });
            continue;
        }
        // O-codes (subroutine / program numbers) like O100
        if c == 'O' || c == 'o' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(Token {
                start,
                length: i - start,
                token_type: TokenType::OCode,
            });
            continue;
        }
        if c.is_ascii_alphabetic() {
            // Handle KEY=VALUE style parameters (allow multi-letter keys). Support quoted strings and
            // bracketed/vector forms in the value (e.g., KEY="str", KEY=[1,2,3], KEY=(1 2 3)).
            let start = i;
            // read identifier (letters only for keys/commands; digits belong to numeric values)
            while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '_') {
                i += 1;
            }
            let id_end = i;
            // skip optional spaces before '='
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            if i < chars.len() && chars[i] == '=' {
                // parse value after '='
                i += 1; // skip '='
                        // skip optional spaces
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                if i < chars.len() {
                    // capture key, operator, and value as separate tokens
                    // key: start .. id_end
                    let key_len = id_end - start;
                    if key_len > 0 {
                        tokens.push(Token {
                            start,
                            length: key_len,
                            token_type: TokenType::ParameterKey,
                        });
                    }

                    // find '=' position (skip spaces already consumed to '=')
                    let mut eq_pos = id_end;
                    while eq_pos < chars.len() && chars[eq_pos].is_whitespace() {
                        eq_pos += 1;
                    }
                    if eq_pos < chars.len() && chars[eq_pos] == '=' {
                        tokens.push(Token {
                            start: eq_pos,
                            length: 1,
                            token_type: TokenType::Operator,
                        });
                        eq_pos += 1;
                    }

                    // value start
                    while eq_pos < chars.len() && chars[eq_pos].is_whitespace() {
                        eq_pos += 1;
                    }
                    let vstart = eq_pos;
                    if vstart < chars.len() {
                        let vc = chars[vstart];
                        if vc == '"' || vc == '\'' {
                            let quote = vc;
                            let mut vend = vstart + 1;
                            while vend < chars.len() && chars[vend] != quote {
                                vend += 1;
                            }
                            if vend < chars.len() {
                                vend += 1;
                            }
                            tokens.push(Token {
                                start: vstart,
                                length: vend - vstart,
                                token_type: TokenType::ParameterValue,
                            });
                            i = vend;
                            continue;
                        } else if vc == '[' {
                            let mut vend = vstart + 1;
                            while vend < chars.len() && chars[vend] != ']' {
                                vend += 1;
                            }
                            if vend < chars.len() {
                                vend += 1;
                            }
                            tokens.push(Token {
                                start: vstart,
                                length: vend - vstart,
                                token_type: TokenType::ParameterValue,
                            });
                            i = vend;
                            continue;
                        } else if vc == '(' {
                            let mut vend = vstart + 1;
                            while vend < chars.len() && chars[vend] != ')' {
                                vend += 1;
                            }
                            if vend < chars.len() {
                                vend += 1;
                            }
                            tokens.push(Token {
                                start: vstart,
                                length: vend - vstart,
                                token_type: TokenType::ParameterValue,
                            });
                            i = vend;
                            continue;
                        } else {
                            let mut vend = vstart;
                            while vend < chars.len()
                                && !chars[vend].is_whitespace()
                                && chars[vend] != ';'
                                && chars[vend] != '('
                                && chars[vend] != ')'
                            {
                                vend += 1;
                            }
                            tokens.push(Token {
                                start: vstart,
                                length: vend - vstart,
                                token_type: TokenType::ParameterValue,
                            });
                            i = vend;
                            continue;
                        }
                    }
                }
            }

            // If the next character is alphabetic, treat the whole run as a variable-like token
            // (e.g., `SET`, `VAR1`). This prevents splitting multi-letter words into separate
            // single-letter commands when the intended token is an identifier.
            if id_end - start > 1 {
                // Multi-letter identifier — include any following alphanumeric characters (e.g., VAR1)
                i = id_end;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '<' || chars[i] == '>')
                {
                    i += 1;
                }
                let token_len = i - start;
                tokens.push(Token {
                    start,
                    length: token_len,
                    token_type: TokenType::Variable,
                });
                continue;
            }

            // Common axis letters (X/Y/Z/E or others) should capture the following numeric value, e.g. `X100`.
            let c_upper = chars[start].to_ascii_uppercase();
            if axis_chars_ref.contains(&c_upper) {
                i = start + 1;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '+'
                        || chars[i] == '-'
                        || chars[i] == 'e'
                        || chars[i] == 'E')
                {
                    i += 1;
                }
                let token_type = TokenType::AxisNamed(c_upper);
                tokens.push(Token {
                    start,
                    length: i - start,
                    token_type,
                });
                continue;
            }
            // Parameters like F (feed), S (spindle), T (tool) often have numeric values too
            if matches!(c_upper, 'F' | 'S' | 'T' | 'P') {
                i = start + 1;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || chars[i] == '+'
                        || chars[i] == '-'
                        || chars[i] == 'e'
                        || chars[i] == 'E')
                {
                    i += 1;
                }
                let token_type = if c_upper == 'P' {
                    TokenType::ParameterP
                } else {
                    TokenType::Parameter
                };
                tokens.push(Token {
                    start,
                    length: i - start,
                    token_type,
                });
                continue;
            }

            let start2 = start;
            i = start2 + 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '<' || chars[i] == '>')
            {
                i += 1;
            }
            tokens.push(Token {
                start: start2,
                length: i - start2,
                token_type: TokenType::Unknown,
            });
            continue;
        }
        // numbers & others
        if c.is_ascii_digit() || c == '.' || c == '+' || c == '-' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_digit()
                    || chars[i] == '.'
                    || chars[i] == 'e'
                    || chars[i] == 'E'
                    || chars[i] == '+'
                    || chars[i] == '-')
            {
                i += 1;
            }
            tokens.push(Token {
                start,
                length: i - start,
                token_type: TokenType::Number,
            });
            continue;
        }
        i += 1;
    }

    tokens
}

/// Create an egui LayoutJob for syntax highlighting using the tokenizer.
pub fn highlight_gcode(text: &str, colors: &SyntaxColors, font_size: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let font_id = egui::FontId::monospace(font_size);

    for line in text.lines() {
        let tokens = tokenize_line_pure_rust(line, Some(&colors.axis_chars));
        let mut pos = 0usize;

        for token in &tokens {
            if token.start < pos {
                continue;
            }
            let line_len = line.len();
            if token.start > pos {
                let gap_end = token.start.min(line_len);
                if pos < gap_end {
                    let gap = &line[pos..gap_end];
                    job.append(
                        gap,
                        0.0,
                        egui::TextFormat {
                            font_id: font_id.clone(),
                            color: egui::Color32::WHITE,
                            ..Default::default()
                        },
                    );
                }
            }

            let color = color_for_token(colors, token.token_type);
            let token_start = token.start.min(line_len);
            let token_end = token.start.saturating_add(token.length).min(line_len);
            if token_start >= token_end {
                continue;
            }

            let token_text = &line[token_start..token_end];
            job.append(
                token_text,
                0.0,
                egui::TextFormat {
                    font_id: font_id.clone(),
                    color,
                    ..Default::default()
                },
            );
            pos = token_end;
        }

        if pos < line.len() {
            job.append(
                &line[pos..],
                0.0,
                egui::TextFormat {
                    font_id: font_id.clone(),
                    color: egui::Color32::WHITE,
                    ..Default::default()
                },
            );
        }

        job.append(
            "\n",
            0.0,
            egui::TextFormat {
                font_id: font_id.clone(),
                color: egui::Color32::WHITE,
                ..Default::default()
            },
        );
    }

    job
}

/// Layouter used by egui TextEdit
pub fn highlight_gcode_layouter(
    text: &str,
    colors: &SyntaxColors,
    font_size: f32,
    wrap_width: f32,
) -> egui::text::LayoutJob {
    let mut job = highlight_gcode(text, colors, font_size);
    job.wrap = egui::text::TextWrapping {
        max_width: wrap_width,
        ..Default::default()
    };
    job
}

/// Show a full editor inside an `egui::Ui`.
/// Returns a list of `EditorEvent`s describing changes observed during this UI pass.
/// - `ContentChanged` is emitted when the text content was modified.
/// - `ActiveLineChanged` is emitted when the active line was changed (by clicking the gutter or externally).
/// - `SelectionChanged` is emitted when the `state.selected_lines` changes.
pub fn show_editor(
    ui: &mut egui::Ui,
    content: &mut String,
    state: &mut EditorState,
    colors: &SyntaxColors,
    font_size: f32,
) -> Vec<EditorEvent> {
    let available_height = (ui.available_height() - 30.0).max(0.0);
    let row_height = ui.fonts_mut(|f| f.row_height(&egui::FontId::monospace(font_size)));
    let desired_rows = ((available_height / row_height).floor() as usize).max(5);

    let colors_clone = colors;
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
        let job = highlight_gcode_layouter(text.as_str(), colors_clone, font_size, wrap_width);
        ui.fonts_mut(|f| f.layout_job(job))
    };

    // Prepare lines for the gutter (owned strings so we don't keep an active borrow on `content` while the editor is borrowed)
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Snapshot prior state so we can detect changes and emit events
    let prior_selected_lines = state.selected_lines;
    let prior_active_line = state.active_line;

    let mut events: Vec<EditorEvent> = Vec::new();

    // Use a shared vertical ScrollArea so the gutter and editor scroll together vertically.
    egui::ScrollArea::vertical()
        .max_height(available_height)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if state.show_line_numbers {
                    // Compute digits from the number of lines: e.g. 1 -> 1, 10 -> 2, 100 -> 3
                    let max_lines = lines.len().max(1);
                    let digits = ((max_lines as f32).log10().floor() as usize) + 1;

                    // Measure glyph widths precisely using the TextStyle used for the gutter
                    let text_style = egui::TextStyle::Button;
                    let font_id = text_style.resolve(ui.style());
                    let digit_w = ui.fonts_mut(|f| f.glyph_width(&font_id, '0'));
                    // Some fonts may not have an exact glyph for arrow; fall back to '>' width if needed
                    let arrow_glyph = '➡';
                    let arrow_w = ui.fonts_mut(|f| f.glyph_width(&font_id, arrow_glyph));

                    let left_pad = 4.0;
                    let right_pad = 6.0;
                    let arrow_space = arrow_w + 2.0; // small extra gap
                    let gutter_width =
                        left_pad + arrow_space + (digits as f32) * digit_w + right_pad;

                    // Create a single tall gutter rect and a matching editor rect so vertical positions match exactly.
                    // Use desired_rows to make the editor span the full available height
                    let total_height = (desired_rows as f32) * row_height;

                    // Allocate gutter rect (single big rect)
                    let (gutter_rect, gutter_resp) = ui.allocate_exact_size(
                        egui::vec2(gutter_width, total_height),
                        egui::Sense::click(),
                    );
                    let gutter_painter = ui.painter_at(gutter_rect);

                    // If embedding app requested a scroll-to-line, do it now (before drawing)
                    if state._internal_should_scroll_to_active_line {
                        if let Some(line) = state.active_line {
                            if line >= 1 && line <= lines.len() {
                                let row_top = (line - 1) as f32 * row_height;
                                ui.scroll_to_rect(
                                    egui::Rect::from_min_size(
                                        egui::pos2(0.0, row_top),
                                        egui::vec2(1.0, row_height),
                                    ),
                                    Some(egui::Align::Center),
                                );
                            }
                        }
                        state._internal_should_scroll_to_active_line = false;
                    }

                    // Draw all line numbers and active arrow inside the gutter_rect
                    // Only draw line numbers for lines that actually exist
                    for i in 0..desired_rows {
                        if i >= lines.len() {
                            break;
                        }
                        let line_idx = i + 1;
                        let y_top = gutter_rect.top() + (i as f32) * row_height;
                        let y_center = y_top + row_height * 0.5;

                        // Background highlight for active line if enabled
                        if state.show_active_line_bg && Some(line_idx) == state.active_line {
                            let bg_color = array_to_color32(&state.active_line_bg);
                            let bg_rect = egui::Rect::from_min_size(
                                egui::pos2(gutter_rect.left(), y_top),
                                egui::vec2(gutter_rect.width(), row_height),
                            );
                            gutter_painter.rect_filled(bg_rect, 0.0, bg_color);
                        }

                        // Arrow
                        if Some(line_idx) == state.active_line {
                            gutter_painter.text(
                                egui::pos2(gutter_rect.left() + left_pad, y_center),
                                egui::Align2::LEFT_CENTER,
                                "➡",
                                egui::TextStyle::Button.resolve(ui.style()),
                                egui::Color32::YELLOW,
                            );
                        }

                        // Line number right-aligned
                        let num_str = format!("{}", line_idx);
                        gutter_painter.text(
                            egui::pos2(gutter_rect.right() - right_pad, y_center),
                            egui::Align2::RIGHT_CENTER,
                            num_str,
                            egui::TextStyle::Button.resolve(ui.style()),
                            ui.visuals().text_color(),
                        );
                    }

                    // Editor rect next to gutter — allocate remaining width
                    let available_w = ui.available_width();
                    let (editor_rect, _editor_resp) = ui.allocate_exact_size(
                        egui::vec2(available_w - gutter_width, total_height),
                        egui::Sense::click(),
                    );

                    // Put a TextEdit into the editor_rect so it doesn't create its own scrolling and lines align exactly
                    let editor_widget = egui::TextEdit::multiline(content)
                        .font(egui::FontId::monospace(font_size))
                        .code_editor()
                        .desired_rows(desired_rows)
                        .lock_focus(true)
                        .layouter(&mut layouter);

                    let response = ui.put(editor_rect, editor_widget);

                    state.focused = response.has_focus();

                    // Draw error backgrounds first (if any) and attach optional tooltips
                    if !state.error_ranges.is_empty() {
                        let editor_painter = ui.painter_at(editor_rect);
                        let font_id = egui::FontId::monospace(font_size);
                        for r in &state.error_ranges {
                            let start_b = r.start.min(content.len());
                            let end_b = r.end.min(content.len());
                            let s = start_b.min(end_b);
                            let e = end_b.max(start_b);
                            if s >= e {
                                continue;
                            }
                            let (s_line, s_col) = byte_index_to_line_col(content, s);
                            let (e_line, e_col) = byte_index_to_line_col(content, e);
                            let s_idx = s_line;
                            let e_idx = e_line;
                            for i in s_idx..=e_idx {
                                if i >= lines.len() {
                                    break;
                                }
                                let line = &lines[i];
                                let row_top = editor_rect.top() + (i as f32) * row_height;
                                let row_h = row_height;

                                let start_col = if i == s_idx { s_col } else { 0 };
                                let end_col = if i == e_idx {
                                    e_col
                                } else {
                                    line.chars().count()
                                };
                                if start_col >= end_col {
                                    continue;
                                }

                                let start_x = editor_rect.left()
                                    + ui.fonts_mut(|f| {
                                        let mut sum = 0.0;
                                        for (ci, ch) in line.chars().enumerate() {
                                            if ci >= start_col {
                                                break;
                                            }
                                            sum += f.glyph_width(&font_id, ch);
                                        }
                                        sum
                                    });
                                let end_x = editor_rect.left()
                                    + ui.fonts_mut(|f| {
                                        let mut sum = 0.0;
                                        for (ci, ch) in line.chars().enumerate() {
                                            if ci >= end_col {
                                                break;
                                            }
                                            sum += f.glyph_width(&font_id, ch);
                                        }
                                        sum
                                    });

                                let err_rect = egui::Rect::from_min_max(
                                    egui::pos2(start_x, row_top),
                                    egui::pos2(end_x, row_top + row_h),
                                );
                                editor_painter.rect_filled(
                                    err_rect,
                                    0.0,
                                    egui::Color32::from_rgba_unmultiplied(255, 80, 80, 80),
                                );

                                // Create hoverable region and show tooltip if present
                                let id = ui.make_persistent_id(("error", r.id));
                                let resp = ui.interact(err_rect, id, egui::Sense::hover());
                                if resp.hovered() {
                                    if let Some(t) = &r.tooltip {
                                        resp.on_hover_ui(|ui| {
                                            ui.label(t);
                                        });
                                    }
                                }
                            }
                        }
                    }

                    // Draw selection background (if any)
                    if let (Some((s_line, s_col)), Some((e_line, e_col))) =
                        (state.selection_start, state.selection_end)
                    {
                        let editor_painter = ui.painter_at(editor_rect);
                        let font_id = egui::FontId::monospace(font_size);
                        // Normalize indices (1-based lines stored in state)
                        let s_idx = s_line.saturating_sub(1);
                        let e_idx = e_line.saturating_sub(1);
                        for i in s_idx..=e_idx {
                            if i >= lines.len() {
                                break;
                            }
                            let line = &lines[i];
                            let row_top = editor_rect.top() + (i as f32) * row_height;
                            let row_h = row_height;

                            let start_col = if i == s_idx { s_col } else { 0 };
                            let end_col = if i == e_idx {
                                e_col
                            } else {
                                line.chars().count()
                            };
                            if start_col >= end_col {
                                continue;
                            }

                            // Measure widths by summing glyph widths (monospace fonts make this cheap and stable)
                            let start_x = editor_rect.left()
                                + ui.fonts_mut(|f| {
                                    let mut sum = 0.0;
                                    for (ci, ch) in line.chars().enumerate() {
                                        if ci >= start_col {
                                            break;
                                        }
                                        sum += f.glyph_width(&font_id, ch);
                                    }
                                    sum
                                });
                            let end_x = editor_rect.left()
                                + ui.fonts_mut(|f| {
                                    let mut sum = 0.0;
                                    for (ci, ch) in line.chars().enumerate() {
                                        if ci >= end_col {
                                            break;
                                        }
                                        sum += f.glyph_width(&font_id, ch);
                                    }
                                    sum
                                });

                            let sel_rect = egui::Rect::from_min_max(
                                egui::pos2(start_x, row_top),
                                egui::pos2(end_x, row_top + row_h),
                            );
                            editor_painter.rect_filled(
                                sel_rect,
                                0.0,
                                egui::Color32::from_rgba_unmultiplied(30, 120, 200, 40),
                            );
                        }
                    }

                    // Draw active-line background inside the editor rect as well
                    if state.show_active_line_bg
                        && state
                            .active_line
                            .is_some_and(|active| active >= 1 && active <= lines.len())
                    {
                        let active = state.active_line.unwrap();
                        let i = active - 1;
                        let y_top = editor_rect.top() + (i as f32) * row_height;
                        let bg_color = array_to_color32(&state.active_line_bg);
                        let editor_painter = ui.painter_at(editor_rect);
                        let bg_rect = egui::Rect::from_min_size(
                            egui::pos2(editor_rect.left(), y_top),
                            egui::vec2(editor_rect.width(), row_height),
                        );
                        editor_painter.rect_filled(bg_rect, 0.0, bg_color);
                    }

                    if response.changed() {
                        events.push(EditorEvent::ContentChanged(EditorChangeEvent {
                            new_content: content.clone(),
                            selected_lines: state.selected_lines,
                        }));
                    }

                    // Click handling: map gutter click position to line index
                    if gutter_resp.clicked() {
                        if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                            if gutter_rect.contains(pos) {
                                let rel_y = pos.y - gutter_rect.top();
                                let idx = (rel_y / row_height).floor() as usize;
                                let line_idx = (idx + 1).min(lines.len());
                                state.active_line = Some(line_idx);
                            }
                        }
                    }
                } else {
                    // No gutter — fall back to the normal editor adding
                    let response = ui.add(
                        egui::TextEdit::multiline(content)
                            .font(egui::FontId::monospace(font_size))
                            .code_editor()
                            .desired_width(f32::INFINITY)
                            .desired_rows(desired_rows)
                            .lock_focus(true)
                            .layouter(&mut layouter),
                    );

                    state.focused = response.has_focus();

                    if response.changed() {
                        events.push(EditorEvent::ContentChanged(EditorChangeEvent {
                            new_content: content.clone(),
                            selected_lines: state.selected_lines,
                        }));
                    }
                }
            });
        });

    // Emit selection/active-line change events if they differ from prior snapshot
    if prior_selected_lines != state.selected_lines {
        events.push(EditorEvent::SelectionChanged {
            old: prior_selected_lines,
            new: state.selected_lines,
        });
    }
    if prior_active_line != state.active_line {
        events.push(EditorEvent::ActiveLineChanged {
            old: prior_active_line,
            new: state.active_line,
        });
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_simple() {
        let toks = tokenize_line_pure_rust("G1 X100 Y50", None);
        assert!(!toks.is_empty());
    }

    #[test]
    fn test_highlight_single_g() {
        let colors = SyntaxColors::default();
        let job = highlight_gcode("G", &colors, 12.0);
        assert!(!job.sections.is_empty());
    }

    #[test]
    fn test_tokenize_g0_x0_y0() {
        let line = "G0 X0 Y0";
        let toks = tokenize_line_pure_rust(line, None);
        assert_eq!(toks.len(), 3);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "G0");
        assert_eq!(toks[0].token_type, TokenType::GCode);
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "X0");
        assert_eq!(toks[1].token_type, TokenType::AxisNamed('X'));
        assert_eq!(&line[toks[2].start..toks[2].start + toks[2].length], "Y0");
        assert_eq!(toks[2].token_type, TokenType::AxisNamed('Y'));
    }

    #[test]
    fn test_tokenize_x100y100() {
        let line = "X100Y100";
        let toks = tokenize_line_pure_rust(line, None);
        assert_eq!(toks.len(), 2);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "X100");
        assert_eq!(toks[0].token_type, TokenType::AxisNamed('X'));
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "Y100");
        assert_eq!(toks[1].token_type, TokenType::AxisNamed('Y'));
    }

    #[test]
    fn test_tokenize_f1500() {
        let line = "F1500";
        let toks = tokenize_line_pure_rust(line, None);
        assert_eq!(toks.len(), 1);
        assert_eq!(
            &line[toks[0].start..toks[0].start + toks[0].length],
            "F1500"
        );
        assert_eq!(toks[0].token_type, TokenType::Parameter);
    }

    #[test]
    fn test_tokenize_xyze_p() {
        let line = "X10 Y20 Z-5 E1.5 P123";
        let toks = tokenize_line_pure_rust(line, None);
        // Expect 5 tokens
        assert_eq!(toks.len(), 5);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "X10");
        assert_eq!(toks[0].token_type, TokenType::AxisNamed('X'));
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "Y20");
        assert_eq!(toks[1].token_type, TokenType::AxisNamed('Y'));
        assert_eq!(&line[toks[2].start..toks[2].start + toks[2].length], "Z-5");
        assert_eq!(toks[2].token_type, TokenType::AxisNamed('Z'));
        assert_eq!(&line[toks[3].start..toks[3].start + toks[3].length], "E1.5");
        assert_eq!(toks[3].token_type, TokenType::AxisNamed('E'));
        assert_eq!(&line[toks[4].start..toks[4].start + toks[4].length], "P123");
        assert_eq!(toks[4].token_type, TokenType::ParameterP);
    }

    #[test]
    fn test_color_mapping_axes_and_p() {
        let mut colors = SyntaxColors::default();
        // Add a custom axis 'A' and set its color
        colors.axis_chars.insert('A');
        colors.axis_overrides.insert('A', [1.0, 0.0, 0.0, 1.0]);

        let job = highlight_gcode("X1 Y2 Z3 E4 P5 A10", &colors, 12.0);

        // Helper to find section color by exact token text using the LayoutSection byte_range
        let input = "X1 Y2 Z3 E4 P5 A10";
        let find_color = |tok: &str| -> Option<egui::Color32> {
            for s in &job.sections {
                let range = s.byte_range.clone();
                if range.end <= input.len() && &input[range] == tok {
                    return Some(s.format.color);
                }
            }
            None
        };

        let cx = find_color("X1").expect("X1 not found in sections");
        let cy = find_color("Y2").expect("Y2 not found in sections");
        let cz = find_color("Z3").expect("Z3 not found in sections");
        let ce = find_color("E4").expect("E4 not found in sections");
        let cp = find_color("P5").expect("P5 not found in sections");
        let ca = find_color("A10").expect("A10 not found in sections");

        assert_eq!(cx, color_for_token(&colors, TokenType::AxisNamed('X')));
        assert_eq!(cy, color_for_token(&colors, TokenType::AxisNamed('Y')));
        assert_eq!(cz, color_for_token(&colors, TokenType::AxisNamed('Z')));
        assert_eq!(ce, color_for_token(&colors, TokenType::AxisNamed('E')));
        assert_eq!(cp, color_for_token(&colors, TokenType::ParameterP));
        assert_eq!(ca, color_for_token(&colors, TokenType::AxisNamed('A')));
    }

    #[test]
    fn test_color_mapping_gmo_and_number() {
        let colors = SyntaxColors::default();
        let input = "G1 M3 O100 123.45";
        let job = highlight_gcode(input, &colors, 12.0);

        let find_color = |tok: &str| -> Option<egui::Color32> {
            for s in &job.sections {
                let range = s.byte_range.clone();
                if range.end <= input.len() && &input[range] == tok {
                    return Some(s.format.color);
                }
            }
            None
        };

        let cg = find_color("G1").expect("G1 not found");
        let cm = find_color("M3").expect("M3 not found");
        let co = find_color("O100").expect("O100 not found");
        let cn = find_color("123.45").expect("123.45 not found");

        assert_eq!(cg, color_for_token(&colors, TokenType::GCode));
        assert_eq!(cm, color_for_token(&colors, TokenType::MCode));
        assert_eq!(co, color_for_token(&colors, TokenType::OCode));
        assert_eq!(cn, color_for_token(&colors, TokenType::Number));
    }

    #[test]
    fn test_tokenize_comments() {
        let line = "G0 X0 ; comment here";
        let toks = tokenize_line_pure_rust(line, None);
        assert!(toks.len() >= 3);
        let comment = toks.last().unwrap();
        assert_eq!(comment.token_type, TokenType::Comment);
        assert_eq!(
            &line[comment.start..comment.start + comment.length],
            "; comment here"
        );

        let line2 = "G0 (paren comment)";
        let toks2 = tokenize_line_pure_rust(line2, None);
        let comment2 = toks2.last().unwrap();
        assert_eq!(comment2.token_type, TokenType::Comment);
        assert_eq!(
            &line2[comment2.start..comment2.start + comment2.length],
            "(paren comment)"
        );
    }

    #[test]
    fn test_tokenize_variables() {
        let line = "SET VAR1 10";
        let toks = tokenize_line_pure_rust(line, None);
        assert_eq!(toks.len(), 3);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "SET");
        assert_eq!(toks[0].token_type, TokenType::Variable);
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "VAR1");
        assert_eq!(toks[1].token_type, TokenType::Variable);
        assert_eq!(toks[2].token_type, TokenType::Number);
    }

    #[test]
    fn test_set_selection_range_bytes() {
        let mut s = EditorState::default();
        let content = "G0 X0\nG1 X100\n";
        let start = content.find("X0").unwrap();
        let end = start + "X0".len();
        s.set_selection_range_bytes(start, end, content, false);
        assert!(s.selection_start.is_some());
        assert!(s.selection_end.is_some());
        assert_eq!(s.selected_lines, Some((1, 1)));
        // Ensure start < end columns
        if let (Some((_, sc)), Some((_, ec))) = (s.selection_start, s.selection_end) {
            assert!(sc < ec);
        } else {
            panic!("selection not set");
        }
    }

    #[test]
    fn test_line_col_roundtrip() {
        let content = "Aá\nBC\n"; // utf8 mixed
                                  // line 0, col 1 should be byte index of the second char in first line
        let b = line_col_to_byte_index(content, 0, 1);
        let (l, c) = byte_index_to_line_col(content, b);
        assert_eq!((l, c), (0, 1));

        // out of range line -> returns content.len()
        assert_eq!(line_col_to_byte_index(content, 10, 0), content.len());
    }

    #[test]
    fn test_error_range_api() {
        let mut s = EditorState::default();
        assert!(s.error_ranges.is_empty());
        let _id = s.add_error_range_bytes(2, 5);
        assert_eq!(s.error_ranges.len(), 1);
        assert_eq!(s.error_ranges[0].start, 2);
        assert_eq!(s.error_ranges[0].end, 5);

        // Replace ranges wholesale
        s.set_error_ranges(vec![
            ErrorRange {
                id: 10,
                start: 0,
                end: 1,
                tooltip: None,
            },
            ErrorRange {
                id: 11,
                start: 3,
                end: 4,
                tooltip: Some("foo".into()),
            },
        ]);
        assert_eq!(s.error_ranges.len(), 2);
        assert_eq!(s.error_ranges[0].start, 0);
        assert_eq!(s.error_ranges[1].start, 3);

        s.clear_error_ranges();
        assert!(s.error_ranges.is_empty());
    }

    #[test]
    fn test_selection_by_line_col_and_selected_text() {
        let mut s = EditorState::default();
        let content = "G0 X0\nG1 X100\n";
        // select 'X100' on line 2 (1-based) which starts at col 3 (G1␣X100)
        s.set_selection_range_line_col(2, 3, 2, 7, false);
        let sel = s.selected_text(content).unwrap();
        assert_eq!(sel, "X100");
    }

    #[test]
    fn test_scroll_to_line_api_sets_flag() {
        let mut s = EditorState::default();
        s.scroll_to_line(5);
        assert_eq!(s.active_line, Some(5));
        assert!(s._internal_should_scroll_to_active_line);
    }

    #[test]
    fn test_tokenize_custom_axis() {
        use std::collections::HashSet;
        let mut axes = HashSet::new();
        axes.insert('A');
        let line = "A10 B1";
        let toks = tokenize_line_pure_rust(line, Some(&axes));
        assert_eq!(toks.len(), 2);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "A10");
        assert_eq!(toks[0].token_type, TokenType::AxisNamed('A'));
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "B1");
        // 'B' not in axis set, so treated as Unknown
        assert_eq!(toks[1].token_type, TokenType::Unknown);
    }

    #[test]
    fn test_color_for_axis_fallback() {
        let mut colors = SyntaxColors::default();
        // remove X override to force fallback to generic axis color
        colors.axis_overrides.remove(&'X');
        let c = color_for_token(&colors, TokenType::AxisNamed('X'));
        assert_eq!(c, array_to_color32(&colors.axis));
    }

    #[test]
    fn test_highlight_custom_axis_section() {
        let mut colors = SyntaxColors::default();
        colors.axis_chars.insert('A');
        colors.axis_overrides.insert('A', [1.0, 0.0, 0.0, 1.0]);

        let input = "A1 X1";
        let job = highlight_gcode(input, &colors, 12.0);
        let mut found = false;
        for s in &job.sections {
            let range = s.byte_range.clone();
            if range.end <= input.len() && &input[range] == "A1" {
                let expected = [1.0, 0.0, 0.0, 1.0];
                assert_eq!(s.format.color, array_to_color32(&expected));
                found = true;
            }
        }
        assert!(found, "A1 not found in sections");
    }

    #[test]
    fn test_add_error_from_selection_and_line() {
        let mut s = EditorState::default();
        let content = String::from("G0 X0\nG1 X100\n");
        // select X100
        s.set_selection_range_line_col(2, 3, 2, 7, false);
        s.add_error_from_selection(&content);
        assert_eq!(s.error_ranges.len(), 1);
        // add an error for line 1
        s.add_error_for_line(1, &content);
        assert_eq!(s.error_ranges.len(), 2);
    }

    #[test]
    fn test_add_error_range_line_col() {
        let mut s = EditorState::default();
        let content = "G0 X0\nG1 X100 Y100\n";
        // Mark 'X100' on line 2 (cols 3..7)
        s.add_error_range_line_col(2, 3, 7, content);
        assert_eq!(s.error_ranges.len(), 1);
        let r = &s.error_ranges[0];
        assert_eq!(&content[r.start..r.end], "X100");
    }

    #[test]
    fn test_tokenize_key_value_simple() {
        let line = "FOO=1 BAR=2.5 BAZ=1,2";
        let toks = tokenize_line_pure_rust(line, None);
        // Expect 3*(key, '=', value) tokens
        assert_eq!(toks.len(), 9);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "FOO");
        assert_eq!(toks[0].token_type, TokenType::ParameterKey);
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "=");
        assert_eq!(toks[1].token_type, TokenType::Operator);
        assert_eq!(&line[toks[2].start..toks[2].start + toks[2].length], "1");
        assert_eq!(toks[2].token_type, TokenType::ParameterValue);

        assert_eq!(&line[toks[3].start..toks[3].start + toks[3].length], "BAR");
        assert_eq!(toks[3].token_type, TokenType::ParameterKey);
        assert_eq!(&line[toks[4].start..toks[4].start + toks[4].length], "=");
        assert_eq!(toks[4].token_type, TokenType::Operator);
        assert_eq!(&line[toks[5].start..toks[5].start + toks[5].length], "2.5");
        assert_eq!(toks[5].token_type, TokenType::ParameterValue);

        assert_eq!(&line[toks[6].start..toks[6].start + toks[6].length], "BAZ");
        assert_eq!(toks[6].token_type, TokenType::ParameterKey);
        assert_eq!(&line[toks[7].start..toks[7].start + toks[7].length], "=");
        assert_eq!(toks[7].token_type, TokenType::Operator);
        assert_eq!(&line[toks[8].start..toks[8].start + toks[8].length], "1,2");
        assert_eq!(toks[8].token_type, TokenType::ParameterValue);
    }

    #[test]
    fn test_tokenize_key_value_string_and_vector() {
        let line = "NAME=\"John Doe\" V=[1, 2, 3] EMPTY=()";
        let toks = tokenize_line_pure_rust(line, None);
        // Expect 3*(key, '=', value)
        assert_eq!(toks.len(), 9);
        assert_eq!(&line[toks[0].start..toks[0].start + toks[0].length], "NAME");
        assert_eq!(toks[0].token_type, TokenType::ParameterKey);
        assert_eq!(&line[toks[1].start..toks[1].start + toks[1].length], "=");
        assert_eq!(toks[1].token_type, TokenType::Operator);
        assert_eq!(
            &line[toks[2].start..toks[2].start + toks[2].length],
            "\"John Doe\""
        );
        assert_eq!(toks[2].token_type, TokenType::ParameterValue);

        assert_eq!(&line[toks[3].start..toks[3].start + toks[3].length], "V");
        assert_eq!(toks[3].token_type, TokenType::ParameterKey);
        assert_eq!(&line[toks[4].start..toks[4].start + toks[4].length], "=");
        assert_eq!(toks[4].token_type, TokenType::Operator);
        assert_eq!(
            &line[toks[5].start..toks[5].start + toks[5].length],
            "[1, 2, 3]"
        );
        assert_eq!(toks[5].token_type, TokenType::ParameterValue);

        assert_eq!(
            &line[toks[6].start..toks[6].start + toks[6].length],
            "EMPTY"
        );
        assert_eq!(toks[6].token_type, TokenType::ParameterKey);
        assert_eq!(&line[toks[7].start..toks[7].start + toks[7].length], "=");
        assert_eq!(toks[7].token_type, TokenType::Operator);
        assert_eq!(&line[toks[8].start..toks[8].start + toks[8].length], "()");
        assert_eq!(toks[8].token_type, TokenType::ParameterValue);
    }

    #[test]
    fn test_highlight_key_value_section() {
        let colors = SyntaxColors::default();
        let input = "FOO=1 BAR=2.0 NAME=\"x\"";
        let job = highlight_gcode(input, &colors, 12.0);
        // Ensure each key, '=', and value appear as colored sections with expected colors
        let mut found = std::collections::HashMap::new();
        for s in &job.sections {
            let range = s.byte_range.clone();
            if range.end <= input.len() {
                let slice = &input[range];
                found.insert(slice.to_string(), s.format.color);
            }
        }
        // FOO
        assert_eq!(
            found.get("FOO").cloned(),
            Some(color_for_token(&colors, TokenType::ParameterKey))
        );
        // '='
        assert_eq!(
            found.get("=").cloned(),
            Some(color_for_token(&colors, TokenType::Operator))
        );
        // value '1'
        assert_eq!(
            found.get("1").cloned(),
            Some(color_for_token(&colors, TokenType::ParameterValue))
        );
        // BAR
        assert_eq!(
            found.get("BAR").cloned(),
            Some(color_for_token(&colors, TokenType::ParameterKey))
        );
        assert_eq!(
            found.get("2.0").cloned(),
            Some(color_for_token(&colors, TokenType::ParameterValue))
        );
        // NAME
        assert_eq!(
            found.get("NAME").cloned(),
            Some(color_for_token(&colors, TokenType::ParameterKey))
        );
        assert_eq!(
            found.get("\"x\"").cloned(),
            Some(color_for_token(&colors, TokenType::ParameterValue))
        );
    }
}
