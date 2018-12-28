use std::fmt::{self, Display, Formatter};

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct Action(u64);
pub type SubAction = u8;

impl Display for Action {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut copy = self.clone();
        try!(write!(f, "(len: {}; ", self.length()));
        let mut vals = Vec::new();
        while !copy.is_empty() {
            vals.push(copy.pop_front());
        }
        for val in vals.iter() {
            try!(write!(f, "{},", val));
        }
        write!(f, ")")
    }
}

pub trait ActionQueue {
    fn push_front(&mut self, action: SubAction);
    fn pop_front(&mut self) -> SubAction;
    fn pop_back(&mut self) -> SubAction;
    fn is_empty(&self) -> bool;
    fn length(&self) -> u32;
    fn new() -> Self;
    fn singleton(subaction: u8) -> Self;
}

pub const MAX_LEN: u64 = 19;
pub const LEN_OFFSET: u64 = 57;
pub const VEC_MASK: u64 = 0x1ffffffffffffff;
pub const VEC_EL_BITWIDTH: u64 = 3;

impl ActionQueue for Action {
    fn new() -> Action {
        Action(0)
    }

    fn singleton(subaction: u8) -> Action {
        let mut action = Action(0);
        action.push_front(subaction);
        action
    }

    /// Push subactions to the front of the deque
    fn push_front(&mut self, action: SubAction) {
        assert!(action < 7);
        let new_len = self.length() as u64 + 1u64;
        assert!(new_len <= MAX_LEN);
        self.0 = (self.0 & VEC_MASK) << VEC_EL_BITWIDTH | action as u64 | new_len << LEN_OFFSET;
    }

    /// Remove subactions from the front of the deque
    fn pop_front(&mut self) -> SubAction {
        let new_len = (self.length() - 1) as u64;
        let mask_length = new_len * VEC_EL_BITWIDTH;
        let popped_action = (self.0 & (7 << mask_length)) >> mask_length;
        let pop_mask = (1 << (mask_length + 1)) - 1;
        self.0 = (self.0 & pop_mask) | new_len << LEN_OFFSET; // add new length bits
        popped_action as SubAction
    }

    /// Remove subactions from the back of the deque
    fn pop_back(&mut self) -> SubAction {
        let new_len = (self.length() - 1) as u64;
        let popped_action = self.0 & 7;
        self.0 = (self.0 & VEC_MASK) >> VEC_EL_BITWIDTH | new_len << LEN_OFFSET; // add new length bits
        popped_action as SubAction
    }

    fn length(&self) -> u32 {
        (self.0 >> LEN_OFFSET) as u32
    }

    fn is_empty(&self) -> bool {
        self.length() == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_fifo_queue() {
        let mut action_list: Action = Action::new();
        action_list.push_front(1);
        action_list.push_front(2);
        action_list.push_front(3);
        action_list.push_front(4);
        action_list.push_front(0);
        assert_eq!(action_list.pop_front(), 1);
        assert_eq!(action_list.pop_front(), 2);
        assert_eq!(action_list.pop_back(), 0);
        assert_eq!(action_list.pop_back(), 4);
        assert_eq!(action_list.pop_front(), 3);
        assert!(action_list.is_empty());
    }

    #[test]
    fn test_packed_internals() {
        let mut action_list: Action = Action::new();
        assert_eq!(action_list.0, 0);
        action_list.push_front(4);
        assert_eq!(action_list.0, 4 | (1 << LEN_OFFSET));
        println!("{:?}", action_list);
        action_list.push_front(3);
        assert_eq!(action_list.0, 4 << 3 | 3 | (2 << LEN_OFFSET));
        assert_eq!(action_list.pop_front(), 4);
        assert_eq!(action_list.0, 3 | (1 << LEN_OFFSET));
        action_list.push_front(2);
        println!("{:?}", action_list);
        assert_eq!(action_list.pop_front(), 3);
        assert_eq!(action_list.pop_front(), 2);
        assert_eq!(action_list.0, 0);
        action_list.push_front(3);
        assert_eq!(action_list.0, 3 | (1 << LEN_OFFSET));
    }
}
