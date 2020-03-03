# rust-minibrowser
mini-browser written in rust

The point of this project is to prove we can build a web-browser in just a few thousand lines of code, provided
we don't care about:

* speed
* implementing the full specs
* error handling
* javascript


The other point of this project is to teach myself Rust.

So far the mini-browser can:

* parse CSS files (not the full spec yet)
* parse HTML files (not the full spec yet)
* layout a page with simple block and simple inline layout
* render with the properties for display, color, background-color, border-color, padding, margin, border-width.


It does not yet, but will soon:
* load files over the network
* handle nested inline styles (spans, em, strong, etc.)
* handle clicking on links (a) 
