use anyhow::{Error, Result, anyhow};
use web_sys::{Document, Element, Node, HtmlElement, CssStyleDeclaration, wasm_bindgen::{JsValue, JsCast}};
use common::primitives::{Percent, Level};
use common::eq;

pub fn document() -> Option<Document> {
    web_sys::window()
        .and_then(|window| window.document() )
}

pub fn new_elem(tag: &str) -> Result<Element> {
    document().ok_or_else(|| anyhow!("No document found"))?.create_element(tag).map_dbg()
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


/// the val of 100% for [`ProgressBar::PERCENTAGE_STYLE_NAME`]
const UNIT_ONE_CSS: f32 = 10000.0;

pub fn percent_to_linear_gradient(percent: Percent) -> u32 {
    let percent: f32 = percent.into();
    let multiplier = percent / 100.0;

    if eq!(multiplier, 0.0) {
        100
    } else {
        (multiplier * UNIT_ONE_CSS).round() as u32
    }
}

// this module is more like a dumping ground for `ui` from now on
/// for the bar, not text (see [`ProgressText`])
pub struct ProgressBar;

impl ProgressBar {
    const LINEAR_GRADIENT_CONTROLLER_CSS: &'static str = "background-size";
    const ELEMENT_ID: &'static str = "progress_bar";

    pub fn new_element() -> Element {
        assert_eq!(get_element(), None, "creating a new progress bar when one's already presesnt");

        let elem = new_elem("div").unwrap();
        // style managed by styles.css
        elem.set_id(Self::ELEMENT_ID);

        elem
    }
    pub fn get_element() -> Option<Element> {
        document()?
            .get_element_by_id(Self::ELEMENT_ID)
    }
    // interior mutability xD
    /// conversion to CSS-compatible val and setting it
    pub fn set_linear_gradient(styling: &CssStyleDeclaration, percent: Percent) {
        styling.set_property(Self::LINEAR_GRADIENT_CONTROLLER_CSS, &format!("{}%", percent_to_linear_gradient(percent))).unwrap();
    }
}

pub struct ProgressText {
    percent_to: Percent,
    level_to: Level
}

impl ProgressText {
    pub fn new() -> Self {
        Self {
            percent_to: 0,
            level_to: Level::Two
        }
    }
    pub fn set(element: &HtmlElement, percent_to: Percent, level_to: Level) {
        element.set_inner_text(&Self { percent_to, level_to }.to_string())
    }
}

impl ToString for ProgressText {
    fn to_string(&self) -> String {
        format!("{}% to level {}", self.percent_to, self.level_to)
    }
}