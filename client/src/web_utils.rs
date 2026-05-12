use anyhow::{Error, Result, anyhow};
use web_sys::{Document, Element, Node, HtmlElement, CssStyleDeclaration, wasm_bindgen::{JsValue, JsCast}};

pub fn document() -> Option<Document> {
    web_sys::window()
        .and_then(|window| window.document() )
}

pub fn new_elem(tag: &str) -> Result<Element> {
    document().ok_or_else(|| anyhow!("No document found"))?.create_element(tag).map_err(to_dbg)
}

pub fn insert_to_body(element: Element) -> Option<Node> {
    document()?
        .body()?
        .insert_before(&element, None)
        .ok()
}

pub fn to_dbg(e: JsValue) -> Error {
    anyhow!("Error:{:?}", e)
}

pub trait ResultJsExt<T> {
    type Error;
    fn map_dbg(self) -> Result<T, Error>;
}

impl<T> ResultJsExt<T> for Result<T, JsValue> {
    type Error =  anyhow::Error;
    fn map_dbg(self) -> Result<T, Self::Error> {
        self.map_err(to_dbg)
    }
}
pub fn element_to_html_element(elem: Element) -> Result<HtmlElement, Element> {
    elem.dyn_into::<HtmlElement>()
}
pub fn get_styling(elem: Element) -> CssStyleDeclaration {
    // who wants error handling
    element_to_html_element(elem).unwrap().style()
}