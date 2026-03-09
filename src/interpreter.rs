use crate::dot::{Dot, DotState, Position};
use crate::parser::DiagnosticKind;
use crate::token::{Direction, MirrorKind, Orientation, SpanToken, Token};

// TODO: Interpreter needs to know if a character is an operator or not
pub fn interpret(
    dot: &mut Dot,
    token: &Token,
    pending_dots: &mut Vec<Dot>,
) -> Result<(), DiagnosticKind> {
    match dot.state {
        DotState::Travel => interpret_travel(dot, token, pending_dots),
        _ => Ok(()),
    }
}

fn interpret_travel(
    dot: &mut Dot,
    token: &Token,
    pending_dots: &mut Vec<Dot>,
) -> Result<(), DiagnosticKind> {
    match *token {
        Token::Dot => {
            dot.step();
        }
        Token::Pipe(orientation) if orientation == dot.orientation() => {
            dot.step();
        }
        Token::CrossPipe => {
            dot.step();
        }
        Token::ForceDirection(dir) => {
            if dot.direction.orthogonal(&dir) {
                dot.change_dir(dir);
            }
            dot.step();
        }
        Token::Mirror(kind) => {
            let Position { x, y } = dot.direction.into();
            let tmp = match kind {
                MirrorKind::Forward => Position { x: -y, y: -x },
                MirrorKind::Backward => Position { x: y, y: -x },
            }
            .into();
            if let Some(dir) = tmp {
                dot.change_dir(dir).step();
            }
        }
        Token::Reflector(kind) if dot.orientation() == Orientation::Horizontal => {
            match kind {
                MirrorKind::Forward => dot.change_dir(Direction::Right),
                MirrorKind::Backward => dot.change_dir(Direction::Left),
            };
            dot.step();
        }
        Token::Duplicate => {
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
        Token::StopProgram => {
            dot.change_state(DotState::EndProgram);
        }
        _ => {
            dot.change_state(DotState::Dead);
        }
    };
    Ok(())
}
