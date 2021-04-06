use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use syntect::{
    highlighting::{ThemeSet, Theme},
    parsing::{SyntaxSet, SyntaxReference},
    html::{append_highlighted_html_for_styled_line, IncludeBackground},
    util::LinesWithEndings,
    easy::HighlightLines,
};
use std::path::Path;
use mime_sniffer::MimeTypeSniffer;

extern crate wee_alloc;

// Use `wee_alloc` as the global allocator.
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

lazy_static! {
    static ref SYNTAX_SET: SyntaxSet = SyntaxSet::load_defaults_newlines();
}

lazy_static!{
    static ref THEME_SET: ThemeSet = ThemeSet::load_defaults();
}

#[wasm_bindgen(js_name = "highlight_file")]
pub fn highlight_file_js(code: String, filepath: String, is_light_theme: bool, highlight_long_lines: bool) -> Result<String, JsValue> {
    highlight(&code, &filepath, is_light_theme, highlight_long_lines).map_err(|e| e.into())
}

pub fn highlight(code: &str, filepath: &str, is_light_theme: bool, highlight_long_lines: bool) -> Result<String, HighlightError>  {
    if is_binary(&code.as_bytes()) {
        return Err(HighlightError::Binary)
    }

    // TODO (@camdencheek): I think we can configure syntect to just output class names rather than
    // in-line styles. We should consider doing this so the syntax highlighting can rely on the
    // site's CSS rather than on the compiled-in theme files.
    // https://docs.rs/syntect/4.5.0/syntect/html/struct.ClassedHTMLGenerator.html
    let theme = if is_light_theme {
        THEME_SET.themes.get("Sourcegraph (light)").expect("theme should be compiled with the binary")
    } else {
        THEME_SET.themes.get("Sourcegraph").expect("theme should be compiled with the binary")
    };

    // Determine syntax definition by extension.
    let path = Path::new(&filepath);
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|x| x.to_str()).unwrap_or("");

    // Split the input path ("foo/myfile.go") into file name
    // ("myfile.go") and extension ("go").

    // To determine the syntax definition, we must first check using the
    // filename as some syntaxes match an "extension" that is actually a
    // whole file name (e.g. "Dockerfile" or "CMakeLists.txt"); see e.g. https://github.com/trishume/syntect/pull/170
    //
    // After that, if we do not find any syntax, we can actually check by
    // extension and lastly via the first line of the code.

    // First try to find a syntax whose "extension" matches our file
    // name. This is done due to some syntaxes matching an "extension"
    // that is actually a whole file name (e.g. "Dockerfile" or "CMakeLists.txt")
    // see https://github.com/trishume/syntect/pull/170
    let syntax_def = SYNTAX_SET.find_syntax_by_extension(file_name).or_else(|| {
        // Now try to find the syntax by the actual file extension.
        SYNTAX_SET.find_syntax_by_extension(extension)
    }).or_else(|| {
        // Fall back: Determine syntax definition by first line.
        SYNTAX_SET.find_syntax_by_first_line(&code)
    }).unwrap_or_else(|| {
        // Render plain text, so the user gets the same HTML output structure.
        SYNTAX_SET.find_syntax_plain_text()
    });


    // TODO(slimsag): return the theme's background color (and other info??) to caller?
    // https://github.com/trishume/syntect/blob/c8b47758a3872d478c7fc740782cd468b2c0a96b/examples/synhtml.rs#L24

    Ok(highlight_file(&code, &SYNTAX_SET, &syntax_def, theme, highlight_long_lines))
}

fn is_binary(content: &[u8]) -> bool {
    if let Ok(_) = std::str::from_utf8(content) {
        return false
    }

    if let Some(m) = content.sniff_mime_type() {
        return !m.starts_with("text/")
    }

    return true
}

fn highlighted_rows<'a>(code: &'a str, ss: &'a SyntaxSet, syntax: &'a SyntaxReference, theme: &'a Theme, highlight_long_lines: bool) -> impl Iterator<Item = String> + 'a {
    let mut highlighter = HighlightLines::new(syntax, theme);

    LinesWithEndings::from(code).enumerate().map(move |(i, line)| {
        let mut highlighted = String::with_capacity(8 * line.len());
        start_table_row(&mut highlighted, i+1);
        if !highlight_long_lines && line.len() > 2000 {
            highlighted.push_str(line);
        } else {
            let regions = highlighter.highlight(line, ss);
            append_highlighted_html_for_styled_line(&regions[..], IncludeBackground::No, &mut highlighted);
        }
        end_table_row(&mut highlighted);
        highlighted
    })
}

fn highlight_ranges(code: &str, ss: &SyntaxSet, syntax: &SyntaxReference, theme: &Theme, highlight_long_lines: bool, ranges: &[LineRange]) -> Vec<Vec<String>> {
    let mut output: Vec<Vec<String>> = vec![Vec::with_capacity(10); ranges.len()];
    for row in highlighted_rows(code, ss, syntax, theme, highlight_long_lines) {
        for (i, v) in output.iter_mut().enumerate() {
            if ranges[i].contains(i) {
                v.push(row.clone());
            }
        }
    }

    output
}

fn highlight_file(code: &str, ss: &SyntaxSet, syntax: &SyntaxReference, theme: &Theme, highlight_long_lines: bool) -> String {
    let mut output = String::with_capacity(8 * code.len()); //Heuristic based on experimental results
    start_highlighted_table(&mut output);

    for line in highlighted_rows(code, ss, syntax, theme, highlight_long_lines) {
        output.push_str(&line)
    }

    end_highlighted_table(&mut output);
    output
}

fn start_highlighted_table(s: &mut String) {
    s.push_str("<table><tbody>")
}

fn end_highlighted_table(s: &mut String) {
    s.push_str("</tbody></table>");
}

fn start_table_row(s: &mut String, row_num: usize) {
    s.push_str(&format!("<tr><td class=\"line\" data-line=\"{}\"></td><td class=\"code\"><div>", row_num));
}

fn end_table_row(s: &mut String) {
    s.push_str("</div></td></tr>");
}

#[derive(Debug)]
pub enum HighlightError {
    Binary,
}

impl From<HighlightError> for JsValue {
    fn from(e: HighlightError) -> JsValue {
        match e {
            HighlightError::Binary => JsValue::from("cannot render binary file"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use super::highlight;
    use html_diff::get_differences;

    struct Asset {
        input: String,
        output: String,
        filename: String,
    }

    fn read_asset(id: usize) -> Asset {
        let input = fs::read_to_string(format!("./src/assets/{}/input", id)).unwrap().trim().to_string();
        let output = fs::read_to_string(format!("./src/assets/{}/output", id)).unwrap().trim().to_string();
        let filename = fs::read_to_string(format!("./src/assets/{}/filename", id)).unwrap().trim().to_string();
        Asset { input, output, filename }
    }

    fn test_asset(id: usize) {
        let asset = read_asset(id);
        let result = highlight(&asset.input, &asset.filename, true, true).unwrap();
        assert_diff(&result, &asset.output);
    }

    fn assert_diff(left: &str, right: &str) {
        let mut has_diffs = false;
        {
            for d in get_differences(left, right) {
                has_diffs = true;
                println!("{}", d.to_string());
            }
        }
        assert!(!has_diffs);
    }

    #[test]
    fn asset1() {
        test_asset(1)
    }

    #[test]
    fn asset2() {
        test_asset(2)
    }
}

struct LineRange{
    pub start: usize,
    pub end: usize,
}

impl LineRange {
    fn contains(&self, line_num: usize) -> bool {
        return line_num >= self.start && line_num <= self.end
    }
}
