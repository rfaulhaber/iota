
#[derive(Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}
