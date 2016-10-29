#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct Action(u32);
pub type SubAction = u8;

pub trait ActionQueue {
    fn push_action(&mut self, action: SubAction);
    fn pop_action(&mut self) -> SubAction;
    fn is_empty(&self) -> bool;
    fn length(&self) -> u16;
    fn new() -> Self;
    fn singleton(subaction: u8) -> Self;
}

impl ActionQueue for Action {
    fn new() -> Action {
        Action(0)
    }

    fn singleton(subaction: u8) -> Action {
        let mut action = Action(0);
        action.push_action(subaction);
        action
    }
    
    fn push_action(&mut self, action: SubAction){
        assert!(action < 7);
        let new_len = self.length() as u32 + 1u32;
        self.0 = (self.0 & 0xFFFF) << 3 | action as u32 | new_len << 16;
    }

    fn pop_action(&mut self) -> SubAction {
        let len = self.length() as u32;
        let shifts = (len-1)*3;
        let pop_mask = (1 << (shifts)) - 1;
        let popped_action = (self.0 & (7 << shifts)) >> shifts;
        self.0 = (self.0 & pop_mask) // clear out top bits
            | (len-1) << 16; // add new length bits
        popped_action as SubAction
    }

    fn length(&self) -> u16 {
        (self.0 >> 16) as u16
    }

    fn is_empty(&self) -> bool {
        self.length() == 0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_packed_actions() {
        let mut action_list: Action = Action::new();
        assert_eq!(action_list.0, 0);
        action_list.push_action(4);
        assert_eq!(action_list.0, 4 | (1<<16));
        println!("{:?}", action_list);
        action_list.push_action(3);
        assert_eq!(action_list.0, 4<<3 | 3 | (2<<16));
        assert_eq!(action_list.pop_action(), 4);
        assert_eq!(action_list.0, 3 | (1<<16));
        action_list.push_action(2);
        println!("{:?}", action_list);
        assert_eq!(action_list.pop_action(), 3);
        assert_eq!(action_list.pop_action(), 2);
        assert_eq!(action_list.0, 0);
        action_list.push_action(3);
        assert_eq!(action_list.0, 3 | (1<<16));
    }
}
