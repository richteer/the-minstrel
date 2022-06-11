pub mod dbadapter;
pub use dbadapter::*;

// Internal only, these types should not leave this crate for now
mod model;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
