pub struct PositionCalculator {
    line_starts: Vec<usize>
}

impl PositionCalculator {
    pub fn new(source: &str) -> PositionCalculator {
        let mut line_starts = vec![0];

        for (i, c) in source.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }

        PositionCalculator { line_starts }
    }

    pub fn compute(&self, start: usize) -> (usize, usize) {
        let line_index = self.line_starts
            .binary_search(&start)
            .unwrap_or_else(|next_line| next_line - 1);

        let line_start = self.line_starts[line_index];

        let line_number = line_index + 1;
        let column_number = start - line_start + 1;

        (line_number, column_number)
    }
}