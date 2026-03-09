use crate::dot::Dot;
use crate::state::{Direction, DotState};

// TODO: Interpreter needs to know if a character is an operator or not
pub fn interpret(dot: &mut Dot, character: char, pending_dots: &mut Vec<Dot>) {
    match dot.state {
        DotState::Travel => interpret_travel(dot, character, pending_dots),
        _ => {}
    };
}

fn interpret_travel(dot: &mut Dot, character: char, pending_dots: &mut Vec<Dot>) {
    match character {
        '-' => {
            if dot.direction.pipe() == '-' {
                dot.step();
            } else {
                dot.change_state(DotState::Dead);
            }
        }
        '|' => {
            if dot.direction.pipe() == '|' {
                dot.step();
            } else {
                dot.change_state(DotState::Dead);
            }
        }
        '+' => {
            dot.step();
        } // TODO: Check if it's an operator in the future
        '>' => {
            if dot.direction == Direction::Up || dot.direction == Direction::Down {
                dot.change_dir(Direction::Right).step();
            } else {
                dot.step();
            }
        }
        '<' => {
            if dot.direction == Direction::Up || dot.direction == Direction::Down {
                dot.change_dir(Direction::Left).step();
            } else {
                dot.step();
            }
        }
        '^' => {
            if dot.direction == Direction::Left || dot.direction == Direction::Right {
                dot.change_dir(Direction::Up).step();
            } else {
                dot.step();
            }
        }
        'v' => {
            if dot.direction == Direction::Left || dot.direction == Direction::Right {
                dot.change_dir(Direction::Down).step();
            } else {
                dot.step();
            }
        }
        '/' => match dot.direction {
            Direction::Up => {
                dot.change_dir(Direction::Right).step();
            }
            Direction::Down => {
                dot.change_dir(Direction::Left).step();
            }
            Direction::Left => {
                dot.change_dir(Direction::Down).step();
            }
            Direction::Right => {
                dot.change_dir(Direction::Up).step();
            }
        },
        '\\' => match dot.direction {
            Direction::Up => {
                dot.change_dir(Direction::Left).step();
            }
            Direction::Down => {
                dot.change_dir(Direction::Right).step();
            }
            Direction::Left => {
                dot.change_dir(Direction::Up).step();
            }
            Direction::Right => {
                dot.change_dir(Direction::Down).step();
            }
        },
        '(' => match dot.direction {
            Direction::Up => {
                dot.change_state(DotState::Dead).step();
            }
            Direction::Down => {
                dot.change_state(DotState::Dead).step();
            }
            Direction::Left => {
                dot.change_dir(Direction::Right).step();
            }
            Direction::Right => {
                dot.step();
            }
        },
        ')' => match dot.direction {
            Direction::Up => {
                dot.change_state(DotState::Dead).step();
            }
            Direction::Down => {
                dot.change_state(DotState::Dead).step();
            }
            Direction::Left => {
                dot.step();
            }
            Direction::Right => {
                dot.change_dir(Direction::Left).step();
            }
        },
        '*' => {
            for d in &[
                Direction::Up,
                Direction::Down,
                Direction::Left,
                Direction::Right,
            ] {
                if d != &dot.direction && d != &dot.direction.reverse() {
                    // only spawn the TWO perpendicular directions
                    let mut new_dot = dot.clone();
                    new_dot.change_dir(*d).step();
                    pending_dots.push(new_dot);
                }
            }
            // original dot keeps going forward
            dot.step();
        }
        ' ' => {
            dot.change_state(DotState::Dead);
        }
        '&' => {
            dot.change_state(DotState::EndProgram);
        } // TODO: check if it's an operator in the future
        _ => {
            dot.step();
        }
    };
}
