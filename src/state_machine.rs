use crate::dot::Dot;
use crate::state::{Direction, DotState};

// TODO: Interpreter needs to know if a character is an operator or not
// TODO: Should return a list of dots in order to support dot splitting
pub fn interpret(dot: &Dot, character: char) -> Dot {
    match dot.state {
        DotState::Travel => interpret_travel(dot, character),
        DotState::Dead => *dot,
        DotState::EndProgram => *dot,
    }
}

fn interpret_travel(dot: &Dot, character: char) -> Dot {
    match character {
        '-' => {
            if dot.direction.pipe() == '-' {
                dot.step()
            } else {
                dot.change_state(DotState::Dead)
            }
        }
        '|' => {
            if dot.direction.pipe() == '|' {
                dot.step()
            } else {
                dot.change_state(DotState::Dead)
            }
        }
        '+' => dot.step(), // TODO: Check if it's an operator in the future
        '>' => {
            if dot.direction == Direction::Up || dot.direction == Direction::Down {
                dot.change_dir(Direction::Right).step()
            } else {
                dot.step()
            }
        }
        '<' => {
            if dot.direction == Direction::Up || dot.direction == Direction::Down {
                dot.change_dir(Direction::Left).step()
            } else {
                dot.step()
            }
        }
        '^' => {
            if dot.direction == Direction::Left || dot.direction == Direction::Right {
                dot.change_dir(Direction::Up).step()
            } else {
                dot.step()
            }
        }
        'v' => {
            if dot.direction == Direction::Left || dot.direction == Direction::Right {
                dot.change_dir(Direction::Down).step()
            } else {
                dot.step()
            }
        }
        '/' => match dot.direction {
            Direction::Up => dot.change_dir(Direction::Right),
            Direction::Down => dot.change_dir(Direction::Left),
            Direction::Left => dot.change_dir(Direction::Down),
            Direction::Right => dot.change_dir(Direction::Up),
        }
        .step(),
        '\\' => match dot.direction {
            Direction::Up => dot.change_dir(Direction::Left),
            Direction::Down => dot.change_dir(Direction::Right),
            Direction::Left => dot.change_dir(Direction::Up),
            Direction::Right => dot.change_dir(Direction::Down),
        }
        .step(),
        '(' => match dot.direction {
            Direction::Up => dot.change_state(DotState::Dead),
            Direction::Down => dot.change_state(DotState::Dead),
            Direction::Left => dot.change_dir(Direction::Right),
            Direction::Right => *dot,
        }
        .step(),
        ')' => match dot.direction {
            Direction::Up => dot.change_state(DotState::Dead),
            Direction::Down => dot.change_state(DotState::Dead),
            Direction::Left => *dot,
            Direction::Right => dot.change_dir(Direction::Left),
        }
        .step(),
        ' ' => dot.change_state(DotState::Dead),
        '&' => dot.change_state(DotState::EndProgram), // TODO: check if it's an operator in the future
        _ => dot.step(),
    }
}
