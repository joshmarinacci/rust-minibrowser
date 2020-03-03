# rust-minibrowser
mini-browser written in rust

The point of this project is to prove we can build a web-browser in just a few thousand lines of code, provided
we don't care about:

* speed
* implementing the full specs
* error handling
* javascript

currently the projct is less than 2000 lines of Rust code, and at least a third of the code is unit tests.

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

![](res/screenshot1.png)



This project is derived from the excellent [Rust browser tutorial](https://limpet.net/mbrubeck/2014/08/08/toy-layout-engine-1.html) by Matt Brubeck. The original only got as far as block layout with backgrounds. I've added inline layout with text drawing. 
