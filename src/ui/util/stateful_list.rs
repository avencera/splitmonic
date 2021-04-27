use tui::widgets::ListState;

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item)
    }

    pub fn pop(&mut self) -> Option<T> {
        self.items.pop()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.is_empty() || i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if self.items.is_empty() {
                    0
                } else if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn move_up(&mut self) {
        match self.state.selected() {
            // top of list, move to the bottom
            Some(0) if self.items.len() >= 2 => {
                let new_index = self.items.len() - 1;

                let item = self.items.remove(0);
                self.items.insert(new_index, item);
                self.state.select(Some(new_index));
            }
            Some(index) if self.items.len() >= 2 => {
                let item = self.items.remove(index);
                self.items.insert(index - 1, item);
                self.state.select(Some(index - 1));
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        match self.state.selected() {
            // bottom of the list, move to top
            Some(index) if self.items.len() - 1 == index => {
                let new_index = 0;

                let item = self.items.remove(index);
                self.items.insert(new_index, item);
                self.state.select(Some(new_index));
            }

            Some(index) if self.items.len() >= 2 => {
                let item = self.items.remove(index);
                self.items.insert(index + 1, item);
                self.state.select(Some(index + 1));
            }
            _ => {}
        }
    }

    pub fn select(&mut self) {
        if !self.items.is_empty() {
            self.state.select(Some(0));
        }
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }

    pub fn delete_selected(&mut self) {
        match self.state.selected() {
            Some(index) if self.items.len() > index => {
                self.items.remove(index);
            }
            _ => {}
        }
    }
}
