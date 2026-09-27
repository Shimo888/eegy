#[unsafe(no_mangle)]
pub extern "C" fn sample_func_add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_func_add() {
        let result = sample_func_add(2, 2);
        assert_eq!(result, 4);
    }
}
