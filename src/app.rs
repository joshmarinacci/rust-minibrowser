use url::Url;
use crate::render::{FontCache};
use crate::layout::{Dimensions, RenderBox, Rect};
use crate::dom::{Document, strip_empty_nodes, expand_entities};
use crate::net::{BrowserError, load_doc_from_net, load_stylesheets_with_fallback, relative_filepath_to_url};
use crate::style::{expand_styles, style_tree};
use crate::layout;
use std::env;

pub fn navigate_to_doc(url:&Url, font_cache:&mut FontCache, containing_block:Dimensions) -> Result<(Document, RenderBox),BrowserError> {
    let mut doc = load_doc_from_net(&url)?;
    strip_empty_nodes(&mut doc);
    expand_entities(&mut doc);
    let mut stylesheet = load_stylesheets_with_fallback(&doc)?;
    expand_styles(&mut stylesheet);
    // font_cache.scan_for_fontface_rules(&stylesheet);
    let styled = style_tree(&doc.root_node,&stylesheet);
    let mut bbox = layout::build_layout_tree(&styled, &doc);
    println!("doing layout with bounds {:#?}", containing_block);
    let render_root = bbox.layout(&mut containing_block.clone(), font_cache, &doc);
    println!("render root is {:#?}",render_root);
    Ok((doc,render_root))
}
/*
pub fn init_fonts() -> FontCache {
    let mut font_cache = FontCache::new();
    font_cache.install_default_font("sans-serif",  400.0,"normal", &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font("sans-serif",  400.0,"normal", &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Regular.ttf").unwrap());
    font_cache.install_font("sans-serif",  700.0,"normal",&relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Bold.ttf").unwrap());

    font_cache.install_font("sans-serif",  400.0,"italic", &relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-Italic.ttf").unwrap());
    font_cache.install_font("sans-serif",  700.0,"italic",&relative_filepath_to_url("tests/fonts/Open_Sans/OpenSans-BoldItalic.ttf").unwrap());
    font_cache.install_font_font("monospace",  400.0,"normal", SystemSource::new()
        .select_best_match(&[font_kit::family_name::FamilyName::Monospace], &Properties::new())
        .expect("monospace should be found")
        .load()
        .unwrap()
    );
    font_cache
}
*/
pub fn parse_args() -> Result<Url, BrowserError> {
    let args: Vec<String> = env::args().collect();
    println!("args = {:?}", args);
    let mut start_page = relative_filepath_to_url("tests/page1.html")?;
    if args.len() > 1 {
        println!("loading url {}", args[1]);
        if args[1].starts_with("http") {
            start_page = Url::parse(args[1].as_str())?;
        } else {
            start_page = relative_filepath_to_url(&*args[1])?;
        }
    }

    // let start_page = relative_filepath_to_url("tests/nested.html")?;
    // let start_page = relative_filepath_to_url("tests/image.html")?;
    // let start_page = Url::parse("https://apps.josh.earth/rust-minibrowser/test1.html").unwrap();
    // let start_page = relative_filepath_to_url("tests/tufte/tufte.html")?;

    Ok(start_page)
}

/*
fn scroll_viewport(window:&Window, viewport:&mut Rect) {
    if let Some(keys) = window.get_keys_pressed(KeyRepeat::Yes) {
        for key in keys {
            match key {
                Key::Up    => viewport.y -= 300.0,
                Key::Down  => viewport.y += 300.0,
                Key::Left  => viewport.x += 100.0,
                Key::Right => viewport.x -= 100.0,
                _ => {}
            }
        }
    }
}
*/
