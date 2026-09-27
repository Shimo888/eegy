#[derive(Clone, Debug, PartialEq)]
pub struct Matrix<T: Default + Clone>{
    pub data: Vec<T>,
    pub rows: usize,
    pub cols: usize,
}

impl<T:Default + Clone> Matrix<T>{
    pub fn new(rows: usize, cols: usize) -> Self{
        Self{
            data: vec![T::default(); rows * cols],
            rows,
            cols
        }
    }
    
    pub fn is_valid(&self) -> bool {
        self.data.len() == self.rows * self.cols
    }
}