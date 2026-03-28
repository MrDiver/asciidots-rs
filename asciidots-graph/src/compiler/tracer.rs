use asciidots_core::{
    interpreter::AsciiField,
    native::NativeFunction,
    parser::{AsciiCell, AsciiFunc, DotsModule},
    primitives::{Direction, Position},
    state_machine::DotMachine,
};
use rustc_hash::FxHashMap;

use crate::ir::{
    dot::GraphDot,
    graph::{Edge, Node, NodeType, SetKind},
    selector::AddrValue,
};

pub struct Tracer {
    field: AsciiField,
    functions: FxHashMap<String, Position>,
    native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>>,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

pub struct TracingDot {
    pos: Position,
    direction: Direction,
}

impl TracingDot {
    pub fn new(pos: Position, direction: Direction) -> Self {
        Self { pos, direction }
    }
}

impl Tracer {
    pub fn new(
        field: DotsModule,
        native_registry: FxHashMap<String, Box<dyn NativeFunction<GraphDot>>>,
    ) -> Self {
        let (asciifield, funcvec) = field.unpack();
        Self {
            field: asciifield,
            functions: funcvec
                .iter()
                .map(|f| (f.name().to_string(), Position::from(*f.position())))
                .collect(),
            native_registry,
            nodes: Vec::default(),
            edges: Vec::default(),
        }
    }

    fn initial_dots(&self) -> Vec<TracingDot> {
        let mut initial_dots = Vec::new();
        let width = self.field.width();
        let height = self.field.height();
        for y in 0..height {
            for x in 0..width {
                let pos: Position = (x, y).into();
                let cell = self.field.at(&pos);
                match cell {
                    AsciiCell::Dot => {
                        for dir in [
                            Direction::Up,
                            Direction::Right,
                            Direction::Down,
                            Direction::Left,
                        ] {
                            match self.field.at(&(pos + dir.into())) {
                                c if c.can_initialize(&dir.orientation()) => {
                                    initial_dots.push(TracingDot {
                                        pos,
                                        direction: dir,
                                    });
                                    break;
                                }
                                _ => {}
                            }
                        }
                        initial_dots.push(TracingDot {
                            pos,
                            direction: Direction::Up,
                        })
                    }
                    _ => {}
                }
            }
        }
        initial_dots
    }

    pub fn trace(&mut self) {
        let width = self.field.width();
        let height = self.field.height();
        let dots = self.initial_dots();
    }
}
