pub mod t;
pub mod x;
pub mod y;
pub mod z;

pub fn combined() -> i32 {
    x::value() + y::value() + z::value() + t::value()
}

#[cfg(test)]
mod tests {
    #[test]
    fn combined_value_is_stable() {
        assert_eq!(crate::combined(), 100);
    }
}
