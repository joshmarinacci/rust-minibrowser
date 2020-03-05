pub mod render;
pub mod layout;
pub mod dom;
pub mod style;
pub mod css;
pub mod net;
pub mod image;
pub mod globals;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2+2,4);
    }
}
