extern crate clap;
extern crate iron;
extern crate handlebars;
extern crate params;
extern crate percent_encoding;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

mod renderer;
mod templates;

use clap::{Arg, App};
use handlebars::Handlebars;
use iron::middleware::Handler;
use iron::prelude::*;
use params::Params;
use percent_encoding::percent_decode;
use renderer::*;
use serde::Serialize;
use templates::*;
use std::collections::HashMap;
use std::str::from_utf8;


/// The Server object passed to Iron
struct Server {
    hbars : Handlebars,
    renderers : Vec<(&'static str, Box<Renderer>)>
}

impl Server {
    /// Create a new server
    fn new() -> Server {
        let mut hb = Handlebars::new();
        
        hb.register_template_string("layout", 
            from_utf8(LAYOUT).expect("Layout is not valid utf-8")).
                expect("Layout is not valid template");

        for &(name, template) in TEMPLATES.iter() {
            hb.register_template_string(name,
                                        from_utf8(template).expect(
                                            &format!("{} is not valid UTF-8", name))).
                expect(&format!("{} is not a valid template", name));
        }

        Server {
            hbars : hb,
            renderers : Vec::new()
        }
    }

    #[allow(dead_code)]
    /// Add a static page for a particular `path`, the `name` is displayed in the 
    /// title nar
    fn add_static(&mut self, path : &'static str, name : &str, content : &'static [u8]) {
        self.renderers.push((path, Box::new(StaticRenderer::new(name.to_string(), content))))
    }

    #[allow(dead_code)]
    /// Add a generic renderer matching a given `path`
    fn add_renderer(&mut self, path: &'static str, renderer : Box<Renderer>) {
        self.renderers.push((path, renderer));
    }

    #[allow(dead_code)]
    /// Add a translator that packages values so they can be loaded with `template`
    /// The `name` is shown on the title bar
    fn add_translator<A>(&mut self, path: &'static str, name: &str, template : &'static str,
                      translator : Box<Translator<A>>) where A : Serialize + 'static {
        self.renderers.push((path, Box::new(TranslatorRenderer(name.to_string(), template.to_string(), translator))))
    }
}

impl Handler for Server {
    fn handle(&self, req : &mut Request) -> IronResult<Response> {
        for &(ref path, ref renderer) in self.renderers.iter() {
            match match_path(path, &req.url.path()) {
                Some(args) => {
                    let depth  = req.url.path().len();
                    let params = req.get_ref::<Params>().unwrap();

                    return renderer.render(args, params, &self.hbars, depth);
                },
                None => {}
            }

        }
        return render_error(&self.hbars, from_utf8(templates::NOT_FOUND).
                              expect("Invalid UTF-8 in 404").to_string(),
                            iron::status::NotFound);

    }
}

/// This function matches a path and returns the captures if any
///
/// # Examples
/// ```
/// let p1 = "foo/bar/:test/:this";
/// let mut path1 = Vec::new();
/// path1.push("foo");
/// path1.push("bar");
/// path1.push("not");
/// path1.push("that");
/// let mut expected = HashMap::new();
/// expected.insert("test".to_string(), "not".to_string());
/// expected.insert("this".to_string(), "that".to_string());
/// assert_eq!(Some(expected), match_path(p1, &path1));
/// ```
fn match_path(pattern : &str, path : &Vec<&str>) -> Option<HashMap<String,String>> {
    let mut captures = HashMap::new();
    let p : Vec<&str> = pattern.split("/").collect();
    if p.len() != path.len() {
        None
    } else {
        for (p1,p2) in p.iter().zip(path.iter()) {
            if p1.starts_with(":") {
                captures.insert(p1[1..].to_string(), urldecode(p2));
            } else if p1 != p2 {
                return None;
            }
        }
        Some(captures)
    }
}

/// Decode a percent-encoded URL
fn urldecode(s : &str) -> String {
    percent_decode(s.as_bytes()).decode_utf8().unwrap().into_owned()
}

fn main() {
    let matches = App::new(APP_TITLE).
        version(VERSION).
        author(AUTHOR).
        about(ABOUT).
        arg(Arg::with_name("port")
             .short("p")
             .long("port")
             .value_name("PORT")
             .help("The port to run the server on")
             .takes_value(true)).
        get_matches();

    let port : u16 = matches.value_of("port").and_then(|pstr| { pstr.parse::<u16>().ok() }).unwrap_or(3000);

    let mut server = Server::new();
    init(&mut server);
    let iron = Iron::new(server);
    iron.http(("localhost",port)).unwrap();
}


//////////////////////////////////////////////////////////////////////////////
// Example code
//
static APP_TITLE : &'static str = "My App";
static VERSION   : &'static str = "0.1";
static AUTHOR    : &'static str = "John McCrae <john@mccr.ae>";
static ABOUT     : &'static str = "Simple framework for making Rust Webapps";

struct TestTranslator;

impl Translator<String> for TestTranslator {
    fn convert(&self, v : HashMap<String, String>) -> String { v["test"].clone() }
}

fn init(server : &mut Server) {
    server.add_static("", "Index", INDEX);
    server.add_translator("foo", "bar", "template", Box::new(TestTranslator));
}



