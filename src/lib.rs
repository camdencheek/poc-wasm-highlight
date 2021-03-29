use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use serde::{Serialize, Deserialize};
use syntect::html::highlighted_html_for_string;
use std::path::Path;


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

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    // Manufacture the element we're gonna append
    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct Query {
    // Deprecated field with a default empty string value, kept for backwards
    // compatability with old clients.
    pub extension: String,

    // default empty string value for backwards compat with clients who do not specify this field.
    pub filepath: String,

    pub theme: String,
    pub code: String,
}

#[derive(Serialize, Deserialize)]
pub struct Highlighted {
    data: String,
    plaintext: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Error {
    error: String,
    code: String,
}

impl Into<JsValue> for Error {
    fn into(self) -> JsValue {
        return JsValue::from_serde(&self).unwrap()
    }
}

#[wasm_bindgen]
pub fn highlight(q: JsValue) -> JsValue {
    let q = match q.into_serde::<Query>() {
        Ok(v) => v,
        Err(e) => return Error{
            error: format!("{}", e),
            code: "".into(),
        }.into()
    };
        // Determine theme to use.
        //
        // TODO(slimsag): We could let the query specify the theme file's actual
        // bytes? e.g. via `load_from_reader`.
        let theme = match THEME_SET.themes.get(&q.theme) {
            Some(v) => v,
            None => return JsValue::from_serde(&Error{
                error: "invalid theme".into(),
                code: "invalid_theme".into()
            }).unwrap(),
        };

        // Determine syntax definition by extension.
        let mut is_plaintext = false;
        let syntax_def = if q.filepath == "" {
            // Legacy codepath, kept for backwards-compatability with old clients.
            match SYNTAX_SET.find_syntax_by_extension(&q.extension) {
                Some(v) => v,
                None =>
                    // Fall back: Determine syntax definition by first line.
                    match SYNTAX_SET.find_syntax_by_first_line(&q.code) {
                        Some(v) => v,
                        None => return JsValue::from_serde(&Error{
                            error: "invalid extension".into(),
                            code: "".into()
                        }).unwrap(),
                },
            }
        } else {
            // Split the input path ("foo/myfile.go") into file name
            // ("myfile.go") and extension ("go").
            let path = Path::new(&q.filepath);
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let extension = path.extension().and_then(|x| x.to_str()).unwrap_or("");

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
            match SYNTAX_SET.find_syntax_by_extension(file_name) {
                Some(v) => v,
                None =>
                    // Now try to find the syntax by the actual file extension.
                    match SYNTAX_SET.find_syntax_by_extension(extension) {
                        Some(v) => v,
                        None =>
                            // Fall back: Determine syntax definition by first line.
                            match SYNTAX_SET.find_syntax_by_first_line(&q.code) {
                                Some(v) => v,
                                None => {
                                    is_plaintext = true;

                                    // Render plain text, so the user gets the same HTML
                                    // output structure.
                                    SYNTAX_SET.find_syntax_plain_text()
                                }
                        },
                    }
            }
        };

        // TODO(slimsag): return the theme's background color (and other info??) to caller?
        // https://github.com/trishume/syntect/blob/c8b47758a3872d478c7fc740782cd468b2c0a96b/examples/synhtml.rs#L24

        JsValue::from_serde(&Highlighted{
            data: highlighted_html_for_string(&q.code, &SYNTAX_SET, &syntax_def, theme),
            plaintext: is_plaintext,
        }).unwrap()
}
