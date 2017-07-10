use handlebars::Handlebars;
use iron;
use iron::headers::ContentType;
use iron::modifiers::Header;
use iron::prelude::*;
use params::Map;
use std::str::from_utf8;
use serde::Serialize;
use serde_json;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug,Clone,Serialize)]
struct LayoutPage {
    title : String,
    body : String
}

/// A renderer that can create a particular page
pub trait Renderer : Send + Sync {
    fn render(&self, HashMap<String, String>, &Map, &Handlebars, usize) -> IronResult<Response>;
}

/// The renderer for showing a static page
pub struct StaticRenderer(String, String);

impl StaticRenderer {
    pub fn new(name: String, data : &'static [u8]) -> StaticRenderer {
        StaticRenderer(name, from_utf8(data).expect("Invalid UTF-8").to_string())
    }
}

impl Renderer for StaticRenderer {
    fn render(&self, _: HashMap<String, String>, _ : &Map, hb : &Handlebars, _ : usize) -> IronResult<Response> {
        let title = format!("{} - {}", ::APP_TITLE, self.0);
        render_ok(hb, title, self.1.clone())
    }
}

/// A translator that can convert query arguments into a serializable object
pub trait Translator<A : Serialize> : Sync + Send {
    fn convert(&self, HashMap<String, String>) -> A;
}

impl<A : Serialize> Renderer for Box<Translator<A>> {
    fn render(&self, args : HashMap<String, String>, _ : &Map, _ : &Handlebars, _:usize) -> IronResult<Response> {
        match serde_json::to_string(&self.convert(args)) {
            Ok(s) => {
                Ok(Response::with((
                    iron::status::Ok,
                    Header(ContentType::json()), s)))
            },
            Err(e) => {
                Ok(Response::with((
                        iron::status::InternalServerError,
                        Header(ContentType::plaintext()), e.description())))
            }
        }
    }
}

/// A renderer that uses a template to produce HTML for a translated object
pub struct TranslatorRenderer<A : Serialize>(pub String, pub String, pub Box<Translator<A>>);

impl<A: Serialize> Renderer for TranslatorRenderer<A> {
    fn render(&self, args : HashMap<String, String>, _ : &Map, hb : &Handlebars, _:usize) -> IronResult<Response> {
        let title = format!("{} - {}", ::APP_TITLE, self.0);
        render_ok(hb, title.to_string(), hb.render(&self.1, &self.2.convert(args)).
                                         expect(&format!("Could not use template {}", self.1)))
    }
}

/// Render using `layout.hbs`
pub fn render_ok(hbars : &Handlebars, title : String, body : String) -> IronResult<Response> {
    Ok(Response::with(
            (iron::status::Ok,
             Header(ContentType::html()),
             hbars.render("layout", &LayoutPage {
                 title : title,
                 body : body
             }).expect("Could not render layout"))))
}

/// Render an error using `layout.hbs`
pub fn render_error(hbars : &Handlebars, body : String, error : iron::status::Status) -> IronResult<Response> {
    Ok(Response::with(
            (error,
             Header(ContentType::html()),
             hbars.render("layout", &LayoutPage {
                 title : ::APP_TITLE.to_string(),
                 body  : body
             }).expect("Could not render error page"))))
}


